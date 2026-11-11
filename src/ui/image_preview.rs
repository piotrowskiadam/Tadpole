//! Lazy thumbnails: only mapped cells fetch images, with bounded concurrency and a small cache.
use super::detail_table::wrapped_label;
use gtk::{gdk_pixbuf::prelude::*, prelude::*};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
    sync::LazyLock,
    time::Duration,
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
type Preview = (Vec<u8>, i32, i32, usize);
static SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
static CACHE: LazyLock<parking_lot::Mutex<VecDeque<(String, Preview)>>> =
    LazyLock::new(Default::default);
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("Tadpole/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("preview HTTP client")
});

async fn load(url: String, local_path: Option<String>) -> Result<Preview, String> {
    let key = format!("{}|{}", url, local_path.as_deref().unwrap_or(""));
    let _permit = SLOTS.acquire().await.map_err(|_| "Preview unavailable")?;
    if let Some((_, preview)) = CACHE.lock().iter().find(|(cached, _)| *cached == key) {
        return Ok(preview.clone());
    }
    let local = if let Some(path) = local_path {
        if tokio::fs::metadata(&path)
            .await
            .is_ok_and(|meta| meta.len() <= MAX_BYTES as u64)
        {
            tokio::fs::read(path)
                .await
                .ok()
                .filter(|bytes| bytes.len() <= MAX_BYTES)
        } else {
            None
        }
    } else {
        None
    };
    let bytes = if let Some(bytes) = local {
        bytes
    } else {
        if !url::Url::parse(&url).is_ok_and(|u| matches!(u.scheme(), "http" | "https")) {
            return Err("Preview needs an HTTP(S) image URL".into());
        }
        let response = CLIENT
            .get(&url)
            .send()
            .await
            .map_err(|_| "Image request failed")?
            .error_for_status()
            .map_err(|_| "Image returned an HTTP error")?;
        crate::http::read_body(response, MAX_BYTES).await?
    };
    let preview = tokio::task::spawn_blocking(move || decode(&bytes))
        .await
        .map_err(|_| "Image decoder failed")??;
    let mut cache = CACHE.lock();
    if cache.len() >= 64 {
        cache.pop_front();
    }
    cache.push_back((key, preview.clone()));
    Ok(preview)
}

fn decode(bytes: &[u8]) -> Result<Preview, String> {
    let loader = gtk::gdk_pixbuf::PixbufLoader::new();
    let dimensions = Rc::new(Cell::new((0, 0)));
    let size = dimensions.clone();
    loader.connect_size_prepared(move |loader, width, height| {
        size.set((width, height));
        let scale = (240. / width.max(1) as f64)
            .min(144. / height.max(1) as f64)
            .min(1.);
        loader.set_size(
            (width as f64 * scale).max(1.) as i32,
            (height as f64 * scale).max(1.) as i32,
        );
    });
    let written = loader.write(bytes);
    let closed = loader.close();
    written
        .and(closed)
        .map_err(|_| "Unsupported or invalid image")?;
    let pixbuf = loader.pixbuf().ok_or("Image could not be decoded")?;
    let png = pixbuf
        .save_to_bufferv("png", &[])
        .map_err(|_| "Thumbnail could not be created")?;
    let (width, height) = dimensions.get();
    Ok((png, width, height, bytes.len()))
}

pub fn widget(url: Option<String>, local_path: Option<String>, height: i32) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let picture = gtk::Picture::builder()
        .can_shrink(true)
        .content_fit(gtk::ContentFit::Contain)
        .height_request(height)
        .build();
    picture.set_alternative_text(Some("Image preview"));
    // Social cards keep the thumbnail's natural size rather than stretching it
    // according to the amount of text in each neighboring card.
    if height > 72 {
        picture.set_halign(gtk::Align::Center);
        picture.set_valign(gtk::Align::Start);
    }
    let status = wrapped_label(if url.is_some() {
        "Loading preview…"
    } else {
        "No image declared"
    });
    status.add_css_class("caption");
    outer.append(&picture);
    outer.append(&status);
    let Some(url) = url else {
        return outer;
    };
    let loaded = Rc::new(Cell::new(false));
    let pending: Rc<RefCell<Option<tokio::task::AbortHandle>>> = Rc::new(RefCell::new(None));
    let pending_unmap = pending.clone();
    outer.connect_unmap(move |_| {
        if let Some(task) = pending_unmap.borrow_mut().take() {
            task.abort();
        }
    });
    let picture = picture.downgrade();
    let status = status.downgrade();
    outer.connect_map(move |_| {
        if loaded.get() || pending.borrow().is_some() {
            return;
        }
        let (Some(picture), Some(status)) = (picture.upgrade(), status.upgrade()) else {
            return;
        };
        status.set_text("Loading preview…");
        let task = tokio::spawn(load(url.clone(), local_path.clone()));
        let handle = task.abort_handle();
        *pending.borrow_mut() = Some(handle.clone());
        let pending = pending.clone();
        let loaded = loaded.clone();
        let picture = picture.downgrade();
        let status = status.downgrade();
        glib::MainContext::default().spawn_local(async move {
            let result = task.await;
            // A recycled/unmapped cell may already have started a newer request.
            if pending
                .borrow()
                .as_ref()
                .is_none_or(|current| current.id() != handle.id())
            {
                return;
            }
            pending.borrow_mut().take();
            let (Some(picture), Some(status)) = (picture.upgrade(), status.upgrade()) else {
                return;
            };
            match result {
                Ok(Ok((png, width, height, bytes))) => {
                    match gdk::Texture::from_bytes(&glib::Bytes::from_owned(png)) {
                        Ok(texture) => {
                            picture.set_paintable(Some(&texture));
                            status.set_text(&format!(
                                "{width} × {height} px\n{:.1} KiB",
                                bytes as f64 / 1024.
                            ));
                            loaded.set(true);
                        }
                        Err(_) => status.set_text("Preview unavailable"),
                    }
                }
                Ok(Err(error)) => {
                    status.set_text(&error);
                    loaded.set(true);
                }
                Err(_) => status.set_text("Preview interrupted"),
            }
        });
    });
    outer
}

#[cfg(test)]
mod tests {
    #[test]
    fn thumbnails_keep_measured_resolution_and_bound_rendered_size() {
        let original = include_bytes!("../../tadpolelogonobg.png");
        let (png, width, height, bytes) = super::decode(original).unwrap();
        assert_eq!((width, height), (1177, 1177));
        assert_eq!(bytes, original.len());
        let (_, thumb_width, thumb_height, _) = super::decode(&png).unwrap();
        assert!(thumb_width <= 240 && thumb_height <= 144);
        assert!(thumb_width > 0 && thumb_height > 0);
        assert_eq!(thumb_width, thumb_height);
    }

    #[test]
    fn rejects_invalid_image_data() {
        assert!(super::decode(b"not an image").is_err());
    }
}

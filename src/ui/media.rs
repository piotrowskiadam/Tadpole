//! Opt-in, offline marketing capture. Never captures the desktop or other apps.
use crate::state::CrawlResult;
use adw::prelude::*;
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) fn demo_results() -> Vec<CrawlResult> {
    let sections = ["guides", "journal", "products", "resources"];
    (0..48).map(|i| {
        let section = sections[i % sections.len()];
        let url = if i == 0 { "https://demo.example/".into() } else { format!("https://demo.example/{section}/page-{i:02}") };
        let mut result = CrawlResult::new_failed(url.clone(), Some(200), None, "Indexable".into(), if i == 0 {0} else {1 + i % 4}, 80 + i as u32 * 7);
        result.indexable = true;
        result.title = Some(if i == 0 { "Fieldnotes — Ideas for a better website".into() } else { format!("{} | Fieldnotes", ["Planning your website", "Writing useful content", "Building better navigation", "A practical SEO checklist"][i % 4]) });
        result.h1 = Some("Build a website people can explore".into());
        result.h1_count = 1;
        result.meta_desc = (i % 7 != 0).then(|| "Practical guides to planning, writing and improving a useful website. Explore our latest ideas and resources.".into());
        result.canonical = Some(url);
        result.word_count = 420 + i * 31;
        result.size_bytes = 18000 + i * 400;
        let next = (i + 1) % 48;
        let next_url = if next == 0 { "https://demo.example/".into() } else { format!("https://demo.example/{}/page-{next:02}", sections[next % 4]) };
        result.outlinks = vec!["https://demo.example/".into(), next_url];
        if i % 13 == 0 && i != 0 { result.status_code = Some(404); result.indexable = false; result.indexability_status = "Client error".into(); }
        result
    }).collect()
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Snapshot the application's own GTK render tree, with its native renderer.
fn capture(window: &gtk::Window, path: &Path) -> Result<(), String> {
    if !window.is_mapped() {
        return Err("Window is not ready; try again after it appears.".into());
    }
    let width = window.width();
    let height = window.height();
    if width <= 0 || height <= 0 {
        return Err("Window has no drawable area.".into());
    }
    let paintable = gtk::WidgetPaintable::new(Some(window));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, width as f64, height as f64);
    let node = snapshot.to_node().ok_or("Window has not rendered yet.")?;
    let renderer = window.renderer().ok_or("No native renderer available.")?;
    let texture = renderer.render_texture(
        &node,
        Some(&gtk::graphene::Rect::new(
            0.,
            0.,
            width as f32,
            height as f32,
        )),
    );
    texture.save_to_png(path).map_err(|e| e.to_string())
}

pub fn run() {
    let app = adw::Application::builder()
        .application_id("com.tadpole.media")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(|app| {
        super::theme::install();
        let main = super::window::MainWindow::new(app);
        main.load_media_demo();
        main.window().set_default_size(1440, 900);
        main.present();
        let target: gtk::Window = main.window().clone().upcast();
        let folder = std::env::var_os("TADPOLE_MEDIA_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("tadpole-media"));
        let folder = if folder.is_absolute() { folder } else { std::env::current_dir().unwrap_or_default().join(folder) };
        let controls = adw::ApplicationWindow::builder().application(app).title("Tadpole media tools").default_width(460).default_height(260).build();
        let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
        body.set_margin_top(20); body.set_margin_bottom(20); body.set_margin_start(20); body.set_margin_end(20);
        let intro = gtk::Label::new(Some("Capture demo windows as PNG images or a short frame sequence. Recording stops after 60 seconds. Convert frames to GIF or MP4 with scripts/media-export.sh."));
        intro.set_wrap(true);
        intro.set_xalign(0.);
        body.append(&intro);
        let targets = gtk::DropDown::from_strings(&["Crawl window", "Site visualizer"]);
        targets.update_property(&[gtk::accessible::Property::Label("Window to capture")]);
        body.append(&targets);
        let screenshot = gtk::Button::with_label("Save screenshot");
        let record = gtk::Button::with_label("Start recording (2 fps)");
        let status = gtk::Label::new(Some(&format!("Output: {}", folder.display())));
        status.set_wrap(true);
        status.set_selectable(true);
        status.set_xalign(0.);
        body.append(&screenshot);
        body.append(&record);
        body.append(&status);
        controls.set_content(Some(&body));
        let choose: Rc<dyn Fn() -> Option<gtk::Window>> = Rc::new({ let targets = targets.clone(); let target = target.clone(); move || {
            if targets.selected() == 0 { Some(target.clone()) } else {
                gtk::Window::list_toplevels().into_iter().filter_map(|w| w.downcast::<gtk::Window>().ok()).find(|w| w.is_visible() && w.transient_for().as_ref() == Some(&target) && w.title().is_some_and(|s| s.contains("visualizations")))
            }
        }});
        screenshot.connect_clicked({ let choose = choose.clone(); let folder = folder.clone(); let status = status.clone(); move |_| {
            let path = folder.join(format!("screenshot-{}.png", stamp()));
            let result = std::fs::create_dir_all(&folder).map_err(|e| e.to_string()).and_then(|_| choose().ok_or("Open the site visualizer first.".into())).and_then(|window| capture(&window, &path));
            status.set_text(&match result { Ok(()) => format!("Saved {}", path.display()), Err(e) => format!("Capture failed: {e}") });
        }});
        let recording = Rc::new(Cell::new(false));
        record.connect_clicked({ let recording = recording.clone(); let folder = folder.clone(); let status = status.clone(); let controls = controls.downgrade(); move |button| {
            if recording.replace(!recording.get()) { button.set_sensitive(false); button.set_label("Stopping…"); return; }
            let Some(window) = choose() else { recording.set(false); status.set_text("Open the site visualizer first."); return; };
            let path = folder.join(format!("recording-{}", stamp()));
            if let Err(e) = std::fs::create_dir_all(&path) { recording.set(false); status.set_text(&format!("Cannot create recording folder: {e}")); return; }
            button.set_label("Stop recording");
            let frame = Rc::new(Cell::new(0));
            glib::timeout_add_local(Duration::from_millis(500), { let recording = recording.clone(); let button = button.clone(); let status = status.clone(); let controls = controls.clone(); move || {
                if !recording.get() || controls.upgrade().is_none() || frame.get() >= 120 {
                    recording.set(false); button.set_sensitive(true); button.set_label("Start recording (2 fps)"); status.set_text(&format!("Saved {} frames to {}", frame.get(), path.display())); return glib::ControlFlow::Break;
                }
                let file = path.join(format!("frame-{:04}.png", frame.get()));
                match capture(&window, &file) { Ok(()) => { frame.set(frame.get() + 1); status.set_text(&format!("Recording · {} frames · {}", frame.get(), path.display())); }, Err(e) => { recording.set(false); button.set_label("Start recording (2 fps)"); status.set_text(&format!("Recording stopped: {e}")); return glib::ControlFlow::Break; } }
                glib::ControlFlow::Continue
            }});
        }});
        controls.connect_close_request(move |_| { recording.set(false); glib::Propagation::Proceed });
        // Explicit automation for reproducible smoke checks; never enabled in normal startup.
        if std::env::var("TADPOLE_MEDIA_AUTOSHOT").as_deref() == Ok("1") {
            let app = app.clone();
            let tools_window: gtk::Window = controls.clone().upcast();
            glib::timeout_add_local_once(Duration::from_secs(3), move || {
                let result = std::fs::create_dir_all(&folder).map_err(|e| e.to_string()).and_then(|_| capture(&target, &folder.join("demo-crawl.png")));
                match result { Ok(()) => eprintln!("Saved demo-crawl.png"), Err(e) => eprintln!("Capture failed: {e}") }
                if let Err(e) = capture(&tools_window, &folder.join("media-tools.png")) { eprintln!("Tools capture failed: {e}"); }
                if std::env::var("TADPOLE_MEDIA_AUTORECORD").as_deref() == Ok("1") {
                    record.emit_clicked();
                    glib::timeout_add_local_once(Duration::from_secs(3), move || { record.emit_clicked(); app.quit(); });
                } else { app.quit(); }
            });
        }
        controls.present();
    });
    app.run();
}

#[cfg(test)]
mod tests {
    #[test]
    fn demo_links_stay_inside_the_fictional_dataset() {
        let pages = super::demo_results();
        let urls: std::collections::HashSet<_> = pages.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(urls.len(), 48);
        assert_eq!(
            pages.iter().filter(|p| p.status_code == Some(404)).count(),
            3
        );
        for page in &pages {
            assert!(page.url.starts_with("https://demo.example/"));
            for link in &page.outlinks {
                assert!(urls.contains(link.as_str()), "Missing demo target: {link}");
            }
        }
    }
}

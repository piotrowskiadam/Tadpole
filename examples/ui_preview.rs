//! Render only Tadpole's own widgets to PNG through GTK's Broadway backend.
//! No desktop capture or paid API calls. See docs/DEVELOPMENT.md.
use adw::prelude::*;
use std::time::Duration;

fn capture(window: &impl IsA<gtk::Window>, name: &str) {
    let window = window.as_ref();
    let snapshot = gtk::Snapshot::new();
    eprintln!(
        "Preview {:?}: {} x {}, mapped={}",
        window.title(),
        window.width(),
        window.height(),
        window.is_mapped()
    );
    window.snapshot_child(&window.child().expect("Window content"), &snapshot);
    let node = snapshot.to_node().expect("Window has no render node");
    let renderer = gtk::gsk::CairoRenderer::new();
    renderer
        .realize(window.surface().as_ref())
        .expect("Cannot realize preview renderer");
    let texture = renderer.render_texture(&node, None);
    let directory =
        std::env::var("TADPOLE_PREVIEW_DIR").unwrap_or_else(|_| "/tmp/tadpole-preview".into());
    std::fs::create_dir_all(&directory).unwrap();
    texture
        .save_to_png(std::path::Path::new(&directory).join(name))
        .unwrap();
    renderer.unrealize();
}

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();
    let app = adw::Application::builder()
        .application_id("com.tadpole.preview")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(|app| {
        tadpole::ui::theme::install();
        if std::env::var_os("TADPOLE_PREVIEW_INTERACTIVE").is_some() {
            let main = tadpole::ui::window::MainWindow::new(app);
            main.present();
            return;
        }
        let main = tadpole::ui::window::MainWindow::new(app);
        main.present();
        let window = main.window().clone();
        let app = app.clone();
        glib::MainContext::default().spawn_local(async move {
            glib::timeout_future(Duration::from_millis(500)).await;
            capture(&window, "crawl-desktop.png");
            window.set_default_size(1024, 720);
            glib::timeout_future(Duration::from_millis(300)).await;
            capture(&window, "crawl-compact.png");
            tadpole::ui::workbench::present(&window, tadpole::state::CrawlState::new(), None);
            glib::timeout_future(Duration::from_millis(400)).await;
            let studio = gtk::Window::list_toplevels()
                .into_iter()
                .filter_map(|w| w.downcast::<gtk::Window>().ok())
                .find(|w| w.title().is_some_and(|t| t.starts_with("SEO Studio")))
                .unwrap();
            capture(&studio, "studio-desktop.png");
            studio.set_default_size(640, 640);
            glib::timeout_future(Duration::from_millis(300)).await;
            capture(&studio, "studio-compact.png");
            app.quit();
        });
    });
    app.run();
}

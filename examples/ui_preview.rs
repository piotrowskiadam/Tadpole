//! Isolated native preview, without production data or paid API calls.
//! Use GDK_BACKEND=broadway and GSK_RENDERER=broadway with a connected
//! Broadway browser client, or run in a graphical test session.
use adw::prelude::*;

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();
    let app = adw::Application::builder()
        .application_id("com.tadpole.preview")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(|app| {
        tadpole::ui::theme::install();
        if let Ok(theme) = std::env::var("TADPOLE_PREVIEW_THEME") {
            adw::StyleManager::default().set_color_scheme(if theme == "dark" {
                adw::ColorScheme::ForceDark
            } else {
                adw::ColorScheme::ForceLight
            });
        } else if std::env::var_os("TADPOLE_PREVIEW_LIGHT").is_some() {
            adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        }
        let main = tadpole::ui::window::MainWindow::new(app);
        if let Some(width) = std::env::var("TADPOLE_PREVIEW_WIDTH")
            .ok()
            .and_then(|w| w.parse().ok())
        {
            let height = std::env::var("TADPOLE_PREVIEW_HEIGHT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(800);
            main.window().set_default_size(width, height);
        }
        main.present();
        if let Ok(seed) = std::env::var("TADPOLE_PREVIEW_URL") {
            main.window()
                .set_title(Some("Tadpole — live local fixture audit"));
            main.preview_discovery_crawl(&seed);
        }
    });
    app.run();
}

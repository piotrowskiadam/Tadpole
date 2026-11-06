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
        if std::env::var_os("TADPOLE_PREVIEW_LIGHT").is_some() {
            adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceLight);
        }
        let main = tadpole::ui::window::MainWindow::new(app);
        if let Some(width) = std::env::var("TADPOLE_PREVIEW_WIDTH")
            .ok()
            .and_then(|w| w.parse().ok())
        {
            main.window().set_default_size(width, 760);
        }
        main.present();
    });
    app.run();
}

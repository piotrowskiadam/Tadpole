mod state;
mod crawler;
mod ui;
mod markdown;

use adw::prelude::*;
use ui::window::MainWindow;

fn main() {
    // 1. Initialize Tokio multi-threaded runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    // 2. Enter Tokio runtime context so any tokio::spawn calls from GTK run inside this runtime
    let _guard = rt.enter();

    // 3. Write application icon to a temporary directory so GtkIconTheme can find it at runtime
    // even when running uninstalled (e.g. during development with cargo run or AppImage)
    let temp_icon_dir = std::env::temp_dir().join("tadpole-icons");
    if std::fs::create_dir_all(&temp_icon_dir).is_ok() {
        let icon_bytes = include_bytes!("../tadpolelogonobg.png");
        let icon_path = temp_icon_dir.join("com.tadpole.seo.png");
        let _ = std::fs::write(&icon_path, icon_bytes);
    }

    // 4. Initialize Libadwaita application
    let app = adw::Application::builder()
        .application_id("com.tadpole.seo")
        .build();

    app.connect_activate(|app| {
        // Load custom styling
        let provider = gtk::CssProvider::new();
        provider.load_from_string("
            .bold { font-weight: bold; }
            .dim-label { opacity: 0.65; }
            .error { color: #e01b24; font-weight: bold; }
            .warning { color: #e5a50a; }
            .success { color: #26a269; }
            .accent { color: #3584e4; }
            .status-bar { border-top: 1px solid rgba(0, 0, 0, 0.1); }
            .numeric { font-variant-numeric: tabular-nums; }
            .data-table { font-size: 13.5px; }
            progressbar.score-good > trough > progress { background: #26a269; }
            progressbar.score-warn > trough > progress { background: #e5a50a; }
            progressbar.score-poor > trough > progress { background: #e01b24; }
        ");
        
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            // Register the temporary icon directory search path
            let icon_theme = gtk::IconTheme::for_display(&display);
            let temp_icon_dir = std::env::temp_dir().join("tadpole-icons");
            icon_theme.add_search_path(&temp_icon_dir);
        }

        let main_window = MainWindow::new(app);
        main_window.present();
    });

    // 5. Run GTK application event loop (blocks until app exits)
    app.run();
}

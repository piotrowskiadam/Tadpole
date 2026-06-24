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

    // 3. Initialize Libadwaita application
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
        }

        let main_window = MainWindow::new(app);
        main_window.present();
    });

    // 4. Run GTK application event loop (blocks until app exits)
    app.run();
}

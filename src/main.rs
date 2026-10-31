#![windows_subsystem = "windows"]

use tadpole::ui;

use adw::prelude::*;
use ui::window::MainWindow;

fn main() {
    #[cfg(target_os = "windows")]
    {
        if let Ok(mut exe_path) = std::env::current_exe() {
            exe_path.pop();
            let loaders_cache = exe_path.join("lib/gdk-pixbuf-2.0/2.10.0/loaders.cache");
            if loaders_cache.exists() {
                unsafe {
                    std::env::set_var("GDK_PIXBUF_MODULE_FILE", loaders_cache);
                }
            }
        }
    }

    // 1. Initialize Tokio multi-threaded runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    // 2. Enter Tokio runtime context so any tokio::spawn calls from GTK run inside this runtime
    let _guard = rt.enter();

    // 4. Initialize Libadwaita application
    let app = adw::Application::builder()
        .application_id("com.tadpole.seo")
        .build();

    app.connect_activate(|app| {
        ui::theme::install();
        let main_window = MainWindow::new(app);
        main_window.present();
    });

    // 5. Run GTK application event loop (blocks until app exits)
    app.run();
}

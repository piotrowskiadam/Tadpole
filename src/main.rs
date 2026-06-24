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

    // 3. Write application icon and desktop launcher to user local directories so GNOME Shell can associate
    // the running window (app_id: com.tadpole.seo) with the desktop launcher and display "Tadpole" and its icon in the Dash.
    if let Ok(home) = std::env::var("HOME") {
        let icon_bytes = include_bytes!("../tadpolelogonobg.png");

        // Copy icon to user's local icons (standard theme path)
        let local_icons = std::path::PathBuf::from(&home).join(".local/share/icons/hicolor/256x256/apps");
        if std::fs::create_dir_all(&local_icons).is_ok() {
            if std::fs::write(local_icons.join("com.tadpole.seo.png"), icon_bytes).is_ok() {
                // Update icon theme cache so GNOME Shell picks up the new icon immediately
                let icon_theme_root = std::path::PathBuf::from(&home).join(".local/share/icons/hicolor");
                let _ = std::process::Command::new("gtk-update-icon-cache")
                    .arg("-f")
                    .arg("-t")
                    .arg(icon_theme_root)
                    .status();
            }
        }

        // Copy icon to user's local pixmaps (fallback search path)
        let local_pixmaps = std::path::PathBuf::from(&home).join(".local/share/pixmaps");
        if std::fs::create_dir_all(&local_pixmaps).is_ok() {
            let _ = std::fs::write(local_pixmaps.join("com.tadpole.seo.png"), icon_bytes);
        }

        // Create user local desktop entry
        let local_apps = std::path::PathBuf::from(&home).join(".local/share/applications");
        if std::fs::create_dir_all(&local_apps).is_ok() {
            let exec_path = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "tadpole".to_string());

            let desktop_content = format!(
                "[Desktop Entry]\n\
                 Name=Tadpole\n\
                 Comment=Local SEO crawler and auditor\n\
                 Exec=\"{}\"\n\
                 Icon=com.tadpole.seo\n\
                 Terminal=false\n\
                 Type=Application\n\
                 Categories=Development;WebDevelopment;\n\
                 StartupWMClass=com.tadpole.seo\n",
                 exec_path
            );
            if std::fs::write(local_apps.join("com.tadpole.seo.desktop"), desktop_content).is_ok() {
                // Update desktop database so the shell registers the new launcher immediately
                let _ = std::process::Command::new("update-desktop-database")
                    .arg(local_apps)
                    .status();
            }
        }
    }

    // Write to /tmp/tadpole-icons as fallback for theme path
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

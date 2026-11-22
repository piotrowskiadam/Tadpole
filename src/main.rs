#![windows_subsystem = "windows"]

use std::cell::RefCell;
use std::rc::Rc;

use tadpole::ui;

use adw::prelude::*;
use ui::window::MainWindow;

fn ensure_window(
    app: &adw::Application,
    slot: &Rc<RefCell<Option<Rc<MainWindow>>>>,
) -> Rc<MainWindow> {
    if let Some(existing) = slot.borrow().as_ref() {
        return existing.clone();
    }
    ui::theme::install();
    let main_window = Rc::new(MainWindow::new(app));
    let slot_on_destroy = slot.clone();
    main_window.window().connect_destroy(move |_| {
        *slot_on_destroy.borrow_mut() = None;
    });
    *slot.borrow_mut() = Some(main_window.clone());
    main_window
}

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

    if std::env::var("TADPOLE_MEDIA_MODE").as_deref() == Ok("1") {
        ui::media::run();
        return;
    }

    // 4. Initialize Libadwaita application
    let flatpak_id = if std::path::Path::new("/.flatpak-info").exists() {
        std::env::var("FLATPAK_ID").ok()
    } else {
        None
    };
    let app = adw::Application::builder()
        .application_id(tadpole::application_id(flatpak_id.as_deref()))
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    let primary_window: Rc<RefCell<Option<Rc<MainWindow>>>> = Rc::new(RefCell::new(None));

    let primary_window_activate = primary_window.clone();
    app.connect_activate(move |app| {
        let main_window = ensure_window(app, &primary_window_activate);
        main_window.present();
    });

    let primary_window_open = primary_window.clone();
    app.connect_open(move |app, files, _hint| {
        let main_window = ensure_window(app, &primary_window_open);
        main_window.present();
        let paths = files.iter().filter_map(|file| file.path());
        if let Some(path) = tadpole::project::resolve_open_project_path(paths) {
            main_window.open_project_path(path);
        } else {
            main_window.show_status("Unsupported file type. Open a .tadpole or .seocrawl project.");
        }
    });

    // 5. Run GTK application event loop (blocks until app exits)
    app.run();
}

use crate::crawler::{CrawlUpdate, Crawler};
use crate::state::{CrawlConfig, CrawlMode, CrawlState};
use crate::ui::details::Details;
use crate::ui::sidebar::{CrawlFilter, Sidebar};
use crate::ui::table::Table;
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MainWindow {
    window: adw::ApplicationWindow,
    state: CrawlState,

    // UI components
    sidebar: Rc<Sidebar>,
    table: Rc<Table>,
    details: Rc<Details>,
    summary_panel: Rc<crate::ui::summary::SummaryPanel>,
    link_opportunities: Rc<crate::ui::link_opportunities::LinkOpportunitiesPanel>,
    visualizer_button: gtk::Button,

    // Header bar controls
    address_entry: gtk::Entry,
    start_button: gtk::Button,
    pause_button: gtk::Button,
    progress_bar: gtk::ProgressBar,
    status_label: gtk::Label,

    mode_dropdown: gtk::DropDown,
    edit_list_button: gtk::Button,
    loaded_list_urls: Rc<RefCell<Vec<String>>>,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        // Limit to 500 URLs by default (Screaming Frog free limit style!)
        let state = CrawlState::new();

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Tadpole")
            .icon_name("com.tadpole.seo")
            .default_width(1200)
            .default_height(800)
            .build();

        // Main layout box (vertical: header bar, content, status bar)
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.set_content(Some(&main_box));

        // --- Header Bar ---
        let header_bar = adw::HeaderBar::new();
        main_box.append(&header_bar);

        // Logo in top-left — shown as small icon + app name
        let logo_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        logo_box.set_valign(gtk::Align::Center);

        // Embed logo at compile-time to guarantee it loads in all packaging formats (AppImage, Snap, Windows, etc.)
        let logo_bytes = include_bytes!("../../tadpolelogonobg.png");
        let gbytes = glib::Bytes::from(&logo_bytes[..]);
        if let Ok(texture) = gdk::Texture::from_bytes(&gbytes) {
            let logo_image = gtk::Image::builder()
                .pixel_size(30)
                .valign(gtk::Align::Center)
                .build();
            logo_image.set_paintable(Some(&texture));
            logo_box.append(&logo_image);
        }

        let app_name_label = gtk::Label::new(Some("Tadpole"));
        app_name_label.add_css_class("heading");
        app_name_label.set_valign(gtk::Align::Center);
        logo_box.append(&app_name_label);

        header_bar.pack_start(&logo_box);

        let open_button = gtk::Button::builder()
            .icon_name("document-open-symbolic")
            .tooltip_text("Open SEO Crawl Project")
            .build();
        header_bar.pack_start(&open_button);

        let save_button = gtk::Button::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Save SEO Crawl Project")
            .build();
        header_bar.pack_start(&save_button);

        let visualizer_button = gtk::Button::builder()
            .icon_name("network-workgroup-symbolic")
            .tooltip_text("Visual Site Map (Directory & 3D Constellation Diagrams)")
            .build();
        header_bar.pack_start(&visualizer_button);

        let pref_button = gtk::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Preferences")
            .build();
        header_bar.pack_end(&pref_button);

        let export_menu_button = gtk::MenuButton::builder()
            .icon_name("folder-download-symbolic")
            .tooltip_text("Export Options")
            .build();
        header_bar.pack_end(&export_menu_button);

        let export_popover = gtk::Popover::new();
        let popover_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        popover_box.set_margin_start(6);
        popover_box.set_margin_end(6);
        popover_box.set_margin_top(6);
        popover_box.set_margin_bottom(6);

        let csv_btn = gtk::Button::builder()
            .css_classes(vec!["flat".to_string()])
            .halign(gtk::Align::Fill)
            .build();
        let csv_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        csv_box.set_halign(gtk::Align::Start);
        let csv_bytes = include_bytes!("../../csv_icon.png");
        let csv_gbytes = glib::Bytes::from(&csv_bytes[..]);
        if let Ok(texture) = gdk::Texture::from_bytes(&csv_gbytes) {
            let csv_img = gtk::Image::builder()
                .pixel_size(16)
                .valign(gtk::Align::Center)
                .build();
            csv_img.set_paintable(Some(&texture));
            csv_box.append(&csv_img);
        }
        let csv_lbl = gtk::Label::new(Some("CSV"));
        csv_box.append(&csv_lbl);
        csv_btn.set_child(Some(&csv_box));

        let md_btn = gtk::Button::builder()
            .css_classes(vec!["flat".to_string()])
            .halign(gtk::Align::Fill)
            .build();
        let md_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        md_box.set_halign(gtk::Align::Start);
        let md_bytes = include_bytes!("../../markdown_icon.png");
        let md_gbytes = glib::Bytes::from(&md_bytes[..]);
        if let Ok(texture) = gdk::Texture::from_bytes(&md_gbytes) {
            let md_img = gtk::Image::builder()
                .pixel_size(16)
                .valign(gtk::Align::Center)
                .build();
            md_img.set_paintable(Some(&texture));
            md_box.append(&md_img);
        }
        let md_lbl = gtk::Label::new(Some("Markdown"));
        md_box.append(&md_lbl);
        md_btn.set_child(Some(&md_box));

        popover_box.append(&csv_btn);
        popover_box.append(&md_btn);
        export_popover.set_child(Some(&popover_box));
        export_menu_button.set_popover(Some(&export_popover));

        // Address entry in header bar
        let address_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        address_box.set_margin_top(6);
        address_box.set_margin_bottom(6);

        let mode_dropdown = gtk::DropDown::from_strings(&["Crawl", "List", "Path", "URL"]);
        mode_dropdown.set_valign(gtk::Align::Center);
        address_box.append(&mode_dropdown);

        let address_entry = gtk::Entry::builder()
            .placeholder_text(
                "Enter website URL (e.g., http://localhost:8080/ or https://example.com/)",
            )
            .hexpand(true)
            .width_chars(24)
            .build();
        address_box.append(&address_entry);

        let edit_list_button = gtk::Button::builder()
            .label("Edit List")
            .visible(false)
            .valign(gtk::Align::Center)
            .build();
        address_box.append(&edit_list_button);

        let start_button = gtk::Button::builder()
            .label("Start")
            .css_classes(vec!["suggested-action".to_string()])
            .build();
        address_box.append(&start_button);

        let pause_button = gtk::Button::builder()
            .label("Pause")
            .sensitive(false)
            .build();
        address_box.append(&pause_button);

        address_box.set_margin_start(12);
        address_box.set_margin_end(12);
        main_box.append(&address_box);
        address_entry.update_property(&[gtk::accessible::Property::Label("Website URL")]);
        mode_dropdown.update_property(&[gtk::accessible::Property::Label("Crawl mode")]);

        // Progress bar at the top of content area
        let progress_bar = gtk::ProgressBar::new();
        progress_bar.set_fraction(0.0);
        progress_bar.set_visible(false);
        main_box.append(&progress_bar);

        // --- Content Area (Paned layout) ---
        // Left: Sidebar
        // Right: Vertical Paned (Top: Table + SearchEntry, Bottom: Details)
        let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        content_box.set_vexpand(true);
        content_box.set_hexpand(true);
        main_box.append(&content_box);

        let main_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        main_paned.set_position(260);
        main_paned.set_wide_handle(true);
        main_paned.set_vexpand(true);
        main_paned.set_hexpand(true);
        content_box.append(&main_paned);

        let table = Rc::new(Table::new());
        let studio_button = gtk::Button::with_label("SEO Studio");
        studio_button.set_tooltip_text(Some("Research Google results and generate content briefs"));
        header_bar.pack_start(&studio_button);
        let state_studio = state.clone();
        let table_studio = table.clone();
        let parent_studio = window.clone();
        studio_button.connect_clicked(move |_| {
            crate::ui::workbench::present(
                &parent_studio,
                state_studio.clone(),
                table_studio.get_selected_url(),
            );
        });

        // Active filter banner shown when a sidebar filter is engaged
        let filter_banner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        filter_banner.add_css_class("card");
        filter_banner.set_margin_start(10);
        filter_banner.set_margin_end(10);
        filter_banner.set_margin_top(0);
        filter_banner.set_margin_bottom(2);
        filter_banner.set_visible(false);

        let filter_icon = gtk::Image::from_icon_name("view-filter-symbolic");
        filter_icon.add_css_class("dim-label");
        filter_icon.set_margin_start(8);
        filter_banner.append(&filter_icon);

        let filter_label = gtk::Label::new(None);
        filter_label.set_halign(gtk::Align::Start);
        filter_label.set_hexpand(true);
        filter_banner.append(&filter_label);

        let filter_copy_btn = gtk::Button::builder()
            .label("Copy for Sheets")
            .icon_name("edit-copy-symbolic")
            .has_frame(false)
            .tooltip_text(
                "Copy filtered rows as TSV to paste directly into Google Sheets or Excel (Ctrl+V)",
            )
            .valign(gtk::Align::Center)
            .build();
        filter_banner.append(&filter_copy_btn);

        let filter_export_btn = gtk::Button::builder()
            .label("Export CSV")
            .icon_name("document-save-symbolic")
            .has_frame(false)
            .tooltip_text("Save filtered rows to a CSV spreadsheet file")
            .valign(gtk::Align::Center)
            .build();
        filter_banner.append(&filter_export_btn);

        let clear_filter_btn = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .has_frame(false)
            .tooltip_text("Clear filter (Show all URLs)")
            .valign(gtk::Align::Center)
            .margin_end(6)
            .build();
        filter_banner.append(&clear_filter_btn);

        let filter_label_clone = filter_label.clone();
        let filter_banner_clone = filter_banner.clone();
        let sidebar = Rc::new(Sidebar::new({
            let table = table.clone();
            move |filter| {
                table.set_filter(filter);
                if filter == CrawlFilter::All {
                    filter_banner_clone.set_visible(false);
                } else {
                    filter_label_clone.set_markup(&format!(
                        "Filtered by: <b>{}</b>",
                        glib::markup_escape_text(filter.name())
                    ));
                    filter_banner_clone.set_visible(true);
                }
            }
        }));

        let sidebar_for_clear = sidebar.clone();
        clear_filter_btn.connect_clicked(move |_| {
            sidebar_for_clear.select_all();
        });

        main_paned.set_start_child(Some(sidebar.widget()));

        let right_paned = gtk::Paned::new(gtk::Orientation::Vertical);
        right_paned.set_position(500);
        right_paned.set_wide_handle(true);
        main_paned.set_end_child(Some(&right_paned));

        // Box for table and search entry above it
        let table_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        table_box.set_vexpand(true);
        table_box.set_hexpand(true);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search by URL, title, meta description, or status...")
            .margin_start(10)
            .margin_end(10)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        table_box.append(&search_entry);
        table_box.append(&filter_banner);
        table_box.append(table.widget());

        let link_opportunities = Rc::new(
            crate::ui::link_opportunities::LinkOpportunitiesPanel::new(state.clone()),
        );

        // Main notebook for crawl results / linking opportunities
        let main_notebook = gtk::Notebook::new();
        main_notebook.set_vexpand(true);
        main_notebook.set_hexpand(true);

        main_notebook.append_page(&table_box, Some(&gtk::Label::new(Some("Crawl Results"))));
        main_notebook.append_page(
            link_opportunities.widget(),
            Some(&gtk::Label::new(Some("Link Opportunities (beta)"))),
        );

        right_paned.set_start_child(Some(&main_notebook));

        let details = Rc::new(Details::new());
        right_paned.set_end_child(Some(details.widget()));

        let sep = gtk::Separator::new(gtk::Orientation::Vertical);
        content_box.append(&sep);

        let summary_panel = Rc::new(crate::ui::summary::SummaryPanel::new());
        content_box.append(summary_panel.widget());
        for (label, widget) in [
            ("Filters", sidebar.widget().clone().upcast::<gtk::Widget>()),
            (
                "Summary",
                summary_panel.widget().clone().upcast::<gtk::Widget>(),
            ),
        ] {
            let toggle = gtk::ToggleButton::with_label(label);
            toggle.set_active(true);
            toggle.set_tooltip_text(Some(&format!("Show or hide {label}")));
            let condition = if label == "Summary" {
                "max-width: 1100px"
            } else {
                "max-width: 850px"
            };
            let breakpoint =
                adw::Breakpoint::new(adw::BreakpointCondition::parse(condition).unwrap());
            breakpoint.add_setter(&toggle, "active", Some(&false.to_value()));
            window.add_breakpoint(breakpoint);
            toggle.connect_toggled(move |button| widget.set_visible(button.is_active()));
            header_bar.pack_end(&toggle);
        }

        // --- Status Bar ---
        let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        status_bar.add_css_class("status-bar");
        status_bar.set_margin_start(15);
        status_bar.set_margin_end(15);
        status_bar.set_margin_top(8);
        status_bar.set_margin_bottom(8);
        main_box.append(&status_bar);

        let status_label = gtk::Label::new(Some(
            "Enter a website URL and choose Start, or open SEO Studio to research a topic.",
        ));
        status_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        status_label.set_hexpand(true);
        status_label.set_xalign(0.0);
        status_label.add_css_class("dim-label");
        status_bar.append(&status_label);

        let loaded_list_urls = Rc::new(RefCell::new(Vec::new()));

        let main_window = Self {
            window,
            state,
            sidebar,
            table,
            details,
            summary_panel,
            link_opportunities,
            visualizer_button,
            address_entry,
            start_button,
            pause_button,
            progress_bar,
            status_label,
            mode_dropdown,
            edit_list_button,
            loaded_list_urls,
        };

        let config = main_window.state.get_config();
        let selected_idx = match config.crawl_mode {
            CrawlMode::Crawl => 0,
            CrawlMode::List => 1,
            CrawlMode::Path => 2,
            CrawlMode::Url => 3,
        };
        main_window.mode_dropdown.set_selected(selected_idx);
        let is_list = config.crawl_mode == CrawlMode::List;
        main_window.address_entry.set_visible(!is_list);
        main_window.edit_list_button.set_visible(is_list);

        main_window.setup_events(
            open_button,
            save_button,
            pref_button,
            csv_btn,
            md_btn,
            export_popover,
            search_entry,
            filter_copy_btn,
            filter_export_btn,
        );

        main_window
    }

    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.window
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn setup_events(
        &self,
        open_button: gtk::Button,
        save_button: gtk::Button,
        pref_button: gtk::Button,
        csv_btn: gtk::Button,
        md_btn: gtk::Button,
        export_popover: gtk::Popover,
        search_entry: gtk::SearchEntry,
        filter_copy_btn: gtk::Button,
        filter_export_btn: gtk::Button,
    ) {
        let state = self.state.clone();
        let table = self.table.clone();
        let details = self.details.clone();
        let sidebar = self.sidebar.clone();
        let address_entry = self.address_entry.clone();
        let start_button = self.start_button.clone();
        let pause_button = self.pause_button.clone();
        let progress_bar = self.progress_bar.clone();
        let status_label = self.status_label.clone();
        let main_window_widget = self.window.clone();

        let summary_panel = self.summary_panel.clone();
        let mode_dropdown = self.mode_dropdown.clone();
        let edit_list_button = self.edit_list_button.clone();
        let loaded_list_urls = self.loaded_list_urls.clone();
        let link_opps = self.link_opportunities.clone();
        let vis_button = self.visualizer_button.clone();

        // Create thread-safe Tokio channel
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<CrawlUpdate>();

        // Handle selected item changed in the main table
        let details_clone = details.clone();
        let state_clone = state.clone();
        table.connect_selection_changed(move |maybe_url| {
            if let Some(url) = maybe_url {
                details_clone.update(&url, &state_clone);
            } else {
                details_clone.clear();
            }
        });

        // Handle selected item changed in the link opportunities table
        let details_clone2 = details.clone();
        let state_clone2 = state.clone();
        link_opps.connect_selection_changed(move |maybe_url| {
            if let Some(url) = maybe_url {
                details_clone2.update(&url, &state_clone2);
            } else {
                details_clone2.clear();
            }
        });

        // Open Visualizer window on button click
        let state_vis = state.clone();
        let parent_win_vis = main_window_widget.clone();
        let details_vis = details.clone();
        vis_button.connect_clicked(move |_| {
            let vis_win = crate::ui::site_visualizer::VisualizerWindow::new(
                &parent_win_vis,
                state_vis.clone(),
            );
            let details_inner = details_vis.clone();
            let state_inner = state_vis.clone();
            vis_win.connect_selection_changed(move |maybe_url: Option<String>| {
                if let Some(url) = maybe_url {
                    details_inner.update(&url, &state_inner);
                }
            });
            vis_win.show();
        });

        // Handle navigation inside details panel
        let table_clone = table.clone();
        details.connect_url_clicked(move |url| {
            // Find and select the URL in the main table
            table_clone.select_url(&url);
        });

        // Handle mode selector dropdown change
        let address_entry_mode = address_entry.clone();
        let edit_list_button_mode = edit_list_button.clone();
        let state_mode = state.clone();
        mode_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let mode = match selected {
                0 => CrawlMode::Crawl,
                1 => CrawlMode::List,
                2 => CrawlMode::Path,
                3 => CrawlMode::Url,
                _ => CrawlMode::Crawl,
            };
            let is_list = mode == CrawlMode::List;
            address_entry_mode.set_visible(!is_list);
            edit_list_button_mode.set_visible(is_list);

            let mut config = state_mode.get_config();
            config.crawl_mode = mode;
            state_mode.set_config(config);
        });

        // Handle Edit List button action
        let parent_win_edit = main_window_widget.clone();
        let loaded_urls_edit = loaded_list_urls.clone();
        edit_list_button.connect_clicked(move |_| {
            let dialog = gtk::Window::builder()
                .title("Edit URL List")
                .transient_for(&parent_win_edit)
                .modal(true)
                .default_width(500)
                .default_height(400)
                .build();

            let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dialog.set_child(Some(&main_box));

            // Custom headerbar
            let header_bar = adw::HeaderBar::new();
            header_bar.set_show_start_title_buttons(false);
            header_bar.set_show_end_title_buttons(false);
            main_box.append(&header_bar);

            let cancel_btn = gtk::Button::builder().label("Cancel").build();
            header_bar.pack_start(&cancel_btn);

            let save_btn = gtk::Button::builder()
                .label("Save")
                .css_classes(vec!["suggested-action".to_string()])
                .build();
            header_bar.pack_end(&save_btn);

            let content_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
            content_box.set_margin_start(15);
            content_box.set_margin_end(15);
            content_box.set_margin_top(15);
            content_box.set_margin_bottom(15);
            content_box.set_vexpand(true);
            main_box.append(&content_box);

            let label = gtk::Label::new(Some("Enter one URL per line:"));
            label.set_halign(gtk::Align::Start);
            content_box.append(&label);

            let scrolled = gtk::ScrolledWindow::new();
            scrolled.set_vexpand(true);
            scrolled.set_hexpand(true);

            let text_view = gtk::TextView::new();
            text_view.set_monospace(true);
            scrolled.set_child(Some(&text_view));
            content_box.append(&scrolled);

            let btn_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let load_file_btn = gtk::Button::builder()
                .label("Load from File...")
                .icon_name("document-open-symbolic")
                .build();
            btn_box.append(&load_file_btn);
            content_box.append(&btn_box);

            let text_view_file = text_view.clone();
            let dialog_file = dialog.clone();
            load_file_btn.connect_clicked(move |_| {
                let file_dialog = gtk::FileDialog::new();
                file_dialog.set_title("Load URL List File");

                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Text Files"));
                filter.add_pattern("*.txt");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                file_dialog.set_filters(Some(&filters));

                let text_view_inner = text_view_file.clone();
                file_dialog.open(
                    Some(&dialog_file),
                    None::<&gio::Cancellable>,
                    move |result| {
                        if let Ok(file) = result {
                            if let Some(path) = file.path() {
                                if let Ok(content) = std::fs::read_to_string(path) {
                                    text_view_inner.buffer().set_text(&content);
                                }
                            }
                        }
                    },
                );
            });

            let dialog_cancel = dialog.clone();
            cancel_btn.connect_clicked(move |_| {
                dialog_cancel.destroy();
            });

            let dialog_save = dialog.clone();
            let loaded_urls_save = loaded_urls_edit.clone();
            let text_view_save = text_view.clone();
            save_btn.connect_clicked(move |_| {
                let buffer = text_view_save.buffer();
                let (start, end) = buffer.bounds();
                let text = buffer.text(&start, &end, false).to_string();

                let urls: Vec<String> = text
                    .lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                *loaded_urls_save.borrow_mut() = urls;
                dialog_save.destroy();
            });

            let current_urls = loaded_urls_edit.borrow().join("\n");
            text_view.buffer().set_text(&current_urls);

            dialog.present();
        });

        // Handle Search Bar filter changes
        let table_search = table.clone();
        search_entry.connect_search_changed(move |entry| {
            table_search.set_search_query(entry.text().as_str());
        });

        // Preferences dialog action
        let state_pref = state.clone();
        let parent_win_pref = main_window_widget.clone();
        pref_button.connect_clicked(move |_| {
            let config = state_pref.get_config();
            let pref_window = adw::PreferencesWindow::builder()
                .title("Preferences")
                .transient_for(&parent_win_pref)
                .modal(true)
                .default_width(550)
                .default_height(480)
                .build();

            let page_crawl = adw::PreferencesPage::builder()
                .title("Crawl Settings")
                .icon_name("preferences-system-symbolic")
                .build();

            // Limits Group
            let group_limits = adw::PreferencesGroup::builder()
                .title("Limits")
                .build();
            let max_urls_row = adw::SpinRow::builder()
                .title("Maximum URLs to crawl")
                .subtitle("Limits the overall crawl size")
                .build();
            max_urls_row.set_range(1.0, 100000.0);
            max_urls_row.set_value(config.max_urls as f64);
            group_limits.add(&max_urls_row);

            let concurrency_row = adw::SpinRow::builder()
                .title("Maximum concurrency")
                .subtitle("Number of simultaneous HTTP requests")
                .build();
            concurrency_row.set_range(1.0, 50.0);
            concurrency_row.set_value(config.max_concurrency as f64);
            group_limits.add(&concurrency_row);

            let max_depth_row = adw::SpinRow::builder()
                .title("Maximum crawl depth")
                .subtitle("0 means unlimited depth")
                .build();
            max_depth_row.set_range(0.0, 100.0);
            let current_depth = config.max_depth.unwrap_or(0) as f64;
            max_depth_row.set_value(current_depth);
            group_limits.add(&max_depth_row);

            page_crawl.add(&group_limits);

            // Behaviour Group
            let group_behavior = adw::PreferencesGroup::builder()
                .title("Crawl Behaviour")
                .build();

            let user_agent_row = adw::EntryRow::builder()
                .title("User-Agent Header")
                .text(&config.user_agent)
                .build();
            group_behavior.add(&user_agent_row);

            let respect_robots_row = adw::SwitchRow::builder()
                .title("Respect robots.txt")
                .active(config.respect_robots)
                .build();
            group_behavior.add(&respect_robots_row);

            let follow_redirects_row = adw::SwitchRow::builder()
                .title("Follow Redirects")
                .active(config.follow_redirects)
                .build();
            group_behavior.add(&follow_redirects_row);

            let js_rendering_row = adw::SwitchRow::builder()
                .title("JavaScript Rendering")
                .subtitle("Executes client-side scripts using headless Google Chrome")
                .active(config.js_rendering)
                .build();
            group_behavior.add(&js_rendering_row);

            let extract_microdata_row = adw::SwitchRow::builder()
                .title("Extract Microdata")
                .subtitle("Extracts inline HTML Microdata and converts it to JSON-LD")
                .active(config.extract_microdata)
                .build();
            group_behavior.add(&extract_microdata_row);

            let download_images_row = adw::SwitchRow::builder()
                .title("Download Image Files")
                .subtitle("Saves images locally to project folder")
                .active(config.download_images)
                .build();
            group_behavior.add(&download_images_row);
            page_crawl.add(&group_behavior);
            pref_window.add(&page_crawl);

            // Filters Page
            let page_filters = adw::PreferencesPage::builder()
                .title("Filters")
                .icon_name("filter-symbolic")
                .build();
            let group_filters = adw::PreferencesGroup::builder()
                .title("Include / Exclude Patterns")
                .build();
            let include_row = adw::EntryRow::builder()
                .title("Include URL Pattern (Regex)")
                .text(config.include_regex.as_deref().unwrap_or(""))
                .build();
            group_filters.add(&include_row);

            let exclude_row = adw::EntryRow::builder()
                .title("Exclude URL Pattern (Regex)")
                .text(config.exclude_regex.as_deref().unwrap_or(""))
                .build();
            group_filters.add(&exclude_row);
            page_filters.add(&group_filters);
            pref_window.add(&page_filters);

            // AI Settings Page
            let page_ai = adw::PreferencesPage::builder()
                .title("AI Assistant")
                .icon_name("avatar-default-symbolic")
                .build();
            let group_ai = adw::PreferencesGroup::builder()
                .title("AI Assistant API Keys")
                .build();

            let provider_row = adw::ComboRow::builder()
                .title("API Provider")
                .model(&gtk::StringList::new(&["OpenAI", "OpenRouter"]))
                .build();
            if config.ai_provider == "OpenRouter" {
                provider_row.set_selected(1);
            } else {
                provider_row.set_selected(0);
            }
            group_ai.add(&provider_row);

            let api_key_row = adw::PasswordEntryRow::builder()
                .title("API Key")
                .text(config.ai_api_key.as_deref().unwrap_or(""))
                .build();
            group_ai.add(&api_key_row);
            let dataforseo_group = adw::PreferencesGroup::builder().title("DataForSEO · Google SERP")
                .description("API credentials stay in memory and are never saved in crawl projects. Requests are made only from SEO Studio.").build();
            let dataforseo_login = adw::EntryRow::builder().title("DataForSEO API login").text(config.dataforseo_login.as_deref().unwrap_or("")).build();
            let dataforseo_password = adw::PasswordEntryRow::builder().title("DataForSEO API password").text(config.dataforseo_password.as_deref().unwrap_or("")).build();
            dataforseo_group.add(&dataforseo_login);
            dataforseo_group.add(&dataforseo_password);
            page_ai.add(&dataforseo_group);

            let openai_models = std::rc::Rc::new(vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
                "Custom...".to_string(),
            ]);

            let openrouter_models = std::rc::Rc::new(std::cell::RefCell::new(vec![
                "meta-llama/llama-3-8b-instruct:free".to_string(),
                "meta-llama/llama-3-70b-instruct:free".to_string(),
                "google/gemini-flash-1.5".to_string(),
                "google/gemini-pro-1.5".to_string(),
                "mistralai/mistral-7b-instruct:free".to_string(),
                "openai/gpt-4o".to_string(),
                "anthropic/claude-3.5-sonnet".to_string(),
                "Custom...".to_string(),
            ]));

            // A PropertyExpression telling GTK which field to match search text against.
            // Without this, enable_search shows the box but does no filtering.
            let search_expression = gtk::PropertyExpression::new(
                gtk::StringObject::static_type(),
                gtk::Expression::NONE,
                "string",
            );

            let model_dropdown = gtk::DropDown::builder()
                .enable_search(true)
                .expression(&search_expression)
                .valign(gtk::Align::Center)
                .build();

            let refresh_button = gtk::Button::builder()
                .icon_name("view-refresh-symbolic")
                .tooltip_text("Fetch latest models from OpenRouter")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();

            let suffix_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(6)
                .build();
            suffix_box.append(&model_dropdown);
            suffix_box.append(&refresh_button);

            let model_action_row = adw::ActionRow::builder()
                .title("Model")
                .build();
            model_action_row.add_suffix(&suffix_box);
            group_ai.add(&model_action_row);

            let custom_model_row = adw::EntryRow::builder()
                .title("Custom Model Name")
                .text("")
                .visible(false)
                .build();
            group_ai.add(&custom_model_row);

            // Connect dynamic model update helper
            let update_models_list = {
                let openai_models = openai_models.clone();
                let openrouter_models = openrouter_models.clone();
                let model_dropdown = model_dropdown.clone();
                let custom_model_row = custom_model_row.clone();

                move |provider_idx: u32, select_model: Option<&str>| {
                    let models_owner;
                    let models = if provider_idx == 1 {
                        models_owner = Some(openrouter_models.borrow().clone());
                        models_owner.as_ref().unwrap()
                    } else {
                        &*openai_models
                    };

                    let string_list = gtk::StringList::new(&models.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                    model_dropdown.set_model(Some(&string_list));
                    // Re-apply the expression after every model swap so filtering stays active.
                    model_dropdown.set_expression(Some(&gtk::PropertyExpression::new(
                        gtk::StringObject::static_type(),
                        gtk::Expression::NONE,
                        "string",
                    )));

                    let mut found_idx = None;
                    if let Some(target) = select_model {
                        for (i, m) in models.iter().enumerate() {
                            if m == target {
                                found_idx = Some(i as u32);
                                break;
                            }
                        }
                    }

                    if let Some(idx) = found_idx {
                        model_dropdown.set_selected(idx);
                        custom_model_row.set_visible(false);
                    } else {
                        let custom_idx = (models.len() - 1) as u32;
                        model_dropdown.set_selected(custom_idx);
                        custom_model_row.set_visible(true);
                        if let Some(target) = select_model {
                            if target != "Custom..." {
                                custom_model_row.set_text(target);
                            }
                        }
                    }
                }
            };
            let update_models_list = std::rc::Rc::new(update_models_list);

            // Handle provider selection change
            let update_models_list_provider = update_models_list.clone();
            let refresh_button_cloned = refresh_button.clone();

            // Handle model selection change to toggle custom input
            let provider_row_for_combo = provider_row.clone();
            let openai_models_for_combo = openai_models.clone();
            let openrouter_models_for_combo = openrouter_models.clone();
            let custom_model_row_for_combo = custom_model_row.clone();

            model_dropdown.connect_selected_notify(move |combo| {
                let provider_idx = provider_row_for_combo.selected();
                let models_owner;
                let models = if provider_idx == 1 {
                    models_owner = Some(openrouter_models_for_combo.borrow().clone());
                    models_owner.as_ref().unwrap()
                } else {
                    &*openai_models_for_combo
                };
                let selected = combo.selected();
                if selected != gtk::INVALID_LIST_POSITION && selected as usize == models.len() - 1 {
                    custom_model_row_for_combo.set_visible(true);
                } else {
                    custom_model_row_for_combo.set_visible(false);
                }
            });

            // Set up Auto-Fetch & Manual Fetch
            let fetched_openrouter_once = std::rc::Rc::new(std::cell::Cell::new(false));

            let trigger_auto_fetch = {
                let openrouter_models = openrouter_models.clone();
                let update_models_list = update_models_list.clone();
                let model_dropdown = model_dropdown.clone();
                let custom_model_row = custom_model_row.clone();
                let provider_row = provider_row.clone();
                let fetched_once = fetched_openrouter_once.clone();
                let refresh_btn = refresh_button.clone();

                move || {
                    if fetched_once.get() {
                        return;
                    }
                    fetched_once.set(true);

                    let openrouter_models = openrouter_models.clone();
                    let update_models_list = update_models_list.clone();
                    let model_dropdown = model_dropdown.clone();
                    let custom_model_row = custom_model_row.clone();
                    let provider_row = provider_row.clone();
                    let refresh_btn = refresh_btn.clone();

                    refresh_btn.set_sensitive(false);
                    glib::MainContext::default().spawn_local(async move {
                        if let Ok(mut fetched) = fetch_openrouter_models().await {
                            if !fetched.is_empty() {
                                fetched.push("Custom...".to_string());

                                let current_selected_model = {
                                    let selected = model_dropdown.selected() as usize;
                                    let provider_idx = provider_row.selected();
                                    let models = if provider_idx == 1 {
                                        openrouter_models.borrow().clone()
                                    } else {
                                        vec![]
                                    };
                                    if selected < models.len() {
                                        if selected == models.len() - 1 {
                                            custom_model_row.text().to_string()
                                        } else {
                                            models[selected].clone()
                                        }
                                    } else {
                                        "".to_string()
                                    }
                                };

                                *openrouter_models.borrow_mut() = fetched;
                                if provider_row.selected() == 1 {
                                    update_models_list(1, Some(&current_selected_model));
                                }
                            }
                        }
                        refresh_btn.set_sensitive(true);
                    });
                }
            };
            let trigger_auto_fetch = std::rc::Rc::new(trigger_auto_fetch);

            let trigger_auto_fetch_provider = trigger_auto_fetch.clone();
            provider_row.connect_selected_notify(move |row| {
                let is_openrouter = row.selected() == 1;
                refresh_button_cloned.set_visible(is_openrouter);
                update_models_list_provider(row.selected(), None);
                if is_openrouter {
                    trigger_auto_fetch_provider();
                }
            });

            let refresh_button_click = refresh_button.clone();
            let openrouter_models_refresh = openrouter_models.clone();
            let update_models_list_refresh = update_models_list.clone();
            let provider_row_refresh = provider_row.clone();
            let model_dropdown_refresh = model_dropdown.clone();
            let custom_model_row_refresh = custom_model_row.clone();

            refresh_button.connect_clicked(move |_| {
                let btn = refresh_button_click.clone();
                let openrouter_models = openrouter_models_refresh.clone();
                let update_models_list = update_models_list_refresh.clone();
                let provider_row = provider_row_refresh.clone();
                let model_dropdown = model_dropdown_refresh.clone();
                let custom_model_row = custom_model_row_refresh.clone();

                btn.set_sensitive(false);

                glib::MainContext::default().spawn_local(async move {
                    match fetch_openrouter_models().await {
                        Ok(mut fetched) => {
                            if !fetched.is_empty() {
                                fetched.push("Custom...".to_string());

                                let current_selected_model = {
                                    let selected = model_dropdown.selected() as usize;
                                    let provider_idx = provider_row.selected();
                                    let models = if provider_idx == 1 {
                                        openrouter_models.borrow().clone()
                                    } else {
                                        vec![]
                                    };
                                    if selected < models.len() {
                                        if selected == models.len() - 1 {
                                            custom_model_row.text().to_string()
                                        } else {
                                            models[selected].clone()
                                        }
                                    } else {
                                        "".to_string()
                                    }
                                };

                                *openrouter_models.borrow_mut() = fetched;
                                update_models_list(1, Some(&current_selected_model));
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to fetch OpenRouter models: {}", e);
                        }
                    }
                    btn.set_sensitive(true);
                });
            });

            // Initialize models lists
            let current_provider_idx = if config.ai_provider == "OpenRouter" { 1 } else { 0 };
            update_models_list(current_provider_idx, Some(&config.ai_model));
            refresh_button.set_visible(current_provider_idx == 1);
            if current_provider_idx == 1 {
                trigger_auto_fetch();
            }

            page_ai.add(&group_ai);
            pref_window.add(&page_ai);

            // Markdown Settings Page
            let page_markdown = adw::PreferencesPage::builder()
                .title("Markdown Exporter")
                .icon_name("document-send-symbolic")
                .build();

            let group_scraping = adw::PreferencesGroup::builder()
                .title("Scraping and Conversion")
                .build();

            let only_main_row = adw::SwitchRow::builder()
                .title("Main content only")
                .subtitle("Ignore header, footer, navigation and sidebars")
                .active(config.md_only_main_content)
                .build();
            group_scraping.add(&only_main_row);

            let ignored_selectors_row = adw::EntryRow::builder()
                .title("Ignored CSS selectors (e.g. .ads, #comments)")
                .text(&config.md_ignored_selectors)
                .build();
            group_scraping.add(&ignored_selectors_row);

            let keep_links_row = adw::SwitchRow::builder()
                .title("Preserve links in text")
                .subtitle("Convert anchors to markdown links [text](url)")
                .active(config.md_keep_links)
                .build();
            group_scraping.add(&keep_links_row);

            let ignore_images_row = adw::SwitchRow::builder()
                .title("Ignore images")
                .subtitle("Skip images in the generated markdown output")
                .active(config.md_ignore_images)
                .build();
            group_scraping.add(&ignore_images_row);

            let clean_whitespace_row = adw::SwitchRow::builder()
                .title("Clean redundant blank lines")
                .subtitle("Normalize multiple consecutive blank lines")
                .active(config.md_clean_whitespace)
                .build();
            group_scraping.add(&clean_whitespace_row);

            page_markdown.add(&group_scraping);
            pref_window.add(&page_markdown);

            // About Page
            let page_about = adw::PreferencesPage::builder()
                .title("About")
                .icon_name("help-about-symbolic")
                .build();

            let group_about = adw::PreferencesGroup::builder()
                .title("Tadpole")
                .description("Tadpole is a fast, native desktop SEO crawler and site auditing tool built in Rust and GTK4.")
                .build();

            // Version Row
            let version_row = adw::ActionRow::builder()
                .title("Version")
                .subtitle(env!("CARGO_PKG_VERSION"))
                .build();
            group_about.add(&version_row);

            // License Row
            let license_row = adw::ActionRow::builder()
                .title("License")
                .subtitle("MIT License")
                .build();
            group_about.add(&license_row);

            // GitHub Row
            let github_row = adw::ActionRow::builder()
                .title("Project Website")
                .subtitle("https://github.com/piotrowskiadam/Tadpole")
                .activatable(true)
                .build();

            let link_icon = gtk::Image::builder()
                .icon_name("external-link-symbolic")
                .valign(gtk::Align::Center)
                .build();
            github_row.add_suffix(&link_icon);

            let pref_window_for_about = pref_window.clone();
            github_row.connect_activated(move |_| {
                let launcher = gtk::UriLauncher::new("https://github.com/piotrowskiadam/Tadpole");
                launcher.launch(Some(&pref_window_for_about), None::<&gio::Cancellable>, |_| {});
            });
            group_about.add(&github_row);

            page_about.add(&group_about);
            pref_window.add(&page_about);

            // Save settings on close
            let state_save = state_pref.clone();
            let model_dropdown_save = model_dropdown.clone();
            let custom_model_row_save = custom_model_row.clone();
            let provider_row_save = provider_row.clone();
            let openai_models_save = openai_models.clone();
            let openrouter_models_save = openrouter_models.clone();

            pref_window.connect_close_request(move |_| {
                let include_text = include_row.text().to_string();
                let exclude_text = exclude_row.text().to_string();
                let api_key_text = api_key_row.text().to_string();
                let provider = if provider_row_save.selected() == 1 { "OpenRouter".to_string() } else { "OpenAI".to_string() };

                // Get model name
                let final_model = {
                    let selected = model_dropdown_save.selected() as usize;
                    let provider_idx = provider_row_save.selected();
                    let models = if provider_idx == 1 {
                        openrouter_models_save.borrow().clone()
                    } else {
                        openai_models_save.clone().to_vec()
                    };
                    if selected < models.len() {
                        if selected == models.len() - 1 {
                            custom_model_row_save.text().to_string()
                        } else {
                            models[selected].clone()
                        }
                    } else {
                        "".to_string()
                    }
                };

                let current_config = state_save.get_config();
                let max_depth_val = max_depth_row.value() as usize;
                let max_depth = if max_depth_val == 0 { None } else { Some(max_depth_val) };

                let md_only_main = only_main_row.is_active();
                let md_ignored_sel = ignored_selectors_row.text().to_string();
                let md_keep_links = keep_links_row.is_active();
                let md_ignore_img = ignore_images_row.is_active();
                let md_clean_ws = clean_whitespace_row.is_active();

                let new_config = CrawlConfig {
                    max_urls: max_urls_row.value() as usize,
                    max_concurrency: concurrency_row.value() as usize,
                    user_agent: user_agent_row.text().to_string(),
                    respect_robots: respect_robots_row.is_active(),
                    follow_redirects: follow_redirects_row.is_active(),
                    js_rendering: js_rendering_row.is_active(),
                    extract_microdata: extract_microdata_row.is_active(),
                    include_regex: if include_text.trim().is_empty() { None } else { Some(include_text) },
                    exclude_regex: if exclude_text.trim().is_empty() { None } else { Some(exclude_text) },
                    ai_provider: provider,
                    ai_api_key: if api_key_text.trim().is_empty() { None } else { Some(api_key_text) },
                    ai_model: final_model,
                    dataforseo_login: Some(dataforseo_login.text().to_string()).filter(|s| !s.trim().is_empty()),
                    dataforseo_password: Some(dataforseo_password.text().to_string()).filter(|s| !s.trim().is_empty()),
                    download_images: download_images_row.is_active(),
                    project_dir: current_config.project_dir, // Preserve
                    crawl_mode: current_config.crawl_mode, // Preserve
                    max_depth,
                    md_only_main_content: md_only_main,
                    md_ignored_selectors: md_ignored_sel,
                    md_keep_links,
                    md_ignore_images: md_ignore_img,
                    md_clean_whitespace: md_clean_ws,
                    md_output_dir: current_config.md_output_dir, // Preserve
                    md_auto_generate: current_config.md_auto_generate, // Preserve
                };
                state_save.set_config(new_config);

                glib::Propagation::Proceed
            });

            pref_window.present();
        });

        // Active filter banner action: Copy for Sheets (TSV to clipboard)
        let table_filter_copy = table.clone();
        let status_label_copy = status_label.clone();
        filter_copy_btn.connect_clicked(move |_| {
            let count = table_filter_copy.copy_to_clipboard_tsv();
            status_label_copy.set_text(&format!(
                "Copied {} filtered rows to clipboard (ready to paste directly into Google Sheets / Excel with Ctrl+V).",
                count
            ));
        });

        // Active filter banner action: Export filtered results to CSV
        let table_filter_export = table.clone();
        let parent_win_filter = main_window_widget.clone();
        let status_label_filter_export = status_label.clone();
        filter_export_btn.connect_clicked(move |_| {
            let active_filter = table_filter_export.get_active_filter();
            let initial_name = if active_filter != CrawlFilter::All {
                format!("tadpole_{}.csv", active_filter.slug())
            } else {
                "tadpole_crawl_results.csv".to_string()
            };
            let title = if active_filter != CrawlFilter::All {
                format!("Export {} to CSV", active_filter.name())
            } else {
                "Export crawl to CSV".to_string()
            };

            let dialog = gtk::FileDialog::new();
            dialog.set_title(&title);
            dialog.set_initial_name(Some(&initial_name));
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("CSV Files"));
            filter.add_pattern("*.csv");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let table_inner = table_filter_export.clone();
            let status_inner = status_label_filter_export.clone();
            dialog.save(
                Some(&parent_win_filter),
                None::<&gio::Cancellable>,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            if let Err(e) = table_inner.export_to_csv(&path) {
                                status_inner.set_text(&format!("Failed to export CSV: {}", e));
                            } else {
                                status_inner.set_text(&format!(
                                    "Successfully exported filtered CSV to {}",
                                    path.display()
                                ));
                            }
                        }
                    }
                },
            );
        });

        // Export CSV click from header bar menu
        let table_export = table.clone();
        let parent_win_export = main_window_widget.clone();
        let export_popover_csv = export_popover.clone();
        let status_label_csv = status_label.clone();
        csv_btn.connect_clicked(move |_| {
            export_popover_csv.popdown();
            let active_filter = table_export.get_active_filter();
            let initial_name = if active_filter != CrawlFilter::All {
                format!("tadpole_{}.csv", active_filter.slug())
            } else {
                "tadpole_crawl_results.csv".to_string()
            };
            let title = if active_filter != CrawlFilter::All {
                format!("Export {} to CSV", active_filter.name())
            } else {
                "Export crawl to CSV".to_string()
            };

            let dialog = gtk::FileDialog::new();
            dialog.set_title(&title);
            dialog.set_initial_name(Some(&initial_name));
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("CSV Files"));
            filter.add_pattern("*.csv");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let table_inner = table_export.clone();
            let status_inner = status_label_csv.clone();
            dialog.save(
                Some(&parent_win_export),
                None::<&gio::Cancellable>,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            if let Err(e) = table_inner.export_to_csv(&path) {
                                status_inner.set_text(&format!("Failed to export CSV: {}", e));
                            } else {
                                status_inner.set_text(&format!(
                                    "Successfully exported CSV to {}",
                                    path.display()
                                ));
                            }
                        }
                    }
                },
            );
        });

        // Export Markdown click
        let state_md = state.clone();
        let parent_win_md = main_window_widget.clone();
        let export_popover_md = export_popover.clone();
        let status_label_md = status_label.clone();
        md_btn.connect_clicked(move |_| {
            export_popover_md.popdown();
            let dialog = gtk::FileDialog::new();
            dialog.set_title("Select Export Directory for Markdown Files");

            let state_inner = state_md.clone();
            let status_label_inner = status_label_md.clone();
            dialog.select_folder(
                Some(&parent_win_md),
                None::<&gio::Cancellable>,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let state = state_inner.clone();
                            let status = status_label_inner.clone();
                            status.set_text("Exporting Markdown…");
                            glib::MainContext::default().spawn_local(async move {
                                let result = tokio::task::spawn_blocking(move || {
                                    let mut saved = 0;
                                    let mut errors = 0;
                                    for page in state.get_all_results() {
                                        if let Some(markdown) = page.markdown {
                                            let target = path
                                                .join(crate::project::markdown_filename(&page.url));
                                            if std::fs::write(target, markdown).is_ok() {
                                                saved += 1;
                                            } else {
                                                errors += 1;
                                            }
                                        }
                                    }
                                    (saved, errors)
                                })
                                .await;
                                match result {
                                    Ok((saved, errors)) => status.set_text(&format!(
                                        "Exported {saved} Markdown files ({errors} errors)."
                                    )),
                                    Err(e) => {
                                        status.set_text(&format!("Could not export Markdown: {e}"))
                                    }
                                }
                            });
                        }
                    }
                },
            );
        });

        // Save project click
        let state_save = state.clone();
        let parent_win_save = main_window_widget.clone();
        let address_entry_save = address_entry.clone();
        let loaded_list_urls_save = loaded_list_urls.clone();
        let status_save = status_label.clone();
        save_button.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::new();
            dialog.set_title("Save Crawl Project");
            dialog.set_initial_name(Some("project.seocrawl"));
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("SEO Crawl Files"));
            filter.add_pattern("*.seocrawl");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let state_inner = state_save.clone();
            let status_inner = status_save.clone();
            let address_inner = address_entry_save.clone();
            let loaded_list_urls_inner = loaded_list_urls_save.clone();
            dialog.save(
                Some(&parent_win_save),
                None::<&gio::Cancellable>,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            if let Some(parent) = path.parent() {
                                let mut config = state_inner.get_config();
                                config.project_dir = Some(parent.to_string_lossy().to_string());
                                state_inner.set_config(config);
                            }

                            let is_list = state_inner.get_config().crawl_mode == CrawlMode::List;
                            let seed_url = if is_list {
                                loaded_list_urls_inner.borrow().join("\n")
                            } else {
                                address_inner.text().to_string()
                            };
                            let state = state_inner.clone();
                            let status = status_inner.clone();
                            status.set_text("Saving project…");
                            glib::MainContext::default().spawn_local(async move {
                                let saved = tokio::task::spawn_blocking(move || {
                                    crate::project::save(&path, &state.get_project(seed_url))
                                })
                                .await;
                                match saved {
                                    Ok(Ok(())) => status
                                        .set_text("Project saved. API credentials were excluded."),
                                    Ok(Err(e)) => {
                                        status.set_text(&format!("Could not save project: {e}"))
                                    }
                                    Err(e) => {
                                        status.set_text(&format!("Could not save project: {e}"))
                                    }
                                }
                            });
                        }
                    }
                },
            );
        });

        // Open project click
        let state_open = state.clone();
        let parent_win_open = main_window_widget.clone();
        let address_entry_open = address_entry.clone();
        let table_open = table.clone();
        let sidebar_open = sidebar.clone();
        let summary_panel_open = summary_panel.clone();
        let mode_dropdown_open = mode_dropdown.clone();
        let edit_list_button_open = edit_list_button.clone();
        let loaded_list_urls_open = loaded_list_urls.clone();
        let link_opps_open = link_opps.clone();
        let status_label_open = status_label.clone();
        open_button.connect_clicked(move |_| {
            if state_open.is_crawling() { status_label_open.set_text("Stop the current crawl before opening a project."); return; }
            let dialog = gtk::FileDialog::new();
            dialog.set_title("Open Crawl Project");
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("SEO Crawl Files"));
            filter.add_pattern("*.seocrawl");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let state_inner = state_open.clone();
            let address_inner = address_entry_open.clone();
            let table_inner = table_open.clone();
            let sidebar_inner = sidebar_open.clone();
            let summary_panel_inner = summary_panel_open.clone();
            let mode_dropdown_inner = mode_dropdown_open.clone();
            let edit_list_button_inner = edit_list_button_open.clone();
            let loaded_list_urls_inner = loaded_list_urls_open.clone();
            let link_opps_inner = link_opps_open.clone();
            let status_label_inner = status_label_open.clone();

            dialog.open(Some(&parent_win_open), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        if state_inner.is_crawling() { status_label_inner.set_text("Stop the current crawl before opening a project."); return; }
                        status_label_inner.set_text("Loading saved crawl project...");

                        let state_bg = state_inner.clone();
                        let address_bg = address_inner.clone();
                        let table_bg = table_inner.clone();
                        let sidebar_bg = sidebar_inner.clone();
                        let summary_bg = summary_panel_inner.clone();
                        let mode_dropdown_bg = mode_dropdown_inner.clone();
                        let edit_list_bg = edit_list_button_inner.clone();
                        let loaded_list_ref = loaded_list_urls_inner.clone();
                        let link_opps_bg = link_opps_inner.clone();
                        let status_label_bg = status_label_inner.clone();

                        glib::MainContext::default().spawn_local(async move {
                            let project_opt = tokio::task::spawn_blocking(move || {
                                if let Ok(f) = std::fs::File::open(&path) {
                                    let project_res: Result<crate::state::CrawlProject, _> = serde_json::from_reader(f);
                                    project_res.ok()
                                } else {
                                    None
                                }
                            }).await.ok().flatten();

                            if state_bg.is_crawling() { status_label_bg.set_text("Project not loaded because a crawl is running. Stop it and open the file again."); return; }
                            if let Some(project) = project_opt {
                                if let Err(e) = project.config.validate() { status_label_bg.set_text(&e); return; }
                                address_bg.set_text(&project.seed_url);
                                state_bg.load_project(project.clone());

                                table_bg.clear();
                                link_opps_bg.clear();

                                let results = project.results.clone();
                                let total = results.len();

                                for (idx, res) in results.into_iter().enumerate() {
                                    table_bg.add_or_update(res);
                                    if idx % 50 == 0 {
                                        status_label_bg.set_text(&format!("Loading rows {} / {}", idx, total));
                                        glib::timeout_future(std::time::Duration::from_millis(1)).await;
                                    }
                                }

                                sidebar_bg.update_counts(&state_bg);
                                summary_bg.update(&state_bg);

                                let config = state_bg.get_config();
                                let selected_idx = match config.crawl_mode {
                                    CrawlMode::Crawl => 0,
                                    CrawlMode::List => 1,
                                    CrawlMode::Path => 2,
                                    CrawlMode::Url => 3,
                                };
                                mode_dropdown_bg.set_selected(selected_idx);
                                let is_list = config.crawl_mode == CrawlMode::List;
                                address_bg.set_visible(!is_list);
                                edit_list_bg.set_visible(is_list);
                                if is_list {
                                    let urls: Vec<String> = project.seed_url.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                                    *loaded_list_ref.borrow_mut() = urls;
                                }
                                status_label_bg.set_text(&format!("Loaded project with {} URLs.", total));
                            } else {
                                status_label_bg.set_text("Failed to open project file.");
                            }
                        });
                    }
                }
            });
        });

        // Tracking crawl velocity and samples for telemetry
        let crawl_start_time = Rc::new(RefCell::new(Option::<std::time::Instant>::None));
        let crawl_samples = Rc::new(RefCell::new(std::collections::VecDeque::<(
            std::time::Instant,
            usize,
        )>::new()));

        // Attach receiver to GLib event loop using spawn_local
        let start_button_clone = start_button.clone();
        let pause_button_clone = pause_button.clone();
        let progress_bar_clone = progress_bar.clone();
        let status_label_clone = status_label.clone();
        let table_clone = table.clone();
        let sidebar_clone = sidebar.clone();
        let state_clone = state.clone();
        let summary_panel_clone = summary_panel.clone();
        let crawl_start_time_clone = crawl_start_time.clone();
        let crawl_samples_clone = crawl_samples.clone();

        glib::MainContext::default().spawn_local(async move {
            let mut last_refresh = std::time::Instant::now();
            while let Some(update) = rx.recv().await {
                match update {
                    CrawlUpdate::Discovered(url) => {
                        status_label_clone.set_text(&format!("Discovered: {}", url));
                    }
                    CrawlUpdate::Crawled(res) => {
                        table_clone.add_or_update(*res);
                        if last_refresh.elapsed() >= std::time::Duration::from_millis(250) {
                            sidebar_clone.update_counts(&state_clone);
                            summary_panel_clone.update(&state_clone);
                            table_clone.set_filter(table_clone.get_active_filter());
                            last_refresh = std::time::Instant::now();
                            glib::timeout_future(std::time::Duration::from_millis(1)).await;
                        }

                        let stats = state_clone.get_stats();
                        let now = std::time::Instant::now();
                        {
                            let mut samples = crawl_samples_clone.borrow_mut();
                            samples.push_back((now, stats.crawled));
                            while let Some(&(t, _)) = samples.front() {
                                if now.duration_since(t).as_secs_f64() > 5.0 && samples.len() > 1 {
                                    samples.pop_front();
                                } else {
                                    break;
                                }
                            }
                        }

                        let speed_urls_per_sec = {
                            let samples = crawl_samples_clone.borrow();
                            if samples.len() >= 2 {
                                let (t_first, c_first) = samples.front().unwrap();
                                let (t_last, c_last) = samples.back().unwrap();
                                let dt = t_last.duration_since(*t_first).as_secs_f64();
                                let dc = c_last.saturating_sub(*c_first);
                                if dt > 0.4 {
                                    dc as f64 / dt
                                } else {
                                    0.0
                                }
                            } else if let Some(start_t) = *crawl_start_time_clone.borrow() {
                                let dt = now.duration_since(start_t).as_secs_f64();
                                if dt > 0.4 {
                                    stats.crawled as f64 / dt
                                } else {
                                    0.0
                                }
                            } else {
                                0.0
                            }
                        };

                        let limit = state_clone.get_limit();
                        let total_expected = stats.discovered.min(limit);
                        let queue_depth = total_expected.saturating_sub(stats.crawled);

                        let eta_str = if speed_urls_per_sec > 0.1 && queue_depth > 0 {
                            let eta_secs = (queue_depth as f64 / speed_urls_per_sec) as u64;
                            if eta_secs < 60 {
                                format!("~{}s", eta_secs)
                            } else if eta_secs < 3600 {
                                format!("~{}m {:02}s", eta_secs / 60, eta_secs % 60)
                            } else {
                                format!("~{}h {:02}m", eta_secs / 3600, (eta_secs % 3600) / 60)
                            }
                        } else if queue_depth == 0 {
                            "finishing...".to_string()
                        } else {
                            "—".to_string()
                        };

                        let rate_str = if speed_urls_per_sec > 0.05 {
                            format!("{:.1} URLs/s", speed_urls_per_sec)
                        } else {
                            "starting...".to_string()
                        };

                        status_label_clone.set_text(&format!(
                            "Crawled: {} / {} | {} | Queue: {} | ETA: {} | 2xx: {} | 3xx: {} | 4xx: {} | 5xx: {} | Err: {}",
                            stats.crawled, stats.discovered, rate_str, queue_depth, eta_str,
                            stats.status_2xx, stats.status_3xx, stats.status_4xx, stats.status_5xx, stats.errors
                        ));

                        let fraction = if stats.discovered > 0 {
                            (stats.crawled as f64) / (stats.discovered as f64).min(limit as f64)
                        } else {
                            0.0
                        };
                        progress_bar_clone.set_fraction(fraction.min(1.0));
                    }
                    CrawlUpdate::Finished => {
                        progress_bar_clone.set_visible(false);
                        start_button_clone.set_label("Start");
                        start_button_clone.set_sensitive(true);
                        start_button_clone.remove_css_class("destructive-action");
                        pause_button_clone.set_sensitive(false);
                        pause_button_clone.set_label("Pause");

                        let stats = state_clone.get_stats();
                        let elapsed_msg = if let Some(start_t) = *crawl_start_time_clone.borrow() {
                            let dt = start_t.elapsed().as_secs_f64();
                            let avg_speed = if dt > 0.1 { stats.crawled as f64 / dt } else { 0.0 };
                            format!(" in {:.1}s ({:.1} URLs/s)", dt, avg_speed)
                        } else {
                            String::new()
                        };
                        status_label_clone.set_text(&format!(
                            "Crawl completed: {} URLs crawled{}. Total discovered: {}.",
                            stats.crawled, elapsed_msg, stats.discovered
                        ));
                        for (index, result) in state_clone.get_all_results().into_iter().enumerate() {
                            table_clone.add_or_update(result);
                            if index % 100 == 0 { glib::timeout_future(std::time::Duration::from_millis(1)).await; }
                        }
                        sidebar_clone.update_counts(&state_clone);
                        summary_panel_clone.update(&state_clone);
                        table_clone.set_filter(table_clone.get_active_filter());
                    }
                }
            }
        });

        // Start button action
        let tx_clone = tx.clone();
        let state_start = state.clone();
        let pause_button_start = pause_button.clone();
        let address_entry_start = address_entry.clone();
        let table_start = table.clone();
        let details_start = details.clone();
        let sidebar_start = sidebar.clone();
        let progress_bar_start = progress_bar.clone();
        let status_label_start = status_label.clone();
        let summary_panel_start = summary_panel.clone();
        let loaded_list_urls_start = loaded_list_urls.clone();
        let link_opps_start = link_opps.clone();
        let window_start = self.window.clone();
        let crawl_start_time_start = crawl_start_time.clone();
        let crawl_samples_start = crawl_samples.clone();

        start_button.connect_clicked(move |btn| {
            let is_crawling = state_start.is_crawling();

            if is_crawling {
                // Stop the crawl
                state_start.set_crawling(false);
                btn.set_sensitive(false);
                btn.set_label("Stopping...");
                pause_button_start.set_sensitive(false);
            } else {
                // Pre-validate input URL
                let config = state_start.get_config();
                if let Err(error) = config.validate() { status_label_start.set_text(&error); return; }
                let urls_str = if config.crawl_mode == CrawlMode::List {
                    let urls = loaded_list_urls_start.borrow();
                    if urls.is_empty() {
                        status_label_start.set_text("Please click 'Edit List' to add URLs before starting.");
                        return;
                    }
                    urls.join("\n")
                } else {
                    let raw_url = address_entry_start.text().to_string().trim().to_string();
                    if raw_url.is_empty() {
                        status_label_start.set_text("Please enter a URL to audit (e.g. https://example.com).");
                        return;
                    }
                    let cleaned = crate::crawler::clean_seed_url(&raw_url);
                    match url::Url::parse(&cleaned) {
                        Ok(parsed) => {
                            if parsed.host_str().is_none() || !matches!(parsed.scheme(), "http" | "https") {
                                status_label_start.set_text("Please enter a valid website address (e.g. https://example.com).");
                                return;
                            }
                        }
                        Err(_) => {
                            status_label_start.set_text("Invalid URL format. Please enter a valid address (e.g. https://example.com).");
                            return;
                        }
                    }
                    raw_url
                };

                let do_start = {
                    let state_start = state_start.clone();
                    let table_start = table_start.clone();
                    let details_start = details_start.clone();
                    let link_opps_start = link_opps_start.clone();
                    let sidebar_start = sidebar_start.clone();
                    let summary_panel_start = summary_panel_start.clone();
                    let pause_button_start = pause_button_start.clone();
                    let progress_bar_start = progress_bar_start.clone();
                    let status_label_start = status_label_start.clone();
                    let crawl_start_time = crawl_start_time_start.clone();
                    let crawl_samples = crawl_samples_start.clone();
                    let tx_worker = tx_clone.clone();
                    let btn = btn.clone();

                    move |urls: String| {
                        state_start.reset();
                        state_start.set_crawling(true);
                        table_start.clear();
                        details_start.clear();
                        link_opps_start.clear();
                        sidebar_start.update_counts(&state_start);
                        summary_panel_start.update(&state_start);

                        *crawl_start_time.borrow_mut() = Some(std::time::Instant::now());
                        crawl_samples.borrow_mut().clear();

                        btn.set_label("Stop");
                        btn.add_css_class("destructive-action");
                        pause_button_start.set_sensitive(true);
                        pause_button_start.set_label("Pause");
                        progress_bar_start.set_visible(true);
                        progress_bar_start.set_fraction(0.0);
                        status_label_start.set_text("Crawling started...");

                        let state_clone = state_start.clone();
                        tokio::spawn(async move {
                            let crawler = Crawler::new(state_clone, tx_worker);
                            crawler.start(urls).await;
                        });
                    }
                };

                let existing_count = state_start.get_stats().crawled;
                if existing_count > 0 {
                    let alert = adw::AlertDialog::new(
                        Some("Discard Current Crawl Results?"),
                        Some(&format!(
                            "Starting a new crawl will permanently discard {} crawled URLs and audit data.",
                            existing_count
                        )),
                    );
                    alert.add_response("cancel", "Cancel");
                    alert.add_response("discard", "Discard and Start");
                    alert.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
                    alert.set_default_response(Some("cancel"));
                    alert.set_close_response("cancel");

                    let urls_for_alert = urls_str.clone();
                    alert.choose(Some(&window_start), None::<&gio::Cancellable>, move |response| {
                        if response == "discard" {
                            do_start(urls_for_alert);
                        }
                    });
                } else {
                    do_start(urls_str);
                }
            }
        });

        // Trigger start button when Enter is pressed in address entry
        let start_button_activate = start_button.clone();
        address_entry.connect_activate(move |_| {
            start_button_activate.emit_clicked();
        });

        // Pause button action
        let state_clone = state.clone();
        pause_button.connect_clicked(move |btn| {
            let is_paused = state_clone.is_paused();
            if is_paused {
                state_clone.set_paused(false);
                btn.set_label("Pause");
                status_label.set_text("Resuming crawl...");
            } else {
                state_clone.set_paused(true);
                btn.set_label("Resume");
                status_label.set_text("Crawl paused.");
            }
        });
    }
}

async fn fetch_openrouter_models() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>
{
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let res: serde_json::Value = client
        .get("https://openrouter.ai/api/v1/models")
        .send()
        .await?
        .json()
        .await?;

    let mut models = Vec::new();
    if let Some(data) = res["data"].as_array() {
        for m in data {
            if let Some(id) = m["id"].as_str() {
                models.push(id.to_string());
            }
        }
    }

    // Sort them alphabetically
    models.sort();
    Ok(models)
}

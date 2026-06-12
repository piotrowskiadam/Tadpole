use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use crate::state::{CrawlState, ImageInfo};

pub struct Details {
    notebook: gtk::Notebook,
    
    // URL Info Tab widgets
    val_url: gtk::Label,
    val_status: gtk::Label,
    val_index: gtk::Label,
    val_canonical: gtk::Label,
    val_title: gtk::Label,
    btn_suggest_title: gtk::Button,
    val_desc: gtk::Label,
    btn_suggest_desc: gtk::Button,
    val_h1: gtk::Label,
    val_h2: gtk::Label,
    val_words: gtk::Label,
    val_size: gtk::Label,

    // Social / OG Tab widgets
    val_og_title: gtk::Label,
    val_og_desc: gtk::Label,
    val_og_image: gtk::Label,
    val_og_url: gtk::Label,
    val_og_type: gtk::Label,
    val_tw_title: gtk::Label,
    val_tw_desc: gtk::Label,
    val_tw_image: gtk::Label,
    val_tw_card: gtk::Label,
    val_social_diag: gtk::Label,

    // Inlinks Tab widgets
    inlinks_list: gtk::ListBox,
    
    // Outlinks Tab widgets
    outlinks_list: gtk::ListBox,

    // Images Tab widgets
    images_list: gtk::ListBox,

    // Headings Tab widgets
    headings_list: gtk::ListBox,

    // Schema Tab widgets
    schema_all_lbl: gtk::Label,
    schema_all_list: gtk::ListBox,
    schema_list: gtk::ListBox,
    schema_errors_list: gtk::ListBox,
    schema_raw_text: gtk::TextView,

    // Callbacks & active data
    on_url_clicked: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
    active_url: Rc<RefCell<Option<String>>>,
    crawl_state: Rc<RefCell<Option<CrawlState>>>,
}

impl Details {
    pub fn new() -> Self {
        let notebook = gtk::Notebook::new();
        notebook.set_vexpand(true);
        notebook.set_height_request(220);

        let on_url_clicked: Rc<RefCell<Option<Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));
        let active_url: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let crawl_state: Rc<RefCell<Option<CrawlState>>> = Rc::new(RefCell::new(None));

        // --- Tab 1: URL Info ---
        let info_scroll = gtk::ScrolledWindow::new();
        let info_list = gtk::ListBox::new();
        info_list.add_css_class("boxed-list");
        info_list.set_margin_start(10);
        info_list.set_margin_end(10);
        info_list.set_margin_top(10);
        info_list.set_margin_bottom(10);
        info_scroll.set_child(Some(&info_list));

        let val_url = Self::add_info_row(&info_list, "Address");
        let val_status = Self::add_info_row(&info_list, "Status Code");
        let val_index = Self::add_info_row(&info_list, "Indexability");
        let val_canonical = Self::add_info_row(&info_list, "Canonical URL");
        
        let (val_title, btn_suggest_title) = Self::add_info_row_with_action(&info_list, "Page Title");
        let (val_desc, btn_suggest_desc) = Self::add_info_row_with_action(&info_list, "Meta Description");

        let val_h1 = Self::add_info_row(&info_list, "H1");
        let val_h2 = Self::add_info_row(&info_list, "H2");
        let val_words = Self::add_info_row(&info_list, "Word Count");
        let val_size = Self::add_info_row(&info_list, "Size");

        notebook.append_page(&info_scroll, Some(&gtk::Label::new(Some("URL Info"))));

        // --- Tab 2: Inlinks ---
        let inlinks_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let inlinks_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        inlinks_toolbar.set_margin_start(10);
        inlinks_toolbar.set_margin_end(10);
        inlinks_toolbar.set_margin_top(6);
        inlinks_toolbar.set_margin_bottom(6);
        
        let inlinks_lbl = gtk::Label::new(Some("Inbound links pointing to this page"));
        inlinks_lbl.add_css_class("dim-label");
        inlinks_lbl.set_halign(gtk::Align::Start);
        inlinks_lbl.set_hexpand(true);
        inlinks_toolbar.append(&inlinks_lbl);

        let inlinks_export_btn = gtk::Button::builder().label("Export CSV").build();
        inlinks_toolbar.append(&inlinks_export_btn);
        inlinks_box.append(&inlinks_toolbar);

        let inlinks_scroll = gtk::ScrolledWindow::new();
        inlinks_scroll.set_vexpand(true);
        let inlinks_list = gtk::ListBox::new();
        inlinks_scroll.set_child(Some(&inlinks_list));
        inlinks_box.append(&inlinks_scroll);
        notebook.append_page(&inlinks_box, Some(&gtk::Label::new(Some("Inlinks"))));

        // --- Tab 3: Outlinks ---
        let outlinks_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let outlinks_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        outlinks_toolbar.set_margin_start(10);
        outlinks_toolbar.set_margin_end(10);
        outlinks_toolbar.set_margin_top(6);
        outlinks_toolbar.set_margin_bottom(6);

        let outlinks_lbl = gtk::Label::new(Some("Outbound links found on this page"));
        outlinks_lbl.add_css_class("dim-label");
        outlinks_lbl.set_halign(gtk::Align::Start);
        outlinks_lbl.set_hexpand(true);
        outlinks_toolbar.append(&outlinks_lbl);

        let outlinks_export_btn = gtk::Button::builder().label("Export CSV").build();
        outlinks_toolbar.append(&outlinks_export_btn);
        outlinks_box.append(&outlinks_toolbar);

        let outlinks_scroll = gtk::ScrolledWindow::new();
        outlinks_scroll.set_vexpand(true);
        let outlinks_list = gtk::ListBox::new();
        outlinks_scroll.set_child(Some(&outlinks_list));
        outlinks_box.append(&outlinks_scroll);
        notebook.append_page(&outlinks_box, Some(&gtk::Label::new(Some("Outlinks"))));

        // --- Tab 4: Images ---
        let images_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let images_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        images_toolbar.set_margin_start(10);
        images_toolbar.set_margin_end(10);
        images_toolbar.set_margin_top(6);
        images_toolbar.set_margin_bottom(6);

        let images_lbl = gtk::Label::new(Some("Images found on this page"));
        images_lbl.add_css_class("dim-label");
        images_lbl.set_halign(gtk::Align::Start);
        images_lbl.set_hexpand(true);
        images_toolbar.append(&images_lbl);

        let images_export_btn = gtk::Button::builder().label("Export CSV").build();
        images_toolbar.append(&images_export_btn);
        images_box.append(&images_toolbar);

        let images_scroll = gtk::ScrolledWindow::new();
        images_scroll.set_vexpand(true);
        let images_list = gtk::ListBox::new();
        images_scroll.set_child(Some(&images_list));
        images_box.append(&images_scroll);
        notebook.append_page(&images_box, Some(&gtk::Label::new(Some("Images"))));

        // --- Tab 5: Social / OG ---
        let social_scroll = gtk::ScrolledWindow::new();
        let social_list = gtk::ListBox::new();
        social_list.add_css_class("boxed-list");
        social_list.set_margin_start(10);
        social_list.set_margin_end(10);
        social_list.set_margin_top(10);
        social_list.set_margin_bottom(10);
        social_scroll.set_child(Some(&social_list));

        let val_og_title = Self::add_info_row(&social_list, "og:title");
        let val_og_desc = Self::add_info_row(&social_list, "og:description");
        let val_og_image = Self::add_info_row(&social_list, "og:image");
        let val_og_url = Self::add_info_row(&social_list, "og:url");
        let val_og_type = Self::add_info_row(&social_list, "og:type");
        
        let val_tw_title = Self::add_info_row(&social_list, "twitter:title");
        let val_tw_desc = Self::add_info_row(&social_list, "twitter:description");
        let val_tw_image = Self::add_info_row(&social_list, "twitter:image");
        let val_tw_card = Self::add_info_row(&social_list, "twitter:card");
        
        let val_social_diag = Self::add_info_row(&social_list, "Diagnostics");

        notebook.append_page(&social_scroll, Some(&gtk::Label::new(Some("Social / OG"))));

        // --- Tab 5b: Headings ---
        let headings_outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let headings_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        headings_toolbar.set_margin_start(10);
        headings_toolbar.set_margin_end(10);
        headings_toolbar.set_margin_top(6);
        headings_toolbar.set_margin_bottom(6);
        let headings_desc = gtk::Label::new(Some("All headings found on this page (H1–H6)"));
        headings_desc.add_css_class("dim-label");
        headings_desc.set_halign(gtk::Align::Start);
        headings_desc.set_hexpand(true);
        headings_toolbar.append(&headings_desc);

        let headings_copy_btn = gtk::Button::builder()
            .label("Copy Outline")
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy headings outline to clipboard")
            .build();
        headings_toolbar.append(&headings_copy_btn);

        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let btn_clone = headings_copy_btn.clone();
        headings_copy_btn.connect_clicked(move |_| {
            if let Some(url) = active_url_clone.borrow().as_ref() {
                if let Some(state) = crawl_state_clone.borrow().as_ref() {
                    if let Some(res) = state.get_result(url) {
                        let mut text = String::new();
                        for heading in &res.headings {
                            text.push_str(&format!("H{}: {}\n", heading.level, heading.text));
                        }
                        if !text.is_empty() {
                            let display = gdk::Display::default().expect("Could not get GDK display");
                            let clipboard = display.clipboard();
                            clipboard.set_text(&text);

                            // Premium visual feedback
                            btn_clone.set_label("Copied!");
                            btn_clone.set_icon_name("emblem-ok-symbolic");

                            let btn_reset = btn_clone.clone();
                            glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                                btn_reset.set_label("Copy Outline");
                                btn_reset.set_icon_name("edit-copy-symbolic");
                            });
                        }
                    }
                }
            }
        });

        headings_outer.append(&headings_toolbar);

        let headings_scroll = gtk::ScrolledWindow::new();
        headings_scroll.set_vexpand(true);
        let headings_list = gtk::ListBox::new();
        headings_list.set_selection_mode(gtk::SelectionMode::None);
        headings_scroll.set_child(Some(&headings_list));
        headings_outer.append(&headings_scroll);
        notebook.append_page(&headings_outer, Some(&gtk::Label::new(Some("Headings"))));

        // --- Tab 6: Schema / Structured Data ---
        let schema_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        schema_box.set_margin_start(10);
        schema_box.set_margin_end(10);
        schema_box.set_margin_top(10);
        schema_box.set_margin_bottom(10);

        let schema_paned = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        schema_paned.set_vexpand(true);
        schema_paned.set_hexpand(true);
        schema_box.append(&schema_paned);

        // Left pane: lists of schema blocks and errors
        let left_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        left_box.set_width_request(240);
        schema_paned.set_start_child(Some(&left_box));

        let schema_all_lbl = gtk::Label::new(Some("Combined View:"));
        schema_all_lbl.set_halign(gtk::Align::Start);
        schema_all_lbl.add_css_class("bold");
        schema_all_lbl.set_visible(false);
        left_box.append(&schema_all_lbl);

        let schema_all_list = gtk::ListBox::new();
        schema_all_list.add_css_class("boxed-list");
        schema_all_list.set_visible(false);
        left_box.append(&schema_all_list);

        let schema_lbl = gtk::Label::new(Some("Schema Blocks:"));
        schema_lbl.set_halign(gtk::Align::Start);
        schema_lbl.add_css_class("bold");
        left_box.append(&schema_lbl);

        let schema_list_scroll = gtk::ScrolledWindow::new();
        schema_list_scroll.set_vexpand(true);
        let schema_list = gtk::ListBox::new();
        schema_list.add_css_class("boxed-list");
        schema_list_scroll.set_child(Some(&schema_list));
        left_box.append(&schema_list_scroll);

        let diag_lbl = gtk::Label::new(Some("Validation Errors:"));
        diag_lbl.set_halign(gtk::Align::Start);
        diag_lbl.add_css_class("bold");
        left_box.append(&diag_lbl);

        let schema_errors_scroll = gtk::ScrolledWindow::new();
        schema_errors_scroll.set_height_request(100);
        let schema_errors_list = gtk::ListBox::new();
        schema_errors_list.add_css_class("boxed-list");
        schema_errors_scroll.set_child(Some(&schema_errors_list));
        left_box.append(&schema_errors_scroll);

        // Right pane: raw json text view
        let right_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
        schema_paned.set_end_child(Some(&right_box));

        // Header row: label + copy button
        let raw_header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let raw_lbl = gtk::Label::new(Some("Raw JSON-LD (Pretty Print):"));
        raw_lbl.set_halign(gtk::Align::Start);
        raw_lbl.set_hexpand(true);
        raw_lbl.add_css_class("bold");
        raw_header.append(&raw_lbl);

        let schema_copy_btn = gtk::Button::builder()
            .label("Copy JSON")
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy raw JSON-LD to clipboard")
            .css_classes(["flat"])
            .build();
        raw_header.append(&schema_copy_btn);
        right_box.append(&raw_header);

        let raw_scroll = gtk::ScrolledWindow::new();
        raw_scroll.set_vexpand(true);
        raw_scroll.set_hexpand(true);
        let schema_raw_text = gtk::TextView::new();
        schema_raw_text.set_editable(false);
        schema_raw_text.set_monospace(true);
        raw_scroll.set_child(Some(&schema_raw_text));
        right_box.append(&raw_scroll);

        // Syntax-highlighting helper — colours JSON keys, strings, numbers, booleans, null.
        let apply_json_highlighting = {

            let schema_raw_text = schema_raw_text.clone();
            move |json: &str| {
                let buf = schema_raw_text.buffer();
                buf.set_text(json);

                // Ensure tags exist (idempotent — only created once per buffer)
                let tag_table = buf.tag_table();
                let ensure_tag = |name: &str, prop: &str, value: &str| {
                    if tag_table.lookup(name).is_none() {
                        let tag = gtk::TextTag::new(Some(name));
                        tag.set_property(prop, value);
                        tag_table.add(&tag);
                    }
                };
                ensure_tag("json-key",    "foreground", "#89b4fa"); // blue   – keys
                ensure_tag("json-string", "foreground", "#a6e3a1"); // green  – string values
                ensure_tag("json-number", "foreground", "#fab387"); // orange – numbers
                ensure_tag("json-bool",   "foreground", "#f38ba8"); // red    – true/false/null

                if json.is_empty() { return; }

                // Walk through *character* indices.
                // IMPORTANT: buf.iter_at_offset() takes CHARACTER offsets (Unicode
                // code-point positions), NOT byte offsets.  chars() already gives us
                // character indices, so we use them directly without any UTF-8
                // byte-length conversion.
                let chars: Vec<char> = json.chars().collect();
                let len = chars.len();
                let mut i = 0usize;

                while i < len {
                    // Skip whitespace and JSON structural punctuation
                    if chars[i].is_whitespace() || "{}[],:".contains(chars[i]) {
                        i += 1;
                        continue;
                    }

                    if chars[i] == '"' {
                        // Scan to closing quote (handle backslash escapes)
                        let start = i; // char offset of opening "
                        i += 1;
                        while i < len {
                            if chars[i] == '\\' { i += 2; continue; }
                            if chars[i] == '"'  { i += 1; break; }
                            i += 1;
                        }
                        let end = i; // char offset just past closing "

                        // Peek ahead past whitespace to see if ':' follows → key
                        let mut j = end;
                        while j < len && chars[j].is_whitespace() { j += 1; }
                        let is_key = j < len && chars[j] == ':';

                        let tag_name = if is_key { "json-key" } else { "json-string" };
                        // Use char offsets directly — iter_at_offset counts code points
                        let iter_s = buf.iter_at_offset(start as i32);
                        let iter_e = buf.iter_at_offset(end   as i32);
                        buf.apply_tag_by_name(tag_name, &iter_s, &iter_e);
                        continue;
                    }

                    // Numbers (including negative and floats)
                    if chars[i].is_ascii_digit() || chars[i] == '-' {
                        let start = i;
                        while i < len && (chars[i].is_ascii_digit() || ".eE+-".contains(chars[i])) {
                            i += 1;
                        }
                        let iter_s = buf.iter_at_offset(start as i32);
                        let iter_e = buf.iter_at_offset(i     as i32);
                        buf.apply_tag_by_name("json-number", &iter_s, &iter_e);
                        continue;
                    }

                    // true / false / null
                    if chars[i..].starts_with(&['t','r','u','e'])
                    || chars[i..].starts_with(&['f','a','l','s','e'])
                    || chars[i..].starts_with(&['n','u','l','l']) {
                        let start = i;
                        while i < len && chars[i].is_alphabetic() { i += 1; }
                        let iter_s = buf.iter_at_offset(start as i32);
                        let iter_e = buf.iter_at_offset(i     as i32);
                        buf.apply_tag_by_name("json-bool", &iter_s, &iter_e);
                        continue;
                    }

                    i += 1;
                }
            }
        };


        let apply_json_highlighting = std::rc::Rc::new(apply_json_highlighting);


        notebook.append_page(&schema_box, Some(&gtk::Label::new(Some("Schema"))));

        // Connect row selected signal on schema_all_list
        let apply_hl_for_all = apply_json_highlighting.clone();
        let active_url_for_all = active_url.clone();
        let crawl_state_for_all = crawl_state.clone();
        let schema_list_weak = schema_list.downgrade();
        schema_all_list.connect_row_selected(move |this, row| {
            if let Some(_) = row {
                // Deselect other list to prevent double selection visual
                if let Some(other) = schema_list_weak.upgrade() {
                    other.select_row(None::<&gtk::ListBoxRow>);
                }
                if let Some(url) = active_url_for_all.borrow().as_ref() {
                    if let Some(state) = crawl_state_for_all.borrow().as_ref() {
                        if let Some(res) = state.get_result(url) {
                            let all: Vec<serde_json::Value> = res.schema_json_ld.iter()
                                .filter_map(|s| serde_json::from_str(s).ok())
                                .collect();
                            let merged = if all.len() == 1 {
                                serde_json::to_string_pretty(&all[0])
                                    .unwrap_or_default()
                            } else {
                                serde_json::to_string_pretty(&serde_json::Value::Array(all))
                                    .unwrap_or_default()
                            };
                            apply_hl_for_all(&merged);
                            return;
                        }
                    }
                }
            }
            // Clear only if both lists have no selection
            if this.selected_row().is_none() {
                let other_has_sel = schema_list_weak
                    .upgrade()
                    .map_or(false, |other| other.selected_row().is_some());
                if !other_has_sel {
                    apply_hl_for_all("");
                }
            }
        });

        // Connect row selected signal on schema list
        let apply_hl_for_select = apply_json_highlighting.clone();
        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let schema_all_list_weak = schema_all_list.downgrade();
        schema_list.connect_row_selected(move |this, row| {
            if let Some(r) = row {
                // Deselect other list to prevent double selection visual
                if let Some(other) = schema_all_list_weak.upgrade() {
                    other.select_row(None::<&gtk::ListBoxRow>);
                }
                let idx = r.index();
                if idx >= 0 {
                    if let Some(url) = active_url_clone.borrow().as_ref() {
                        if let Some(state) = crawl_state_clone.borrow().as_ref() {
                            if let Some(res) = state.get_result(url) {
                                let block_idx = idx as usize;
                                if block_idx < res.schema_json_ld.len() {
                                    let json_str = &res.schema_json_ld[block_idx];
                                    let formatted = if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                                        serde_json::to_string_pretty(&val).unwrap_or_else(|_| json_str.to_string())
                                    } else {
                                        json_str.to_string()
                                    };
                                    apply_hl_for_select(&formatted);
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            // Clear only if both lists have no selection
            if this.selected_row().is_none() {
                let other_has_sel = schema_all_list_weak
                    .upgrade()
                    .map_or(false, |other| other.selected_row().is_some());
                if !other_has_sel {
                    apply_hl_for_select("");
                }
            }
        });

        // Copy button — copies current raw text buffer content to clipboard
        let schema_raw_text_for_copy = schema_raw_text.clone();
        let schema_copy_btn_clone = schema_copy_btn.clone();
        schema_copy_btn.connect_clicked(move |_| {
            let buf = schema_raw_text_for_copy.buffer();
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
            if !text.is_empty() {
                let display = gdk::Display::default().expect("Could not get GDK display");
                display.clipboard().set_text(&text);

                schema_copy_btn_clone.set_label("Copied!");
                schema_copy_btn_clone.set_icon_name("emblem-ok-symbolic");
                let btn_reset = schema_copy_btn_clone.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                    btn_reset.set_label("Copy JSON");
                    btn_reset.set_icon_name("edit-copy-symbolic");
                });
            }
        });


        // Connect click handler for inlinks / outlinks navigation
        let on_url_clicked_clone = on_url_clicked.clone();
        inlinks_list.connect_row_activated(move |_, row| {
            if let Some(child) = row.child() {
                if let Some(label) = child.downcast_ref::<gtk::Label>() {
                    let url = label.text().to_string();
                    if let Some(ref cb) = *on_url_clicked_clone.borrow() {
                        cb(url);
                    }
                }
            }
        });

        let on_url_clicked_clone = on_url_clicked.clone();
        outlinks_list.connect_row_activated(move |_, row| {
            if let Some(child) = row.child() {
                if let Some(label) = child.downcast_ref::<gtk::Label>() {
                    let url = label.text().to_string();
                    if let Some(ref cb) = *on_url_clicked_clone.borrow() {
                        cb(url);
                    }
                }
            }
        });

        // Wire up suggestion buttons
        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let val_title_clone = val_title.clone();
        let btn_suggest_title_clone = btn_suggest_title.clone();
        btn_suggest_title.connect_clicked(move |_| {
            if let Some(url) = active_url_clone.borrow().as_ref() {
                if let Some(state) = crawl_state_clone.borrow().as_ref() {
                    suggest_title_with_ai(url, state, val_title_clone.clone(), btn_suggest_title_clone.clone());
                }
            }
        });

        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let val_desc_clone = val_desc.clone();
        let btn_suggest_desc_clone = btn_suggest_desc.clone();
        btn_suggest_desc.connect_clicked(move |_| {
            if let Some(url) = active_url_clone.borrow().as_ref() {
                if let Some(state) = crawl_state_clone.borrow().as_ref() {
                    suggest_desc_with_ai(url, state, val_desc_clone.clone(), btn_suggest_desc_clone.clone());
                }
            }
        });

        // Wire up export buttons
        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let notebook_weak = notebook.downgrade();
        inlinks_export_btn.connect_clicked(move |_| {
            if let Some(url) = active_url_clone.borrow().as_ref() {
                if let Some(state) = crawl_state_clone.borrow().as_ref() {
                    if let Some(res) = state.get_result(url) {
                        let root_window = notebook_weak.upgrade()
                            .and_then(|nb| nb.root())
                            .and_downcast::<gtk::Window>();
                        export_inlinks_to_csv(&res.inlinks, root_window.as_ref());
                    }
                }
            }
        });

        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let notebook_weak = notebook.downgrade();
        outlinks_export_btn.connect_clicked(move |_| {
            if let Some(url) = active_url_clone.borrow().as_ref() {
                if let Some(state) = crawl_state_clone.borrow().as_ref() {
                    if let Some(res) = state.get_result(url) {
                        let root_window = notebook_weak.upgrade()
                            .and_then(|nb| nb.root())
                            .and_downcast::<gtk::Window>();
                        export_outlinks_to_csv(&res.outlinks, root_window.as_ref());
                    }
                }
            }
        });

        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let notebook_weak = notebook.downgrade();
        images_export_btn.connect_clicked(move |_| {
            if let Some(url) = active_url_clone.borrow().as_ref() {
                if let Some(state) = crawl_state_clone.borrow().as_ref() {
                    if let Some(res) = state.get_result(url) {
                        let root_window = notebook_weak.upgrade()
                            .and_then(|nb| nb.root())
                            .and_downcast::<gtk::Window>();
                        export_images_to_csv(&res.images, root_window.as_ref());
                    }
                }
            }
        });

        Self {
            notebook,
            val_url,
            val_status,
            val_index,
            val_canonical,
            val_title,
            btn_suggest_title,
            val_desc,
            btn_suggest_desc,
            val_h1,
            val_h2,
            val_words,
            val_size,
            val_og_title,
            val_og_desc,
            val_og_image,
            val_og_url,
            val_og_type,
            val_tw_title,
            val_tw_desc,
            val_tw_image,
            val_tw_card,
            val_social_diag,
            inlinks_list,
            outlinks_list,
            images_list,
            headings_list,
            schema_all_lbl,
            schema_all_list,
            schema_list,
            schema_errors_list,
            schema_raw_text,
            on_url_clicked,
            active_url,
            crawl_state,
        }
    }

    pub fn widget(&self) -> &gtk::Notebook {
        &self.notebook
    }

    pub fn connect_url_clicked<F>(&self, callback: F)
    where
        F: Fn(String) + 'static,
    {
        *self.on_url_clicked.borrow_mut() = Some(Box::new(callback));
    }

    pub fn clear(&self) {
        self.val_url.set_text("");
        self.val_status.set_text("");
        self.val_index.set_text("");
        self.val_canonical.set_text("");
        self.val_title.set_text("");
        self.btn_suggest_title.set_visible(false);
        self.val_desc.set_text("");
        self.btn_suggest_desc.set_visible(false);
        self.val_h1.set_text("");
        self.val_h2.set_text("");
        self.val_words.set_text("");
        self.val_size.set_text("");

        self.val_og_title.set_text("");
        self.val_og_desc.set_text("");
        self.val_og_image.set_text("");
        self.val_og_url.set_text("");
        self.val_og_type.set_text("");
        self.val_tw_title.set_text("");
        self.val_tw_desc.set_text("");
        self.val_tw_image.set_text("");
        self.val_tw_card.set_text("");
        self.val_social_diag.set_text("");

        self.clear_list_box(&self.inlinks_list);
        self.clear_list_box(&self.outlinks_list);
        self.clear_list_box(&self.images_list);
        self.clear_list_box(&self.headings_list);
        self.schema_all_lbl.set_visible(false);
        self.schema_all_list.set_visible(false);
        self.clear_list_box(&self.schema_all_list);
        self.clear_list_box(&self.schema_list);
        self.clear_list_box(&self.schema_errors_list);
        self.schema_raw_text.buffer().set_text("");
    }

    pub fn update(&self, url: &str, state: &CrawlState) {
        eprintln!("[Details Update] Updating for URL: {}", url);
        self.clear();
        *self.active_url.borrow_mut() = Some(url.to_string());
        *self.crawl_state.borrow_mut() = Some(state.clone());

        if let Some(res) = state.get_result(url) {
            eprintln!("[Details Update] Found CrawlResult in state for URL: {}", url);
            // URL Info
            self.val_url.set_text(&res.url);
            self.val_status.set_text(&res.status_code.map_or("Err/Unknown".to_string(), |c| c.to_string()));
            self.val_index.set_text(&res.indexability_status);
            self.val_canonical.set_text(res.canonical.as_deref().unwrap_or("None"));

            // Social / OG
            self.val_og_title.set_text(res.og_title.as_deref().unwrap_or("None"));
            self.val_og_desc.set_text(res.og_description.as_deref().unwrap_or("None"));
            self.val_og_image.set_text(res.og_image.as_deref().unwrap_or("None"));
            self.val_og_url.set_text(res.og_url.as_deref().unwrap_or("None"));
            self.val_og_type.set_text(res.og_type.as_deref().unwrap_or("None"));
            self.val_tw_title.set_text(res.twitter_title.as_deref().unwrap_or("None"));
            self.val_tw_desc.set_text(res.twitter_description.as_deref().unwrap_or("None"));
            self.val_tw_image.set_text(res.twitter_image.as_deref().unwrap_or("None"));
            self.val_tw_card.set_text(res.twitter_card.as_deref().unwrap_or("None"));

            // Calculate diagnostics
            let mut diags = vec![];
            
            // Check missing essential OG tags
            if res.og_title.is_none() {
                diags.push("Missing og:title tag.".to_string());
            } else if let Some(ref og_t) = res.og_title {
                if let Some(ref t) = res.title {
                    if og_t != t {
                        diags.push("Warning: og:title does not match HTML title tag.".to_string());
                    }
                }
            }
            
            if res.og_description.is_none() {
                diags.push("Missing og:description tag.".to_string());
            } else if let Some(ref og_d) = res.og_description {
                if let Some(ref d) = res.meta_desc {
                    if og_d != d {
                        diags.push("Warning: og:description does not match HTML meta description.".to_string());
                    }
                }
            }

            if res.og_image.is_none() {
                diags.push("Missing og:image tag.".to_string());
            }

            if res.twitter_card.is_none() {
                diags.push("Missing twitter:card tag.".to_string());
            }

            if diags.is_empty() {
                self.val_social_diag.set_text("All Open Graph and Twitter Card tags are correctly configured.");
                self.val_social_diag.remove_css_class("error");
                self.val_social_diag.add_css_class("success");
            } else {
                self.val_social_diag.set_text(&diags.join("\n"));
                self.val_social_diag.add_css_class("error");
                self.val_social_diag.remove_css_class("success");
            }
            
            // Title suggests
            if res.title.as_ref().map_or(true, |t| t.trim().is_empty()) {
                self.val_title.set_text("[Missing Page Title]");
                self.btn_suggest_title.set_visible(true);
            } else {
                let t_len = res.title.as_ref().map_or(0, |t| t.chars().count());
                self.val_title.set_text(&format!("{} ({} chars)", res.title.as_deref().unwrap_or("None"), t_len));
                self.btn_suggest_title.set_visible(false);
            }

            // Description suggests
            if res.meta_desc.as_ref().map_or(true, |d| d.trim().is_empty()) {
                self.val_desc.set_text("[Missing Meta Description]");
                self.btn_suggest_desc.set_visible(true);
            } else {
                let d_len = res.meta_desc.as_ref().map_or(0, |d| d.chars().count());
                self.val_desc.set_text(&format!("{} ({} chars)", res.meta_desc.as_deref().unwrap_or("None"), d_len));
                self.btn_suggest_desc.set_visible(false);
            }

            // H1 details
            let h1_text = if res.h1_count > 1 {
                format!("{} (Multiple tags: {} found)", res.h1.as_deref().unwrap_or("None"), res.h1_count)
            } else {
                res.h1.clone().unwrap_or_else(|| "None".to_string())
            };
            self.val_h1.set_text(&h1_text);

            // H2 details
            let h2_text = if res.h2_count > 1 {
                format!("{} (Multiple tags: {} found)", res.h2.as_deref().unwrap_or("None"), res.h2_count)
            } else {
                res.h2.clone().unwrap_or_else(|| "None".to_string())
            };
            self.val_h2.set_text(&h2_text);

            self.val_words.set_text(&res.word_count.to_string());
            self.val_size.set_text(&format!("{:.2} KB", res.size_bytes as f64 / 1024.0));

            // Inlinks
            for inlink in &res.inlinks {
                let lbl = gtk::Label::new(Some(inlink));
                lbl.set_halign(gtk::Align::Start);
                lbl.set_margin_start(10);
                lbl.set_margin_end(10);
                lbl.set_margin_top(4);
                lbl.set_margin_bottom(4);
                
                let list_row = gtk::ListBoxRow::new();
                list_row.set_child(Some(&lbl));
                self.inlinks_list.append(&list_row);
            }

            // Outlinks
            for outlink in &res.outlinks {
                let lbl = gtk::Label::new(Some(outlink));
                lbl.set_halign(gtk::Align::Start);
                lbl.set_margin_start(10);
                lbl.set_margin_end(10);
                lbl.set_margin_top(4);
                lbl.set_margin_bottom(4);

                let list_row = gtk::ListBoxRow::new();
                list_row.set_child(Some(&lbl));
                self.outlinks_list.append(&list_row);
            }

            // Images with inline AI suggestions
            for img in &res.images {
                let img_box = gtk::Box::new(gtk::Orientation::Horizontal, 15);
                img_box.set_margin_start(10);
                img_box.set_margin_end(10);
                img_box.set_margin_top(4);
                img_box.set_margin_bottom(4);

                let src_lbl = gtk::Label::new(Some(&img.src));
                src_lbl.set_halign(gtk::Align::Start);
                src_lbl.set_hexpand(true);
                img_box.append(&src_lbl);

                let alt_text = img.alt.as_deref().unwrap_or("[Missing Alt Text]");
                let alt_lbl = gtk::Label::new(Some(alt_text));
                alt_lbl.set_halign(gtk::Align::End);
                
                if img.alt.is_none() {
                    alt_lbl.add_css_class("error");
                    
                    let suggest_alt_btn = gtk::Button::builder()
                        .label("Suggest Alt")
                        .build();
                    suggest_alt_btn.add_css_class("flat");
                    
                    let img_src = img.src.clone();
                    let page_url = url.to_string();
                    let state_clone = state.clone();
                    let alt_lbl_clone = alt_lbl.clone();
                    let suggest_alt_btn_clone = suggest_alt_btn.clone();
                    
                    suggest_alt_btn.connect_clicked(move |_| {
                        suggest_alt_text_with_ai(&page_url, &img_src, &state_clone, alt_lbl_clone.clone(), suggest_alt_btn_clone.clone());
                    });
                    
                    img_box.append(&suggest_alt_btn);
                } else {
                    alt_lbl.add_css_class("dim-label");
                }
                img_box.append(&alt_lbl);

                let list_row = gtk::ListBoxRow::new();
                list_row.set_child(Some(&img_box));
                self.images_list.append(&list_row);
            }

            // Headings (H1–H6)
            if res.headings.is_empty() {
                let empty_row = gtk::ListBoxRow::new();
                empty_row.set_selectable(false);
                let empty_lbl = gtk::Label::new(Some("No headings found on this page."));
                empty_lbl.add_css_class("dim-label");
                empty_lbl.set_margin_start(12);
                empty_lbl.set_margin_top(8);
                empty_lbl.set_margin_bottom(8);
                empty_row.set_child(Some(&empty_lbl));
                self.headings_list.append(&empty_row);
            } else {
                // Count H1s to detect duplicates
                let h1_count = res.headings.iter().filter(|h| h.level == 1).count();
                for heading in &res.headings {
                    let row = gtk::ListBoxRow::new();
                    row.set_selectable(false);

                    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    // Indent by level: H1=0, H2=16, H3=32, ...
                    let indent = (heading.level as i32 - 1) * 18;
                    row_box.set_margin_start(12 + indent);
                    row_box.set_margin_end(12);
                    row_box.set_margin_top(5);
                    row_box.set_margin_bottom(5);

                    // Level badge
                    let badge = gtk::Label::new(Some(&format!("H{}", heading.level)));
                    badge.set_width_chars(3);
                    badge.add_css_class("numeric");
                    match heading.level {
                        1 => { badge.add_css_class("accent"); badge.add_css_class("bold"); }
                        2 => { badge.add_css_class("bold"); }
                        _ => { badge.add_css_class("dim-label"); }
                    }
                    row_box.append(&badge);

                    // Heading text
                    let text_lbl = gtk::Label::new(Some(&heading.text));
                    text_lbl.set_halign(gtk::Align::Start);
                    text_lbl.set_hexpand(true);
                    text_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    // Highlight duplicate H1s
                    if heading.level == 1 && h1_count > 1 {
                        text_lbl.add_css_class("warning");
                    }
                    row_box.append(&text_lbl);

                    row.set_child(Some(&row_box));
                    self.headings_list.append(&row);
                }
            }

            // Structured Data / Schema
            if res.schema_json_ld.is_empty() {
                self.schema_all_lbl.set_visible(false);
                self.schema_all_list.set_visible(false);
                let empty_lbl = gtk::Label::new(Some("No Schema.org structured data found."));
                empty_lbl.add_css_class("dim-label");
                empty_lbl.set_margin_start(10);
                empty_lbl.set_margin_top(6);
                empty_lbl.set_margin_bottom(6);
                let list_row = gtk::ListBoxRow::new();
                list_row.set_child(Some(&empty_lbl));
                list_row.set_selectable(false);
                self.schema_list.append(&list_row);
            } else {
                self.schema_all_lbl.set_visible(true);
                self.schema_all_list.set_visible(true);

                // Populate "All Blocks" row in schema_all_list
                {
                    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
                    row_box.set_margin_start(10);
                    row_box.set_margin_end(10);
                    row_box.set_margin_top(6);
                    row_box.set_margin_bottom(6);

                    let title_lbl = gtk::Label::new(Some(&format!(
                        "All Blocks ({} total)", res.schema_json_ld.len()
                    )));
                    title_lbl.set_halign(gtk::Align::Start);
                    title_lbl.add_css_class("bold");
                    row_box.append(&title_lbl);

                    let sub_lbl = gtk::Label::new(Some("Combined view of every schema block"));
                    sub_lbl.set_halign(gtk::Align::Start);
                    sub_lbl.add_css_class("dim-label");
                    row_box.append(&sub_lbl);

                    let all_row = gtk::ListBoxRow::new();
                    all_row.set_child(Some(&row_box));
                    self.schema_all_list.append(&all_row);
                }

                for (idx, json_str) in res.schema_json_ld.iter().enumerate() {
                    let (types, contexts) = get_schema_summary(json_str);
                    
                    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
                    row_box.set_margin_start(10);
                    row_box.set_margin_end(10);
                    row_box.set_margin_top(6);
                    row_box.set_margin_bottom(6);
                    
                    let title_lbl = gtk::Label::new(Some(&format!("Schema Block #{} ({})", idx + 1, types)));
                    title_lbl.set_halign(gtk::Align::Start);
                    title_lbl.add_css_class("bold");
                    row_box.append(&title_lbl);
                    
                    let ctx_lbl = gtk::Label::new(Some(&format!("Context: {}", contexts)));
                    ctx_lbl.set_halign(gtk::Align::Start);
                    ctx_lbl.add_css_class("dim-label");
                    row_box.append(&ctx_lbl);
                    
                    let list_row = gtk::ListBoxRow::new();
                    list_row.set_child(Some(&row_box));
                    self.schema_list.append(&list_row);
                }
                
                // Select "All Blocks" row by default
                if let Some(first_row) = self.schema_all_list.row_at_index(0) {
                    self.schema_all_list.select_row(Some(&first_row));
                }
            }

            // Schema errors
            if res.schema_errors.is_empty() {
                let success_lbl = gtk::Label::new(Some("✓ No validation errors or warnings found."));
                success_lbl.add_css_class("success");
                success_lbl.set_halign(gtk::Align::Start);
                success_lbl.set_margin_start(10);
                success_lbl.set_margin_top(6);
                success_lbl.set_margin_bottom(6);
                let list_row = gtk::ListBoxRow::new();
                list_row.set_child(Some(&success_lbl));
                list_row.set_selectable(false);
                self.schema_errors_list.append(&list_row);
            } else {
                for err in &res.schema_errors {
                    let err_lbl = gtk::Label::new(Some(&format!("✗ {}", err)));
                    err_lbl.add_css_class("error");
                    err_lbl.set_halign(gtk::Align::Start);
                    err_lbl.set_wrap(true);
                    err_lbl.set_margin_start(10);
                    err_lbl.set_margin_top(6);
                    err_lbl.set_margin_bottom(6);
                    let list_row = gtk::ListBoxRow::new();
                    list_row.set_child(Some(&err_lbl));
                    list_row.set_selectable(false);
                    self.schema_errors_list.append(&list_row);
                }
            }
        } else {
            eprintln!("[Details Update] NO CrawlResult found in state for URL: {}", url);
        }
    }

    fn add_info_row(list_box: &gtk::ListBox, title: &str) -> gtk::Label {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 15);
        row_box.set_margin_start(10);
        row_box.set_margin_end(10);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);

        let title_lbl = gtk::Label::new(Some(title));
        title_lbl.set_halign(gtk::Align::Start);
        title_lbl.add_css_class("bold");
        title_lbl.set_width_request(140);
        row_box.append(&title_lbl);

        let val_lbl = gtk::Label::new(None);
        val_lbl.set_halign(gtk::Align::Start);
        val_lbl.set_hexpand(true);
        val_lbl.set_selectable(true);
        val_lbl.set_wrap(true);
        val_lbl.add_css_class("selectable");
        row_box.append(&val_lbl);

        let list_row = gtk::ListBoxRow::new();
        list_row.set_child(Some(&row_box));
        list_row.set_selectable(false);
        list_box.append(&list_row);

        val_lbl
    }



    fn add_info_row_with_action(list_box: &gtk::ListBox, title: &str) -> (gtk::Label, gtk::Button) {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 15);
        row_box.set_margin_start(10);
        row_box.set_margin_end(10);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);

        let title_lbl = gtk::Label::new(Some(title));
        title_lbl.set_halign(gtk::Align::Start);
        title_lbl.add_css_class("bold");
        title_lbl.set_width_request(140);
        row_box.append(&title_lbl);

        let val_lbl = gtk::Label::new(None);
        val_lbl.set_halign(gtk::Align::Start);
        val_lbl.set_hexpand(true);
        val_lbl.set_selectable(true);
        val_lbl.set_wrap(true);
        val_lbl.add_css_class("selectable");
        row_box.append(&val_lbl);

        let action_btn = gtk::Button::builder()
            .label("Suggest with AI")
            .visible(false)
            .build();
        action_btn.add_css_class("flat");
        row_box.append(&action_btn);

        let list_row = gtk::ListBoxRow::new();
        list_row.set_child(Some(&row_box));
        list_row.set_selectable(false);
        list_box.append(&list_row);

        (val_lbl, action_btn)
    }

    fn clear_list_box(&self, list_box: &gtk::ListBox) {
        while let Some(row) = list_box.row_at_index(0) {
            list_box.remove(&row);
        }
    }
}

// AI helpers
fn suggest_title_with_ai(page_url: &str, state: &CrawlState, val_lbl: gtk::Label, btn: gtk::Button) {
    let config = state.get_config();
    let api_key = match config.ai_api_key {
        Some(ref k) if !k.is_empty() => k.clone(),
        _ => {
            val_lbl.set_text("Set AI API Key in Preferences");
            return;
        }
    };
    
    btn.set_sensitive(false);
    btn.set_label("Generating...");
    
    let page_url = page_url.to_string();
    let provider = config.ai_provider.clone();
    let model = config.ai_model.clone();
    
    glib::MainContext::default().spawn_local(async move {
        let prompt = format!(
            "Suggest a high-density, SEO-friendly page Title tag (max 60 characters) for the webpage with URL: '{}'. Respond with ONLY the suggested title, no quotes, no explanations.",
            page_url
        );
        
        let res = call_ai_api(&provider, &api_key, &model, &prompt).await;
        
        btn.set_sensitive(true);
        btn.set_label("Suggest with AI");
        match res {
            Ok(suggestion) => {
                val_lbl.set_text(&format!("{} (suggested)", suggestion));
                btn.set_visible(false);
            }
            Err(e) => {
                val_lbl.set_text(&format!("AI Error: {}", e));
            }
        }
    });
}

fn suggest_desc_with_ai(page_url: &str, state: &CrawlState, val_lbl: gtk::Label, btn: gtk::Button) {
    let config = state.get_config();
    let api_key = match config.ai_api_key {
        Some(ref k) if !k.is_empty() => k.clone(),
        _ => {
            val_lbl.set_text("Set AI API Key in Preferences");
            return;
        }
    };
    
    btn.set_sensitive(false);
    btn.set_label("Generating...");
    
    let page_url = page_url.to_string();
    let provider = config.ai_provider.clone();
    let model = config.ai_model.clone();
    
    glib::MainContext::default().spawn_local(async move {
        let prompt = format!(
            "Suggest a compelling, click-worthy Meta Description (max 155 characters) for the webpage with URL: '{}'. Respond with ONLY the suggested meta description, no quotes, no explanations.",
            page_url
        );
        
        let res = call_ai_api(&provider, &api_key, &model, &prompt).await;
        
        btn.set_sensitive(true);
        btn.set_label("Suggest with AI");
        match res {
            Ok(suggestion) => {
                val_lbl.set_text(&format!("{} (suggested)", suggestion));
                btn.set_visible(false);
            }
            Err(e) => {
                val_lbl.set_text(&format!("AI Error: {}", e));
            }
        }
    });
}

fn suggest_alt_text_with_ai(page_url: &str, img_src: &str, state: &CrawlState, alt_lbl: gtk::Label, btn: gtk::Button) {
    let config = state.get_config();
    let api_key = match config.ai_api_key {
        Some(ref k) if !k.is_empty() => k.clone(),
        _ => {
            alt_lbl.set_text("Set AI API Key in Preferences");
            return;
        }
    };
    
    btn.set_sensitive(false);
    btn.set_label("Generating...");
    
    let page_url = page_url.to_string();
    let img_src = img_src.to_string();
    let provider = config.ai_provider.clone();
    let model = config.ai_model.clone();
    
    glib::MainContext::default().spawn_local(async move {
        let prompt = format!(
            "Suggest a concise and SEO-friendly image alt text (max 12 words) for the image with URL: '{}' located on the webpage: '{}'. Respond with ONLY the suggested alt text, no quotes, no explanations.",
            img_src, page_url
        );
        
        let res = call_ai_api(&provider, &api_key, &model, &prompt).await;
        
        btn.set_sensitive(true);
        btn.set_label("Suggest Alt");
        match res {
            Ok(suggestion) => {
                alt_lbl.set_text(&suggestion);
                alt_lbl.remove_css_class("error");
                alt_lbl.add_css_class("dim-label");
                btn.set_visible(false);
            }
            Err(e) => {
                alt_lbl.set_text(&format!("AI Error: {}", e));
            }
        }
    });
}

async fn call_ai_api(provider: &str, api_key: &str, model: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let url = if provider == "OpenRouter" {
        "https://openrouter.ai/api/v1/chat/completions"
    } else {
        "https://api.openai.com/v1/chat/completions"
    };

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.7
    });

    let mut request = client.post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key));
        
    if provider == "OpenRouter" {
        request = request
            .header("HTTP-Referer", "https://github.com/local-seo-crawler")
            .header("X-Title", "Local SEO Crawler");
    }

    let response = request.json(&body).send().await?;
    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        return Err(format!("API returned status {}: {}", status, text).into());
    }

    let parsed_res: serde_json::Value = serde_json::from_str(&text)?;
    let content = parsed_res["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| Box::<dyn std::error::Error + Send + Sync>::from("Failed to parse completion content"))?
        .trim()
        .to_string();

    Ok(content)
}

// Export helpers
fn export_inlinks_to_csv(inlinks: &[String], parent_window: Option<&gtk::Window>) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Export Inlinks to CSV");
    dialog.set_initial_name(Some("inlinks.csv"));
    
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("CSV Files"));
    filter.add_pattern("*.csv");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));

    let inlinks = inlinks.to_vec();
    dialog.save(parent_window, None::<&gio::Cancellable>, move |result| {
        if let Ok(file) = result {
            if let Some(path) = file.path() {
                if let Ok(f) = std::fs::File::create(&path) {
                    let mut wtr = csv::Writer::from_writer(f);
                    let _ = wtr.write_record(&["Source Inlink URL"]);
                    for link in inlinks {
                        let _ = wtr.write_record(&[link]);
                    }
                    let _ = wtr.flush();
                }
            }
        }
    });
}

fn export_outlinks_to_csv(outlinks: &[String], parent_window: Option<&gtk::Window>) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Export Outlinks to CSV");
    dialog.set_initial_name(Some("outlinks.csv"));
    
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("CSV Files"));
    filter.add_pattern("*.csv");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));

    let outlinks = outlinks.to_vec();
    dialog.save(parent_window, None::<&gio::Cancellable>, move |result| {
        if let Ok(file) = result {
            if let Some(path) = file.path() {
                if let Ok(f) = std::fs::File::create(&path) {
                    let mut wtr = csv::Writer::from_writer(f);
                    let _ = wtr.write_record(&["Destination Outlink URL"]);
                    for link in outlinks {
                        let _ = wtr.write_record(&[link]);
                    }
                    let _ = wtr.flush();
                }
            }
        }
    });
}

fn export_images_to_csv(images: &[ImageInfo], parent_window: Option<&gtk::Window>) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Export Images to CSV");
    dialog.set_initial_name(Some("images.csv"));
    
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("CSV Files"));
    filter.add_pattern("*.csv");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));

    let images = images.to_vec();
    dialog.save(parent_window, None::<&gio::Cancellable>, move |result| {
        if let Ok(file) = result {
            if let Some(path) = file.path() {
                if let Ok(f) = std::fs::File::create(&path) {
                    let mut wtr = csv::Writer::from_writer(f);
                    let _ = wtr.write_record(&["Image Source", "Alt Text", "Local Path"]);
                    for img in images {
                        let _ = wtr.write_record(&[
                            img.src,
                            img.alt.unwrap_or_default(),
                            img.local_path.unwrap_or_default()
                        ]);
                    }
                    let _ = wtr.flush();
                }
            }
        }
    });
}

fn get_schema_summary(json_str: &str) -> (String, String) {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
        let get_types = |v: &serde_json::Value| -> String {
            if let Some(arr) = v.as_array() {
                arr.iter()
                    .filter_map(|item| item.get("@type").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                v.get("@type").and_then(|t| t.as_str()).unwrap_or("Unknown").to_string()
            }
        };

        let get_contexts = |v: &serde_json::Value| -> String {
            if let Some(arr) = v.as_array() {
                arr.iter()
                    .filter_map(|item| item.get("@context").and_then(|c| c.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                v.get("@context").and_then(|c| c.as_str()).unwrap_or("schema.org").to_string()
            }
        };

        if let Some(arr) = val.as_array() {
            if !arr.is_empty() {
                let types = arr.iter().map(|item| get_types(item)).collect::<Vec<_>>().join(" + ");
                let contexts = arr.iter().map(|item| get_contexts(item)).collect::<Vec<_>>().join(" + ");
                return (types, contexts);
            }
        } else if val.is_object() {
            return (get_types(&val), get_contexts(&val));
        }
    }
    ("Unknown (Invalid JSON)".to_string(), "Unknown".to_string())
}

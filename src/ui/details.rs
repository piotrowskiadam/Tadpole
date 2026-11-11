use super::detail_table::{DetailTable, wrapped_label};
use crate::inspection::{self, LinkRow};
use crate::state::{CrawlState, ImageInfo};
use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

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
    inlinks_table: DetailTable,
    inlinks_lbl: gtk::Label,

    // Outlinks Tab widgets
    outlinks_table: DetailTable,
    outlinks_lbl: gtk::Label,

    // Images Tab widgets
    images_table: DetailTable,
    images_lbl: gtk::Label,
    social_previews: gtk::Box,
    social_extra: gtk::ListBox,

    // Headings Tab widgets
    headings_list: gtk::ListBox,

    // Schema Tab widgets
    schema_all_lbl: gtk::Label,
    schema_all_list: gtk::ListBox,
    schema_list: gtk::ListBox,
    schema_errors_list: gtk::ListBox,
    schema_raw_text: gtk::TextView,

    // Markdown Tab widgets
    md_text_view: gtk::TextView,
    md_copy_btn: gtk::Button,
    md_save_btn: gtk::Button,

    // Callbacks & active data
    on_url_clicked: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
    active_url: Rc<RefCell<Option<String>>>,
    crawl_state: Rc<RefCell<Option<CrawlState>>>,
}

impl Details {
    pub fn new() -> Self {
        let notebook = gtk::Notebook::new();
        notebook.set_vexpand(true);
        notebook.set_scrollable(true);
        notebook.set_height_request(220);

        let on_url_clicked: Rc<RefCell<Option<Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));
        let active_url: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let crawl_state: Rc<RefCell<Option<CrawlState>>> = Rc::new(RefCell::new(None));

        // --- Tab 1: URL Info ---
        let info_scroll = gtk::ScrolledWindow::new();
        info_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
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

        let (val_title, btn_suggest_title) =
            Self::add_info_row_with_action(&info_list, "Page Title");
        let (val_desc, btn_suggest_desc) =
            Self::add_info_row_with_action(&info_list, "Meta Description");

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
        inlinks_lbl.set_wrap(true);
        inlinks_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        inlinks_lbl.set_halign(gtk::Align::Fill);
        inlinks_lbl.set_xalign(0.);
        inlinks_lbl.set_width_chars(1);
        inlinks_toolbar.append(&inlinks_lbl);

        let inlinks_export_btn = gtk::Button::builder().label("Export CSV").build();
        inlinks_toolbar.append(&inlinks_export_btn);
        inlinks_box.append(&inlinks_toolbar);

        let inlinks_table = link_table();
        inlinks_box.append(&inlinks_table.scroll);
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
        outlinks_lbl.set_wrap(true);
        outlinks_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        outlinks_lbl.set_halign(gtk::Align::Fill);
        outlinks_lbl.set_xalign(0.);
        outlinks_lbl.set_width_chars(1);
        outlinks_toolbar.append(&outlinks_lbl);

        let outlinks_export_btn = gtk::Button::builder().label("Export CSV").build();
        outlinks_toolbar.append(&outlinks_export_btn);
        outlinks_box.append(&outlinks_toolbar);

        let outlinks_table = link_table();
        outlinks_box.append(&outlinks_table.scroll);
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
        images_lbl.set_wrap(true);
        images_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        images_lbl.set_halign(gtk::Align::Fill);
        images_lbl.set_xalign(0.);
        images_lbl.set_width_chars(1);
        images_toolbar.append(&images_lbl);

        let images_export_btn = gtk::Button::builder().label("Export CSV").build();
        images_toolbar.append(&images_export_btn);
        images_box.append(&images_toolbar);

        let images_table = image_table();
        images_box.append(&images_table.scroll);
        notebook.append_page(&images_box, Some(&gtk::Label::new(Some("Images"))));

        // --- Tab 5: Social / OG ---
        let social_scroll = gtk::ScrolledWindow::new();
        social_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        let social_list = gtk::ListBox::new();
        social_list.add_css_class("boxed-list");
        social_list.set_margin_start(10);
        social_list.set_margin_end(10);
        social_list.set_margin_top(10);
        social_list.set_margin_bottom(10);
        let social_content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let preview_note = wrapped_label(
            "Illustrative previews using declared metadata; social platforms may render differently. Images load when visible.",
        );
        preview_note.add_css_class("dim-label");
        social_content.append(&preview_note);
        let social_previews = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        social_previews.set_homogeneous(true);
        social_previews.set_margin_start(10);
        social_previews.set_margin_end(10);
        social_content.append(&social_previews);
        social_content.append(&social_list);
        let social_extra = gtk::ListBox::new();
        social_extra.add_css_class("boxed-list");
        social_extra.set_selection_mode(gtk::SelectionMode::None);
        social_extra.set_margin_start(10);
        social_extra.set_margin_end(10);
        social_extra.set_margin_bottom(10);
        social_content.append(&social_extra);
        social_scroll.set_child(Some(&social_content));

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
        headings_desc.set_wrap(true);
        headings_desc.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        headings_desc.set_halign(gtk::Align::Fill);
        headings_desc.set_xalign(0.);
        headings_desc.set_width_chars(1);
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
                            let display =
                                gdk::Display::default().expect("Could not get GDK display");
                            let clipboard = display.clipboard();
                            clipboard.set_text(&text);

                            // Premium visual feedback
                            btn_clone.set_label("Copied!");
                            btn_clone.set_icon_name("emblem-ok-symbolic");

                            let btn_reset = btn_clone.clone();
                            glib::timeout_add_local_once(
                                std::time::Duration::from_secs(2),
                                move || {
                                    btn_reset.set_label("Copy Outline");
                                    btn_reset.set_icon_name("edit-copy-symbolic");
                                },
                            );
                        }
                    }
                }
            }
        });

        headings_outer.append(&headings_toolbar);

        let headings_scroll = gtk::ScrolledWindow::new();
        headings_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
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
        schema_list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
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
        schema_errors_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
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
        raw_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        raw_scroll.set_vexpand(true);
        raw_scroll.set_hexpand(true);
        let schema_raw_text = gtk::TextView::new();
        schema_raw_text.set_editable(false);
        schema_raw_text.set_monospace(true);
        schema_raw_text.set_wrap_mode(gtk::WrapMode::WordChar);
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
                ensure_tag("json-key", "foreground", "#89b4fa"); // blue   – keys
                ensure_tag("json-string", "foreground", "#a6e3a1"); // green  – string values
                ensure_tag("json-number", "foreground", "#fab387"); // orange – numbers
                ensure_tag("json-bool", "foreground", "#f38ba8"); // red    – true/false/null

                if json.is_empty() {
                    return;
                }

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
                            if chars[i] == '\\' {
                                i += 2;
                                continue;
                            }
                            if chars[i] == '"' {
                                i += 1;
                                break;
                            }
                            i += 1;
                        }
                        let end = i; // char offset just past closing "

                        // Peek ahead past whitespace to see if ':' follows → key
                        let mut j = end;
                        while j < len && chars[j].is_whitespace() {
                            j += 1;
                        }
                        let is_key = j < len && chars[j] == ':';

                        let tag_name = if is_key { "json-key" } else { "json-string" };
                        // Use char offsets directly — iter_at_offset counts code points
                        let iter_s = buf.iter_at_offset(start as i32);
                        let iter_e = buf.iter_at_offset(end as i32);
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
                        let iter_e = buf.iter_at_offset(i as i32);
                        buf.apply_tag_by_name("json-number", &iter_s, &iter_e);
                        continue;
                    }

                    // true / false / null
                    if chars[i..].starts_with(&['t', 'r', 'u', 'e'])
                        || chars[i..].starts_with(&['f', 'a', 'l', 's', 'e'])
                        || chars[i..].starts_with(&['n', 'u', 'l', 'l'])
                    {
                        let start = i;
                        while i < len && chars[i].is_alphabetic() {
                            i += 1;
                        }
                        let iter_s = buf.iter_at_offset(start as i32);
                        let iter_e = buf.iter_at_offset(i as i32);
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
                            let all: Vec<serde_json::Value> = res
                                .schema_json_ld
                                .iter()
                                .filter_map(|s| serde_json::from_str(s).ok())
                                .collect();
                            let merged = if all.len() == 1 {
                                serde_json::to_string_pretty(&all[0]).unwrap_or_default()
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
                                    let formatted = if let Ok(val) =
                                        serde_json::from_str::<serde_json::Value>(json_str)
                                    {
                                        serde_json::to_string_pretty(&val)
                                            .unwrap_or_else(|_| json_str.to_string())
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

        for (table, inbound) in [(&inlinks_table, true), (&outlinks_table, false)] {
            let callback = on_url_clicked.clone();
            let store = table.store.clone();
            table.view.connect_activate(move |_, position| {
                if let Some(item) = store.item(position).and_downcast::<glib::BoxedAnyObject>() {
                    let row = item.borrow::<LinkRow>();
                    let url = if inbound { &row.source } else { &row.target };
                    if let Some(callback) = callback.borrow().as_ref() {
                        callback(inspection::without_fragment(url));
                    }
                }
            });
        }

        // Wire up suggestion buttons
        let active_url_clone = active_url.clone();
        let crawl_state_clone = crawl_state.clone();
        let val_title_clone = val_title.clone();
        let btn_suggest_title_clone = btn_suggest_title.clone();
        btn_suggest_title.connect_clicked(move |_| {
            if let Some(url) = active_url_clone.borrow().as_ref() {
                if let Some(state) = crawl_state_clone.borrow().as_ref() {
                    suggest_title_with_ai(
                        url,
                        state,
                        val_title_clone.clone(),
                        btn_suggest_title_clone.clone(),
                    );
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
                    suggest_desc_with_ai(
                        url,
                        state,
                        val_desc_clone.clone(),
                        btn_suggest_desc_clone.clone(),
                    );
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
                        let root_window = notebook_weak
                            .upgrade()
                            .and_then(|nb| nb.root())
                            .and_downcast::<gtk::Window>();
                        export_links_to_csv(
                            inspection::inlinks(&res, state),
                            "inlinks.csv",
                            root_window.as_ref(),
                        );
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
                        let root_window = notebook_weak
                            .upgrade()
                            .and_then(|nb| nb.root())
                            .and_downcast::<gtk::Window>();
                        export_links_to_csv(
                            inspection::outlinks(&res),
                            "outlinks.csv",
                            root_window.as_ref(),
                        );
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
                        let root_window = notebook_weak
                            .upgrade()
                            .and_then(|nb| nb.root())
                            .and_downcast::<gtk::Window>();
                        export_images_to_csv(&res.images, root_window.as_ref());
                    }
                }
            }
        });

        // --- Tab 7: Markdown Preview ---
        let md_outer_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let md_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        md_toolbar.set_margin_start(10);
        md_toolbar.set_margin_end(10);
        md_toolbar.set_margin_top(6);
        md_toolbar.set_margin_bottom(6);

        let md_desc = gtk::Label::new(Some("Scraped subpage content in Markdown format"));
        md_desc.add_css_class("dim-label");
        md_desc.set_halign(gtk::Align::Start);
        md_desc.set_hexpand(true);
        md_desc.set_wrap(true);
        md_desc.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        md_desc.set_halign(gtk::Align::Fill);
        md_desc.set_xalign(0.);
        md_desc.set_width_chars(1);
        md_toolbar.append(&md_desc);

        let md_copy_btn = gtk::Button::builder()
            .label("Copy to Clipboard")
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy markdown content to clipboard")
            .sensitive(false)
            .build();
        md_toolbar.append(&md_copy_btn);

        let md_save_btn = gtk::Button::builder()
            .label("Save as...")
            .icon_name("document-save-symbolic")
            .tooltip_text("Save markdown to file")
            .sensitive(false)
            .build();
        md_toolbar.append(&md_save_btn);

        md_outer_box.append(&md_toolbar);

        let md_scroll = gtk::ScrolledWindow::new();
        md_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        md_scroll.set_vexpand(true);
        md_scroll.set_hexpand(true);

        let md_text_view = gtk::TextView::new();
        md_text_view.set_editable(false);
        md_text_view.set_monospace(true);
        md_text_view.set_wrap_mode(gtk::WrapMode::WordChar);
        md_text_view.set_margin_start(10);
        md_text_view.set_margin_end(10);
        md_text_view.set_margin_top(6);
        md_text_view.set_margin_bottom(6);
        md_scroll.set_child(Some(&md_text_view));
        md_outer_box.append(&md_scroll);

        notebook.append_page(&md_outer_box, Some(&gtk::Label::new(Some("Markdown"))));

        // Copy button — copies current raw text buffer content to clipboard
        let md_text_view_copy = md_text_view.clone();
        let md_copy_btn_clone = md_copy_btn.clone();
        md_copy_btn.connect_clicked(move |_| {
            let buf = md_text_view_copy.buffer();
            let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
            if !text.is_empty() {
                let display = gdk::Display::default().expect("Could not get GDK display");
                let clipboard = display.clipboard();
                clipboard.set_text(&text);

                md_copy_btn_clone.set_label("Copied!");
                md_copy_btn_clone.set_icon_name("emblem-ok-symbolic");
                let btn_reset = md_copy_btn_clone.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                    btn_reset.set_label("Copy to Clipboard");
                    btn_reset.set_icon_name("edit-copy-symbolic");
                });
            }
        });

        // Save button
        let md_text_view_save = md_text_view.clone();
        let active_url_save = active_url.clone();
        let notebook_weak = notebook.downgrade();
        md_save_btn.connect_clicked(move |_| {
            let buf = md_text_view_save.buffer();
            let text = buf
                .text(&buf.start_iter(), &buf.end_iter(), false)
                .to_string();
            if !text.is_empty() {
                let dialog = gtk::FileDialog::new();
                dialog.set_title("Save Markdown File");

                let default_name = if let Some(ref url) = *active_url_save.borrow() {
                    format!("{}.md", crate::crawler::get_slug_filename(url))
                } else {
                    "content.md".to_string()
                };
                dialog.set_initial_name(Some(&default_name));

                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Markdown Files"));
                filter.add_pattern("*.md");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                dialog.set_filters(Some(&filters));

                let root_window = notebook_weak
                    .upgrade()
                    .and_then(|nb| nb.root())
                    .and_downcast::<gtk::Window>();

                dialog.save(
                    root_window.as_ref(),
                    None::<&gio::Cancellable>,
                    move |result| {
                        if let Ok(file) = result {
                            if let Some(path) = file.path() {
                                let _ = std::fs::write(path, text);
                            }
                        }
                    },
                );
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
            inlinks_table,
            inlinks_lbl,
            outlinks_table,
            outlinks_lbl,
            images_table,
            images_lbl,
            social_previews,
            social_extra,
            headings_list,
            schema_all_lbl,
            schema_all_list,
            schema_list,
            schema_errors_list,
            schema_raw_text,
            md_text_view,
            md_copy_btn,
            md_save_btn,
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

        self.active_url.borrow_mut().take();
        self.crawl_state.borrow_mut().take();
        self.inlinks_table.store.remove_all();
        self.outlinks_table.store.remove_all();
        self.images_table.store.remove_all();
        self.inlinks_lbl
            .set_text("Inbound links pointing to this page");
        self.outlinks_lbl
            .set_text("Outbound links found on this page");
        self.images_lbl.set_text("Images found on this page");
        while let Some(child) = self.social_previews.first_child() {
            self.social_previews.remove(&child);
        }
        self.clear_list_box(&self.social_extra);
        self.clear_list_box(&self.headings_list);
        self.schema_all_lbl.set_visible(false);
        self.schema_all_list.set_visible(false);
        self.clear_list_box(&self.schema_all_list);
        self.clear_list_box(&self.schema_list);
        self.clear_list_box(&self.schema_errors_list);
        self.schema_raw_text.buffer().set_text("");
        self.md_text_view.buffer().set_text("");
        self.md_copy_btn.set_sensitive(false);
        self.md_save_btn.set_sensitive(false);
    }

    pub fn update(&self, url: &str, state: &CrawlState) {
        eprintln!("[Details Update] Updating for URL: {}", url);
        self.clear();
        *self.active_url.borrow_mut() = Some(url.to_string());
        *self.crawl_state.borrow_mut() = Some(state.clone());

        if let Some(res) = state.get_result(url) {
            eprintln!(
                "[Details Update] Found CrawlResult in state for URL: {}",
                url
            );
            // URL Info
            self.val_url.set_text(&res.url);
            self.val_status.set_text(
                &res.status_code
                    .map_or("Err/Unknown".to_string(), |c| c.to_string()),
            );
            self.val_index.set_text(&res.indexability_status);
            self.val_canonical
                .set_text(res.canonical.as_deref().unwrap_or("None"));

            // Social / OG
            self.val_og_title
                .set_text(res.og_title.as_deref().unwrap_or("None"));
            self.val_og_desc
                .set_text(res.og_description.as_deref().unwrap_or("None"));
            self.val_og_image
                .set_text(res.og_image.as_deref().unwrap_or("None"));
            self.val_og_url
                .set_text(res.og_url.as_deref().unwrap_or("None"));
            self.val_og_type
                .set_text(res.og_type.as_deref().unwrap_or("None"));
            self.val_tw_title
                .set_text(res.twitter_title.as_deref().unwrap_or("None"));
            self.val_tw_desc
                .set_text(res.twitter_description.as_deref().unwrap_or("None"));
            self.val_tw_image
                .set_text(res.twitter_image.as_deref().unwrap_or("None"));
            self.val_tw_card
                .set_text(res.twitter_card.as_deref().unwrap_or("None"));

            // Calculate diagnostics
            let mut diags = vec![];

            // Check missing essential OG tags
            if res
                .og_title
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
            {
                diags.push("Missing og:title tag.".to_string());
            } else if let Some(ref og_t) = res.og_title {
                if let Some(ref t) = res.title {
                    if og_t != t {
                        diags.push("Warning: og:title does not match HTML title tag.".to_string());
                    }
                }
            }

            if res
                .og_description
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
            {
                diags.push("Missing og:description tag.".to_string());
            } else if let Some(ref og_d) = res.og_description {
                if let Some(ref d) = res.meta_desc {
                    if og_d != d {
                        diags.push(
                            "Warning: og:description does not match HTML meta description."
                                .to_string(),
                        );
                    }
                }
            }

            if res
                .og_image
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
            {
                diags.push("Missing og:image tag.".to_string());
            }

            if res
                .twitter_card
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
            {
                diags.push("Missing twitter:card tag.".to_string());
            }

            if diags.is_empty() {
                self.val_social_diag
                    .set_text("Essential OG tags and a Twitter card type are present. Preview and verify the remaining fields.");
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
                self.val_title.set_text(&format!(
                    "{} ({} chars)",
                    res.title.as_deref().unwrap_or("None"),
                    t_len
                ));
                self.btn_suggest_title.set_visible(false);
            }

            // Description suggests
            if res.meta_desc.as_ref().map_or(true, |d| d.trim().is_empty()) {
                self.val_desc.set_text("[Missing Meta Description]");
                self.btn_suggest_desc.set_visible(true);
            } else {
                let d_len = res.meta_desc.as_ref().map_or(0, |d| d.chars().count());
                self.val_desc.set_text(&format!(
                    "{} ({} chars)",
                    res.meta_desc.as_deref().unwrap_or("None"),
                    d_len
                ));
                self.btn_suggest_desc.set_visible(false);
            }

            // H1 details
            let h1_text = if res.h1_count > 1 {
                format!(
                    "{} (Multiple tags: {} found)",
                    res.h1.as_deref().unwrap_or("None"),
                    res.h1_count
                )
            } else {
                res.h1.clone().unwrap_or_else(|| "None".to_string())
            };
            self.val_h1.set_text(&h1_text);

            // H2 details
            let h2_text = if res.h2_count > 1 {
                format!(
                    "{} (Multiple tags: {} found)",
                    res.h2.as_deref().unwrap_or("None"),
                    res.h2_count
                )
            } else {
                res.h2.clone().unwrap_or_else(|| "None".to_string())
            };
            self.val_h2.set_text(&h2_text);

            self.val_words.set_text(&res.word_count.to_string());
            self.val_size
                .set_text(&format!("{:.2} KB", res.size_bytes as f64 / 1024.0));

            let inbound = inspection::inlinks(&res, state);
            let outbound = inspection::outlinks(&res);
            self.inlinks_lbl.set_text(&format!(
                "{} inbound link occurrences · Activate a row to select its crawled source",
                inbound.len()
            ));
            self.outlinks_lbl.set_text(&format!(
                "{} outbound link occurrences · Activate a row to select its crawled target",
                outbound.len()
            ));
            self.images_lbl.set_text(&format!("{} images · {} missing alt · Previews show measured resolution; HTML size is declared", res.images.len(), res.images.iter().filter(|image| image.alt.is_none()).count()));
            self.inlinks_table.replace(inbound);
            self.outlinks_table.replace(outbound);
            self.images_table.replace(
                res.images
                    .iter()
                    .cloned()
                    .map(|image| ImageRow {
                        image,
                        page: url.to_string(),
                        state: state.clone(),
                    })
                    .collect(),
            );
            for twitter in [false, true] {
                self.social_previews.append(&social_card(&res, twitter));
            }
            for (key, value) in &res.social_metadata {
                if ![
                    "og:title",
                    "og:description",
                    "og:image",
                    "og:url",
                    "og:type",
                    "twitter:title",
                    "twitter:description",
                    "twitter:image",
                    "twitter:card",
                ]
                .contains(&key.as_str())
                {
                    Self::add_info_row(&self.social_extra, key).set_text(value);
                }
            }
            self.social_extra
                .set_visible(self.social_extra.first_child().is_some());

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
                        1 => {
                            badge.add_css_class("accent");
                            badge.add_css_class("bold");
                        }
                        2 => {
                            badge.add_css_class("bold");
                        }
                        _ => {
                            badge.add_css_class("dim-label");
                        }
                    }
                    row_box.append(&badge);

                    // Heading text
                    let text_lbl = gtk::Label::new(Some(&heading.text));
                    text_lbl.set_halign(gtk::Align::Start);
                    text_lbl.set_hexpand(true);
                    text_lbl.set_wrap(true);
                    text_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                    text_lbl.set_width_chars(1);
                    text_lbl.set_max_width_chars(1);
                    text_lbl.set_halign(gtk::Align::Fill);
                    text_lbl.set_xalign(0.);
                    text_lbl.set_selectable(true);
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
                        "All Blocks ({} total)",
                        res.schema_json_ld.len()
                    )));
                    title_lbl.set_halign(gtk::Align::Start);
                    title_lbl.set_wrap(true);
                    title_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                    title_lbl.set_width_chars(1);
                    title_lbl.set_hexpand(true);
                    title_lbl.set_halign(gtk::Align::Fill);
                    title_lbl.set_xalign(0.);
                    title_lbl.add_css_class("bold");
                    row_box.append(&title_lbl);

                    let sub_lbl = gtk::Label::new(Some("Combined view of every schema block"));
                    sub_lbl.set_halign(gtk::Align::Start);
                    sub_lbl.set_wrap(true);
                    sub_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                    sub_lbl.set_width_chars(1);
                    sub_lbl.set_hexpand(true);
                    sub_lbl.set_halign(gtk::Align::Fill);
                    sub_lbl.set_xalign(0.);
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

                    let title_lbl =
                        gtk::Label::new(Some(&format!("Schema Block #{} ({})", idx + 1, types)));
                    title_lbl.set_halign(gtk::Align::Start);
                    title_lbl.set_wrap(true);
                    title_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                    title_lbl.set_width_chars(1);
                    title_lbl.set_hexpand(true);
                    title_lbl.set_halign(gtk::Align::Fill);
                    title_lbl.set_xalign(0.);
                    title_lbl.add_css_class("bold");
                    row_box.append(&title_lbl);

                    let ctx_lbl = gtk::Label::new(Some(&format!("Context: {}", contexts)));
                    ctx_lbl.set_halign(gtk::Align::Start);
                    ctx_lbl.set_wrap(true);
                    ctx_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                    ctx_lbl.set_width_chars(1);
                    ctx_lbl.set_hexpand(true);
                    ctx_lbl.set_halign(gtk::Align::Fill);
                    ctx_lbl.set_xalign(0.);
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
                let success_lbl =
                    gtk::Label::new(Some("✓ No validation errors or warnings found."));
                success_lbl.add_css_class("success");
                success_lbl.set_halign(gtk::Align::Start);
                success_lbl.set_wrap(true);
                success_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                success_lbl.set_width_chars(1);
                success_lbl.set_hexpand(true);
                success_lbl.set_halign(gtk::Align::Fill);
                success_lbl.set_xalign(0.);
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
                    err_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                    err_lbl.set_width_chars(1);
                    err_lbl.set_hexpand(true);
                    err_lbl.set_halign(gtk::Align::Fill);
                    err_lbl.set_xalign(0.);
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

            // Markdown Preview Update
            if let Some(ref md_content) = res.markdown {
                self.md_text_view.buffer().set_text(md_content);
                self.md_copy_btn.set_sensitive(true);
                self.md_save_btn.set_sensitive(true);
            } else {
                self.md_text_view.buffer().set_text("");
                self.md_copy_btn.set_sensitive(false);
                self.md_save_btn.set_sensitive(false);
            }
        } else {
            eprintln!(
                "[Details Update] NO CrawlResult found in state for URL: {}",
                url
            );
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
        title_lbl.set_max_width_chars(20);
        title_lbl.set_wrap(true);
        title_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        title_lbl.set_valign(gtk::Align::Start);
        row_box.append(&title_lbl);

        let val_lbl = gtk::Label::new(None);
        val_lbl.set_halign(gtk::Align::Start);
        val_lbl.set_hexpand(true);
        val_lbl.set_selectable(true);
        val_lbl.set_wrap(true);
        val_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        val_lbl.set_halign(gtk::Align::Fill);
        val_lbl.set_xalign(0.);
        val_lbl.set_width_chars(1);
        val_lbl.set_max_width_chars(1);
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
        title_lbl.set_max_width_chars(20);
        title_lbl.set_wrap(true);
        title_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        title_lbl.set_valign(gtk::Align::Start);
        row_box.append(&title_lbl);

        let val_lbl = gtk::Label::new(None);
        val_lbl.set_halign(gtk::Align::Start);
        val_lbl.set_hexpand(true);
        val_lbl.set_selectable(true);
        val_lbl.set_wrap(true);
        val_lbl.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        val_lbl.set_halign(gtk::Align::Fill);
        val_lbl.set_xalign(0.);
        val_lbl.set_width_chars(1);
        val_lbl.set_max_width_chars(1);
        val_lbl.add_css_class("selectable");
        row_box.append(&val_lbl);

        let action_btn = gtk::Button::builder()
            .label("Suggest with AI")
            .visible(false)
            .build();
        action_btn.add_css_class("flat");
        row_box.append(&ai_action_group(&action_btn));

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

fn link_table() -> DetailTable {
    let columns = [
        ("Source URL", 0),
        ("Target URL", 1),
        ("Anchor text", 2),
        ("Rel", 3),
    ]
    .into_iter()
    .map(|(title, index)| {
        let render: super::detail_table::CellRenderer = Rc::new(move |item| {
            let row = item.borrow::<LinkRow>();
            let text = match index {
                0 => row.source.as_str(),
                1 => row.target.as_str(),
                2 => row
                    .anchor
                    .as_deref()
                    .map(|s| if s.is_empty() { "[Empty anchor]" } else { s })
                    .unwrap_or("Not recorded — recrawl page"),
                _ => row
                    .rel
                    .as_deref()
                    .map(|s| if s.is_empty() { "—" } else { s })
                    .unwrap_or("Not recorded"),
            };
            wrapped_label(text).upcast()
        });
        (title, if index == 3 { Some(105) } else { None }, render)
    })
    .collect();
    let table = DetailTable::new(columns);
    let empty = wrapped_label("No links recorded for this page.");
    empty.add_css_class("dim-label");
    table.set_empty_message(&empty.text());
    table
}

#[derive(Clone)]
struct ImageRow {
    image: ImageInfo,
    page: String,
    state: CrawlState,
}

fn image_table() -> DetailTable {
    let preview: super::detail_table::CellRenderer = Rc::new(|item| {
        let row = item.borrow::<ImageRow>();
        super::image_preview::widget(
            Some(row.image.src.clone()),
            row.image.local_path.clone(),
            72,
        )
        .upcast()
    });
    let source: super::detail_table::CellRenderer = Rc::new(|item| {
        let row = item.borrow::<ImageRow>();
        wrapped_label(&row.image.src).upcast()
    });
    let alt: super::detail_table::CellRenderer = Rc::new(|item| {
        let row = item.borrow::<ImageRow>();
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let label = wrapped_label(
            row.image
                .alt
                .as_deref()
                .unwrap_or("[Missing alt attribute]"),
        );
        if row.image.alt.is_none() {
            label.add_css_class("error");
        }
        cell.append(&label);
        let status = wrapped_label(inspection::alt_status(row.image.alt.as_deref()));
        status.add_css_class("dim-label");
        cell.append(&status);
        if row.image.alt.is_none() {
            let button = gtk::Button::with_label("Suggest Alt");
            button.add_css_class("flat");
            let row = row.clone();
            let label = label.clone();
            button.connect_clicked(move |button| {
                suggest_alt_text_with_ai(
                    &row.page,
                    &row.image.src,
                    &row.state,
                    label.clone(),
                    button.clone(),
                );
            });
            cell.append(&ai_action_group(&button));
        }
        cell.upcast()
    });
    let details: super::detail_table::CellRenderer = Rc::new(|item| {
        let row = item.borrow::<ImageRow>();
        let image = &row.image;
        let size = format!(
            "HTML size: {} × {}",
            image.width.map_or("?".into(), |v| v.to_string()),
            image.height.map_or("?".into(), |v| v.to_string())
        );
        let mut fields = vec![size];
        if image.width.is_none() && image.height.is_none() {
            fields[0] = "HTML size: not declared".into();
        }
        if let Some(title) = &image.title {
            fields.push(format!("Title: {title}"));
        }
        if let Some(loading) = &image.loading {
            fields.push(format!("Loading: {loading}"));
        }
        if let Some(path) = &image.local_path {
            fields.push(format!("Local file: {path}"));
        }
        wrapped_label(&fields.join("\n")).upcast()
    });
    let table = DetailTable::new(vec![
        ("Preview / resolution", Some(150), preview),
        ("Image URL", None, source),
        ("Alt text", None, alt),
        ("HTML details", None, details),
    ]);
    let empty = wrapped_label("No images recorded for this page.");
    empty.add_css_class("dim-label");
    table.set_empty_message(&empty.text());
    table
}

fn social_value(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.trim().is_empty())
}

fn social_card(page: &crate::state::CrawlResult, twitter: bool) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 4);
    card.set_hexpand(true);
    card.set_valign(gtk::Align::Start);
    card.set_homogeneous(false);
    let heading = wrapped_label(if twitter {
        "Twitter / X Card"
    } else {
        "Open Graph"
    });
    heading.add_css_class("heading");
    card.append(&heading);
    let title = if twitter {
        social_value(&page.twitter_title).or(social_value(&page.og_title))
    } else {
        social_value(&page.og_title)
    };
    let description = if twitter {
        social_value(&page.twitter_description).or(social_value(&page.og_description))
    } else {
        social_value(&page.og_description)
    };
    let image = if twitter {
        social_value(&page.twitter_image).or(social_value(&page.og_image))
    } else {
        social_value(&page.og_image)
    };
    let image_url = image.and_then(|value| inspection::preview_url(&page.url, value));
    let local = image_url
        .as_ref()
        .and_then(|url| page.images.iter().find(|image| &image.src == url))
        .and_then(|image| image.local_path.clone());
    card.append(&super::image_preview::widget(image_url, local, 144));
    let title = wrapped_label(
        title
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("[Missing social title]"),
    );
    title.add_css_class("title-4");
    card.append(&title);
    card.append(&wrapped_label(
        description
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("[Missing social description]"),
    ));
    let address = wrapped_label(social_value(&page.og_url).unwrap_or(&page.url));
    address.add_css_class("dim-label");
    card.append(&address);
    let image_alt = if twitter {
        page.social_metadata
            .get("twitter:image:alt")
            .or_else(|| page.social_metadata.get("og:image:alt"))
    } else {
        page.social_metadata.get("og:image:alt")
    };
    if let Some(alt) = image_alt {
        card.append(&wrapped_label(&format!("Image alt: {alt}")));
    }
    if twitter {
        card.append(&wrapped_label(&format!(
            "Card type: {}",
            social_value(&page.twitter_card).unwrap_or("Not declared")
        )));
        if social_value(&page.twitter_title).is_none()
            || social_value(&page.twitter_description).is_none()
            || social_value(&page.twitter_image).is_none()
        {
            let fallback = wrapped_label(
                "Missing Twitter fields use available Open Graph values in this preview.",
            );
            fallback.add_css_class("dim-label");
            card.append(&fallback);
        }
    }
    card
}

fn ai_action_group(button: &gtk::Button) -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Vertical, 4);
    group.append(button);
    let disclosure = gtk::Label::builder()
        .label("Sends page text to your AI provider")
        .wrap(true)
        .max_width_chars(20)
        .css_classes(["dim-label"])
        .build();
    group.append(&disclosure);
    button.set_tooltip_text(Some("Sends the page URL, metadata, headings and up to 12,000 characters of page text to the configured AI provider. This uses your API balance."));
    button
        .bind_property("visible", &group, "visible")
        .sync_create()
        .build();
    group
}

// AI helpers
fn suggest_title_with_ai(
    page_url: &str,
    state: &CrawlState,
    val_lbl: gtk::Label,
    btn: gtk::Button,
) {
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

    let page_url = state
        .get_result(page_url)
        .as_ref()
        .map(crate::ai::page_context)
        .unwrap_or_else(|| page_url.to_string());
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

    let page_url = state
        .get_result(page_url)
        .as_ref()
        .map(crate::ai::page_context)
        .unwrap_or_else(|| page_url.to_string());
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

fn suggest_alt_text_with_ai(
    page_url: &str,
    img_src: &str,
    state: &CrawlState,
    alt_lbl: gtk::Label,
    btn: gtk::Button,
) {
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

    let page_url = state
        .get_result(page_url)
        .as_ref()
        .map(crate::ai::page_context)
        .unwrap_or_else(|| page_url.to_string());
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

async fn call_ai_api(
    provider: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> crate::ai::ApiResult<String> {
    let config = crate::state::CrawlConfig {
        ai_provider: provider.into(),
        ai_api_key: Some(api_key.into()),
        ai_model: model.into(),
        ..Default::default()
    };
    crate::ai::AiClient::new()?.complete(&config, prompt).await
}

// Exports contain the same recorded values as the detail tables.
fn export_links_to_csv(rows: Vec<LinkRow>, filename: &str, parent: Option<&gtk::Window>) {
    let records = rows
        .into_iter()
        .map(|row| {
            vec![
                row.source,
                row.target,
                row.anchor.unwrap_or_default(),
                row.rel.unwrap_or_default(),
            ]
        })
        .collect();
    export_detail_csv(
        filename,
        &["Source URL", "Target URL", "Anchor Text", "Rel"],
        records,
        parent,
    );
}

fn export_images_to_csv(images: &[ImageInfo], parent: Option<&gtk::Window>) {
    let records = images
        .iter()
        .map(|image| {
            vec![
                image.src.clone(),
                image.alt.clone().unwrap_or_default(),
                inspection::alt_status(image.alt.as_deref()).into(),
                image.width.map(|v| v.to_string()).unwrap_or_default(),
                image.height.map(|v| v.to_string()).unwrap_or_default(),
                image.title.clone().unwrap_or_default(),
                image.loading.clone().unwrap_or_default(),
                image.local_path.clone().unwrap_or_default(),
            ]
        })
        .collect();
    export_detail_csv(
        "images.csv",
        &[
            "Image Source",
            "Alt Text",
            "Alt Status",
            "HTML Width",
            "HTML Height",
            "Title",
            "Loading",
            "Local Path",
        ],
        records,
        parent,
    );
}

fn export_detail_csv(
    filename: &str,
    headers: &[&str],
    records: Vec<Vec<String>>,
    parent: Option<&gtk::Window>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Export details to CSV");
    dialog.set_initial_name(Some(filename));
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("CSV Files"));
    filter.add_pattern("*.csv");
    dialog.set_default_filter(Some(&filter));
    let headers: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
    let parent = parent.cloned();
    dialog.save(parent.as_ref(), None::<&gio::Cancellable>, {
        let parent = parent.clone();
        move |result| {
            if let Ok(file) = result
                && let Some(path) = file.path()
            {
                let write = || -> Result<(), Box<dyn std::error::Error>> {
                    use std::io::Write;
                    let mut file = std::fs::File::create(path)?;
                    file.write_all(b"\xEF\xBB\xBF")?;
                    let mut writer = csv::Writer::from_writer(file);
                    writer.write_record(headers)?;
                    for record in records {
                        writer.write_record(record)?;
                    }
                    writer.flush()?;
                    Ok(())
                };
                if let Err(error) = write() {
                    let dialog = adw::AlertDialog::builder()
                        .heading("Could not export details")
                        .body(format!(
                            "{error}. Choose a writable location and try again."
                        ))
                        .build();
                    dialog.add_response("ok", "OK");
                    dialog.present(parent.as_ref());
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
                v.get("@type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("Unknown")
                    .to_string()
            }
        };

        let get_contexts = |v: &serde_json::Value| -> String {
            if let Some(arr) = v.as_array() {
                arr.iter()
                    .filter_map(|item| item.get("@context").and_then(|c| c.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                v.get("@context")
                    .and_then(|c| c.as_str())
                    .unwrap_or("schema.org")
                    .to_string()
            }
        };

        if let Some(arr) = val.as_array() {
            if !arr.is_empty() {
                let types = arr
                    .iter()
                    .map(|item| get_types(item))
                    .collect::<Vec<_>>()
                    .join(" + ");
                let contexts = arr
                    .iter()
                    .map(|item| get_contexts(item))
                    .collect::<Vec<_>>()
                    .join(" + ");
                return (types, contexts);
            }
        } else if val.is_object() {
            return (get_types(&val), get_contexts(&val));
        }
    }
    ("Unknown (Invalid JSON)".to_string(), "Unknown".to_string())
}

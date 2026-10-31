use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::state::CrawlState;
use crate::visualization::export::*;
use crate::visualization::icicle::IcicleTreeNode;
use crate::visualization::*;

pub struct VisualizerState {
    pub icicle_metric: IcicleWeightMetric,
    pub module: DomainVisualizationModule,
    pub filter: GraphFilter,
    pub search_query: String,
    pub selected_node_id: Option<NodeId>,
    pub matching_nodes: HashSet<NodeId>,

    // Structure Map Interactivity & Drill-down State
    pub icicle_zoom_path: Option<String>,
    pub hovered_icicle_path: Option<String>,
}

impl Default for VisualizerState {
    fn default() -> Self {
        Self {
            icicle_metric: IcicleWeightMetric::UrlCount,
            module: DomainVisualizationModule::new(),
            filter: GraphFilter::default(),
            search_query: String::new(),
            selected_node_id: None,
            matching_nodes: HashSet::new(),

            icicle_zoom_path: None,
            hovered_icicle_path: None,
        }
    }
}

pub struct VisualizerWindow {
    window: gtk::Window,
    pub state: Rc<RefCell<VisualizerState>>,
    pub crawl_state: CrawlState,
    pub drawing_area: gtk::DrawingArea,
    selection_callback: Rc<RefCell<Option<Box<dyn Fn(Option<String>) + 'static>>>>,
}

impl VisualizerWindow {
    pub fn new(
        parent: &(impl IsA<gtk::Window> + IsA<glib::Object>),
        crawl_state: CrawlState,
    ) -> Self {
        // Fully Responsive Default Window Sizing (Allows shrinking down to 800px width)
        let window = gtk::Window::builder()
            .title("Domain Structure Visualizer — Directory Icicle Plot")
            .transient_for(parent)
            .modal(false)
            .default_width(1080)
            .default_height(680)
            .build();

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        main_box.set_margin_start(4);
        main_box.set_margin_end(4);
        main_box.set_margin_top(4);
        main_box.set_margin_bottom(4);

        let state = Rc::new(RefCell::new(VisualizerState::default()));
        let selection_callback: Rc<RefCell<Option<Box<dyn Fn(Option<String>) + 'static>>>> =
            Rc::new(RefCell::new(None));

        // Build snapshot from crawl state
        {
            let results = crawl_state.get_all_results();
            let snapshot = SnapshotBuilder::build_from_results(&results);
            let mut st = state.borrow_mut();
            st.module.load_snapshot(snapshot);
            let config = PageRankConfig::default();
            st.module.compute_internal_rank(&config);
        }

        // Header Toolbar Box
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);

        let weight_lbl = gtk::Label::new(Some("Weight by:"));
        toolbar.append(&weight_lbl);

        let weight_combo = gtk::DropDown::from_strings(&[
            "URL Count",
            "Page Size (Bytes)",
            "Word Count",
            "HTTP Errors Only",
            "Internal Rank Sum",
            "Response Time (ms)",
        ]);
        toolbar.append(&weight_combo);

        // Directory Tree Up Button (Visible when zoomed into a sub-folder)
        let icicle_up_btn = gtk::Button::builder()
            .label("⬆ Up Directory")
            .tooltip_text("Zoom out to parent directory level")
            .visible(false)
            .build();
        toolbar.append(&icicle_up_btn);

        let reset_btn = gtk::Button::builder()
            .label("Reset Zoom")
            .tooltip_text("Reset to Root /")
            .build();
        toolbar.append(&reset_btn);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Search Directory...")
            .width_request(140)
            .hexpand(false)
            .build();
        toolbar.append(&search_entry);

        // Export Menu Button & Popover
        let export_popover = gtk::Popover::new();
        let export_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        export_box.set_margin_start(8);
        export_box.set_margin_end(8);
        export_box.set_margin_top(8);
        export_box.set_margin_bottom(8);

        let export_csv_nodes_btn = gtk::Button::with_label("📊 Export Nodes to CSV");
        let export_csv_edges_btn = gtk::Button::with_label("🔗 Export Edges to CSV");
        let export_json_btn = gtk::Button::with_label("🌐 Export Subgraph JSON");
        let copy_url_btn = gtk::Button::with_label("📋 Copy Selected URL");

        export_box.append(&export_csv_nodes_btn);
        export_box.append(&export_csv_edges_btn);
        export_box.append(&export_json_btn);
        export_box.append(&copy_url_btn);
        export_popover.set_child(Some(&export_box));

        let export_menu_btn = gtk::MenuButton::builder()
            .label("Export ▾")
            .popover(&export_popover)
            .build();
        toolbar.append(&export_menu_btn);

        // Compact Legend Label
        let legend_lbl =
            gtk::Label::new(Some("● 2xx OK  ● 3xx Redirect  ● 4xx/5xx Error  ● Noindex"));
        legend_lbl.add_css_class("dim-label");
        legend_lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        legend_lbl.set_hexpand(true);
        legend_lbl.set_halign(gtk::Align::End);
        toolbar.append(&legend_lbl);

        main_box.append(&toolbar);

        // Paned Split Layout: Canvas (Flexible) & Details Panel (Responsive 240px)
        let split_paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        split_paned.set_position(740);
        split_paned.set_wide_handle(true);
        split_paned.set_vexpand(true);
        split_paned.set_hexpand(true);
        split_paned.set_shrink_start_child(true);
        split_paned.set_shrink_end_child(true);
        main_box.append(&split_paned);

        // Drawing Canvas Area
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_vexpand(true);
        drawing_area.set_hexpand(true);
        split_paned.set_start_child(Some(&drawing_area));

        // Details Panel Box (Right Sidebar)
        let details_sidebar_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        details_sidebar_box.set_margin_start(6);
        details_sidebar_box.set_margin_end(6);
        details_sidebar_box.set_margin_top(6);
        details_sidebar_box.set_margin_bottom(6);
        details_sidebar_box.set_width_request(240);

        let sidebar_title = gtk::Label::new(Some("Directory Details & Structure"));
        sidebar_title.add_css_class("bold");
        sidebar_title.set_halign(gtk::Align::Start);
        details_sidebar_box.append(&sidebar_title);

        let details_scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        let details_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        details_scroll.set_child(Some(&details_content));
        details_sidebar_box.append(&details_scroll);

        split_paned.set_end_child(Some(&details_sidebar_box));

        window.set_child(Some(&main_box));

        // Status bar
        let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        status_bar.set_margin_start(6);
        status_bar.set_margin_end(6);
        status_bar.set_margin_top(2);
        status_bar.set_margin_bottom(2);
        let status_text =
            gtk::Label::new(Some("Ready | Click directory block to drill down / zoom"));
        status_text.add_css_class("dim-label");
        status_bar.append(&status_text);
        main_box.append(&status_bar);

        // Connect Draw Event
        let state_draw = state.clone();
        drawing_area.set_draw_func(move |_area, cr, width, height| {
            let mut st = state_draw.borrow_mut();
            Self::draw_canvas(&mut st, cr, width as f64, height as f64);
        });

        // Motion Handler for On-Hover Highlighting
        let gesture_motion = gtk::EventControllerMotion::new();
        let state_motion = state.clone();
        let area_motion = drawing_area.clone();
        let status_text_motion = status_text.clone();

        gesture_motion.connect_motion(move |_controller, x, y| {
            let width = area_motion.width() as f64;
            let height = area_motion.height() as f64;
            let mut st = state_motion.borrow_mut();

            if let Some(snapshot) = st.module.get_snapshot() {
                let root = build_icicle_tree_from_snapshot(snapshot, st.icicle_metric);
                let target_root = st
                    .icicle_zoom_path
                    .as_ref()
                    .and_then(|p| root.find_by_path(p))
                    .unwrap_or(&root);

                if let Some(hit) = Self::hit_test_icicle(
                    target_root,
                    x,
                    y,
                    20.0,
                    30.0,
                    width - 40.0,
                    height - 60.0,
                    0,
                ) {
                    if st.hovered_icicle_path.as_deref() != Some(&hit.full_path) {
                        st.hovered_icicle_path = Some(hit.full_path.clone());
                        status_text_motion.set_text(&format!(
                            "Directory: {} | Total Pages: {} | Errors: {}",
                            hit.full_path, hit.total_pages, hit.error_count
                        ));
                        area_motion.queue_draw();
                    }
                    return;
                }
            }
            if st.hovered_icicle_path.is_some() {
                st.hovered_icicle_path = None;
                area_motion.queue_draw();
            }
        });
        drawing_area.add_controller(gesture_motion);

        // Click Gesture: Drill Down / Zoom into directory
        let gesture_click = gtk::GestureClick::new();
        let state_click = state.clone();
        let area_click = drawing_area.clone();
        let cb_click = selection_callback.clone();
        let details_content_click = details_content.clone();
        let status_text_click = status_text.clone();
        let icicle_up_btn_c = icicle_up_btn.clone();

        gesture_click.connect_pressed(move |_gesture, _n, x, y| {
            let width = area_click.width() as f64;
            let height = area_click.height() as f64;
            let mut st = state_click.borrow_mut();

            if let Some(snapshot) = st.module.get_snapshot() {
                let root = build_icicle_tree_from_snapshot(snapshot, st.icicle_metric);
                let target_root = st
                    .icicle_zoom_path
                    .as_ref()
                    .and_then(|p| root.find_by_path(p))
                    .unwrap_or(&root);

                if let Some(hit) = Self::hit_test_icicle(
                    target_root,
                    x,
                    y,
                    20.0,
                    30.0,
                    width - 40.0,
                    height - 60.0,
                    0,
                ) {
                    // Zoom / Drill Down into clicked directory!
                    st.icicle_zoom_path = Some(hit.full_path.clone());
                    icicle_up_btn_c.set_visible(true);

                    if let Some(ref cb) = *cb_click.borrow() {
                        cb(Some(hit.full_path.clone()));
                    }

                    Self::update_details_sidebar_icicle(
                        &details_content_click,
                        hit,
                        &state_click,
                        &area_click,
                    );

                    status_text_click.set_text(&format!(
                        "Drilled down to: {} ({} pages)",
                        hit.full_path, hit.total_pages
                    ));
                    area_click.queue_draw();
                }
            }
        });
        drawing_area.add_controller(gesture_click);

        // Weight Metric Combo Handler
        let state_weight = state.clone();
        let area_weight = drawing_area.clone();
        weight_combo.connect_selected_notify(move |combo| {
            let mut st = state_weight.borrow_mut();
            st.icicle_metric = match combo.selected() {
                0 => IcicleWeightMetric::UrlCount,
                1 => IcicleWeightMetric::PageSizeBytes,
                2 => IcicleWeightMetric::WordCount,
                3 => IcicleWeightMetric::IssueCount,
                4 => IcicleWeightMetric::InternalRankSum,
                _ => IcicleWeightMetric::ResponseTimeMs,
            };
            area_weight.queue_draw();
        });

        // Icicle Up Button Handler (Drill back up one directory level)
        let state_up = state.clone();
        let area_up = drawing_area.clone();
        let btn_up = icicle_up_btn.clone();
        icicle_up_btn.connect_clicked(move |_| {
            let mut st = state_up.borrow_mut();
            if let Some(ref path) = st.icicle_zoom_path.clone() {
                let trimmed = path.trim_end_matches('/');
                if let Some(last_slash) = trimmed.rfind('/') {
                    let parent = &trimmed[..=last_slash];
                    if parent.is_empty() || parent == "/" {
                        st.icicle_zoom_path = None;
                        btn_up.set_visible(false);
                    } else {
                        st.icicle_zoom_path = Some(parent.to_string());
                    }
                } else {
                    st.icicle_zoom_path = None;
                    btn_up.set_visible(false);
                }
            } else {
                btn_up.set_visible(false);
            }
            area_up.queue_draw();
        });

        // Reset Zoom Button Handler
        let state_reset = state.clone();
        let area_reset = drawing_area.clone();
        let btn_up_reset = icicle_up_btn.clone();
        reset_btn.connect_clicked(move |_| {
            let mut st = state_reset.borrow_mut();
            st.icicle_zoom_path = None;
            btn_up_reset.set_visible(false);
            area_reset.queue_draw();
        });

        // Export CSV Nodes Button
        let state_exp_nodes = state.clone();
        let parent_win_nodes = window.clone();
        export_csv_nodes_btn.connect_clicked(move |_| {
            let st = state_exp_nodes.borrow();
            if let Some(snapshot) = st.module.get_snapshot() {
                let dialog = gtk::FileDialog::new();
                dialog.set_title("Export Nodes to CSV");
                dialog.set_initial_name(Some("nodes_export.csv"));

                let snap = snapshot.clone();
                dialog.save(
                    Some(&parent_win_nodes),
                    gio::Cancellable::NONE,
                    move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                let _ = export_nodes_to_csv(&snap, &path.to_string_lossy());
                            }
                        }
                    },
                );
            }
        });

        // Export CSV Edges Button
        let state_exp_edges = state.clone();
        let parent_win_edges = window.clone();
        export_csv_edges_btn.connect_clicked(move |_| {
            let st = state_exp_edges.borrow();
            if let Some(snapshot) = st.module.get_snapshot() {
                let dialog = gtk::FileDialog::new();
                dialog.set_title("Export Edges to CSV");
                dialog.set_initial_name(Some("edges_export.csv"));

                let snap = snapshot.clone();
                dialog.save(
                    Some(&parent_win_edges),
                    gio::Cancellable::NONE,
                    move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                let _ = export_edges_to_csv(&snap, &path.to_string_lossy());
                            }
                        }
                    },
                );
            }
        });

        // Export Subgraph JSON Button
        let state_exp_json = state.clone();
        let parent_win_json = window.clone();
        export_json_btn.connect_clicked(move |_| {
            let st = state_exp_json.borrow();
            if let Some(snapshot) = st.module.get_snapshot() {
                let dialog = gtk::FileDialog::new();
                dialog.set_title("Export Subgraph to JSON");
                dialog.set_initial_name(Some("subgraph_export.json"));

                let json_str = export_subgraph_json(snapshot, st.selected_node_id);
                dialog.save(Some(&parent_win_json), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            let _ = std::fs::write(path, &json_str);
                        }
                    }
                });
            }
        });

        // Copy Selected URL Button
        let state_copy = state.clone();
        let window_copy = window.clone();
        copy_url_btn.connect_clicked(move |_| {
            let st = state_copy.borrow();
            if let Some(snapshot) = st.module.get_snapshot() {
                if let Some(node_id) = st.selected_node_id {
                    if let Some(node) = snapshot.nodes.get(node_id as usize) {
                        let url = snapshot.node_url(node);
                        let clipboard = window_copy.clipboard();
                        clipboard.set_text(url);
                    }
                }
            }
        });

        // Search entry handler
        let state_search = state.clone();
        let area_search = drawing_area.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            let mut st = state_search.borrow_mut();
            st.search_query = query;
            area_search.queue_draw();
        });

        Self {
            window,
            state,
            crawl_state,
            drawing_area,
            selection_callback,
        }
    }

    pub fn connect_selection_changed<F: Fn(Option<String>) + 'static>(&self, f: F) {
        *self.selection_callback.borrow_mut() = Some(Box::new(f));
    }

    pub fn show(&self) {
        self.window.present();
    }

    fn update_details_sidebar_icicle(
        container: &gtk::Box,
        icicle_node: &IcicleTreeNode,
        _state_rc: &Rc<RefCell<VisualizerState>>,
        _area: &gtk::DrawingArea,
    ) {
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }

        let grp = gtk::Box::new(gtk::Orientation::Vertical, 4);

        let path_lbl = gtk::Label::new(Some(&format!("Directory: {}", icicle_node.full_path)));
        path_lbl.add_css_class("bold");
        path_lbl.set_wrap(true);
        path_lbl.set_halign(gtk::Align::Start);
        grp.append(&path_lbl);

        let stat_str = format!(
            "Total Pages: {} | Indexable: {}\nHTTP Errors (4xx/5xx): {}",
            icicle_node.total_pages, icicle_node.indexable_pages, icicle_node.error_count
        );
        let stat_lbl = gtk::Label::new(Some(&stat_str));
        stat_lbl.set_halign(gtk::Align::Start);
        grp.append(&stat_lbl);

        container.append(&grp);
        container.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let sub_title = gtk::Label::new(Some("Sub-directories"));
        sub_title.add_css_class("bold");
        sub_title.set_halign(gtk::Align::Start);
        container.append(&sub_title);

        if icicle_node.children.is_empty() {
            let no_child = gtk::Label::new(Some(
                "No deeper sub-directories. Click '⬆ Up Directory' to zoom out.",
            ));
            no_child.add_css_class("dim-label");
            no_child.set_wrap(true);
            container.append(&no_child);
        } else {
            let child_box = gtk::Box::new(gtk::Orientation::Vertical, 3);
            for child in &icicle_node.children {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                let lbl = gtk::Label::new(Some(&format!(
                    "📁 {} ({} pages)",
                    child.name, child.total_pages
                )));
                lbl.set_halign(gtk::Align::Start);
                row.append(&lbl);
                child_box.append(&row);
            }
            container.append(&child_box);
        }
    }

    fn hit_test_icicle<'a>(
        node: &'a IcicleTreeNode,
        click_x: f64,
        click_y: f64,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        depth: usize,
    ) -> Option<&'a IcicleTreeNode> {
        if w < 2.0 || h < 4.0 {
            return None;
        }

        let level_h = 40.0;
        let curr_y = y + (depth as f64 * (level_h + 4.0));

        if curr_y + level_h > y + h {
            return None;
        }

        if click_x >= x && click_x <= x + w && click_y >= curr_y && click_y <= curr_y + level_h {
            return Some(node);
        }

        if node.children.is_empty() || node.weight_value <= 0.0 {
            return None;
        }

        let total_weight = node.weight_value;
        let mut curr_x = x;

        for child in &node.children {
            let child_w = (child.weight_value / total_weight) * w;
            if let Some(hit) =
                Self::hit_test_icicle(child, click_x, click_y, curr_x, y, child_w, h, depth + 1)
            {
                return Some(hit);
            }
            curr_x += child_w;
        }

        None
    }

    fn draw_canvas(st: &mut VisualizerState, cr: &cairo::Context, width: f64, height: f64) {
        // Background
        cr.set_source_rgb(0.08, 0.09, 0.11); // Dark background
        cr.paint().unwrap();

        Self::draw_structure_map(st, cr, width, height);
    }

    fn draw_structure_map(st: &VisualizerState, cr: &cairo::Context, width: f64, height: f64) {
        let snapshot = match st.module.get_snapshot() {
            Some(s) => s,
            None => return,
        };

        let icicle_root = build_icicle_tree_from_snapshot(snapshot, st.icicle_metric);
        let target_root = st
            .icicle_zoom_path
            .as_ref()
            .and_then(|p| icicle_root.find_by_path(p))
            .unwrap_or(&icicle_root);

        let h_path = st.hovered_icicle_path.as_deref();
        let z_path = st.icicle_zoom_path.as_deref();

        Self::draw_icicle_node(
            target_root,
            cr,
            20.0,
            30.0,
            width - 40.0,
            height - 60.0,
            0,
            h_path,
            z_path,
        );
    }

    fn draw_icicle_node(
        node: &IcicleTreeNode,
        cr: &cairo::Context,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        depth: usize,
        hovered_path: Option<&str>,
        selected_path: Option<&str>,
    ) {
        if w < 2.0 || h < 4.0 {
            return;
        }

        let level_h = 40.0;
        let curr_y = y + (depth as f64 * (level_h + 4.0));

        if curr_y + level_h > y + h {
            return;
        }

        let is_hovered = hovered_path.map_or(false, |p| p == node.full_path);
        let is_selected = selected_path.map_or(false, |p| p == node.full_path);

        // Color based on hover, error count & depth
        if is_hovered {
            cr.set_source_rgb(0.3, 0.7, 1.0); // Bright Blue on Hover
        } else if node.error_count > 0 {
            cr.set_source_rgb(0.85, 0.25, 0.25);
        } else if depth % 2 == 0 {
            cr.set_source_rgb(0.2, 0.45, 0.7);
        } else {
            cr.set_source_rgb(0.25, 0.55, 0.8);
        }

        cr.rectangle(x, curr_y, w, level_h);
        cr.fill().unwrap();

        // Stroke outline
        if is_hovered || is_selected {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.set_line_width(2.5);
        } else {
            cr.set_source_rgb(0.08, 0.09, 0.11);
            cr.set_line_width(1.0);
        }
        cr.rectangle(x, curr_y, w, level_h);
        cr.stroke().unwrap();

        if w > 25.0 {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.set_font_size(11.0);
            cr.move_to(x + 4.0, curr_y + 22.0);
            let label = format!("{} ({})", node.name, node.total_pages);
            cr.show_text(&label).unwrap();
        }

        if node.children.is_empty() || node.weight_value <= 0.0 {
            return;
        }

        let total_weight = node.weight_value;
        let mut curr_x = x;

        for child in &node.children {
            let child_w = (child.weight_value / total_weight) * w;
            Self::draw_icicle_node(
                child,
                cr,
                curr_x,
                y,
                child_w,
                h,
                depth + 1,
                hovered_path,
                selected_path,
            );
            curr_x += child_w;
        }
    }
}

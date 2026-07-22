use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::state::{CrawlResult, CrawlState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VisualizerViewMode {
    DirectoryTree,
    ForceDirectedFolders,
    ThreeDConstellation,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LabelDisplayMode {
    SmartAuto,
    HoverOnly,
    ShowAll,
}

/// Recursive directory tree node for hierarchical path parsing
struct DirectoryTreeNode {
    name: String,
    full_path: String,
    subfolders: HashMap<String, DirectoryTreeNode>,
    pages: Vec<CrawlResult>,
    total_pages_count: usize,
    error_count: usize,
}

impl DirectoryTreeNode {
    fn new(name: String, full_path: String) -> Self {
        Self {
            name,
            full_path,
            subfolders: HashMap::new(),
            pages: Vec::new(),
            total_pages_count: 0,
            error_count: 0,
        }
    }

    fn insert_url(&mut self, segments: &[&str], seg_idx: usize, res: &CrawlResult) {
        self.total_pages_count += 1;
        if let Some(code) = res.status_code {
            if code >= 400 {
                self.error_count += 1;
            }
        }

        if seg_idx >= segments.len() {
            self.pages.push(res.clone());
            return;
        }

        let seg = segments[seg_idx];
        let is_last = seg_idx == segments.len() - 1;

        if is_last && (seg.contains('.') || !res.url.ends_with('/')) {
            self.pages.push(res.clone());
        } else {
            let child_full_path = if self.full_path == "/" {
                format!("/{}/", seg)
            } else {
                format!("{}{}/", self.full_path, seg)
            };

            let child_node = self
                .subfolders
                .entry(seg.to_string())
                .or_insert_with(|| DirectoryTreeNode::new(seg.to_string(), child_full_path));

            child_node.insert_url(segments, seg_idx + 1, res);
        }
    }
}

#[derive(Clone)]
struct RenderNode {
    is_folder: bool,
    label: String,
    full_url: Option<String>,
    folder_path: String,
    error_count: usize,
    status_code: Option<u16>,
    indexable: bool,
    // 2D Layout coordinates
    x: f64,
    y: f64,
    // 3D Spatial coordinates
    x_3d: f64,
    y_3d: f64,
    z_3d: f64,
    // Computed 3D Projected coordinates
    proj_x: f64,
    proj_y: f64,
    proj_z: f64,
    proj_scale: f64,
    radius: f64,
    pages_count: usize,
    _is_expanded: bool,
    _subfolder_count: usize,
    _direct_pages_count: usize,
}

#[derive(Clone)]
struct RenderEdge {
    source_idx: usize,
    target_idx: usize,
}

struct VisualizerState {
    mode: VisualizerViewMode,
    label_mode: LabelDisplayMode,
    nodes: Vec<RenderNode>,
    edges: Vec<RenderEdge>,
    expanded_folders: HashSet<String>,
    selected_node: Option<usize>,
    hovered_node: Option<usize>,
    search_query: String,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
    // 3D Camera Controls
    yaw: f64,
    pitch: f64,
    camera_dist: f64,
    total_crawl_pages: usize,
}

impl Default for VisualizerState {
    fn default() -> Self {
        let mut expanded = HashSet::new();
        expanded.insert("/".to_string()); // Root expanded by default
        Self {
            mode: VisualizerViewMode::DirectoryTree,
            label_mode: LabelDisplayMode::SmartAuto,
            nodes: Vec::new(),
            edges: Vec::new(),
            expanded_folders: expanded,
            selected_node: None,
            hovered_node: None,
            search_query: String::new(),
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            yaw: 0.4,
            pitch: 0.3,
            camera_dist: 600.0,
            total_crawl_pages: 0,
        }
    }
}

pub struct VisualizerWindow {
    window: gtk::Window,
    state: Rc<RefCell<VisualizerState>>,
    crawl_state: CrawlState,
    drawing_area: gtk::DrawingArea,
    selection_callback: Rc<RefCell<Option<Box<dyn Fn(Option<String>) + 'static>>>>,
}

impl VisualizerWindow {
    pub fn new(parent: &(impl IsA<gtk::Window> + IsA<glib::Object>), crawl_state: CrawlState) -> Self {
        let window = gtk::Window::builder()
            .title("Visual Site Map — Directory & TensorFlow 3D Projector")
            .transient_for(parent)
            .modal(false)
            .default_width(1250)
            .default_height(820)
            .build();

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        main_box.set_margin_start(10);
        main_box.set_margin_end(10);
        main_box.set_margin_top(10);
        main_box.set_margin_bottom(10);

        let state = Rc::new(RefCell::new(VisualizerState::default()));
        let selection_callback: Rc<RefCell<Option<Box<dyn Fn(Option<String>) + 'static>>>> =
            Rc::new(RefCell::new(None));

        // Header Toolbar
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);

        let mode_lbl = gtk::Label::new(Some("Diagram:"));
        toolbar.append(&mode_lbl);

        let mode_combo = gtk::DropDown::from_strings(&[
            "Directory Tree (Recursive)",
            "Force-Directed Directory Hubs",
            "3D Constellation (TensorFlow Projector)",
        ]);
        toolbar.append(&mode_combo);

        let label_lbl = gtk::Label::new(Some("Labels:"));
        toolbar.append(&label_lbl);

        let label_combo = gtk::DropDown::from_strings(&[
            "Smart Auto (Clean)",
            "Hover & Selected Only",
            "Show All Labels",
        ]);
        toolbar.append(&label_combo);

        let zoom_in_btn = gtk::Button::builder().label("+ Zoom").build();
        let zoom_out_btn = gtk::Button::builder().label("- Zoom").build();
        let reset_btn = gtk::Button::builder().label("Fit View").build();
        let expand_all_btn = gtk::Button::builder().label("Expand All").build();
        let collapse_all_btn = gtk::Button::builder().label("Collapse All").build();

        toolbar.append(&zoom_in_btn);
        toolbar.append(&zoom_out_btn);
        toolbar.append(&reset_btn);
        toolbar.append(&expand_all_btn);
        toolbar.append(&collapse_all_btn);

        let search_entry = gtk::SearchEntry::builder()
            .placeholder_text("Filter folders/URLs...")
            .width_request(180)
            .build();
        toolbar.append(&search_entry);

        let legend_lbl = gtk::Label::new(Some("📁 Folder  ● 2xx Page  ● 3xx Redirect  ● 4xx/5xx Error"));
        legend_lbl.add_css_class("dim-label");
        legend_lbl.set_hexpand(true);
        legend_lbl.set_halign(gtk::Align::End);
        toolbar.append(&legend_lbl);

        main_box.append(&toolbar);

        // Drawing Canvas
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_content_width(1150);
        drawing_area.set_content_height(720);
        drawing_area.set_vexpand(true);
        drawing_area.set_hexpand(true);
        main_box.append(&drawing_area);

        window.set_child(Some(&main_box));

        let state_draw = state.clone();
        drawing_area.set_draw_func(move |_area, cr, width, height| {
            let mut st = state_draw.borrow_mut();
            Self::draw_visualization(&mut st, cr, width as f64, height as f64);
        });

        // Mouse click handler
        let gesture_click = gtk::GestureClick::new();
        let state_click = state.clone();
        let area_click = drawing_area.clone();
        let crawl_click = crawl_state.clone();
        let cb_click = selection_callback.clone();

        gesture_click.connect_pressed(move |_gesture, _n, x, y| {
            let width = area_click.width() as f64;
            let height = area_click.height() as f64;
            let mut st = state_click.borrow_mut();

            if let Some(clicked_idx) = Self::hit_test(&st, x, y, width, height) {
                st.selected_node = Some(clicked_idx);
                let node = st.nodes[clicked_idx].clone();

                if node.is_folder {
                    if st.expanded_folders.contains(&node.folder_path) {
                        st.expanded_folders.remove(&node.folder_path);
                    } else {
                        st.expanded_folders.insert(node.folder_path.clone());
                    }
                    Self::rebuild_layout(&mut st, &crawl_click);
                } else if let Some(ref url) = node.full_url {
                    if let Some(ref cb) = *cb_click.borrow() {
                        cb(Some(url.clone()));
                    }
                }
                area_click.queue_draw();
            } else {
                st.selected_node = None;
                area_click.queue_draw();
            }
        });

        // Drag Handler
        let gesture_drag = gtk::GestureDrag::new();
        let state_drag = state.clone();
        let area_drag = drawing_area.clone();

        gesture_drag.connect_drag_update(move |_gesture, offset_x, offset_y| {
            let mut st = state_drag.borrow_mut();
            if st.mode == VisualizerViewMode::ThreeDConstellation {
                st.yaw += offset_x * 0.005;
                st.pitch = (st.pitch + offset_y * 0.005).clamp(-1.4, 1.4);
            } else {
                st.pan_x += offset_x * 0.15;
                st.pan_y += offset_y * 0.15;
            }
            area_drag.queue_draw();
        });

        let scroll_controller =
            gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let state_scroll = state.clone();
        let area_scroll = drawing_area.clone();

        scroll_controller.connect_scroll(move |_ctrl, _dx, dy| {
            let mut st = state_scroll.borrow_mut();
            if st.mode == VisualizerViewMode::ThreeDConstellation {
                if dy < 0.0 {
                    st.camera_dist = (st.camera_dist * 0.9).max(200.0);
                } else {
                    st.camera_dist = (st.camera_dist * 1.1).min(2000.0);
                }
            } else if dy < 0.0 {
                st.zoom = (st.zoom * 1.15).min(4.0);
            } else {
                st.zoom = (st.zoom / 1.15).max(0.2);
            }
            area_scroll.queue_draw();
            glib::Propagation::Stop
        });

        let motion_controller = gtk::EventControllerMotion::new();
        let state_motion = state.clone();
        let area_motion = drawing_area.clone();

        motion_controller.connect_motion(move |_ctrl, x, y| {
            let width = area_motion.width() as f64;
            let height = area_motion.height() as f64;
            let mut st = state_motion.borrow_mut();
            let prev_hover = st.hovered_node;
            st.hovered_node = Self::hit_test(&st, x, y, width, height);
            if prev_hover != st.hovered_node {
                area_motion.queue_draw();
            }
        });

        drawing_area.add_controller(gesture_click);
        drawing_area.add_controller(gesture_drag);
        drawing_area.add_controller(scroll_controller);
        drawing_area.add_controller(motion_controller);

        // Toolbar Handlers
        let state_mode = state.clone();
        let area_mode = drawing_area.clone();
        let crawl_mode = crawl_state.clone();
        mode_combo.connect_selected_notify(move |combo| {
            let mode = match combo.selected() {
                0 => VisualizerViewMode::DirectoryTree,
                1 => VisualizerViewMode::ForceDirectedFolders,
                _ => VisualizerViewMode::ThreeDConstellation,
            };
            {
                let mut st = state_mode.borrow_mut();
                st.mode = mode;
                Self::rebuild_layout(&mut st, &crawl_mode);
            }
            area_mode.queue_draw();
        });

        let state_lbl_mode = state.clone();
        let area_lbl_mode = drawing_area.clone();
        label_combo.connect_selected_notify(move |combo| {
            let l_mode = match combo.selected() {
                0 => LabelDisplayMode::SmartAuto,
                1 => LabelDisplayMode::HoverOnly,
                _ => LabelDisplayMode::ShowAll,
            };
            state_lbl_mode.borrow_mut().label_mode = l_mode;
            area_lbl_mode.queue_draw();
        });

        let state_exp = state.clone();
        let area_exp = drawing_area.clone();
        let crawl_exp = crawl_state.clone();
        expand_all_btn.connect_clicked(move |_| {
            let mut st = state_exp.borrow_mut();
            let results = crawl_exp.get_all_results();
            for res in &results {
                if let Ok(u) = url::Url::parse(&res.url) {
                    let mut accum = String::new();
                    for seg in u.path().split('/').filter(|s| !s.is_empty()) {
                        accum.push('/');
                        accum.push_str(seg);
                        st.expanded_folders.insert(format!("{}/", accum));
                    }
                }
            }
            st.expanded_folders.insert("/".to_string());
            Self::rebuild_layout(&mut st, &crawl_exp);
            area_exp.queue_draw();
        });

        let state_col = state.clone();
        let area_col = drawing_area.clone();
        let crawl_col = crawl_state.clone();
        collapse_all_btn.connect_clicked(move |_| {
            let mut st = state_col.borrow_mut();
            st.expanded_folders.clear();
            st.expanded_folders.insert("/".to_string());
            Self::rebuild_layout(&mut st, &crawl_col);
            area_col.queue_draw();
        });

        let state_zi = state.clone();
        let area_zi = drawing_area.clone();
        zoom_in_btn.connect_clicked(move |_| {
            let mut st = state_zi.borrow_mut();
            if st.mode == VisualizerViewMode::ThreeDConstellation {
                st.camera_dist = (st.camera_dist * 0.85).max(200.0);
            } else {
                st.zoom = (st.zoom * 1.25).min(4.0);
            }
            area_zi.queue_draw();
        });

        let state_zo = state.clone();
        let area_zo = drawing_area.clone();
        zoom_out_btn.connect_clicked(move |_| {
            let mut st = state_zo.borrow_mut();
            if st.mode == VisualizerViewMode::ThreeDConstellation {
                st.camera_dist = (st.camera_dist * 1.18).min(2000.0);
            } else {
                st.zoom = (st.zoom / 1.25).max(0.2);
            }
            area_zo.queue_draw();
        });

        let state_res = state.clone();
        let area_res = drawing_area.clone();
        reset_btn.connect_clicked(move |_| {
            let mut st = state_res.borrow_mut();
            st.zoom = 1.0;
            st.pan_x = 0.0;
            st.pan_y = 0.0;
            st.yaw = 0.4;
            st.pitch = 0.3;
            st.camera_dist = 600.0;
            area_res.queue_draw();
        });

        let state_srch = state.clone();
        let area_srch = drawing_area.clone();
        search_entry.connect_search_changed(move |entry| {
            let query = entry.text().to_lowercase();
            let mut st = state_srch.borrow_mut();
            st.search_query = query;
            area_srch.queue_draw();
        });

        let vis_win = Self {
            window,
            state,
            crawl_state,
            drawing_area,
            selection_callback,
        };

        vis_win.refresh();
        vis_win
    }

    pub fn show(&self) {
        self.refresh();
        self.window.present();
    }

    pub fn refresh(&self) {
        let mut st = self.state.borrow_mut();
        Self::rebuild_layout(&mut st, &self.crawl_state);
        self.drawing_area.queue_draw();
    }

    pub fn connect_selection_changed<F>(&self, callback: F)
    where
        F: Fn(Option<String>) + 'static,
    {
        *self.selection_callback.borrow_mut() = Some(Box::new(callback));
    }

    fn rebuild_layout(st: &mut VisualizerState, crawl_state: &CrawlState) {
        let results = crawl_state.get_all_results();
        st.nodes.clear();
        st.edges.clear();
        st.total_crawl_pages = results.len();

        if results.is_empty() {
            return;
        }

        let mut root_tree = DirectoryTreeNode::new("Root".to_string(), "/".to_string());

        for res in &results {
            let path = url::Url::parse(&res.url)
                .map(|u| u.path().to_string())
                .unwrap_or_else(|_| "/".to_string());

            let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            root_tree.insert_url(&segments, 0, res);
        }

        let mut current_y = 0.0;
        Self::flatten_tree_node(st, &root_tree, 0, &mut current_y, None);
    }

    fn flatten_tree_node(
        st: &mut VisualizerState,
        dir_node: &DirectoryTreeNode,
        depth: usize,
        current_y: &mut f64,
        parent_node_idx: Option<usize>,
    ) -> usize {
        let node_idx = st.nodes.len();
        let is_exp = st.expanded_folders.contains(&dir_node.full_path);

        let radius = (12.0 + (dir_node.total_pages_count as f64).sqrt() * 2.2).min(35.0);

        let column_x = (depth as f64) * 240.0 - 450.0;
        let node_y = *current_y;

        let theta = (depth as f64 * 0.7) + (node_idx as f64 * 0.45);
        let phi = (node_idx as f64 * 0.35) % std::f64::consts::PI;
        let r_3d = 140.0 + (depth as f64 * 90.0);

        let x_3d = r_3d * phi.sin() * theta.cos();
        let y_3d = r_3d * phi.sin() * theta.sin();
        let z_3d = r_3d * phi.cos();

        let has_children = !dir_node.subfolders.is_empty() || !dir_node.pages.is_empty();
        let label = if has_children {
            format!(
                "{} ({}) [{}]",
                if dir_node.full_path == "/" { "Root (/)" } else { &dir_node.name },
                dir_node.total_pages_count,
                if is_exp { "-" } else { "+" }
            )
        } else {
            format!("{} ({})", dir_node.name, dir_node.total_pages_count)
        };

        st.nodes.push(RenderNode {
            is_folder: true,
            label,
            full_url: None,
            folder_path: dir_node.full_path.clone(),
            error_count: dir_node.error_count,
            status_code: None,
            indexable: true,
            x: column_x,
            y: node_y,
            x_3d,
            y_3d,
            z_3d,
            proj_x: 0.0,
            proj_y: 0.0,
            proj_z: 0.0,
            proj_scale: 1.0,
            radius,
            pages_count: dir_node.total_pages_count,
            _is_expanded: is_exp,
            _subfolder_count: dir_node.subfolders.len(),
            _direct_pages_count: dir_node.pages.len(),
        });

        if let Some(parent_idx) = parent_node_idx {
            st.edges.push(RenderEdge {
                source_idx: parent_idx,
                target_idx: node_idx,
            });
        }

        if is_exp {
            let mut subfolder_keys: Vec<&String> = dir_node.subfolders.keys().collect();
            subfolder_keys.sort();

            for key in subfolder_keys {
                let child_dir = &dir_node.subfolders[key];
                *current_y += 65.0;
                Self::flatten_tree_node(st, child_dir, depth + 1, current_y, Some(node_idx));
            }

            let page_limit = dir_node.pages.len().min(25);
            for (p_i, page_res) in dir_node.pages.iter().take(page_limit).enumerate() {
                *current_y += 36.0;
                let page_idx = st.nodes.len();

                let page_path = url::Url::parse(&page_res.url)
                    .map(|u| u.path().to_string())
                    .unwrap_or_else(|_| page_res.url.clone());

                let page_label = if page_path.len() > 30 {
                    format!("...{}", &page_path[page_path.len() - 28..])
                } else {
                    page_path
                };

                let page_angle = theta + (p_i as f64 * 0.15);
                let page_r_3d = r_3d + 60.0;
                let px_3d = page_r_3d * phi.sin() * page_angle.cos();
                let py_3d = page_r_3d * phi.sin() * page_angle.sin();
                let pz_3d = page_r_3d * phi.cos();

                st.nodes.push(RenderNode {
                    is_folder: false,
                    label: page_label,
                    full_url: Some(page_res.url.clone()),
                    folder_path: dir_node.full_path.clone(),
                    error_count: 0,
                    status_code: page_res.status_code,
                    indexable: page_res.indexable,
                    x: ((depth + 1) as f64) * 240.0 - 450.0,
                    y: *current_y,
                    x_3d: px_3d,
                    y_3d: py_3d,
                    z_3d: pz_3d,
                    proj_x: 0.0,
                    proj_y: 0.0,
                    proj_z: 0.0,
                    proj_scale: 1.0,
                    radius: (6.0 + (page_res.inlinks.len() as f64).sqrt() * 1.5).min(18.0),
                    pages_count: 1,
                    _is_expanded: false,
                    _subfolder_count: 0,
                    _direct_pages_count: 0,
                });

                st.edges.push(RenderEdge {
                    source_idx: node_idx,
                    target_idx: page_idx,
                });
            }
        }

        node_idx
    }

    fn update_3d_projections(st: &mut VisualizerState) {
        let cos_y = st.yaw.cos();
        let sin_y = st.yaw.sin();
        let cos_p = st.pitch.cos();
        let sin_p = st.pitch.sin();
        let focal_len = st.camera_dist;

        for node in &mut st.nodes {
            let x1 = node.x_3d * cos_y + node.z_3d * sin_y;
            let z1 = -node.x_3d * sin_y + node.z_3d * cos_y;

            let y2 = node.y_3d * cos_p - z1 * sin_p;
            let z2 = node.y_3d * sin_p + z1 * cos_p;

            let depth_z = z2 + focal_len;
            let scale = if depth_z > 50.0 {
                focal_len / depth_z
            } else {
                0.1
            };

            node.proj_x = x1 * scale;
            node.proj_y = y2 * scale;
            node.proj_z = z2;
            node.proj_scale = scale;
        }
    }

    fn hit_test(
        st: &VisualizerState,
        screen_x: f64,
        screen_y: f64,
        width: f64,
        height: f64,
    ) -> Option<usize> {
        let center_x = width / 2.0 + st.pan_x;
        let center_y = height / 2.0 + st.pan_y;

        for (idx, node) in st.nodes.iter().enumerate() {
            let (nx, ny, hit_r) = if st.mode == VisualizerViewMode::ThreeDConstellation {
                (
                    center_x + node.proj_x,
                    center_y + node.proj_y,
                    (node.radius * node.proj_scale).max(12.0),
                )
            } else {
                (
                    center_x + node.x * st.zoom,
                    center_y + node.y * st.zoom,
                    (node.radius * st.zoom).max(12.0),
                )
            };

            let dx = screen_x - nx;
            let dy = screen_y - ny;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= hit_r * hit_r {
                return Some(idx);
            }
        }
        None
    }

    fn draw_visualization(st: &mut VisualizerState, cr: &cairo::Context, width: f64, height: f64) {
        cr.set_source_rgb(0.08, 0.08, 0.10);
        cr.paint().unwrap();

        if st.nodes.is_empty() {
            cr.set_source_rgb(0.6, 0.6, 0.65);
            cr.set_font_size(14.0);
            cr.move_to(width / 2.0 - 150.0, height / 2.0);
            let _ = cr.show_text("No crawl data available. Run a crawl or open a file to visualize.");
            return;
        }

        if st.mode == VisualizerViewMode::ThreeDConstellation {
            Self::update_3d_projections(st);
        }

        let center_x = width / 2.0 + st.pan_x;
        let center_y = height / 2.0 + st.pan_y;

        // Draw edges
        for edge in &st.edges {
            if edge.source_idx >= st.nodes.len() || edge.target_idx >= st.nodes.len() {
                continue;
            }
            let src = &st.nodes[edge.source_idx];
            let tgt = &st.nodes[edge.target_idx];

            let (x1, y1, x2, y2) = if st.mode == VisualizerViewMode::ThreeDConstellation {
                (
                    center_x + src.proj_x,
                    center_y + src.proj_y,
                    center_x + tgt.proj_x,
                    center_y + tgt.proj_y,
                )
            } else {
                (
                    center_x + src.x * st.zoom,
                    center_y + src.y * st.zoom,
                    center_x + tgt.x * st.zoom,
                    center_y + tgt.y * st.zoom,
                )
            };

            if (x1 < -50.0 && x2 < -50.0) || (x1 > width + 50.0 && x2 > width + 50.0) {
                continue;
            }
            if (y1 < -50.0 && y2 < -50.0) || (y1 > height + 50.0 && y2 > height + 50.0) {
                continue;
            }

            let is_hl = st.selected_node == Some(edge.source_idx)
                || st.selected_node == Some(edge.target_idx)
                || st.hovered_node == Some(edge.source_idx)
                || st.hovered_node == Some(edge.target_idx);

            if is_hl {
                cr.set_source_rgba(0.3, 0.85, 1.0, 0.95);
                cr.set_line_width(3.2);
            } else if st.mode == VisualizerViewMode::ThreeDConstellation {
                cr.set_source_rgba(0.3, 0.45, 0.65, 0.35);
                cr.set_line_width(1.4);
            } else {
                cr.set_source_rgba(0.4, 0.45, 0.55, 0.45);
                cr.set_line_width(1.8 * st.zoom.clamp(0.6, 2.0));
            }

            if st.mode == VisualizerViewMode::DirectoryTree {
                let mid_x = (x1 + x2) / 2.0;
                cr.move_to(x1, y1);
                cr.curve_to(mid_x, y1, mid_x, y2, x2, y2);
            } else {
                cr.move_to(x1, y1);
                cr.line_to(x2, y2);
            }
            let _ = cr.stroke();
        }

        // Draw nodes
        let mut draw_order: Vec<usize> = (0..st.nodes.len()).collect();
        if st.mode == VisualizerViewMode::ThreeDConstellation {
            draw_order.sort_by(|&a, &b| {
                st.nodes[a]
                    .proj_z
                    .partial_cmp(&st.nodes[b].proj_z)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        let total_pages = st.total_crawl_pages.max(1);

        for &idx in &draw_order {
            let node = &st.nodes[idx];

            let (nx, ny, r) = if st.mode == VisualizerViewMode::ThreeDConstellation {
                (
                    center_x + node.proj_x,
                    center_y + node.proj_y,
                    node.radius * node.proj_scale,
                )
            } else {
                (
                    center_x + node.x * st.zoom,
                    center_y + node.y * st.zoom,
                    node.radius * st.zoom,
                )
            };

            if nx + r < -50.0 || nx - r > width + 50.0 || ny + r < -50.0 || ny - r > height + 50.0 {
                continue;
            }

            let is_selected = st.selected_node == Some(idx);
            let is_hovered = st.hovered_node == Some(idx);
            let is_search_match = !st.search_query.is_empty()
                && (node.folder_path.to_lowercase().contains(&st.search_query)
                    || node.label.to_lowercase().contains(&st.search_query));

            let alpha = if st.mode == VisualizerViewMode::ThreeDConstellation {
                (node.proj_scale * 1.2).clamp(0.25, 1.0)
            } else {
                1.0
            };

            // Color coding
            if node.is_folder {
                if node.error_count > 0 {
                    cr.set_source_rgba(0.95, 0.35, 0.2, alpha);
                } else {
                    cr.set_source_rgba(0.2, 0.65, 0.95, alpha);
                }
            } else {
                match node.status_code {
                    Some(code) if code >= 200 && code < 300 => {
                        if node.indexable {
                            cr.set_source_rgba(0.2, 0.85, 0.45, alpha);
                        } else {
                            cr.set_source_rgba(0.6, 0.6, 0.7, alpha);
                        }
                    }
                    Some(code) if code >= 300 && code < 400 => {
                        cr.set_source_rgba(0.95, 0.65, 0.2, alpha)
                    }
                    Some(_) => cr.set_source_rgba(0.95, 0.25, 0.25, alpha),
                    None => cr.set_source_rgba(0.5, 0.5, 0.5, alpha),
                }
            }

            cr.arc(nx, ny, r, 0.0, 2.0 * std::f64::consts::PI);
            let _ = cr.fill_preserve();

            if is_selected {
                cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                cr.set_line_width(3.5);
                let _ = cr.stroke();
            } else if is_hovered || is_search_match {
                cr.set_source_rgba(0.3, 0.9, 1.0, 1.0);
                cr.set_line_width(3.0);
                let _ = cr.stroke();
            } else {
                cr.set_source_rgba(0.0, 0.0, 0.0, alpha * 0.6);
                cr.set_line_width(1.5);
                let _ = cr.stroke();
            }

            // SMART LEVEL-OF-DETAIL (LOD) LABEL CULLING
            let should_draw_label = match st.label_mode {
                LabelDisplayMode::HoverOnly => is_hovered || is_selected || is_search_match,
                LabelDisplayMode::ShowAll => true,
                LabelDisplayMode::SmartAuto => {
                    if is_hovered || is_selected || is_search_match {
                        true
                    } else if st.mode == VisualizerViewMode::ThreeDConstellation {
                        node.proj_scale > 0.82 && (node.is_folder || node.pages_count >= total_pages / 15)
                    } else {
                        // Folders representing > 3% of site pages or Root
                        node.folder_path == "/" || (node.is_folder && node.pages_count >= (total_pages / 35).max(2))
                    }
                }
            };

            if should_draw_label {
                cr.set_source_rgba(0.92, 0.92, 0.96, alpha);
                let font_size = if st.mode == VisualizerViewMode::ThreeDConstellation {
                    (11.0 * node.proj_scale).clamp(9.0, 16.0)
                } else {
                    (12.0 * st.zoom).clamp(10.0, 15.0)
                };
                cr.set_font_size(font_size);
                cr.move_to(nx + r + 7.0, ny + 4.0);
                let _ = cr.show_text(&node.label);
            }
        }
    }
}

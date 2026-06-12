use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};
use crate::state::{CrawlState, CrawlResult};
use cairo;

// ──────────────────────────────────────────────────────────────────
// Layout constants
// ──────────────────────────────────────────────────────────────────
const PADDING: f64 = 60.0;

// ──────────────────────────────────────────────────────────────────
// A node in the layout
// ──────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
struct Node {
    url: String,
    label: String,
    is_dir: bool,
    status: Option<u16>,
    title: Option<String>,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    radius: f64,
    visible: bool,
    expanded: bool,
    parent_dir: Option<String>,
    degree: usize,
    child_count: usize,
}

// ──────────────────────────────────────────────────────────────────
// SiteVisualizer
// ──────────────────────────────────────────────────────────────────
pub struct SiteVisualizer {
    window: gtk::Window,
    canvas: gtk::DrawingArea,
    #[allow(dead_code)]
    zoom_scale: gtk::Scale,   // kept alive so the widget isn't dropped
    state: Rc<RefCell<VisualizerState>>,
}

struct VisualizerState {
    nodes: Vec<Node>,
    edges: Vec<(usize, usize)>,  // indices into nodes
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
    canvas_w: f64,
    canvas_h: f64,
    on_url_selected: Option<Box<dyn Fn(String)>>,
    // url -> node index
    url_to_idx: HashMap<String, usize>,
    selected_idx: Option<usize>,
    dragged_idx: Option<usize>,
    is_ticking: bool,
    cooling: f64,
    hovered_idx: Option<usize>,
}

impl SiteVisualizer {
    pub fn new(parent: &impl IsA<gtk::Window>) -> Self {
        let window = gtk::Window::builder()
            .title("Tadpole — Site Map")
            .icon_name("com.tadpole.seo")
            .transient_for(parent)
            .default_width(1100)
            .default_height(700)
            .build();

        // In GTK4 the default close-request handler DESTROYS the window.
        // We intercept it and hide instead, so the struct stays usable
        // when the user re-opens the visualizer from the header bar.
        window.connect_close_request(|w| {
            w.set_visible(false);
            glib::Propagation::Stop   // prevent the default destroy
        });

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.set_child(Some(&main_box));

        // ── Toolbar ───────────────────────────────────────────────
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        toolbar.set_margin_start(12);
        toolbar.set_margin_end(12);
        toolbar.set_margin_top(8);
        toolbar.set_margin_bottom(8);

        let info_lbl = gtk::Label::new(Some("Drag to pan · Ctrl+Scroll to zoom · Click node to inspect"));
        info_lbl.add_css_class("dim-label");
        info_lbl.set_hexpand(true);
        toolbar.append(&info_lbl);

        let zoom_lbl = gtk::Label::new(Some("Zoom:"));
        toolbar.append(&zoom_lbl);

        let zoom_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.2, 2.5, 0.05);
        zoom_scale.set_value(1.0);
        zoom_scale.set_width_request(150);
        zoom_scale.set_draw_value(true);
        zoom_scale.set_format_value_func(|_, v| format!("{:.0}%", v * 100.0));
        toolbar.append(&zoom_scale);

        let reset_btn = gtk::Button::builder()
            .icon_name("zoom-original-symbolic")
            .tooltip_text("Center view on graph")
            .build();
        toolbar.append(&reset_btn);

        let expand_all_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Expand all directory folders")
            .build();
        toolbar.append(&expand_all_btn);

        let collapse_all_btn = gtk::Button::builder()
            .icon_name("list-remove-symbolic")
            .tooltip_text("Collapse all folders")
            .build();
        toolbar.append(&collapse_all_btn);

        // Legend
        let legend = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        legend.set_margin_start(12);
        legend.set_margin_end(12);
        legend.set_margin_bottom(6);

        for (label, r, g, b) in [
            ("Directory", 0.25, 0.45, 0.85),
            ("2xx OK", 0.2, 0.65, 0.3),
            ("3xx Redirect", 0.85, 0.65, 0.1),
            ("4xx Error", 0.8, 0.2, 0.2),
            ("5xx Error", 0.6, 0.1, 0.6),
            ("Failed", 0.45, 0.45, 0.45),
        ] {
            let dot = gtk::DrawingArea::new();
            dot.set_size_request(12, 12);
            let (r, g, b) = (r, g, b);
            dot.set_draw_func(move |_, cr, _, _| {
                cr.set_source_rgb(r, g, b);
                cr.arc(6.0, 6.0, 5.0, 0.0, std::f64::consts::TAU);
                cr.fill().ok();
            });
            legend.append(&dot);
            let lbl = gtk::Label::new(Some(label));
            lbl.add_css_class("dim-label");
            legend.append(&lbl);

            let sep = gtk::Separator::new(gtk::Orientation::Vertical);
            sep.set_margin_start(4);
            sep.set_margin_end(4);
            legend.append(&sep);
        }

        main_box.append(&toolbar);
        main_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        main_box.append(&legend);

        // ── Canvas inside ScrolledWindow ──────────────────────────
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);
        main_box.append(&scrolled);

        let canvas = gtk::DrawingArea::new();
        canvas.set_vexpand(true);
        canvas.set_hexpand(true);
        scrolled.set_child(Some(&canvas));

        let state = Rc::new(RefCell::new(VisualizerState {
            nodes: vec![],
            edges: vec![],
            zoom: 1.0,
            pan_x: PADDING,
            pan_y: PADDING,
            canvas_w: 0.0,
            canvas_h: 0.0,
            on_url_selected: None,
            url_to_idx: HashMap::new(),
            selected_idx: None,
            dragged_idx: None,
            is_ticking: false,
            cooling: 0.0,
            hovered_idx: None,
        }));

        // ── Wire zoom scale ───────────────────────────────────────
        let state_zoom = state.clone();
        let canvas_zoom = canvas.clone();
        zoom_scale.connect_value_changed(move |scale| {
            let mut s = state_zoom.borrow_mut();
            let old_zoom = s.zoom;
            let new_zoom = scale.value();
            if old_zoom > 0.0 && new_zoom != old_zoom {
                let width = canvas_zoom.width() as f64;
                let height = canvas_zoom.height() as f64;
                let cx = if width > 0.0 { width / 2.0 } else { 550.0 };
                let cy = if height > 0.0 { height / 2.0 } else { 350.0 };
                s.pan_x = cx - (cx - s.pan_x) * (new_zoom / old_zoom);
                s.pan_y = cy - (cy - s.pan_y) * (new_zoom / old_zoom);
                s.zoom = new_zoom;
                eprintln!(
                    "[SiteVisualizer] Zoom changed: {:.0}% -> {:.0}%. Pan offset is ({:.1}, {:.1})",
                    old_zoom * 100.0, new_zoom * 100.0, s.pan_x, s.pan_y
                );
            }
            canvas_zoom.queue_draw();
        });

        // ── Wire reset ────────────────────────────────────────────
        let state_reset = state.clone();
        let canvas_reset = canvas.clone();
        let scale_reset = zoom_scale.clone();
        reset_btn.connect_clicked(move |_| {
            eprintln!("[SiteVisualizer] Reset button clicked: centering and fitting view.");
            let mut s = state_reset.borrow_mut();
            let width = canvas_reset.width() as f64;
            let height = canvas_reset.height() as f64;
            let w = if width > 0.0 { width } else { 1100.0 };
            let h = if height > 0.0 { height } else { 700.0 };
            Self::fit_to_screen_inner(&mut s, w, h);
            let zoom = s.zoom;
            drop(s);
            scale_reset.set_value(zoom);
            canvas_reset.queue_draw();
        });

        let state_expand = state.clone();
        let canvas_expand = canvas.clone();
        let scale_expand = zoom_scale.clone();
        expand_all_btn.connect_clicked(move |_| {
            eprintln!("[SiteVisualizer] Expand All button clicked.");
            let mut s = state_expand.borrow_mut();
            for n in &mut s.nodes {
                if n.is_dir {
                    n.expanded = true;
                }
            }
            let url_to_idx = s.url_to_idx.clone();
            Self::update_visibility(&mut s.nodes, &url_to_idx);

            let width = canvas_expand.width() as f64;
            let height = canvas_expand.height() as f64;
            let w = if width > 0.0 { width } else { 1100.0 };
            let h = if height > 0.0 { height } else { 700.0 };
            Self::fit_to_screen_inner(&mut s, w, h);
            let zoom = s.zoom;
            drop(s);
            scale_expand.set_value(zoom);
            Self::start_ticking(&state_expand, &canvas_expand);
        });

        let state_collapse = state.clone();
        let canvas_collapse = canvas.clone();
        let scale_collapse = zoom_scale.clone();
        collapse_all_btn.connect_clicked(move |_| {
            eprintln!("[SiteVisualizer] Collapse All button clicked.");
            let mut s = state_collapse.borrow_mut();
            for n in &mut s.nodes {
                if n.is_dir {
                    let is_root = n.parent_dir.is_none();
                    n.expanded = is_root;
                }
            }
            let url_to_idx = s.url_to_idx.clone();
            Self::update_visibility(&mut s.nodes, &url_to_idx);

            let width = canvas_collapse.width() as f64;
            let height = canvas_collapse.height() as f64;
            let w = if width > 0.0 { width } else { 1100.0 };
            let h = if height > 0.0 { height } else { 700.0 };
            Self::fit_to_screen_inner(&mut s, w, h);
            let zoom = s.zoom;
            drop(s);
            scale_collapse.set_value(zoom);
            Self::start_ticking(&state_collapse, &canvas_collapse);
        });

        // ── Pan/Drag gesture ──────────────────────────────────────
        let drag = gtk::GestureDrag::new();
        drag.set_button(1);

        let state_drag = state.clone();
        let canvas_drag = canvas.clone();
        let start_pos_rc = Rc::new(RefCell::new((0.0, 0.0)));

        let start_pos_rc_begin = start_pos_rc.clone();
        let state_begin = state_drag.clone();
        drag.connect_drag_begin(move |_, x, y| {
            let mut s = state_begin.borrow_mut();
            let zoom = s.zoom;
            let pan_x = s.pan_x;
            let pan_y = s.pan_y;
            let cx = (x - pan_x) / zoom;
            let cy = (y - pan_y) / zoom;

            let mut hit_idx = None;
            for (i, node) in s.nodes.iter().enumerate() {
                if !node.visible { continue; }
                let dx = cx - node.x;
                let dy = cy - node.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= node.radius + 8.0 {
                    hit_idx = Some(i);
                    break;
                }
            }

            s.dragged_idx = hit_idx;
            if let Some(idx) = hit_idx {
                *start_pos_rc_begin.borrow_mut() = (s.nodes[idx].x, s.nodes[idx].y);
            } else {
                *start_pos_rc_begin.borrow_mut() = (s.pan_x, s.pan_y);
            }
        });

        let start_pos_rc_update = start_pos_rc.clone();
        let state_update = state_drag.clone();
        let canvas_update = canvas_drag.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            let mut s = state_update.borrow_mut();
            let start = *start_pos_rc_update.borrow();
            if let Some(idx) = s.dragged_idx {
                let zoom = s.zoom;
                s.nodes[idx].x = start.0 + dx / zoom;
                s.nodes[idx].y = start.1 + dy / zoom;
                // Give it some floaty velocity
                s.nodes[idx].vx = dx * 0.15;
                s.nodes[idx].vy = dy * 0.15;
                drop(s);
                Self::start_ticking(&state_update, &canvas_update);
            } else {
                s.pan_x = start.0 + dx;
                s.pan_y = start.1 + dy;
                drop(s);
                canvas_update.queue_draw();
            }
        });

        let state_end = state_drag.clone();
        drag.connect_drag_end(move |_, _, _| {
            let mut s = state_end.borrow_mut();
            s.dragged_idx = None;
        });
        canvas.add_controller(drag);

        // ── Zoom via Ctrl+Scroll ──────────────────────────────────
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let scale_scroll = zoom_scale.clone();
        scroll.connect_scroll(move |evt, _dx, dy| {
            if evt.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
                let delta = -dy * 0.08;
                let current_zoom = scale_scroll.value();
                let new_zoom = (current_zoom + delta).max(0.2).min(2.5);
                scale_scroll.set_value(new_zoom);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        canvas.add_controller(scroll);

        // ── Click to select and expand/collapse directories ───────
        let click = gtk::GestureClick::new();
        let state_click = state.clone();
        let canvas_click = canvas.clone();
        click.connect_released(move |_, _n, x, y| {
            let mut s = state_click.borrow_mut();
            let zoom = s.zoom;
            let pan_x = s.pan_x;
            let pan_y = s.pan_y;
            let cx = (x - pan_x) / zoom;
            let cy = (y - pan_y) / zoom;
            
            let mut clicked_idx = None;
            for (i, node) in s.nodes.iter().enumerate() {
                if !node.visible { continue; }
                let dx = cx - node.x;
                let dy = cy - node.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= node.radius + 6.0 {
                    clicked_idx = Some(i);
                    break;
                }
            }
            
            s.selected_idx = clicked_idx;
            if let Some(idx) = clicked_idx {
                let url = s.nodes[idx].url.clone();
                eprintln!(
                    "[SiteVisualizer] Clicked node index {}: URL={:?}, is_dir={}, title={:?}",
                    idx, url, s.nodes[idx].is_dir, s.nodes[idx].title
                );
                
                // If it is a directory node, toggle expanded state
                if s.nodes[idx].is_dir {
                    let was_expanded = s.nodes[idx].expanded;
                    s.nodes[idx].expanded = !was_expanded;
                    eprintln!(
                        "[SiteVisualizer] Toggled directory visibility: expanded={}",
                        !was_expanded
                    );
                    
                    // If expanding, trigger child burst positions
                    if !was_expanded {
                        let px = s.nodes[idx].x;
                        let py = s.nodes[idx].y;
                        let p_url = s.nodes[idx].url.clone();
                        let mut count = 0;
                        for i in 0..s.nodes.len() {
                            if s.nodes[i].parent_dir == Some(p_url.clone()) {
                                let angle = count as f64 * 0.75;
                                s.nodes[i].x = px + angle.cos() * 20.0;
                                s.nodes[i].y = py + angle.sin() * 20.0;
                                s.nodes[i].vx = angle.cos() * 12.0;
                                s.nodes[i].vy = angle.sin() * 12.0;
                                count += 1;
                            }
                        }
                    }
                    
                    // Recompute visible nodes recursively
                    let url_to_idx = s.url_to_idx.clone();
                    Self::update_visibility(&mut s.nodes, &url_to_idx);
                }
                
                // If it represents a real crawled URL, notify table selection
                if !s.nodes[idx].url.starts_with("dir:") {
                    if let Some(ref cb) = s.on_url_selected {
                        cb(url);
                    }
                }
            }
            drop(s);
            Self::start_ticking(&state_click, &canvas_click);
        });
        canvas.add_controller(click);

        // ── Mouse motion gesture (Hover tracking) ──────────────────
        let motion = gtk::EventControllerMotion::new();
        let state_motion = state.clone();
        let canvas_motion = canvas.clone();
        motion.connect_motion(move |_, x, y| {
            let mut s = state_motion.borrow_mut();
            let zoom = s.zoom;
            let pan_x = s.pan_x;
            let pan_y = s.pan_y;
            let cx = (x - pan_x) / zoom;
            let cy = (y - pan_y) / zoom;

            let mut hovered = None;
            for (i, node) in s.nodes.iter().enumerate() {
                if !node.visible { continue; }
                let dx = cx - node.x;
                let dy = cy - node.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= node.radius + 8.0 {
                    hovered = Some(i);
                    break;
                }
            }

            if s.hovered_idx != hovered {
                s.hovered_idx = hovered;
                canvas_motion.queue_draw();
            }
        });

        let state_leave = state.clone();
        let canvas_leave = canvas.clone();
        motion.connect_leave(move |_| {
            let mut s = state_leave.borrow_mut();
            if s.hovered_idx.is_some() {
                s.hovered_idx = None;
                canvas_leave.queue_draw();
            }
        });
        canvas.add_controller(motion);


        // ── Draw function ─────────────────────────────────────────
        let state_draw = state.clone();
        canvas.set_draw_func(move |_widget, cr, _w, _h| {
            let s = state_draw.borrow();
            Self::draw_canvas(&s, cr);
        });

        let viz = Self {
            window,
            canvas,
            zoom_scale,
            state,
        };

        viz
    }

    // ── Public API ─────────────────────────────────────────────────

    pub fn present(&self) {
        eprintln!("[SiteVisualizer] Window presented.");
        self.window.present();
    }

    pub fn connect_url_selected<F: Fn(String) + 'static>(&self, cb: F) {
        self.state.borrow_mut().on_url_selected = Some(Box::new(cb));
    }

    /// Rebuild the graph from the current crawl state.
    pub fn refresh(&self, state: &CrawlState) {
        let results = state.get_all_results();
        if results.is_empty() {
            return;
        }

        // Keep track of the currently selected URL
        let mut old_selected_url = None;
        if let Some(ref old_s_idx) = self.state.borrow().selected_idx {
            if let Some(old_node) = self.state.borrow().nodes.get(*old_s_idx) {
                old_selected_url = Some(old_node.url.clone());
            }
        }

        // Keep track of expanded directories
        let mut expanded_urls = HashSet::new();
        for node in &self.state.borrow().nodes {
            if node.is_dir && node.expanded {
                expanded_urls.insert(node.url.clone());
            }
        }

        let (mut nodes, edges) = Self::build_layout(&results);
        if nodes.is_empty() {
            return;
        }

        // Restore expanded states
        for node in &mut nodes {
            if node.is_dir && expanded_urls.contains(&node.url) {
                node.expanded = true;
            }
        }

        let url_to_idx: HashMap<String, usize> = nodes.iter()
            .enumerate()
            .map(|(i, n)| (n.url.clone(), i))
            .collect();

        Self::update_visibility(&mut nodes, &url_to_idx);

        // Restore selected index if the URL is still present in results
        let selected_idx = old_selected_url.and_then(|url| url_to_idx.get(&url).copied());

        // Calculate size bounds for canvas size request
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for n in &nodes {
            min_x = min_x.min(n.x - n.radius);
            max_x = max_x.max(n.x + n.radius);
            min_y = min_y.min(n.y - n.radius);
            max_y = max_y.max(n.y + n.radius);
        }

        let canvas_w = ((max_x - min_x) + PADDING * 2.0).max(1200.0);
        let canvas_h = ((max_y - min_y) + PADDING * 2.0).max(800.0);

        let mut s = self.state.borrow_mut();
        s.nodes = nodes;
        s.edges = edges;
        s.canvas_w = canvas_w;
        s.canvas_h = canvas_h;
        s.url_to_idx = url_to_idx;
        s.selected_idx = selected_idx;
        eprintln!(
            "[SiteVisualizer] Refreshed graph layout: {} nodes, {} edges.",
            s.nodes.len(), s.edges.len()
        );

        drop(s);
        self.canvas.set_size_request(canvas_w as i32, canvas_h as i32);
        Self::start_ticking(&self.state, &self.canvas);
    }

    fn fit_to_screen_inner(s: &mut VisualizerState, width: f64, height: f64) {
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        let mut any_visible = false;

        for n in &s.nodes {
            if n.visible {
                min_x = min_x.min(n.x - n.radius);
                max_x = max_x.max(n.x + n.radius);
                min_y = min_y.min(n.y - n.radius);
                max_y = max_y.max(n.y + n.radius);
                any_visible = true;
            }
        }

        if any_visible && max_x > min_x && max_y > min_y {
            let graph_w = max_x - min_x;
            let graph_h = max_y - min_y;
            let margin = 80.0;
            let target_zoom_w = (width - margin) / graph_w;
            let target_zoom_h = (height - margin) / graph_h;
            let target_zoom = target_zoom_w.min(target_zoom_h).clamp(0.2, 2.5);

            s.zoom = target_zoom;

            let cx = min_x + graph_w / 2.0;
            let cy = min_y + graph_h / 2.0;

            s.pan_x = (width / 2.0) - cx * s.zoom;
            s.pan_y = (height / 2.0) - cy * s.zoom;
        } else {
            s.zoom = 1.0;
            s.pan_x = width / 2.0;
            s.pan_y = height / 2.0;
        }
    }

    fn update_visibility(nodes: &mut [Node], url_to_idx: &HashMap<String, usize>) {
        // Find root node (parent_dir is None, or shortest URL path)
        let root_idx = nodes.iter().position(|n| n.parent_dir.is_none());

        for n in nodes.iter_mut() {
            n.visible = false;
        }

        if let Some(idx) = root_idx {
            nodes[idx].visible = true;
            // Always expand root by default so there's something to see
            nodes[idx].expanded = true;
        } else if !nodes.is_empty() {
            // Fallback
            nodes[0].visible = true;
            nodes[0].expanded = true;
        }

        // Propagate visibility down the directory tree
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..nodes.len() {
                if nodes[i].visible { continue; }
                if let Some(ref parent_url) = nodes[i].parent_dir {
                    if let Some(&p_idx) = url_to_idx.get(parent_url) {
                        if nodes[p_idx].visible && nodes[p_idx].expanded {
                            nodes[i].visible = true;
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    fn start_ticking(state: &Rc<RefCell<VisualizerState>>, canvas: &gtk::DrawingArea) {
        let mut s = state.borrow_mut();
        s.cooling = 1.0; // reset to full energy when action is taken
        if s.is_ticking {
            return;
        }
        s.is_ticking = true;
        drop(s);
        eprintln!("[SiteVisualizer] Physics simulation ticking started.");

        let state_tick = state.clone();
        let canvas_tick = canvas.clone();
        canvas.add_tick_callback(move |_canvas, _frame_clock| {
            let mut s = state_tick.borrow_mut();
            let active = Self::step_physics(&mut s);
            canvas_tick.queue_draw();
            if active {
                glib::ControlFlow::Continue
            } else {
                s.is_ticking = false;
                eprintln!("[SiteVisualizer] Physics simulation cooled down and ticking stopped.");
                glib::ControlFlow::Break
            }
        });
    }

    fn step_physics(s: &mut VisualizerState) -> bool {
        let num_nodes = s.nodes.len();
        if num_nodes == 0 {
            return false;
        }

        // We only simulate visible nodes
        let mut visible_indices = vec![];
        for i in 0..num_nodes {
            if s.nodes[i].visible {
                visible_indices.push(i);
            }
        }

        if visible_indices.is_empty() {
            return false;
        }

        let repel_constant = 2500.0;
        let attract_constant = 0.08;
        let parent_attraction = 0.05;
        let gravity = 0.008;
        let center_x = s.canvas_w / 2.0;
        let center_y = s.canvas_h / 2.0;
        let cx = if center_x.is_finite() && center_x > 0.0 { center_x } else { 600.0 };
        let cy = if center_y.is_finite() && center_y > 0.0 { center_y } else { 400.0 };

        // Safety check: recover any nodes with non-finite coordinates
        for i in 0..num_nodes {
            if !s.nodes[i].x.is_finite() || !s.nodes[i].y.is_finite() {
                eprintln!(
                    "[SiteVisualizer Warning] Recovering node {} from non-finite position (x={:?}, y={:?})",
                    s.nodes[i].label, s.nodes[i].x, s.nodes[i].y
                );
                s.nodes[i].x = cx;
                s.nodes[i].y = cy;
                s.nodes[i].vx = 0.0;
                s.nodes[i].vy = 0.0;
            }
            if !s.nodes[i].vx.is_finite() || !s.nodes[i].vy.is_finite() {
                s.nodes[i].vx = 0.0;
                s.nodes[i].vy = 0.0;
            }
        }

        let mut fx = vec![0.0; num_nodes];
        let mut fy = vec![0.0; num_nodes];

        // 1. Repulsion between all pairs of visible nodes
        for idx_i in 0..visible_indices.len() {
            let i = visible_indices[idx_i];
            for idx_j in (idx_i + 1)..visible_indices.len() {
                let j = visible_indices[idx_j];
                let dx = s.nodes[i].x - s.nodes[j].x;
                let dy = s.nodes[i].y - s.nodes[j].y;
                let dist_sq = dx * dx + dy * dy + 900.0;
                let dist = dist_sq.sqrt();

                if dist.is_finite() && dist > 0.0 {
                    let min_dist = s.nodes[i].radius + s.nodes[j].radius + 45.0;
                    let force = if dist < min_dist {
                        repel_constant * 3.5 / dist_sq
                    } else {
                        repel_constant / dist_sq
                    };

                    let rfx = (dx / dist) * force;
                    let rfy = (dy / dist) * force;

                    if rfx.is_finite() && rfy.is_finite() {
                        fx[i] += rfx;
                        fy[i] += rfy;
                        fx[j] -= rfx;
                        fy[j] -= rfy;
                    }
                }
            }
        }

        // 2. Attraction along visible edges
        for &(from, to) in &s.edges {
            if s.nodes[from].visible && s.nodes[to].visible {
                let dx = s.nodes[to].x - s.nodes[from].x;
                let dy = s.nodes[to].y - s.nodes[from].y;
                let dist = (dx * dx + dy * dy + 1.0).sqrt();

                if dist.is_finite() && dist > 1.0 {
                    let target_dist = s.nodes[from].radius + s.nodes[to].radius + 50.0;
                    let displacement = dist - target_dist;

                    if displacement > 0.0 {
                        let force = attract_constant * displacement;
                        let afx = (dx / dist) * force;
                        let afy = (dy / dist) * force;

                        if afx.is_finite() && afy.is_finite() {
                            fx[from] += afx;
                            fy[from] += afy;
                            fx[to] -= afx;
                            fy[to] -= afy;
                        }
                    }
                }
            }
        }

        // 3. Attraction of children to their parent directory (keeps sub-hierarchies close)
        for i in 0..num_nodes {
            if !s.nodes[i].visible { continue; }
            if let Some(ref parent_url) = s.nodes[i].parent_dir {
                if let Some(&p_idx) = s.url_to_idx.get(parent_url) {
                    if s.nodes[p_idx].visible {
                        let dx = s.nodes[p_idx].x - s.nodes[i].x;
                        let dy = s.nodes[p_idx].y - s.nodes[i].y;
                        let dist = (dx * dx + dy * dy + 1.0).sqrt();
                        if dist.is_finite() && dist > 15.0 {
                            let afx = dx * parent_attraction;
                            let afy = dy * parent_attraction;
                            fx[i] += afx;
                            fy[i] += afy;
                            fx[p_idx] -= afx * 0.4;
                            fy[p_idx] -= afy * 0.4;
                        }
                    }
                }
            }
        }

        // 4. Center Gravity (pull toward viewport center)
        for &i in &visible_indices {
            let dx = center_x - s.nodes[i].x;
            let dy = center_y - s.nodes[i].y;
            let afx = dx * gravity;
            let afy = dy * gravity;
            if afx.is_finite() && afy.is_finite() {
                fx[i] += afx;
                fy[i] += afy;
            }
        }

        // 5. Update velocities and positions
        let friction = 0.65; // increased damping (less floaty)
        for &i in &visible_indices {
            if s.dragged_idx == Some(i) {
                s.nodes[i].vx = 0.0;
                s.nodes[i].vy = 0.0;
                continue;
            }

            let new_vx = (s.nodes[i].vx + fx[i]) * friction;
            let new_vy = (s.nodes[i].vy + fy[i]) * friction;

            // Clamp velocity to prevent coordinate explosions
            let max_v = 150.0;
            s.nodes[i].vx = new_vx.clamp(-max_v, max_v);
            s.nodes[i].vy = new_vy.clamp(-max_v, max_v);

            // Apply cooling directly to positions so movement slows down and stops
            let dx = s.nodes[i].vx * s.cooling;
            let dy = s.nodes[i].vy * s.cooling;
            if dx.is_finite() && dy.is_finite() {
                s.nodes[i].x += dx;
                s.nodes[i].y += dy;
            }
        }

        // Decay the cooling factor
        s.cooling *= 0.88; // faster decay to freeze motion quickly (settles in ~30 frames)

        // Stop ticking when cooled down
        if s.cooling < 0.03 {
            s.cooling = 0.0;
            for i in 0..num_nodes {
                s.nodes[i].vx = 0.0;
                s.nodes[i].vy = 0.0;
            }
            false // Stop tick callback
        } else {
            true // Continue tick callback
        }
    }

    // ── Layout algorithm (Force-Directed) ──────────────────────────
    fn build_layout(results: &[CrawlResult]) -> (Vec<Node>, Vec<(usize, usize)>) {
        if results.is_empty() {
            return (vec![], vec![]);
        }

        // 1. Gather all unique directories recursively
        let mut all_dirs = HashSet::new();
        for res in results {
            let mut current = res.url.clone();
            while let Some(parent) = get_parent_directory(&current) {
                all_dirs.insert(parent.clone());
                current = parent;
            }
        }

        // 2. Struct to store temporarily
        struct TempNode {
            url: String,
            is_dir: bool,
            status: Option<u16>,
            title: Option<String>,
        }

        let mut temp_nodes = vec![];
        let mut crawled_urls = HashSet::new();

        // Add crawled pages
        for res in results {
            crawled_urls.insert(res.url.clone());
            let is_dir = all_dirs.contains(&res.url);
            temp_nodes.push(TempNode {
                url: res.url.clone(),
                is_dir,
                status: res.status_code,
                title: res.title.clone(),
            });
        }

        // Add virtual folder directories that weren't crawled directly
        for dir in all_dirs {
            if !crawled_urls.contains(&dir) {
                temp_nodes.push(TempNode {
                    url: dir.clone(),
                    is_dir: true,
                    status: None,
                    title: None,
                });
            }
        }

        // Sort by depth of path and URL to keep order deterministic
        temp_nodes.sort_by(|a, b| {
            let depth_a = a.url.chars().filter(|&c| c == '/').count();
            let depth_b = b.url.chars().filter(|&c| c == '/').count();
            depth_a.cmp(&depth_b).then(a.url.cmp(&b.url))
        });

        // url -> index map
        let url_to_idx: HashMap<String, usize> = temp_nodes.iter()
            .enumerate()
            .map(|(i, n)| (n.url.clone(), i))
            .collect();

        // 3. Build structural hierarchy edges
        let mut edges = vec![];
        for (i, node) in temp_nodes.iter().enumerate() {
            if let Some(parent) = get_parent_directory(&node.url) {
                if let Some(&p_idx) = url_to_idx.get(&parent) {
                    if p_idx != i {
                        edges.push((p_idx, i));
                    }
                }
            }
        }

        // 4. Compute child counts for folders
        let mut child_counts = vec![0; temp_nodes.len()];
        for node in &temp_nodes {
            if let Some(parent) = get_parent_directory(&node.url) {
                if let Some(&p_idx) = url_to_idx.get(&parent) {
                    child_counts[p_idx] += 1;
                }
            }
        }

        // 5. Initialize node coordinates using LCG random distribution
        let center_x = 800.0;
        let center_y = 600.0;
        let mut lcg_state = 987654321_u64;
        let mut next_random = move || {
            lcg_state = lcg_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (lcg_state as f64) / (u64::MAX as f64)
        };

        let mut nodes: Vec<Node> = temp_nodes.into_iter().enumerate().map(|(i, item)| {
            let angle = next_random() * std::f64::consts::TAU;
            let dist = 60.0 + next_random() * 320.0;
            let x = center_x + angle.cos() * dist;
            let y = center_y + angle.sin() * dist;

            let radius = if item.is_dir {
                12.0 + (child_counts[i] as f64).sqrt() * 3.5
            } else {
                6.5
            };

            Node {
                url: item.url.clone(),
                label: get_url_label(&item.url, item.is_dir),
                is_dir: item.is_dir,
                status: item.status,
                title: item.title,
                x,
                y,
                vx: 0.0,
                vy: 0.0,
                radius,
                visible: false,
                expanded: false,
                parent_dir: get_parent_directory(&item.url),
                degree: 0,
                child_count: child_counts[i],
            }
        }).collect();

        // 6. Compute structural degrees
        let mut degree = vec![0; nodes.len()];
        for &(from, to) in &edges {
            degree[from] += 1;
            degree[to] += 1;
        }
        for i in 0..nodes.len() {
            nodes[i].degree = degree[i];
        }

        (nodes, edges)
    }

    // ── Cairo draw ────────────────────────────────────────────────
    fn draw_canvas(s: &VisualizerState, cr: &cairo::Context) {
        // Clear background (Obsidian dark graphite style)
        cr.set_source_rgb(0.09, 0.09, 0.10);
        cr.paint().ok();

        cr.save().ok();
        let pan_x = if s.pan_x.is_finite() { s.pan_x } else { 0.0 };
        let pan_y = if s.pan_y.is_finite() { s.pan_y } else { 0.0 };
        let zoom = if s.zoom.is_finite() && s.zoom > 0.0 { s.zoom } else { 1.0 };
        cr.translate(pan_x, pan_y);
        cr.scale(zoom, zoom);

        // A. Draw edges (only visible edges)
        cr.set_source_rgba(0.4, 0.4, 0.45, 0.32);
        cr.set_line_width(1.0);
        for &(from, to) in &s.edges {
            if from >= s.nodes.len() || to >= s.nodes.len() { continue; }
            let fn_ = &s.nodes[from];
            let tn  = &s.nodes[to];
            if fn_.visible && tn.visible {
                if fn_.x.is_finite() && fn_.y.is_finite() && tn.x.is_finite() && tn.y.is_finite() {
                    cr.move_to(fn_.x, fn_.y);
                    cr.line_to(tn.x, tn.y);
                    cr.stroke().ok();
                }
            }
        }

        // B. Draw nodes (dots)
        for (i, node) in s.nodes.iter().enumerate() {
            if !node.visible { continue; }
            if !node.x.is_finite() || !node.y.is_finite() || !node.radius.is_finite() {
                continue;
            }

            let is_selected = s.selected_idx == Some(i);
            
            // Draw glowing halo for selected
            if is_selected {
                cr.arc(node.x, node.y, node.radius + 6.0, 0.0, std::f64::consts::TAU);
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.12);
                cr.fill().ok();

                cr.arc(node.x, node.y, node.radius + 3.0, 0.0, std::f64::consts::TAU);
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
                cr.fill().ok();
            }

            let (r, g, b) = if node.is_dir {
                // Folder colors: premium blue/indigo
                (0.25, 0.45, 0.85)
            } else {
                Self::node_color(node.status)
            };

            // Core dot
            cr.arc(node.x, node.y, node.radius, 0.0, std::f64::consts::TAU);
            cr.set_source_rgba(r, g, b, 0.90);
            cr.fill_preserve().ok();

            // Stroke outline
            if is_selected {
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.set_line_width(2.0);
            } else {
                if node.is_dir && !node.expanded {
                    // Dotted/dashed border if collapsed directory
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.7);
                    cr.set_line_width(1.5);
                } else {
                    cr.set_source_rgba(r * 1.3, g * 1.3, b * 1.3, 1.0);
                    cr.set_line_width(1.0);
                }
            }
            cr.stroke().ok();

            // D. Draw label text inside collapsed directory or below it
            if node.is_dir && !node.expanded && node.child_count > 0 {
                let txt = format!("+{}", node.child_count);
                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.set_font_size(9.0);
                if let Ok(ext) = cr.text_extents(&txt) {
                    cr.move_to(node.x - ext.width() / 2.0, node.y + ext.height() / 2.0 - 0.5);
                    cr.show_text(&txt).ok();
                }
            }

            // Draw text label next to/below nodes to keep graph legible
            // Directories (catalogues) and the root always display their name.
            // Leaf pages show their meta-title only when hovered over or selected (clicked).
            let is_hovered = s.hovered_idx == Some(i);
            let is_root = node.parent_dir.is_none();

            let (show_text, label_text) = if node.is_dir || is_root {
                (true, node.label.clone())
            } else {
                let show = is_selected || is_hovered;
                let text = if show {
                    if let Some(ref t) = node.title {
                        if !t.is_empty() {
                            t.clone()
                        } else {
                            node.label.clone()
                        }
                    } else {
                        node.label.clone()
                    }
                } else {
                    String::new()
                };
                (show, text)
            };

            if show_text && !label_text.is_empty() {
                cr.set_font_size(9.5);
                if is_selected {
                    cr.set_source_rgb(1.0, 1.0, 1.0);
                } else if is_hovered {
                    cr.set_source_rgb(0.95, 0.95, 0.98);
                } else if node.is_dir || is_root {
                    cr.set_source_rgb(0.75, 0.85, 1.0); // light blue for directories/root
                } else {
                    cr.set_source_rgb(0.72, 0.72, 0.74);
                }

                if let Ok(ext) = cr.text_extents(&label_text) {
                    cr.move_to(node.x - ext.width() / 2.0, node.y + node.radius + 12.0);
                    cr.show_text(&label_text).ok();
                }
            }
        }

        cr.restore().ok();
    }

    fn node_color(status: Option<u16>) -> (f64, f64, f64) {
        match status {
            Some(200..=299) => (0.15, 0.55, 0.25),
            Some(300..=399) => (0.70, 0.52, 0.08),
            Some(400..=499) => (0.65, 0.15, 0.15),
            Some(500..=599) => (0.50, 0.08, 0.50),
            _               => (0.30, 0.30, 0.32),
        }
    }
}

// ──────────────────────────────────────────────────────────────────
// Hierarchy Helpers
// ──────────────────────────────────────────────────────────────────

fn get_parent_directory(url_str: &str) -> Option<String> {
    if let Ok(parsed) = url::Url::parse(url_str) {
        let path = parsed.path();
        if path == "/" || path.is_empty() {
            return None; // Root has no parent
        }
        
        let mut segments: Vec<&str> = path.split('/').collect();
        if segments.last() == Some(&"") {
            segments.pop(); // remove trailing slash segment
        }
        if !segments.is_empty() {
            segments.pop(); // remove last file/folder segment
        }
        
        let new_path = if segments.len() <= 1 {
            "/".to_string()
        } else {
            let mut p = segments.join("/");
            if !p.ends_with('/') {
                p.push('/');
            }
            p
        };
        
        let mut parent_url = parsed.clone();
        parent_url.set_path(&new_path);
        parent_url.set_query(None);
        parent_url.set_fragment(None);
        Some(parent_url.to_string())
    } else {
        None
    }
}

fn get_url_label(url_str: &str, is_dir: bool) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        let path = parsed.path();
        if path == "/" || path.is_empty() {
            return "(root)".to_string();
        }
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if let Some(&last) = segments.last() {
            if is_dir {
                format!("{}/", last)
            } else {
                last.to_string()
            }
        } else {
            path.to_string()
        }
    } else {
        url_str.to_string()
    }
}


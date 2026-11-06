mod draw;
use crate::{
    state::CrawlState,
    visualization::{IcicleWeightMetric, NodeId, explorer::*},
};
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

type SelectionCallback = Rc<RefCell<Option<Box<dyn Fn(Option<String>)>>>>;
const PAGE_SIZE: usize = 200;
const VIEWS: [View; 6] = [
    View::Directory,
    View::Icicle,
    View::Paths,
    View::Links,
    View::Matrix,
    View::Rank,
];
const METRICS: [IcicleWeightMetric; 6] = [
    IcicleWeightMetric::UrlCount,
    IcicleWeightMetric::PageSizeBytes,
    IcicleWeightMetric::WordCount,
    IcicleWeightMetric::IssueCount,
    IcicleWeightMetric::InternalRankSum,
    IcicleWeightMetric::ResponseTimeMs,
];
const FILTERS: [PageFilter; 5] = [
    PageFilter::All,
    PageFilter::Indexable,
    PageFilter::Issues,
    PageFilter::Errors,
    PageFilter::Unreachable,
];
pub struct VisualizerWindow {
    window: gtk::Window,
    callback: SelectionCallback,
}
struct EvidenceItem {
    row: EvidenceRow,
    index: Arc<ExplorerIndex>,
    depths: Arc<Depths>,
    ranks: Option<Arc<[f32]>>,
}
struct Ui {
    window: glib::WeakRef<gtk::Window>,
    session: VisualizationSession,
    request: RefCell<ViewRequest>,
    ready: RefCell<Option<ReadyView>>,
    selected: RefCell<String>,
    hover: Cell<Option<Target>>,
    callback: SelectionCallback,
    dirty: Cell<Option<Instant>>,
    busy: Cell<bool>,
    updating: Cell<bool>,
    area: gtk::DrawingArea,
    mode: gtk::DropDown,
    metric: gtk::DropDown,
    filter: gtk::DropDown,
    normalize: gtk::CheckButton,
    matrix_controls: gtk::Box,
    matrix_source: gtk::DropDown,
    matrix_target: gtk::DropDown,
    search: gtk::SearchEntry,
    start: gtk::Entry,
    status: gtk::Label,
    legend: gtk::Label,
    selected_label: gtk::Label,
    store: gio::ListStore,
    selection: gtk::SingleSelection,
    evidence: gtk::ListView,
    count: gtk::Label,
    page: Cell<usize>,
    previous: gtk::Button,
    next: gtk::Button,
    cancel: gtk::Button,
    export: gtk::MenuButton,
    copy: gtk::Button,
    export_rx: RefCell<Option<std::sync::mpsc::Receiver<Result<String, String>>>>,
}
impl Ui {
    fn schedule(&self) {
        if !self.updating.get() {
            self.dirty.set(Some(Instant::now()));
        }
    }
    fn submit(&self, refresh: bool) {
        if refresh {
            let mut r = self.request.borrow_mut();
            r.directory = 0;
            r.tree_focus = None;
            r.scope = None;
        }
        self.busy.set(true);
        self.cancel.set_sensitive(true);
        self.evidence.set_sensitive(false);
        self.status.set_text(if refresh {
            "Preparing a fresh crawl snapshot…"
        } else {
            "Updating… Previous diagram remains visible."
        });
        self.session.request(self.request.borrow().clone(), refresh);
        self.dirty.set(None);
    }
    fn select(&self, url: String) {
        *self.selected.borrow_mut() = url.clone();
        self.selected_label.set_text(&url);
        self.selected_label.set_tooltip_text(Some(&url));
        self.copy.set_sensitive(!url.is_empty());
        if let Some(cb) = self.callback.borrow().as_ref() {
            cb(Some(url));
        }
        self.area.queue_draw();
    }
    fn selected_id(&self) -> Option<NodeId> {
        self.ready
            .borrow()
            .as_ref()
            .and_then(|v| v.index.node(&self.selected.borrow()))
    }
    fn rebuild_evidence(&self) {
        self.updating.set(true);
        self.store.remove_all();
        if let Some(ready) = self.ready.borrow().as_ref() {
            let total = ready.frame.evidence.len();
            let page = self.page.get().min(total.saturating_sub(1) / PAGE_SIZE);
            self.page.set(page);
            let from = page * PAGE_SIZE;
            let end = (from + PAGE_SIZE).min(total);
            for row in &ready.frame.evidence[from..end] {
                self.store.append(&glib::BoxedAnyObject::new(EvidenceItem {
                    row: *row,
                    index: ready.index.clone(),
                    depths: ready.frame.depths.clone(),
                    ranks: ready.frame.ranks.clone(),
                }));
            }
            self.count.set_text(&format!(
                "Evidence · {}–{} of {} {}",
                if total == 0 { 0 } else { from + 1 },
                end,
                total,
                if ready
                    .frame
                    .evidence
                    .iter()
                    .take(1)
                    .any(|r| r.target.is_some())
                {
                    "URL pairs"
                } else {
                    "URLs"
                }
            ));
            self.previous.set_sensitive(page > 0);
            self.next.set_sensitive(end < total);
        }
        self.selection.set_selected(gtk::INVALID_LIST_POSITION);
        self.updating.set(false);
    }
    fn row(&self) -> Option<(String, Option<String>)> {
        let object = self
            .selection
            .selected_item()?
            .downcast::<glib::BoxedAnyObject>()
            .ok()?;
        let item = object.borrow::<EvidenceItem>();
        Some((
            item.index.url(item.row.page).into(),
            item.row.target.map(|id| item.index.url(id).into()),
        ))
    }
    fn navigate(&self, url: String, focus: bool) {
        self.select(url.clone());
        if focus && matches!(self.request.borrow().view, View::Links | View::Paths) {
            let mut r = self.request.borrow_mut();
            r.selected_url = url.clone();
            r.scope = None;
            if r.view == View::Paths {
                r.tree_focus = self
                    .ready
                    .borrow()
                    .as_ref()
                    .and_then(|v| v.index.node(&url));
            }
            drop(r);
            self.schedule();
        }
    }
    fn restore(&self) {
        if let Some(v) = self.ready.borrow().as_ref() {
            self.updating.set(true);
            *self.request.borrow_mut() = v.request.clone();
            self.mode
                .set_selected(VIEWS.iter().position(|x| *x == v.request.view).unwrap_or(0) as u32);
            self.metric.set_selected(
                METRICS
                    .iter()
                    .position(|x| *x == v.request.metric)
                    .unwrap_or(0) as u32,
            );
            self.filter.set_selected(
                FILTERS
                    .iter()
                    .position(|x| *x == v.request.filter)
                    .unwrap_or(0) as u32,
            );
            self.search.set_text(&v.request.query);
            self.start.set_text(&v.request.start_url);
            self.normalize.set_active(v.request.normalize);
            self.metric
                .set_visible(matches!(v.request.view, View::Directory | View::Icicle));
            self.normalize.set_visible(v.request.view == View::Matrix);
            self.updating.set(false);
        }
    }
    fn hit(&self, x: f64, y: f64) -> Option<Target> {
        let ready = self.ready.borrow();
        let f = &ready.as_ref()?.frame;
        let scale = (self.area.width() as f64 / f.width).min(self.area.height() as f64 / f.height);
        f.hit_test(x / scale, y / scale).map(|m| m.target)
    }
    fn export(self: &Rc<Self>, kind: usize) {
        if self.export_rx.borrow().is_some() {
            return;
        }
        let Some(parent) = self.window.upgrade() else {
            return;
        };
        let Some((index, ranks)) = self
            .ready
            .borrow()
            .as_ref()
            .map(|r| (r.index.clone(), r.frame.ranks.clone()))
        else {
            return;
        };
        let selected = index.node(&self.selected.borrow());
        if kind == 2 && selected.is_none() {
            self.status
                .set_text("Select a URL in this snapshot before exporting its neighborhood.");
            return;
        }
        let dialog = gtk::FileDialog::new();
        dialog.set_title("Export crawl visualization evidence");
        dialog.set_initial_name(Some(["nodes.csv", "edges.csv", "neighborhood.json"][kind]));
        let weak = Rc::downgrade(self);
        dialog.save(Some(&parent), gio::Cancellable::NONE, move |result| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let file = match result {
                Ok(f) => f,
                Err(e) => {
                    if !e.matches(gtk::DialogError::Dismissed) {
                        ui.status.set_text(&format!("Export not saved: {e}"));
                    }
                    return;
                }
            };
            let Some(path) = file.path() else {
                ui.status.set_text("Choose a local file for export.");
                return;
            };
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            *ui.export_rx.borrow_mut() = Some(rx);
            ui.export.set_sensitive(false);
            ui.status.set_text("Preparing export in the background…");
            std::thread::spawn(move || {
                let mut snapshot = index.snapshot.clone();
                if kind != 1 {
                    let ranks = ranks.unwrap_or_else(|| {
                        crate::visualization::calculate_internal_rank(
                            &snapshot,
                            &Default::default(),
                        )
                        .into()
                    });
                    let mut nodes = snapshot.nodes.to_vec();
                    for (node, rank) in nodes.iter_mut().zip(ranks.iter()) {
                        node.internal_rank = *rank;
                    }
                    snapshot.nodes = nodes.into();
                }
                let result = match kind {
                    0 => crate::visualization::export::export_nodes_to_csv(
                        &snapshot,
                        &path.to_string_lossy(),
                    ),
                    1 => crate::visualization::export::export_edges_to_csv(
                        &snapshot,
                        &path.to_string_lossy(),
                    ),
                    _ => std::fs::write(
                        &path,
                        crate::visualization::export::export_subgraph_json(&snapshot, selected),
                    ),
                };
                let _ = tx.send(
                    result
                        .map(|_| format!("Export saved: {}", path.display()))
                        .map_err(|e| {
                            format!("Export failed: {e}. Choose another file or check permissions.")
                        }),
                );
            });
        });
    }
}
impl VisualizerWindow {
    pub fn new(
        parent: &(impl IsA<gtk::Window> + IsA<glib::Object>),
        crawl_state: CrawlState,
        start_url: Option<String>,
    ) -> Self {
        let window = gtk::Window::builder()
            .title("Tadpole — Site visualizations")
            .transient_for(parent)
            .default_width(1180)
            .default_height(850)
            .build();
        let callback: SelectionCallback = Rc::new(RefCell::new(None));
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_start(12);
        root.set_margin_end(12);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let mode = gtk::DropDown::from_strings(&[
            "Directory area",
            "Directory icicle",
            "Link-depth tree",
            "URL links",
            "Section links",
            "Depth + rank",
        ]);
        mode.set_tooltip_text(Some("Choose the SEO question to explore"));
        let metric = gtk::DropDown::from_strings(&[
            "URL count",
            "Page bytes",
            "Words",
            "Pages with issues",
            "Internal rank",
            "Response time",
        ]);
        metric.set_tooltip_text(Some("Additive directory weight"));
        let normalize = gtk::CheckButton::with_label("Pairs per source page");
        normalize.set_visible(false);
        let up = gtk::Button::with_label("Up");
        let reset = gtk::Button::with_label("Reset");
        let expand = gtk::Button::with_label("Expand");
        let refresh = gtk::Button::with_label("Refresh");
        let cancel = gtk::Button::with_label("Cancel");
        cancel.set_sensitive(false);
        row.append(&mode);
        row.append(&metric);
        row.append(&normalize);
        row.append(&up);
        row.append(&reset);
        row.append(&expand);
        row.append(&refresh);
        row.append(&cancel);
        root.append(&row);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let search = gtk::SearchEntry::builder()
            .placeholder_text("Filter URLs…")
            .hexpand(true)
            .build();
        let filter = gtk::DropDown::from_strings(&[
            "All crawled URLs",
            "Indexable",
            "Pages with issues",
            "HTTP errors",
            "Unreachable from start",
        ]);
        let start = gtk::Entry::builder()
            .placeholder_text("Start URL for shortest paths")
            .hexpand(true)
            .build();
        start.set_tooltip_text(Some(
            "Enter a crawled URL and press Enter, or use the selected URL as start",
        ));
        if let Some(url) = start_url.as_ref() {
            start.set_text(url);
        }
        let set_start = gtk::Button::with_label("Use selected as start");
        controls.append(&search);
        controls.append(&filter);
        controls.append(&start);
        controls.append(&set_start);
        root.append(&controls);
        let paned = gtk::Paned::new(gtk::Orientation::Vertical);
        paned.set_vexpand(true);
        paned.set_position(500);
        paned.set_shrink_start_child(true);
        paned.set_shrink_end_child(true);
        let area = gtk::DrawingArea::builder()
            .content_width(640)
            .content_height(440)
            .hexpand(true)
            .vexpand(true)
            .build();
        area.update_property(&[gtk::accessible::Property::Label(
            "Site diagram. The complete evidence list below supports keyboard selection.",
        )]);
        let canvas = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&area)
            .build();
        paned.set_start_child(Some(&canvas));
        let bottom = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let legend = gtk::Label::new(None);
        legend.set_wrap(true);
        legend.set_xalign(0.);
        legend.add_css_class("dim-label");
        bottom.append(&legend);
        let evidence_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let count = gtk::Label::new(Some("Evidence will appear here"));
        count.set_xalign(0.);
        count.set_hexpand(true);
        let previous = gtk::Button::with_label("Previous");
        let next = gtk::Button::with_label("Next");
        let open_source = gtk::Button::with_label("Focus source / URL");
        let open_target = gtk::Button::with_label("Focus target");
        evidence_header.append(&count);
        evidence_header.append(&open_source);
        evidence_header.append(&open_target);
        evidence_header.append(&previous);
        evidence_header.append(&next);
        bottom.append(&evidence_header);
        let matrix_controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let matrix_source = gtk::DropDown::from_strings(&[]);
        let matrix_target = gtk::DropDown::from_strings(&[]);
        matrix_source.set_hexpand(true);
        matrix_target.set_hexpand(true);
        matrix_source.update_property(&[gtk::accessible::Property::Label("Matrix source section")]);
        matrix_target.update_property(&[gtk::accessible::Property::Label("Matrix target section")]);
        let matrix_open = gtk::Button::with_label("Show section links");
        matrix_controls.append(&matrix_source);
        matrix_controls.append(&matrix_target);
        matrix_controls.append(&matrix_open);
        matrix_controls.set_visible(false);
        bottom.append(&matrix_controls);

        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let selection = gtk::SingleSelection::new(Some(store.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, object| {
            let item = object.downcast_ref::<gtk::ListItem>().unwrap();
            let b = gtk::Box::new(gtk::Orientation::Vertical, 2);
            b.set_margin_top(4);
            b.set_margin_bottom(4);
            let main = gtk::Label::new(None);
            main.set_xalign(0.);
            main.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            let sub = gtk::Label::new(None);
            sub.set_xalign(0.);
            sub.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            sub.add_css_class("dim-label");
            b.append(&main);
            b.append(&sub);
            item.set_child(Some(&b));
        });
        factory.connect_bind(|_, object| {
            let item = object.downcast_ref::<gtk::ListItem>().unwrap();
            let Some(value) = item.item().and_downcast::<glib::BoxedAnyObject>() else {
                return;
            };
            let data = value.borrow::<EvidenceItem>();
            let b = item.child().unwrap();
            let main = b.first_child().unwrap().downcast::<gtk::Label>().unwrap();
            let sub = main
                .next_sibling()
                .unwrap()
                .downcast::<gtk::Label>()
                .unwrap();
            let url = data.index.url(data.row.page);
            main.set_text(url);
            main.set_tooltip_text(Some(url));
            let node = &data.index.snapshot.nodes[data.row.page as usize];
            let depth = data.depths.depth[data.row.page as usize]
                .map_or("No path / unknown start".into(), |d| {
                    format!("{d} links from start")
                });
            let rank = data
                .ranks
                .as_ref()
                .map(|r| format!(" · {:.5}% internal rank", r[data.row.page as usize] * 100.0))
                .unwrap_or_default();
            let detail = data.row.target.map_or_else(
                || {
                    format!(
                        "HTTP {} · {} · {} incoming / {} outgoing unique links",
                        node.status_code, depth, node.unique_in_degree, node.unique_out_degree
                    ) + &rank
                },
                |target| {
                    format!(
                        "→ {} · target HTTP {}",
                        data.index.url(target),
                        data.index.snapshot.nodes[target as usize].status_code
                    )
                },
            );
            sub.set_text(&detail);
            sub.set_tooltip_text(Some(&detail));
        });
        let evidence = gtk::ListView::new(Some(selection.clone()), Some(factory));
        evidence.set_single_click_activate(false);
        let list_scroll = gtk::ScrolledWindow::builder()
            .min_content_height(120)
            .vexpand(true)
            .child(&evidence)
            .build();
        bottom.append(&list_scroll);
        paned.set_end_child(Some(&bottom));
        root.append(&paned);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let selected_label =
            gtk::Label::new(Some("Choose a URL from the diagram or evidence list"));
        selected_label.set_selectable(true);
        selected_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        selected_label.set_hexpand(true);
        selected_label.set_xalign(0.);
        let copy = gtk::Button::with_label("Copy URL");
        copy.set_sensitive(false);
        let export = gtk::MenuButton::builder().label("Export").build();
        export.set_sensitive(false);
        let popover = gtk::Popover::new();
        let export_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let export_buttons = [
            gtk::Button::with_label("All crawled nodes CSV"),
            gtk::Button::with_label("All crawled edges CSV"),
            gtk::Button::with_label("Selected URL neighborhood JSON"),
        ];
        for button in &export_buttons {
            export_box.append(button);
        }
        popover.set_child(Some(&export_box));
        export.set_popover(Some(&popover));
        footer.append(&selected_label);
        footer.append(&copy);
        footer.append(&export);
        root.append(&footer);
        let status = gtk::Label::new(Some("Preparing visualization in the background…"));
        status.set_xalign(0.);
        status.set_wrap(true);
        status.add_css_class("dim-label");
        root.append(&status);
        window.set_child(Some(&root));
        let session = VisualizationSession::new(move || crawl_state.visualization_results());
        let request = ViewRequest {
            start_url: start_url.unwrap_or_default(),
            ..Default::default()
        };
        let ui = Rc::new(Ui {
            window: window.downgrade(),
            session,
            request: RefCell::new(request),
            ready: RefCell::new(None),
            selected: RefCell::new(String::new()),
            hover: Cell::new(None),
            callback: callback.clone(),
            dirty: Cell::new(None),
            busy: Cell::new(false),
            updating: Cell::new(false),
            area,
            mode,
            metric,
            normalize,
            matrix_controls,
            matrix_source,
            matrix_target,
            filter,
            search,
            start,
            status,
            legend,
            selected_label,
            store,
            selection,
            evidence,
            count,
            page: Cell::new(0),
            previous,
            next,
            cancel,
            export,
            copy,
            export_rx: RefCell::new(None),
        });
        let weak = Rc::downgrade(&ui);
        ui.area.set_draw_func(move |area, cr, w, h| {
            if let Some(u) = weak.upgrade() {
                let ready = u.ready.borrow();
                draw::paint(
                    area,
                    cr,
                    w,
                    h,
                    ready.as_ref().map(|r| r.frame.as_ref()),
                    u.selected_id(),
                    u.hover.get(),
                );
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.area.connect_resize(move |_, w, h| {
            if let Some(u) = weak.upgrade() {
                let mut r = u.request.borrow_mut();
                if r.width != w as u32 || r.height != h as u32 {
                    r.width = w.max(640) as u32;
                    r.height = h.max(320) as u32;
                    drop(r);
                    u.schedule();
                }
            }
        });
        let motion = gtk::EventControllerMotion::new();
        let weak = Rc::downgrade(&ui);
        motion.connect_motion(move |_, x, y| {
            if let Some(u) = weak.upgrade() {
                let target = u.hit(x, y);
                if u.hover.replace(target) != target {
                    let ready = u.ready.borrow();
                    let tip = ready
                        .as_ref()
                        .and_then(|v| {
                            let scale = (u.area.width() as f64 / v.frame.width)
                                .min(u.area.height() as f64 / v.frame.height);
                            v.frame.hit_test(x / scale, y / scale)
                        })
                        .map(|m| m.tooltip.as_str());
                    u.area.set_tooltip_text(tip);
                    u.area.queue_draw();
                }
            }
        });
        let weak = Rc::downgrade(&ui);
        motion.connect_leave(move |_| {
            if let Some(u) = weak.upgrade() {
                u.hover.set(None);
                u.area.queue_draw();
            }
        });
        ui.area.add_controller(motion);
        let click = gtk::GestureClick::new();
        let weak = Rc::downgrade(&ui);
        click.connect_pressed(move |_, press, x, y| {
            if let Some(u) = weak.upgrade() {
                if u.busy.get() {
                    return;
                }
                let Some(target) = u.hit(x, y) else {
                    return;
                };
                match target {
                    Target::Page(id) => {
                        let url = u
                            .ready
                            .borrow()
                            .as_ref()
                            .map(|v| v.index.url(id).to_string());
                        if let Some(url) = url {
                            let view = u.request.borrow().view;
                            u.navigate(url, view == View::Links || press == 2);
                        }
                    }
                    Target::Directory(id) => {
                        let mut r = u.request.borrow_mut();
                        r.directory = id;
                        r.scope = None;
                        drop(r);
                        u.schedule();
                    }
                    other => {
                        u.request.borrow_mut().scope = Some(other);
                        u.schedule();
                    }
                }
            }
        });
        ui.area.add_controller(click);
        let weak = Rc::downgrade(&ui);
        ui.mode.connect_selected_notify(move |d| {
            if let Some(u) = weak.upgrade() {
                if u.updating.get() {
                    return;
                }
                let view = VIEWS[d.selected() as usize];
                {
                    let mut r = u.request.borrow_mut();
                    r.view = view;
                    r.scope = None;
                    r.selected_url = u.selected.borrow().clone();
                }
                u.metric
                    .set_visible(matches!(view, View::Directory | View::Icicle));
                u.normalize.set_visible(view == View::Matrix);
                u.page.set(0);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.metric.connect_selected_notify(move |d| {
            if let Some(u) = weak.upgrade() {
                if u.updating.get() {
                    return;
                }
                u.request.borrow_mut().metric = METRICS[d.selected() as usize];
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.normalize.connect_toggled(move |b| {
            if let Some(u) = weak.upgrade() {
                if u.updating.get() {
                    return;
                }
                u.request.borrow_mut().normalize = b.is_active();
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.filter.connect_selected_notify(move |d| {
            if let Some(u) = weak.upgrade() {
                if u.updating.get() {
                    return;
                }
                let mut r = u.request.borrow_mut();
                r.filter = FILTERS[d.selected() as usize];
                r.scope = None;
                drop(r);
                u.page.set(0);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.search.connect_search_changed(move |entry| {
            if let Some(u) = weak.upgrade() {
                if u.updating.get() {
                    return;
                }
                let mut r = u.request.borrow_mut();
                if r.query == entry.text().as_str() {
                    return;
                }
                r.query = entry.text().to_string();
                r.scope = None;
                drop(r);
                u.page.set(0);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.start.connect_activate(move |entry| {
            if let Some(u) = weak.upgrade() {
                let mut r = u.request.borrow_mut();
                r.start_url = entry.text().trim().into();
                r.tree_focus = None;
                r.scope = None;
                drop(r);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        set_start.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                let url = u.selected.borrow().clone();
                if url.is_empty() {
                    u.status.set_text(
                        "Select a crawled URL first, or enter a start URL and press Enter.",
                    );
                    return;
                }
                u.start.set_text(&url);
                let mut r = u.request.borrow_mut();
                r.start_url = url;
                r.tree_focus = None;
                r.scope = None;
                drop(r);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        up.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                let ready = u.ready.borrow();
                if let Some(v) = ready.as_ref() {
                    let mut r = u.request.borrow_mut();
                    r.scope = None;
                    match r.view {
                        View::Directory | View::Icicle => {
                            r.directory = v
                                .index
                                .directories
                                .get(r.directory)
                                .and_then(|d| d.parent)
                                .unwrap_or(0)
                        }
                        View::Paths => {
                            r.tree_focus = r.tree_focus.and_then(|id| {
                                v.frame.depths.parent.get(id as usize).copied().flatten()
                            })
                        }
                        _ => {}
                    }
                }
                drop(ready);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        reset.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                let mut r = u.request.borrow_mut();
                r.directory = 0;
                r.tree_focus = None;
                r.scope = None;
                drop(r);
                u.page.set(0);
                u.area.set_content_height(440);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        expand.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                let height = (u.area.content_height() + 400).min(8192);
                u.area.set_content_height(height);
                u.schedule();
            }
        });
        let weak = Rc::downgrade(&ui);
        refresh.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                u.submit(true);
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.cancel.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                u.session.cancel();
                u.dirty.set(None);
                u.busy.set(false);
                u.cancel.set_sensitive(false);
                u.evidence.set_sensitive(true);
                u.restore();
                u.status.set_text(
                    "Cancelled. Showing the last completed view; Refresh retries preparation.",
                );
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.selection.connect_selected_notify(move |_| {
            if let Some(u) = weak.upgrade() {
                if !u.updating.get() {
                    if let Some((url, _)) = u.row() {
                        u.select(url);
                    }
                }
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.evidence.connect_activate(move |_, _| {
            if let Some(u) = weak.upgrade() {
                if let Some((source, target)) = u.row() {
                    u.navigate(target.unwrap_or(source), true);
                }
            }
        });
        for (target, button) in [(false, open_source), (true, open_target)] {
            let weak = Rc::downgrade(&ui);
            button.connect_clicked(move |_| {
                if let Some(u) = weak.upgrade() {
                    if let Some((source, dest)) = u.row() {
                        u.navigate(
                            if target {
                                dest.unwrap_or(source)
                            } else {
                                source
                            },
                            true,
                        );
                    }
                }
            });
        }
        for (forward, button) in [(false, ui.previous.clone()), (true, ui.next.clone())] {
            let weak = Rc::downgrade(&ui);
            button.connect_clicked(move |_| {
                if let Some(u) = weak.upgrade() {
                    u.page.set(if forward {
                        u.page.get() + 1
                    } else {
                        u.page.get().saturating_sub(1)
                    });
                    u.rebuild_evidence();
                }
            });
        }
        let weak = Rc::downgrade(&ui);
        matrix_open.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                let ready = u.ready.borrow();
                if let Some(v) = ready.as_ref() {
                    if v.frame.view == View::Matrix {
                        let n = (v.frame.marks.len() as f64).sqrt() as usize;
                        let a = u.matrix_source.selected() as usize;
                        let b = u.matrix_target.selected() as usize;
                        if a < n && b < n {
                            if let Some(mark) = v.frame.marks.get(a * n + b) {
                                u.request.borrow_mut().scope = Some(mark.target);
                                u.page.set(0);
                                u.schedule();
                            }
                        }
                    }
                }
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.copy.connect_clicked(move |_| {
            if let Some(u) = weak.upgrade() {
                if let Some(display) = gdk::Display::default() {
                    display.clipboard().set_text(&u.selected.borrow());
                }
            }
        });
        for (kind, button) in export_buttons.into_iter().enumerate() {
            let weak = Rc::downgrade(&ui);
            let pop = popover.downgrade();
            button.connect_clicked(move |_| {
                if let Some(pop) = pop.upgrade() {
                    pop.popdown();
                }
                if let Some(u) = weak.upgrade() {
                    u.export(kind);
                }
            });
        }
        let weak = Rc::downgrade(&ui);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            let Some(u) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if u.dirty
                .get()
                .is_some_and(|at| at.elapsed() >= Duration::from_millis(80))
            {
                u.submit(false);
            }
            if let Some(event) = u.session.poll() {
                u.busy.set(false);
                u.cancel.set_sensitive(false);
                u.evidence.set_sensitive(true);
                match event {
                    SessionEvent::Ready(v) => {
                        u.legend.set_text(&v.frame.legend);
                        u.matrix_controls.set_visible(v.frame.view == View::Matrix);
                        if v.frame.view == View::Matrix {
                            let n = (v.frame.marks.len() as f64).sqrt() as usize;
                            let sources: Vec<String> = (0..n)
                                .map(|i| {
                                    v.frame.marks[i * n]
                                        .tooltip
                                        .split(" → ")
                                        .next()
                                        .unwrap_or("")
                                        .to_string()
                                })
                                .collect();
                            let targets: Vec<String> = (0..n)
                                .map(|i| {
                                    v.frame.marks[i]
                                        .tooltip
                                        .split(" → ")
                                        .nth(1)
                                        .unwrap_or("")
                                        .rsplit_once(": ")
                                        .map_or("", |(name, _)| name)
                                        .to_string()
                                })
                                .collect();
                            let source_refs: Vec<&str> =
                                sources.iter().map(String::as_str).collect();
                            let target_refs: Vec<&str> =
                                targets.iter().map(String::as_str).collect();
                            u.matrix_source
                                .set_model(Some(&gtk::StringList::new(&source_refs)));
                            u.matrix_target
                                .set_model(Some(&gtk::StringList::new(&target_refs)));
                        }

                        let stamp = glib::DateTime::now_local()
                            .ok()
                            .and_then(|d| d.format("%H:%M:%S").ok())
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        u.status.set_text(&format!(
                            "View updated at {stamp}. {} Refresh captures crawl updates.",
                            v.frame.status
                        ));
                        if u.start.text().is_empty() {
                            if let Some(start) = v.index.inferred_start() {
                                u.start.set_text(start);
                                u.request.borrow_mut().start_url = start.into();
                            }
                        }
                        if u.selected.borrow().is_empty() {
                            if let Some(id) = v.frame.focus {
                                u.select(v.index.url(id).into());
                            }
                        }
                        *u.ready.borrow_mut() = Some(v);
                        u.hover.set(None);
                        u.export.set_sensitive(true);
                        u.page.set(0);
                        u.rebuild_evidence();
                        u.area.queue_draw();
                    }
                    SessionEvent::Failed { message, .. } => u.status.set_text(&message),
                }
            }
            let export_result = u
                .export_rx
                .borrow()
                .as_ref()
                .and_then(|rx| rx.try_recv().ok());
            if let Some(result) = export_result {
                u.status.set_text(&result.unwrap_or_else(|e| e));
                u.export_rx.borrow_mut().take();
                u.export.set_sensitive(true);
            }
            glib::ControlFlow::Continue
        });
        let keep = ui.clone();
        window.connect_destroy(move |_| keep.session.close());
        ui.submit(false);
        Self { window, callback }
    }
    pub fn connect_selection_changed<F: Fn(Option<String>) + 'static>(&self, f: F) {
        *self.callback.borrow_mut() = Some(Box::new(f));
    }
    pub fn show(&self) {
        self.window.present();
    }
}

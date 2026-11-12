//! Native presentation of the current crawl's sitemap/discovery evidence.
use crate::{
    discovery::{self, DiscoveryRow, DiscoverySnapshot, Finding},
    state::CrawlState,
};
use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type Data = Rc<RefCell<Option<DiscoverySnapshot>>>;

fn text(value: &str, css: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(value));
    label.set_xalign(0.0);
    if !css.is_empty() {
        label.add_css_class(css);
    }
    label
}
fn paragraph(value: &str) -> gtk::Label {
    let label = text(value, "");
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_selectable(true);
    label
}
fn box_with_margin(orientation: gtk::Orientation, spacing: i32, margin: i32) -> gtk::Box {
    let widget = gtk::Box::new(orientation, spacing);
    widget.set_margin_start(margin);
    widget.set_margin_end(margin);
    widget.set_margin_top(margin);
    widget.set_margin_bottom(margin);
    widget
}
fn clear(widget: &gtk::Box) {
    while let Some(child) = widget.first_child() {
        widget.remove(&child);
    }
}

pub struct DiscoveryPanel {
    widget: gtk::Box,
    state: CrawlState,
    data: Data,
    filter: Cell<usize>,
    model: gtk::StringList,
    navigation: gtk::ListBox,
    selection: gtk::SingleSelection,
    search: gtk::SearchEntry,
    heading: gtk::Label,
    count: gtk::Label,
    counts: Vec<gtk::Label>,
    coverage: gtk::Label,
    totals: gtk::Label,
    detail: gtk::Box,
    report: gtk::Paned,
    empty: adw::StatusPage,
    sources: gtk::Box,
    manual_sources: gtk::Box,
    controls: gtk::Box,
    enabled: gtk::CheckButton,
    export: gtk::Button,
    message: gtk::Label,
    on_link: Rc<dyn Fn(String)>,
}

impl DiscoveryPanel {
    pub fn new(
        state: CrawlState,
        on_link: impl Fn(String) + 'static,
        on_recrawl: impl Fn() + 'static,
    ) -> Rc<Self> {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.add_css_class("discovery-report");
        let intro = box_with_margin(gtk::Orientation::Vertical, 7, 14);
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let title = text("Sitemap & Discovery", "title-2");
        title.set_hexpand(true);
        header.append(&title);
        let manage = gtk::ToggleButton::with_label("Manage sitemaps");
        header.append(&manage);
        let export = gtk::Button::with_label("Export CSV");
        export.set_sensitive(false);
        header.append(&export);
        intro.append(&header);
        intro.append(&paragraph(
            "Compare sitemap declarations with pages reachable through internal links.",
        ));
        let coverage = paragraph(
            "Start a crawl to collect discovery evidence, or configure sitemap sources below.",
        );
        intro.append(&coverage);
        widget.append(&intro);

        let source_revealer = gtk::Revealer::new();
        source_revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        let source_body = box_with_margin(gtk::Orientation::Vertical, 8, 14);
        source_body.set_margin_top(0);
        let controls = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let enabled = gtk::CheckButton::with_label("Collect XML sitemaps during crawls");
        enabled.set_active(state.get_config().sitemap_enabled);
        controls.append(&enabled);
        controls.append(&paragraph("Crawl and Path modes read robots.txt. Manual sources must be HTTP(S) URLs on the crawl site. Changes apply to the next crawl."));
        let manual_sources = gtk::Box::new(gtk::Orientation::Vertical, 4);
        controls.append(&manual_sources);
        let add_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let url_entry = gtk::Entry::builder()
            .placeholder_text("https://example.com/sitemap.xml")
            .hexpand(true)
            .build();
        url_entry.update_property(&[gtk::accessible::Property::Label("Sitemap URL")]);
        add_row.append(&url_entry);
        let add = gtk::Button::with_label("Add URL");
        add_row.append(&add);
        let recrawl = gtk::Button::with_label("Apply and recrawl");
        add_row.append(&recrawl);
        controls.append(&add_row);
        source_body.append(&controls);
        let sources = gtk::Box::new(gtk::Orientation::Vertical, 6);
        source_body.append(&sources);
        let source_scroll = gtk::ScrolledWindow::builder()
            .max_content_height(210)
            .propagate_natural_height(true)
            .child(&source_body)
            .build();
        source_revealer.set_child(Some(&source_scroll));
        widget.append(&source_revealer);
        widget.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        let totals = text("", "dim-label");
        totals.set_wrap(true);
        totals.set_margin_start(14);
        totals.set_margin_top(8);
        totals.set_margin_bottom(8);
        widget.append(&totals);

        let report = gtk::Paned::new(gtk::Orientation::Horizontal);
        report.set_position(225);
        report.set_resize_start_child(false);
        report.set_shrink_start_child(false);
        report.set_vexpand(true);
        widget.append(&report);
        let sidebar = box_with_margin(gtk::Orientation::Vertical, 8, 10);
        sidebar.set_width_request(195);
        sidebar.append(&text("Discovery findings", "heading"));
        let navigation = gtk::ListBox::new();
        navigation.add_css_class("navigation-sidebar");
        sidebar.append(&navigation);
        let mut counts = Vec::new();
        let names = std::iter::once("All known URLs")
            .chain(Finding::ALL.iter().map(|f| f.label()))
            .chain(std::iter::once("No finding"));
        for name in names {
            let row = box_with_margin(gtk::Orientation::Horizontal, 8, 7);
            let name = text(name, "");
            name.set_wrap(true);
            name.set_hexpand(true);
            row.append(&name);
            let count = text("0", "numeric");
            row.append(&count);
            counts.push(count);
            navigation.append(&row);
        }
        let hint = paragraph(
            "One URL can have several findings. Counts describe the collected crawl and imported sources.",
        );
        hint.add_css_class("dim-label");
        sidebar.append(&hint);
        let sidebar_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&sidebar)
            .build();
        report.set_start_child(Some(&sidebar_scroll));

        let work = gtk::Paned::new(gtk::Orientation::Vertical);
        work.set_position(250);
        work.set_shrink_start_child(false);
        report.set_end_child(Some(&work));
        let table_area = box_with_margin(gtk::Orientation::Vertical, 8, 10);
        let table_header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let heading = text("All known URLs", "heading");
        heading.set_hexpand(true);
        table_header.append(&heading);
        let count = text("0 URLs", "dim-label");
        table_header.append(&count);
        table_area.append(&table_header);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search URL, finding or HTTP status"));
        search.update_property(&[gtk::accessible::Property::Label(
            "Search discovery findings",
        )]);
        table_area.append(&search);
        let data: Data = Rc::new(RefCell::new(None));
        let model = gtk::StringList::new(&[]);
        let table = gtk::ColumnView::new(None::<gtk::SelectionModel>);
        table.add_css_class("data-table");
        table.set_show_row_separators(true);
        let sorted = gtk::SortListModel::new(Some(model.clone()), table.sorter());
        let selection = gtk::SingleSelection::new(Some(sorted));
        table.set_model(Some(&selection));
        for (name, width, expand, kind) in [
            ("URL", 330, true, 0),
            ("HTTP", 65, false, 1),
            ("Inlinks", 65, false, 2),
            ("Finding", 210, false, 3),
        ] {
            table.append_column(&column(name, width, expand, kind, data.clone()));
        }
        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&table)
            .build();
        table_area.append(&scrolled);
        work.set_start_child(Some(&table_area));
        let detail = box_with_margin(gtk::Orientation::Vertical, 10, 14);
        let details_scroll = gtk::ScrolledWindow::builder()
            .min_content_height(220)
            .child(&detail)
            .build();
        work.set_end_child(Some(&details_scroll));
        let empty = adw::StatusPage::new();
        empty.set_vexpand(true);
        widget.append(&empty);
        let message = paragraph("");
        message.set_visible(false);
        message.connect_label_notify(|label| label.set_visible(!label.text().is_empty()));
        message.set_margin_start(14);
        message.set_margin_end(14);
        message.set_margin_top(6);
        message.set_margin_bottom(8);
        message.add_css_class("dim-label");
        widget.append(&message);

        let panel = Rc::new(Self {
            widget,
            state,
            data,
            filter: Cell::new(0),
            model,
            navigation: navigation.clone(),
            selection: selection.clone(),
            search: search.clone(),
            heading,
            count,
            counts,
            coverage,
            totals,
            detail,
            report,
            empty,
            sources,
            manual_sources,
            controls,
            enabled: enabled.clone(),
            export: export.clone(),
            message,
            on_link: Rc::new(on_link),
        });
        navigation.select_row(navigation.row_at_index(0).as_ref());
        navigation.connect_row_selected({
            let panel = Rc::downgrade(&panel);
            move |_, row| {
                if let (Some(p), Some(row)) = (panel.upgrade(), row) {
                    p.filter.set(row.index() as usize);
                    p.refilter();
                }
            }
        });
        search.connect_search_changed({
            let p = Rc::downgrade(&panel);
            move |_| {
                if let Some(p) = p.upgrade() {
                    p.refilter();
                }
            }
        });
        selection.connect_selected_notify({
            let p = Rc::downgrade(&panel);
            move |_| {
                if let Some(p) = p.upgrade() {
                    p.show_selected();
                }
            }
        });
        manage.connect_toggled(move |button| source_revealer.set_reveal_child(button.is_active()));
        enabled.connect_toggled({
            let p = Rc::downgrade(&panel);
            move |button| {
                if let Some(p) = p.upgrade()
                    && !p.state.is_crawling()
                {
                    let mut config = p.state.get_config();
                    config.sitemap_enabled = button.is_active();
                    p.state.set_config(config);
                }
            }
        });
        add.connect_clicked({ let p = Rc::downgrade(&panel); move |_| {
            let Some(p) = p.upgrade() else { return };
            if p.state.is_crawling() { return; }
            let Some(url) = discovery::normalize_url(url_entry.text().as_str()) else { p.message.set_text("Enter an HTTP(S) sitemap URL without embedded credentials."); return };
            let mut config = p.state.get_config();
            if config.sitemap_urls.contains(&url) { p.message.set_text("This manual source is already configured."); return; }
            if config.sitemap_urls.len() >= crate::sitemap::MAX_SOURCES { p.message.set_text("The manual sitemap source limit has been reached. Remove a source before adding another."); return; }
            config.sitemap_urls.push(url); config.sitemap_enabled = true; p.state.set_config(config);
            url_entry.set_text(""); p.enabled.set_active(true); p.render_manual_sources();
            p.message.set_text("Source added for the next crawl. Apply and recrawl to fetch it and update findings.");
        } });
        recrawl.connect_clicked(move |_| on_recrawl());
        export.connect_clicked({
            let p = Rc::downgrade(&panel);
            move |_| {
                let Some(p) = p.upgrade() else { return };
                let Some(snapshot) = p.data.borrow().clone() else {
                    return;
                };
                let indices = p.visible_indices();
                let parent = p.widget.root().and_downcast::<gtk::Window>();
                let dialog = gtk::FileDialog::builder()
                    .title("Export discovery findings")
                    .initial_name("tadpole-discovery.csv")
                    .build();
                let p = Rc::downgrade(&p);
                dialog.save(parent.as_ref(), None::<&gio::Cancellable>, move |result| {
                    let (Some(p), Ok(file)) = (p.upgrade(), result) else {
                        return;
                    };
                    let Some(path) = file.path() else {
                        p.message.set_text("Choose a local file for this export.");
                        return;
                    };
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    std::thread::spawn(move || {
                        let rows: Vec<_> = indices
                            .iter()
                            .filter_map(|i| snapshot.rows.get(*i))
                            .collect();
                        let result = std::fs::File::create(path)
                            .map_err(|e| e.to_string())
                            .and_then(|file| {
                                discovery::export_csv(file, &snapshot, &rows)
                                    .map_err(|e| e.to_string())
                            });
                        let _ = tx.send(result);
                    });
                    glib::MainContext::default().spawn_local(async move {
                        p.message.set_text(&match rx.await {
                            Ok(Ok(())) => {
                                "Exported the filtered discovery report with coverage and evidence."
                                    .into()
                            }
                            Ok(Err(error)) => {
                                format!("Export failed: {error}. Choose a writable file and retry.")
                            }
                            Err(_) => "Export could not finish. Retry the export.".into(),
                        });
                    });
                });
            }
        });
        panel.refresh();
        panel
    }

    pub fn select_finding(&self, finding: Finding) {
        if let Some(index) = Finding::ALL.iter().position(|f| *f == finding) {
            self.navigation
                .select_row(self.navigation.row_at_index(index as i32 + 1).as_ref());
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }

    pub fn refresh(self: &Rc<Self>) {
        self.model.splice(0, self.model.n_items(), &[]);
        let snapshot = self.state.discovery();
        self.coverage.set_text(&snapshot.as_ref().map_or_else(|| "Discovery data has not been collected. Configure sources and start a new crawl. Older projects do not contain this evidence.".into(), |s| s.coverage.summary()));
        if let Some(s) = &snapshot {
            let reached = s.rows.iter().filter(|r| r.reached_depth.is_some()).count();
            let failed = s
                .collection
                .sources
                .iter()
                .filter(|s| s.error.is_some())
                .count();
            self.totals.set_text(&format!("{} in sitemaps · {} reached through links · {} known URLs · {} unavailable sources{}", s.collection.memberships.len(), reached, s.rows.len(), failed, if s.collection.truncated { " · sitemap collection truncated" } else { "" }));
        } else {
            self.totals.set_text("");
        }
        *self.data.borrow_mut() = snapshot;
        self.controls.set_sensitive(!self.state.is_crawling());
        self.enabled
            .set_active(self.state.get_config().sitemap_enabled);
        self.render_manual_sources();
        clear(&self.sources);
        if let Some(s) = self.data.borrow().as_ref() {
            self.sources
                .append(&text("Sources observed in this audit", "heading"));
            for source in &s.collection.sources {
                self.sources.append(&paragraph(&source.url));
                self.sources.append(&paragraph(&format!(
                    "{} · {} URLs{}",
                    source.status,
                    source.url_count,
                    source
                        .error
                        .as_ref()
                        .map_or(String::new(), |e| format!(" · {e}"))
                )));
                if let Some(parent) = &source.parent {
                    self.sources
                        .append(&paragraph(&format!("From index: {parent}")));
                }
                self.sources
                    .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            }
            for warning in &s.collection.warnings {
                self.sources.append(&paragraph(warning));
            }
        }
        self.refilter();
    }

    fn render_manual_sources(self: &Rc<Self>) {
        clear(&self.manual_sources);
        for url in self.state.get_config().sitemap_urls {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let value = paragraph(&url);
            value.set_hexpand(true);
            row.append(&value);
            let remove = gtk::Button::builder()
                .icon_name("list-remove-symbolic")
                .tooltip_text("Remove this manual source for the next crawl")
                .build();
            row.append(&remove);
            self.manual_sources.append(&row);
            remove.connect_clicked({ let p = Rc::downgrade(self); move |_| { if let Some(p) = p.upgrade() && !p.state.is_crawling() { let mut config = p.state.get_config(); config.sitemap_urls.retain(|s| s != &url); p.state.set_config(config); p.render_manual_sources(); p.message.set_text("Source removed for the next crawl. Current findings still describe the saved audit."); } } });
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        (0..self.selection.n_items())
            .filter_map(|i| {
                self.selection
                    .item(i)
                    .and_downcast::<gtk::StringObject>()?
                    .string()
                    .parse()
                    .ok()
            })
            .collect()
    }

    fn refilter(&self) {
        let selected = self
            .selection
            .selected_item()
            .and_downcast::<gtk::StringObject>()
            .and_then(|s| s.string().parse::<usize>().ok());
        let query = self.search.text().to_lowercase();
        let data = self.data.borrow();
        let available = data.as_ref().is_some_and(|s| {
            !s.rows.is_empty()
                && (!s.collection.memberships.is_empty()
                    || s.collection.sources.iter().any(|s| s.error.is_none()))
        });
        self.report.set_visible(available);
        self.empty.set_visible(!available);
        if !available {
            let (title, description) = match data.as_ref() {
                Some(s) if !s.coverage.finished => (
                    "Collecting discovery evidence",
                    "Results will be reconciled when the crawl finishes. You can pause or stop the crawl using the main toolbar.",
                ),
                Some(s) if s.collection.sources.iter().any(|s| s.error.is_some()) => (
                    "Sitemap sources could not be read",
                    "Open Manage sitemaps to inspect source errors. Apply and recrawl retries them within a new audit.",
                ),
                Some(_) => (
                    "Add a sitemap to compare discovery",
                    "No usable sitemap sources were collected. Open Manage sitemaps to add a URL. The ordinary crawl remains available in Crawl Results.",
                ),
                None => (
                    "Collect sitemap and link evidence",
                    "Start a new crawl, or enable sitemap collection in Manage sitemaps. Existing projects without discovery data need a new crawl.",
                ),
            };
            self.empty.set_title(title);
            self.empty.set_description(Some(description));
            self.empty.set_icon_name(Some("folder-open-symbolic"));
        }
        let mut values = Vec::new();
        let name = if self.filter.get() == 0 {
            "All known URLs"
        } else if self.filter.get() <= Finding::ALL.len() {
            Finding::ALL[self.filter.get() - 1].label()
        } else {
            "No finding"
        };
        self.heading.set_text(name);
        for (index, count) in self.counts.iter().enumerate() {
            let total = data.as_ref().map_or(0, |s| {
                s.rows.iter().filter(|r| matches_filter(r, index)).count()
            });
            count.set_text(&total.to_string());
        }
        if available && let Some(snapshot) = data.as_ref() {
            for (index, row) in snapshot.rows.iter().enumerate() {
                if matches_filter(row, self.filter.get())
                    && format!(
                        "{} {} {}",
                        row.url,
                        row.finding_text(),
                        row.http.map_or(String::new(), |s| s.to_string())
                    )
                    .to_lowercase()
                    .contains(&query)
                {
                    values.push(index.to_string());
                }
            }
        }
        drop(data);
        let entries: Vec<_> = values.iter().map(String::as_str).collect();
        self.model.splice(0, self.model.n_items(), &entries);
        self.count.set_text(&format!("{} URLs", entries.len()));
        self.export.set_sensitive(!entries.is_empty());
        if let Some(index) =
            selected.and_then(|id| self.visible_indices().iter().position(|i| *i == id))
        {
            self.selection.set_selected(index as u32);
        } else if !entries.is_empty() {
            self.selection.set_selected(0);
        }
        self.show_selected();
    }

    fn show_selected(&self) {
        clear(&self.detail);
        let Some(id) = self
            .selection
            .selected_item()
            .and_downcast::<gtk::StringObject>()
            .and_then(|s| s.string().parse::<usize>().ok())
        else {
            self.detail.append(&text("No matching URLs", "heading"));
            self.detail.append(&paragraph(
                "Choose another finding or clear the search field.",
            ));
            return;
        };
        let data = self.data.borrow();
        let Some(snapshot) = data.as_ref() else {
            return;
        };
        let Some(row) = snapshot.rows.get(id) else {
            return;
        };
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let active_finding = self
            .filter
            .get()
            .checked_sub(1)
            .and_then(|index| Finding::ALL.get(index))
            .filter(|f| row.findings.contains(f))
            .copied()
            .or_else(|| {
                row.findings
                    .iter()
                    .find(|f| **f != Finding::Unreached)
                    .copied()
            })
            .or_else(|| row.findings.first().copied());
        let title = text(
            active_finding.map_or("No finding in these checks", Finding::label),
            "heading",
        );
        title.set_wrap(true);
        title.set_hexpand(true);
        header.append(&title);
        if row.findings.contains(&Finding::Unreached) {
            let action = gtk::Button::with_label("Find linking pages");
            action.add_css_class("suggested-action");
            action.set_valign(gtk::Align::Start);
            action.set_sensitive(
                snapshot.coverage.finished
                    && row.http.is_some_and(|s| (200..300).contains(&s))
                    && !row.findings.iter().any(|f| {
                        matches!(
                            f,
                            Finding::Unchecked
                                | Finding::Noindex
                                | Finding::Redirect
                                | Finding::Canonical
                        )
                    }),
            );
            action.set_tooltip_text(Some("Analyze this fetched, technically indexable target using the current crawl's page content"));
            let on_link = self.on_link.clone();
            let url = row.url.clone();
            action.connect_clicked(move |_| on_link(url.clone()));
            header.append(&action);
        }
        self.detail.append(&header);
        self.detail.append(&paragraph(&row.url));
        let reachability = if !snapshot.coverage.reachability_available() {
            "Reachability unavailable in this mode".into()
        } else {
            row.reached_depth
                .map_or("No path observed from the crawl start".into(), |d| {
                    format!("Reached from the crawl start at depth {d}")
                })
        };
        let mut fields = vec![
            (
                "Link evidence",
                format!("{reachability} · {} observed inlinks", row.inlinks),
            ),
            (
                "Sitemaps",
                if row.sitemaps.is_empty() {
                    "Not present in imported sources".into()
                } else {
                    row.sitemaps.join("\n")
                },
            ),
            (
                "HTTP / observation",
                format!(
                    "{} · {}",
                    row.http.map_or("Not fetched".into(), |s| s.to_string()),
                    row.observation
                ),
            ),
        ];
        if let Some(canonical) = &row.canonical {
            fields.push(("Canonical", canonical.clone()));
        }
        if row.response.redirected {
            fields.push((
                "Destination / final URL",
                row.response
                    .final_url
                    .clone()
                    .unwrap_or_else(|| "Not observed".into()),
            ));
        }
        let grid = gtk::Grid::builder()
            .column_spacing(16)
            .row_spacing(6)
            .build();
        for (index, (name, value)) in fields.into_iter().enumerate() {
            let name = text(name, "dim-label");
            name.set_valign(gtk::Align::Start);
            grid.attach(&name, 0, index as i32, 1, 1);
            let value = paragraph(&value);
            value.set_hexpand(true);
            grid.attach(&value, 1, index as i32, 1, 1);
        }
        self.detail.append(&grid);
        self.detail.append(&paragraph(active_finding.map_or(
            "These checks found no issue for this URL. This does not establish Google indexing.",
            Finding::guidance,
        )));
        let others: Vec<_> = row
            .findings
            .iter()
            .filter(|f| Some(**f) != active_finding)
            .map(|f| f.label())
            .collect();
        if !others.is_empty() {
            self.detail
                .append(&paragraph(&format!("Also flagged: {}", others.join("; "))));
        }
    }
}

fn matches_filter(row: &DiscoveryRow, index: usize) -> bool {
    if index == 0 {
        true
    } else if index <= Finding::ALL.len() {
        row.findings.contains(&Finding::ALL[index - 1])
    } else {
        row.findings.is_empty()
    }
}
fn column(name: &str, width: i32, expand: bool, kind: usize, data: Data) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().unwrap();
        let label = text("", "");
        label.set_margin_start(10);
        label.set_margin_end(10);
        label.set_margin_top(8);
        label.set_margin_bottom(8);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        item.set_child(Some(&label));
    });
    factory.connect_bind({
        let data = data.clone();
        move |_, object| {
            let item = object.downcast_ref::<gtk::ListItem>().unwrap();
            let Some(id) = item
                .item()
                .and_downcast::<gtk::StringObject>()
                .and_then(|s| s.string().parse::<usize>().ok())
            else {
                return;
            };
            let data = data.borrow();
            let Some(row) = data.as_ref().and_then(|s| s.rows.get(id)) else {
                return;
            };
            let value = match kind {
                0 => row.url.clone(),
                1 => row.http.map_or("—".into(), |s| s.to_string()),
                2 => row.inlinks.to_string(),
                _ => row.finding_text(),
            };
            let label = item.child().and_downcast::<gtk::Label>().unwrap();
            label.set_text(&value);
            label.set_tooltip_text(Some(&value));
        }
    });
    let column = gtk::ColumnViewColumn::new(Some(name), Some(factory));
    column.set_fixed_width(width);
    column.set_expand(expand);
    column.set_resizable(true);
    let sorter = gtk::CustomSorter::new(move |a, b| {
        let data = data.borrow();
        let get = |item: &glib::Object| {
            item.downcast_ref::<gtk::StringObject>()
                .and_then(|s| s.string().parse::<usize>().ok())
                .and_then(|id| data.as_ref()?.rows.get(id))
        };
        let ordering = match (get(a), get(b)) {
            (Some(a), Some(b)) => match kind {
                0 => a.url.cmp(&b.url),
                1 => a.http.cmp(&b.http),
                2 => a.inlinks.cmp(&b.inlinks),
                _ => a.finding_text().cmp(&b.finding_text()),
            },
            _ => std::cmp::Ordering::Equal,
        };
        match ordering {
            std::cmp::Ordering::Less => gtk::Ordering::Smaller,
            std::cmp::Ordering::Equal => gtk::Ordering::Equal,
            std::cmp::Ordering::Greater => gtk::Ordering::Larger,
        }
    });
    column.set_sorter(Some(&sorter));
    column
}

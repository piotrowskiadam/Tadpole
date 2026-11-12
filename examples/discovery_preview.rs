//! Clickable, offline design concept. All records and audit states are fictional.
//! Run with `cargo run --locked --example discovery_preview`.
use adw::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Finding {
    Unreached,
    Error,
    Redirect,
    Noindex,
    Canonical,
    Absent,
    Ready,
}

impl Finding {
    fn label(self) -> &'static str {
        match self {
            Self::Unreached => "Not reached through links",
            Self::Error => "HTTP error in sitemap",
            Self::Redirect => "Redirect in sitemap",
            Self::Noindex => "Noindex in sitemap",
            Self::Canonical => "Different canonical",
            Self::Absent => "Not in imported sitemaps",
            Self::Ready => "No finding",
        }
    }

    fn explanation(self) -> &'static str {
        match self {
            Self::Unreached => {
                "This URL was found in a sitemap, but no linking path from the homepage was observed. It is a candidate orphan within this crawl's scope."
            }
            Self::Error => {
                "This sitemap URL returned an HTTP error. Restore the intended page, or remove a URL that should no longer be submitted."
            }
            Self::Redirect => {
                "The sitemap URL redirects. Review whether its final destination should replace the old URL in this sitemap."
            }
            Self::Noindex => {
                "This sitemap URL declares noindex. If it should appear in search, review that directive; otherwise remove it from the sitemap."
            }
            Self::Canonical => {
                "This sitemap URL declares another canonical URL. Verify the intended canonical before changing the sitemap or page."
            }
            Self::Absent => {
                "This reachable, technically indexable URL is absent from the imported sitemaps. Review whether it belongs there; absence alone is not an error."
            }
            Self::Ready => {
                "This sitemap URL is reachable through internal links and has no finding in the checks shown here. This does not establish Google indexing."
            }
        }
    }
}

struct Page {
    path: &'static str,
    finding: Finding,
    http: u16,
    inlinks: usize,
    sitemap: &'static str,
    canonical: &'static str,
}

fn pages() -> Vec<Page> {
    use Finding::*;
    let records = [
        (
            "/products/trail-jacket",
            Unreached,
            200,
            0,
            "products.xml",
            "/products/trail-jacket",
        ),
        (
            "/guides/winter-layering",
            Unreached,
            200,
            2,
            "guides.xml",
            "/guides/winter-layering",
        ),
        (
            "/guides/cold-weather-kit",
            Unreached,
            200,
            1,
            "guides.xml",
            "/guides/cold-weather-kit",
        ),
        (
            "/offers/weekend-outdoors",
            Unreached,
            200,
            0,
            "pages.xml",
            "/offers/weekend-outdoors",
        ),
        ("/products/old-daypack", Error, 404, 3, "products.xml", ""),
        ("/guides/packing-checklist", Error, 503, 1, "guides.xml", ""),
        (
            "/products/rain-shell-v1",
            Redirect,
            301,
            4,
            "products.xml",
            "/products/rain-shell",
        ),
        (
            "/guides/hiking-basics",
            Redirect,
            302,
            2,
            "guides.xml",
            "/guides/first-hike",
        ),
        (
            "/sale/preview",
            Noindex,
            200,
            1,
            "pages.xml",
            "/sale/preview",
        ),
        (
            "/products/test-fleece",
            Noindex,
            200,
            2,
            "products.xml",
            "/products/test-fleece",
        ),
        (
            "/products/trail-boot?colour=green",
            Canonical,
            200,
            5,
            "products.xml",
            "/products/trail-boot",
        ),
        (
            "/guides/choose-a-daypack",
            Absent,
            200,
            8,
            "",
            "/guides/choose-a-daypack",
        ),
        (
            "/products/summit-fleece",
            Absent,
            200,
            12,
            "",
            "/products/summit-fleece",
        ),
        (
            "/collections/weekend-walks",
            Absent,
            200,
            7,
            "",
            "/collections/weekend-walks",
        ),
        ("/", Ready, 200, 19, "pages.xml", "/"),
        ("/products", Ready, 200, 14, "pages.xml", "/products"),
        ("/guides", Ready, 200, 11, "pages.xml", "/guides"),
        (
            "/products/rain-shell",
            Ready,
            200,
            9,
            "products.xml",
            "/products/rain-shell",
        ),
        (
            "/products/trail-boot",
            Ready,
            200,
            13,
            "products.xml",
            "/products/trail-boot",
        ),
        (
            "/guides/first-hike",
            Ready,
            200,
            6,
            "guides.xml",
            "/guides/first-hike",
        ),
    ];
    records
        .into_iter()
        .map(|(path, finding, http, inlinks, sitemap, canonical)| Page {
            path,
            finding,
            http,
            inlinks,
            sitemap,
            canonical,
        })
        .collect()
}

const ORIGIN: &str = "https://northstar.example";

fn label(text: &str, css: &str) -> gtk::Label {
    let item = gtk::Label::new(Some(text));
    item.set_xalign(0.0);
    if !css.is_empty() {
        item.add_css_class(css);
    }
    item
}

fn paragraph(text: &str) -> gtk::Label {
    let item = label(text, "");
    item.set_wrap(true);
    item.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    item.set_selectable(true);
    item
}

fn padded(orientation: gtk::Orientation, spacing: i32, margin: i32) -> gtk::Box {
    let item = gtk::Box::new(orientation, spacing);
    item.set_margin_start(margin);
    item.set_margin_end(margin);
    item.set_margin_top(margin);
    item.set_margin_bottom(margin);
    item
}

fn clear(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

struct Preview {
    pages: Vec<Page>,
    filter: Cell<usize>,
    scenario: Cell<u32>,
    model: gtk::StringList,
    selection: gtk::SingleSelection,
    search: gtk::SearchEntry,
    detail: gtk::Box,
    heading: gtk::Label,
    count: gtk::Label,
    coverage: gtk::Label,
    report: gtk::Box,
    empty: adw::StatusPage,
    export: gtk::Button,
    status: gtk::Label,
    notebook: gtk::Notebook,
    link_content: gtk::Box,
}

impl Preview {
    fn matching(&self) -> Vec<usize> {
        let query = self.search.text().to_lowercase();
        self.pages
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let included = match self.filter.get() {
                    0 => true,
                    1 => p.finding == Finding::Unreached,
                    2 => p.finding == Finding::Error,
                    3 => p.finding == Finding::Redirect,
                    4 => p.finding == Finding::Noindex,
                    5 => p.finding == Finding::Canonical,
                    6 => p.finding == Finding::Absent,
                    _ => p.finding == Finding::Ready,
                };
                included
                    && format!("{ORIGIN}{} {} {}", p.path, p.finding.label(), p.http)
                        .to_lowercase()
                        .contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn refresh(&self) {
        let titles = [
            "All known URLs",
            "Not reached through links",
            "HTTP errors in sitemap",
            "Redirects in sitemap",
            "Noindex in sitemap",
            "Different canonical",
            "Not in imported sitemaps",
            "No finding",
        ];
        self.heading.set_text(titles[self.filter.get()]);
        let ready = self.scenario.get() < 2;
        self.report.set_visible(ready);
        self.empty.set_visible(!ready);
        if self.scenario.get() == 2 {
            self.empty.set_title("Add a sitemap to compare discovery");
            self.empty.set_description(Some("No sitemap declarations were found in this example. Use Manage sitemaps to supply a sitemap URL. The link crawl is still available in Crawl Results."));
            self.empty.set_icon_name(Some("folder-open-symbolic"));
        } else if self.scenario.get() == 3 {
            self.empty.set_title("The sitemap could not be read");
            self.empty.set_description(Some("sitemap_index.xml returned HTTP 503 in this example. Open Manage sitemaps to retry the sample source. No sitemap findings can be concluded."));
            self.empty.set_icon_name(Some("dialog-warning-symbolic"));
        }
        self.coverage.set_text(match self.scenario.get() {
            0 => "Link crawl complete · 16 URLs reached from the homepage · no scope limits hit",
            1 => "Depth limit reached · reachability is incomplete · candidate orphans require review",
            2 => "Sitemap comparison unavailable · no sitemap sources loaded",
            _ => "Sitemap comparison unavailable · source fetch failed (HTTP 503)",
        });
        if self.scenario.get() == 1 {
            self.coverage.add_css_class("warning");
        } else {
            self.coverage.remove_css_class("warning");
        }
        let ids: Vec<String> = if ready {
            self.matching().iter().map(usize::to_string).collect()
        } else {
            vec![]
        };
        let values: Vec<&str> = ids.iter().map(String::as_str).collect();
        self.model.splice(0, self.model.n_items(), &values);
        self.count.set_text(&format!("{} URLs", ids.len()));
        self.export.set_sensitive(!ids.is_empty());
        if ids.is_empty() {
            clear(&self.detail);
            self.detail.append(&label("No matching URLs", "heading"));
            self.detail
                .append(&paragraph("Try another finding or clear the search field."));
        } else {
            self.selection.set_selected(0);
            self.show_selected();
        }
    }

    fn show_selected(&self) {
        let Some(item) = self
            .selection
            .selected_item()
            .and_downcast::<gtk::StringObject>()
        else {
            return;
        };
        let Ok(id) = item.string().parse::<usize>() else {
            return;
        };
        let p = &self.pages[id];
        clear(&self.detail);
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let title = label(p.finding.label(), "heading");
        title.set_hexpand(true);
        title.set_wrap(true);
        top.append(&title);
        let review = label(
            if p.finding == Finding::Absent {
                "Review opportunity"
            } else if p.finding == Finding::Unreached {
                "Candidate orphan"
            } else {
                "Observed evidence"
            },
            "dim-label",
        );
        top.append(&review);
        self.detail.append(&top);
        self.detail
            .append(&paragraph(&format!("{ORIGIN}{}", p.path)));
        self.detail.append(&paragraph(p.finding.explanation()));

        let evidence = gtk::Grid::builder()
            .column_spacing(20)
            .row_spacing(5)
            .build();
        let reach = if p.finding == Finding::Unreached {
            "No path observed from homepage"
        } else {
            "Reached from homepage"
        };
        let membership = if p.sitemap.is_empty() {
            "Not present in imported sources".into()
        } else {
            format!("{ORIGIN}/{}", p.sitemap)
        };
        let canonical = if p.canonical.is_empty() {
            "Not evaluated: HTTP error".into()
        } else if p.finding == Finding::Redirect {
            format!("Redirect destination: {ORIGIN}{}", p.canonical)
        } else {
            format!("{ORIGIN}{}", p.canonical)
        };
        for (row, (name, value)) in [
            ("Sitemap", membership),
            (
                "Link evidence",
                format!("{reach} · {} observed inlinks", p.inlinks),
            ),
            (
                "HTTP / directive",
                format!(
                    "{} · {}",
                    p.http,
                    if p.finding == Finding::Noindex {
                        "noindex"
                    } else if p.http != 200 {
                        "see response status"
                    } else {
                        "no noindex observed"
                    }
                ),
            ),
            (
                if p.finding == Finding::Redirect {
                    "Destination"
                } else {
                    "Canonical"
                },
                canonical,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            evidence.attach(&label(name, "dim-label"), 0, row as i32, 1, 1);
            let value = paragraph(&value);
            value.set_hexpand(true);
            evidence.attach(&value, 1, row as i32, 1, 1);
        }
        self.detail.append(&evidence);
        if self.scenario.get() == 1 {
            self.detail.append(&paragraph(
                "The link crawl hit its depth limit. A path may exist outside the observed crawl.",
            ));
        }
        if p.finding == Finding::Unreached {
            let action = gtk::Button::with_label("Find linking pages");
            action.add_css_class("suggested-action");
            action.set_halign(gtk::Align::Start);
            let content = self.link_content.clone();
            let notebook = self.notebook.clone();
            let target = format!("{ORIGIN}{}", p.path);
            action.connect_clicked(move |_| {
                clear(&content);
                content.append(&label("Possible linking pages", "title-2"));
                content.append(&paragraph(&format!("Target: {target}")));
                content.append(&paragraph("Illustrative suggestions only. In production, run Tadpole's internal-link analysis for this target and review the proposed anchor in its source context."));
                for (path, context) in [
                    ("/guides/first-hike", "A beginner's checklist mentions weather protection and choosing an outer layer."),
                    ("/products/rain-shell", "Related equipment could link to complementary clothing or a buying guide."),
                    ("/guides", "A relevant guide index can provide a reachable, contextual link."),
                ] {
                    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
                    content.append(&paragraph(&format!("{ORIGIN}{path}")));
                    content.append(&paragraph(context));
                }
                notebook.set_current_page(Some(2));
            });
            top.append(&action);
        }
    }
}

fn column(
    title: &str,
    width: i32,
    expand: bool,
    select: impl Fn(&Page) -> String + 'static,
    data: Rc<Vec<Page>>,
) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().unwrap();
        let text = label("", "");
        text.set_margin_start(10);
        text.set_margin_end(10);
        text.set_margin_top(9);
        text.set_margin_bottom(9);
        text.set_ellipsize(gtk::pango::EllipsizeMode::End);
        item.set_child(Some(&text));
    });
    factory.connect_bind(move |_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().unwrap();
        let Some(value) = item.item().and_downcast::<gtk::StringObject>() else {
            return;
        };
        let Ok(id) = value.string().parse::<usize>() else {
            return;
        };
        let text = item.child().and_downcast::<gtk::Label>().unwrap();
        let value = select(&data[id]);
        text.set_text(&value);
        text.set_tooltip_text(Some(&value));
    });
    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column.set_fixed_width(width);
    column.set_expand(expand);
    column.set_resizable(true);
    column
}

fn main() {
    let app = adw::Application::builder()
        .application_id("com.tadpole.discovery.preview")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build);
    app.run();
}

fn build(app: &adw::Application) {
    tadpole::ui::theme::install();
    let contrast = gtk::CssProvider::new();
    contrast.load_from_string(
        ".discovery-preview .dim-label { color: @window_fg_color; opacity: 0.85; }
         .discovery-preview columnview header,
         .discovery-preview columnview header button,
         .discovery-preview columnview header label { color: @view_fg_color; opacity: 1; }",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &contrast,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
    }
    if let Ok(theme) = std::env::var("TADPOLE_PREVIEW_THEME") {
        adw::StyleManager::default().set_color_scheme(if theme == "dark" {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });
    }
    let width = std::env::var("TADPOLE_PREVIEW_WIDTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1200);
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Tadpole — Sitemap & Discovery · fictional data")
        .default_width(width)
        .default_height(860)
        .width_request(800)
        .build();
    window.add_css_class("discovery-preview");
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    window.set_content(Some(&root));
    let header = adw::HeaderBar::new();
    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    if let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from_static(include_bytes!(
        "../tadpolelogonobg.png"
    ))) {
        let mascot = gtk::Image::from_paintable(Some(&texture));
        mascot.set_pixel_size(28);
        brand.append(&mascot);
    }
    brand.append(&label("Tadpole", "heading"));
    header.pack_start(&brand);
    header.set_title_widget(Some(&label("northstar.example", "heading")));
    header.pack_end(&label("Fictional data · design preview", "dim-label"));
    root.append(&header);

    let toolbar = padded(gtk::Orientation::Horizontal, 8, 10);
    let address = gtk::Entry::builder()
        .text(ORIGIN)
        .editable(false)
        .hexpand(true)
        .build();
    address.update_property(&[gtk::accessible::Property::Label("Sample website URL")]);
    toolbar.append(&address);
    let run = gtk::Button::with_label("Run sample audit");
    toolbar.append(&run);
    root.append(&toolbar);

    let notebook = gtk::Notebook::new();
    notebook.set_vexpand(true);
    root.append(&notebook);
    let crawl = padded(gtk::Orientation::Vertical, 12, 20);
    crawl.append(&label("Crawl Results", "title-2"));
    crawl.append(&paragraph("The link crawl reached 16 URLs from the homepage. Sitemap checks add 4 more known URLs. The production app's full crawl table remains here."));
    for path in [
        "/",
        "/products",
        "/guides",
        "/products/rain-shell",
        "/products/trail-boot",
        "/guides/first-hike",
    ] {
        crawl.append(&paragraph(&format!("200   {ORIGIN}{path}")));
    }
    let open_discovery = gtk::Button::with_label("Review sitemap discovery");
    open_discovery.set_halign(gtk::Align::Start);
    crawl.append(&open_discovery);
    notebook.append_page(&crawl, Some(&label("Crawl Results", "")));

    let discovery = gtk::Box::new(gtk::Orientation::Vertical, 0);
    notebook.append_page(&discovery, Some(&label("Sitemap & Discovery", "")));
    let link_content = padded(gtk::Orientation::Vertical, 12, 24);
    link_content.append(&label("Find a useful place to add a link", "title-2"));
    link_content.append(&paragraph("Select a candidate orphan in Sitemap & Discovery, then choose Find linking pages. This preview will show an illustrative handoff to Tadpole's existing link-analysis workspace."));
    notebook.append_page(&link_content, Some(&label("Link Opportunities", "")));

    let intro = padded(gtk::Orientation::Vertical, 7, 16);
    let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let title = label("Sitemap & Discovery", "title-2");
    title.set_hexpand(true);
    title_row.append(&title);
    let manage = gtk::ToggleButton::with_label("Manage sitemaps");
    title_row.append(&manage);
    let export = gtk::Button::with_label("Export CSV");
    title_row.append(&export);
    intro.append(&title_row);
    intro.append(&paragraph(
        "Compare sitemap declarations with pages reachable through internal links.",
    ));
    let coverage = label("", "");
    coverage.set_wrap(true);
    intro.append(&coverage);
    discovery.append(&intro);

    let sources_revealer = gtk::Revealer::new();
    sources_revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
    let sources = padded(gtk::Orientation::Vertical, 8, 16);
    sources.set_margin_top(0);
    sources.append(&label("Sitemap sources", "heading"));
    let sources_summary = paragraph(
        "From robots.txt: sitemap_index.xml → products.xml, guides.xml, pages.xml · 17 unique URLs",
    );
    sources.append(&sources_summary);
    let add_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let source_url = gtk::Entry::builder()
        .placeholder_text("https://northstar.example/sitemap.xml")
        .hexpand(true)
        .build();
    source_url.update_property(&[gtk::accessible::Property::Label("Sitemap URL")]);
    let add_source = gtk::Button::with_label("Add sample source");
    let retry = gtk::Button::with_label("Retry sample source");
    add_row.append(&source_url);
    add_row.append(&add_source);
    add_row.append(&retry);
    sources.append(&add_row);
    let source_feedback =
        paragraph("Prototype only: source entries are validated locally. No requests are sent.");
    sources.append(&source_feedback);
    sources_revealer.set_child(Some(&sources));
    discovery.append(&sources_revealer);
    discovery.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let report = gtk::Box::new(gtk::Orientation::Vertical, 0);
    report.set_vexpand(true);
    discovery.append(&report);
    let totals = label(
        "17 in sitemaps     16 reached through links     20 known URLs",
        "dim-label",
    );
    totals.set_margin_start(16);
    totals.set_margin_top(10);
    totals.set_margin_bottom(10);
    report.append(&totals);
    let split = gtk::Paned::new(gtk::Orientation::Horizontal);
    split.set_position(225);
    split.set_resize_start_child(false);
    split.set_shrink_start_child(false);
    split.set_vexpand(true);
    report.append(&split);
    let filters = padded(gtk::Orientation::Vertical, 10, 12);
    filters.set_width_request(190);
    filters.append(&label("Discovery findings", "heading"));
    let navigation = gtk::ListBox::new();
    navigation.add_css_class("navigation-sidebar");
    filters.append(&navigation);
    for (name, count) in [
        ("All known URLs", 20),
        ("Not reached through links", 4),
        ("HTTP errors in sitemap", 2),
        ("Redirects in sitemap", 2),
        ("Noindex in sitemap", 2),
        ("Different canonical", 1),
        ("Not in sitemaps", 3),
        ("No finding", 6),
    ] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_top(8);
        row.set_margin_bottom(8);
        let name = label(name, "");
        name.set_wrap(true);
        name.set_hexpand(true);
        row.append(&name);
        row.append(&label(&count.to_string(), "numeric"));
        navigation.append(&row);
    }
    let caveat = paragraph(
        "Counts describe this sample crawl. One URL can have several findings in the working feature.",
    );
    caveat.add_css_class("dim-label");
    filters.append(&caveat);
    let filter_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&filters)
        .build();
    split.set_start_child(Some(&filter_scroll));

    let work = gtk::Paned::new(gtk::Orientation::Vertical);
    work.set_position(255);
    work.set_shrink_start_child(false);
    work.set_shrink_end_child(false);
    split.set_end_child(Some(&work));
    let table_area = padded(gtk::Orientation::Vertical, 8, 12);
    table_area.set_margin_top(4);
    let table_heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let heading = label("", "heading");
    heading.set_hexpand(true);
    let count = label("", "dim-label");
    table_heading.append(&heading);
    table_heading.append(&count);
    table_area.append(&table_heading);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search URL, finding or HTTP status"));
    search.update_property(&[gtk::accessible::Property::Label(
        "Search discovery findings",
    )]);
    table_area.append(&search);
    let model = gtk::StringList::new(&[]);
    let selection = gtk::SingleSelection::new(Some(model.clone()));
    let table = gtk::ColumnView::new(Some(selection.clone()));
    table.add_css_class("data-table");
    table.set_show_row_separators(true);
    let data = Rc::new(pages());
    table.append_column(&column(
        "URL",
        330,
        true,
        |p| format!("{ORIGIN}{}", p.path),
        data.clone(),
    ));
    table.append_column(&column(
        "HTTP",
        65,
        false,
        |p| p.http.to_string(),
        data.clone(),
    ));
    table.append_column(&column(
        "Inlinks",
        65,
        false,
        |p| p.inlinks.to_string(),
        data.clone(),
    ));
    table.append_column(&column(
        "Finding",
        190,
        false,
        |p| p.finding.label().into(),
        data,
    ));
    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&table)
        .build();
    table_area.append(&scrolled);
    work.set_start_child(Some(&table_area));
    let detail = padded(gtk::Orientation::Vertical, 10, 14);
    let detail_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(230)
        .child(&detail)
        .build();
    work.set_end_child(Some(&detail_scroll));
    let empty = adw::StatusPage::new();
    empty.set_vexpand(true);
    empty.set_visible(false);
    discovery.append(&empty);

    let footer = padded(gtk::Orientation::Horizontal, 8, 10);
    footer.add_css_class("status-bar");
    let status = label(
        "Fictional sample · local preview · no live crawl or Google index data",
        "dim-label",
    );
    status.set_wrap(true);
    status.set_hexpand(true);
    footer.append(&status);
    footer.append(&label("Preview state", "dim-label"));
    let scenario =
        gtk::DropDown::from_strings(&["Complete", "Depth limited", "No sitemap", "Source failed"]);
    scenario.update_property(&[gtk::accessible::Property::Label("Preview audit state")]);
    footer.append(&scenario);
    root.append(&footer);

    let preview = Rc::new(Preview {
        pages: pages(),
        filter: Cell::new(1),
        scenario: Cell::new(0),
        model,
        selection: selection.clone(),
        search: search.clone(),
        detail,
        heading,
        count,
        coverage,
        report,
        empty,
        export: export.clone(),
        status,
        notebook: notebook.clone(),
        link_content,
    });
    navigation.select_row(navigation.row_at_index(1).as_ref());
    navigation.connect_row_selected({
        let p = preview.clone();
        move |_, row| {
            if let Some(row) = row {
                p.filter.set(row.index() as usize);
                p.refresh();
            }
        }
    });
    search.connect_search_changed({
        let p = preview.clone();
        move |_| p.refresh()
    });
    selection.connect_selected_notify({
        let p = preview.clone();
        move |_| p.show_selected()
    });
    manage.connect_toggled(move |button| sources_revealer.set_reveal_child(button.is_active()));
    open_discovery.connect_clicked({
        let notebook = notebook.clone();
        move |_| notebook.set_current_page(Some(1))
    });
    let added_sources = Rc::new(RefCell::new(Vec::<String>::new()));
    add_source.connect_clicked({ let p = preview.clone(); let feedback = source_feedback.clone(); let added = added_sources.clone(); move |_| {
        match url::Url::parse(source_url.text().trim()) {
            Ok(url) if matches!(url.scheme(), "https" | "http") && url.host_str() == Some("northstar.example") && url.username().is_empty() && url.password().is_none() => {
                if added.borrow().iter().any(|s| s == url.as_str()) { feedback.set_text("This sample source is already listed. Enter a different URL."); return; }
                added.borrow_mut().push(url.to_string());
                feedback.set_text(&format!("Added sample source: {url} · awaiting fetch in the working feature. Current findings still use the original sample sources."));
                source_url.set_text("");
                p.status.set_text("Sample source added locally. No network request was made.");
            }
            _ => feedback.set_text("Enter an HTTP(S) sitemap URL on northstar.example without embedded credentials."),
        }
    }});
    scenario.connect_selected_notify({ let p = preview.clone(); let feedback = source_feedback.clone(); move |dropdown| {
        p.scenario.set(dropdown.selected());
        sources_summary.set_text(match dropdown.selected() {
            2 => "No sources found in robots.txt. Add a sitemap URL to begin comparison.",
            3 => "sitemap_index.xml · HTTP 503 · not parsed · retry available",
            _ => "From robots.txt: sitemap_index.xml → products.xml, guides.xml, pages.xml · 17 unique URLs",
        });
        feedback.set_text("Prototype only: source entries are validated locally. No requests are sent.");
        p.refresh();
    }});
    retry.connect_clicked({ let scenario = scenario.clone(); let feedback = source_feedback.clone(); move |_| { scenario.set_selected(0); feedback.set_text("Sample source loaded. 17 unique URLs across 3 child sitemaps. No request was sent."); } });
    run.connect_clicked({
        let p = preview.clone();
        let scenario = scenario.clone();
        move |button| {
            button.set_sensitive(false);
            button.set_label("Running sample…");
            p.export.set_sensitive(false);
            p.status.set_text(
                "Simulating sitemap parsing and link reconciliation. No network requests.",
            );
            let p = p.clone();
            let button = button.clone();
            let scenario = scenario.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(900), move || {
                scenario.set_selected(0);
                p.scenario.set(0);
                p.refresh();
                button.set_sensitive(true);
                button.set_label("Run sample audit");
                p.status
                    .set_text("Sample audit complete · 20 known URLs · no network requests made");
            });
        }
    });
    export.connect_clicked({
        let p = preview.clone();
        let parent = window.clone();
        move |_| {
            let dialog = gtk::FileDialog::builder()
                .title("Export sample findings")
                .initial_name("tadpole-discovery-sample.csv")
                .build();
            let rows = p.matching();
            let audit_coverage = if p.scenario.get() == 1 {
                "Depth limited"
            } else {
                "Complete sample"
            };
            let p = p.clone();
            dialog.save(Some(&parent), None::<&gio::Cancellable>, move |result| {
                let Ok(file) = result else { return };
                let Some(path) = file.path() else {
                    p.status
                        .set_text("Choose a local file path for this preview export.");
                    return;
                };
                let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                    let mut csv = csv::Writer::from_path(path)?;
                    csv.write_record([
                        "URL",
                        "Finding",
                        "HTTP",
                        "Observed inlinks",
                        "Sitemap",
                        "Reachability",
                        "Audit coverage",
                        "Data",
                    ])?;
                    for id in &rows {
                        let page = &p.pages[*id];
                        csv.write_record([
                            format!("{ORIGIN}{}", page.path),
                            page.finding.label().into(),
                            page.http.to_string(),
                            page.inlinks.to_string(),
                            if page.sitemap.is_empty() {
                                String::new()
                            } else {
                                format!("{ORIGIN}/{}", page.sitemap)
                            },
                            if page.finding == Finding::Unreached {
                                "No path observed"
                            } else {
                                "Reached from homepage"
                            }
                            .into(),
                            audit_coverage.into(),
                            "Fictional sample".into(),
                        ])?;
                    }
                    csv.flush()?;
                    Ok(())
                })();
                p.status.set_text(&match result {
                    Ok(()) => format!(
                        "Exported {} sample findings. All records are labeled fictional.",
                        rows.len()
                    ),
                    Err(e) => format!("Export failed: {e}. Choose a writable location and retry."),
                });
            });
        }
    });
    notebook.set_current_page(Some(1));
    preview.refresh();
    window.present();
}

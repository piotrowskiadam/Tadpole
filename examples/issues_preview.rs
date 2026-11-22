//! Offline native GTK4/Libadwaita preview for the `Issues` workspace tab.
//! All crawl records and discovery evidence are fictional.
//!
//! Run with:
//!   `cargo run --locked --example issues_preview`
//!   `TADPOLE_PREVIEW_THEME=dark cargo run --locked --example issues_preview`
//!   `TADPOLE_PREVIEW_THEME=light cargo run --locked --example issues_preview`

use adw::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use tadpole::{
    discovery::{
        AuditCoverage, DiscoveryRow, DiscoverySnapshot, Finding, ResponseEvidence,
        SitemapCollection, SitemapSource,
    },
    issues,
    state::{CrawlConfig, CrawlResult, CrawlState, ImageInfo},
    ui::issues::IssuesPanel,
};

const ORIGIN: &str = "https://northstar.example";

fn base_page(path: &str) -> CrawlResult {
    let url = format!("{ORIGIN}{path}");
    let mut res = CrawlResult::new_failed(url.clone(), Some(200), None, "Indexable".into(), 1, 85);
    res.indexable = true;
    res.title = Some("Northstar Outdoor Gear — Technical Trail Equipment".to_string());
    res.meta_desc = Some(
        "Explore lightweight waterproof shells, insulated layers, and mountain packs tested on alpine trails."
            .to_string(),
    );
    res.h1 = Some(format!("Northstar {path}"));
    res.h1_count = 1;
    res.h2 = Some("Product highlights".to_string());
    res.h2_count = 1;
    res.word_count = 420;
    res.size_bytes = 18_400;
    res.canonical = Some(url);
    res.og_title = Some("Northstar Outdoor Gear".to_string());
    res.og_description = Some("Lightweight alpine shells and trail packs.".to_string());
    res
}

fn sample_results() -> Vec<CrawlResult> {
    let mut home = base_page("/");
    home.depth = 0;
    home.title = Some("Northstar Outdoor Gear — Alpine Shells & Trail Packs".to_string());
    home.h1 = Some("Alpine Shells & Trail Packs".to_string());

    let mut server_err = base_page("/checkout/shipping");
    server_err.status_code = Some(503);
    server_err.indexable = false;
    server_err.indexability_status = "Server Error".to_string();
    server_err.error_message = Some("Service Unavailable".to_string());

    let mut not_found = base_page("/products/old-daypack");
    not_found.status_code = Some(404);
    not_found.indexable = false;
    not_found.indexability_status = "Client Error".to_string();

    let mut redirect = base_page("/products/rain-shell-v1");
    redirect.status_code = Some(301);
    redirect.indexable = false;
    redirect.indexability_status = "Redirect".to_string();

    let mut missing_title = base_page("/guides/alpine-layering");
    missing_title.title = None;
    missing_title.h1 = None;
    missing_title.h1_count = 0;

    let mut dup_title_a = base_page("/products/trail-boot");
    dup_title_a.title = Some("Waterproof Hiking Boots — Northstar Outdoor".to_string());
    dup_title_a.schema_errors = vec!["Missing required \"offers\" field on Product".to_string()];

    let mut dup_title_b = base_page("/products/trail-boot-wide");
    dup_title_b.title = Some("Waterproof Hiking Boots — Northstar Outdoor".to_string());
    dup_title_b.canonical = Some(format!("{ORIGIN}/products/trail-boot"));

    let mut missing_alt = base_page("/guides/first-hike");
    missing_alt.meta_desc = Some("Short summary.".to_string());
    missing_alt.og_title = None;
    missing_alt.images = vec![
        ImageInfo {
            src: format!("{ORIGIN}/img/boots.jpg"),
            alt: None,
            local_path: None,
            width: None,
            height: None,
            title: None,
            loading: None,
        },
        ImageInfo {
            src: format!("{ORIGIN}/img/pack.jpg"),
            alt: Some("Daypack on trail".to_string()),
            local_path: None,
            width: None,
            height: None,
            title: None,
            loading: None,
        },
    ];

    let mut long_title = base_page("/collections/winter-mountaineering-expedition-gear");
    long_title.title = Some(
        "Winter Mountaineering Expedition Gear, Insulated Shells & Cold-Weather Packs — Northstar"
            .to_string(),
    );
    long_title.canonical = None;

    vec![
        home,
        server_err,
        not_found,
        redirect,
        missing_title,
        dup_title_a,
        dup_title_b,
        missing_alt,
        long_title,
    ]
}

fn sample_discovery(scenario: u32) -> Option<DiscoverySnapshot> {
    if scenario == 3 {
        return None;
    }

    let mut memberships: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in [
        "/",
        "/products/old-daypack",
        "/products/rain-shell-v1",
        "/sale/preview",
        "/products/trail-jacket",
    ] {
        let mut set = BTreeSet::new();
        set.insert(format!("{ORIGIN}/sitemap.xml"));
        memberships.insert(format!("{ORIGIN}{path}"), set);
    }

    let rows = vec![
        DiscoveryRow {
            url: format!("{ORIGIN}/products/old-daypack"),
            sitemaps: vec![format!("{ORIGIN}/sitemap.xml")],
            inlinks: 2,
            reached_depth: Some(1),
            http: Some(404),
            canonical: None,
            response: ResponseEvidence::default(),
            observation: "HTTP 404".to_string(),
            findings: vec![Finding::HttpError],
        },
        DiscoveryRow {
            url: format!("{ORIGIN}/sale/preview"),
            sitemaps: vec![format!("{ORIGIN}/sitemap.xml")],
            inlinks: 1,
            reached_depth: Some(2),
            http: Some(200),
            canonical: Some(format!("{ORIGIN}/sale/preview")),
            response: ResponseEvidence::default(),
            observation: "Noindex".to_string(),
            findings: vec![Finding::Noindex],
        },
        DiscoveryRow {
            url: format!("{ORIGIN}/products/rain-shell-v1"),
            sitemaps: vec![format!("{ORIGIN}/sitemap.xml")],
            inlinks: 3,
            reached_depth: Some(1),
            http: Some(301),
            canonical: None,
            response: ResponseEvidence {
                redirected: true,
                noindex: false,
                final_url: Some(format!("{ORIGIN}/products/rain-shell")),
            },
            observation: "Redirect".to_string(),
            findings: vec![Finding::Redirect],
        },
        DiscoveryRow {
            url: format!("{ORIGIN}/products/trail-jacket"),
            sitemaps: vec![format!("{ORIGIN}/sitemap.xml")],
            inlinks: 0,
            reached_depth: None,
            http: Some(200),
            canonical: Some(format!("{ORIGIN}/products/trail-jacket")),
            response: ResponseEvidence::default(),
            observation: "Indexable".to_string(),
            findings: vec![Finding::Unreached],
        },
        DiscoveryRow {
            url: format!("{ORIGIN}/guides/first-hike"),
            sitemaps: vec![],
            inlinks: 4,
            reached_depth: Some(1),
            http: Some(200),
            canonical: Some(format!("{ORIGIN}/guides/first-hike")),
            response: ResponseEvidence::default(),
            observation: "Indexable".to_string(),
            findings: vec![Finding::Absent],
        },
    ];

    let mut responses = BTreeMap::new();
    responses.insert(
        format!("{ORIGIN}/products/rain-shell-v1"),
        ResponseEvidence {
            redirected: true,
            noindex: false,
            final_url: Some(format!("{ORIGIN}/products/rain-shell")),
        },
    );

    let cfg = CrawlConfig {
        max_urls: if scenario == 2 { 9 } else { 500 },
        ..Default::default()
    };
    let mut coverage = AuditCoverage::new(vec![ORIGIN.to_string()], &cfg);
    coverage.finished = true;
    coverage.stopped = scenario == 1;
    coverage.url_limit_hit = scenario == 2;
    coverage.finished_at = Some(coverage.started_at + 12);

    Some(DiscoverySnapshot {
        coverage,
        collection: SitemapCollection {
            sources: vec![SitemapSource {
                url: format!("{ORIGIN}/sitemap.xml"),
                parent: None,
                status: "200 OK".to_string(),
                url_count: 5,
                error: None,
                is_index: Some(false),
            }],
            memberships,
            warnings: vec![],
            truncated: false,
        },
        responses,
        rows,
    })
}

fn build_report(scenario: u32) -> issues::IssueReport {
    let results = sample_results();
    let discovery = sample_discovery(scenario);
    let config = CrawlConfig {
        max_urls: if scenario == 2 { 9 } else { 500 },
        ..Default::default()
    };
    issues::evaluate(&results, discovery.as_ref(), Some(&config))
}

fn main() {
    let app = adw::Application::builder()
        .application_id("com.tadpole.issues.preview")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build);
    app.run();
}

fn build(app: &adw::Application) {
    tadpole::ui::theme::install();

    let style_manager = adw::StyleManager::default();
    if let Ok(theme) = std::env::var("TADPOLE_PREVIEW_THEME") {
        style_manager.set_color_scheme(if theme == "dark" {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });
    }

    let width = std::env::var("TADPOLE_PREVIEW_WIDTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1220);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Tadpole — Issues Workspace · fictional data")
        .default_width(width)
        .default_height(860)
        .width_request(820)
        .build();

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
    let title = gtk::Label::new(Some("Tadpole"));
    title.add_css_class("heading");
    brand.append(&title);
    header.pack_start(&brand);

    let center_title = gtk::Label::new(Some("northstar.example"));
    center_title.add_css_class("heading");
    header.set_title_widget(Some(&center_title));

    let dark_toggle = gtk::ToggleButton::with_label("Dark Theme");
    dark_toggle.set_active(style_manager.is_dark());
    {
        let sm = style_manager.clone();
        dark_toggle.connect_toggled(move |btn| {
            sm.set_color_scheme(if btn.is_active() {
                adw::ColorScheme::ForceDark
            } else {
                adw::ColorScheme::ForceLight
            });
        });
    }
    header.pack_end(&dark_toggle);
    root.append(&header);

    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    status_bar.add_css_class("status-bar");
    status_bar.set_margin_start(14);
    status_bar.set_margin_end(14);
    status_bar.set_margin_top(8);
    status_bar.set_margin_bottom(8);

    let status_label = gtk::Label::new(Some(
        "Fictional sample · offline preview · activate any Affected URL to test Jump navigation",
    ));
    status_label.set_xalign(0.0);
    status_label.set_hexpand(true);
    status_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status_label.add_css_class("dim-label");
    status_bar.append(&status_label);

    let state_label = gtk::Label::new(Some("Preview state"));
    state_label.add_css_class("dim-label");
    status_bar.append(&state_label);

    let scenario_dropdown = gtk::DropDown::from_strings(&[
        "Complete",
        "Stopped early",
        "URL limit reached",
        "No sitemap discovery",
    ]);
    scenario_dropdown
        .update_property(&[gtk::accessible::Property::Label("Preview crawl state")]);
    status_bar.append(&scenario_dropdown);

    let state = CrawlState::new();
    let status_for_jump = status_label.clone();
    let issues_panel = IssuesPanel::new(state, move |url, kind, source| {
        status_for_jump.set_text(&format!(
            "Jump triggered → {} ({}) for {url}",
            source.label(),
            kind.name()
        ));
    });
    issues_panel.apply_report(build_report(0));

    let panel_for_scenario = issues_panel.clone();
    scenario_dropdown.connect_selected_notify(move |dropdown| {
        panel_for_scenario.apply_report(build_report(dropdown.selected()));
    });

    let notebook = gtk::Notebook::new();
    notebook.set_vexpand(true);
    notebook.append_page(
        issues_panel.widget(),
        Some(&gtk::Label::new(Some("Issues"))),
    );
    root.append(&notebook);
    root.append(&status_bar);

    window.present();
}

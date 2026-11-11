//! Offline fixture for visually inspecting the real detail tabs.
//! Use the native desktop or a connected GTK Broadway client (see docs/DEVELOPMENT.md).
use adw::prelude::*;
use tadpole::state::{CrawlConfig, CrawlResult, CrawlState};

fn fixtures() -> (CrawlState, String, tempfile::TempDir) {
    let assets = tempfile::tempdir().unwrap();
    let image_path = assets.path().join("preview.png");
    std::fs::write(&image_path, include_bytes!("../tadpolelogonobg.png")).unwrap();
    let url = format!(
        "https://demo.example/guides/{}",
        "long-url-segment".repeat(12)
    );
    let html = format!(
        r##"
        <title>Planning a useful website with clear navigation and accessible content</title>
        <meta property="og:title" content="Planning a useful website with clear navigation and accessible content">
        <meta property="og:description" content="Explore a practical guide to creating useful content, improving internal links and making a website easier to navigate.">
        <meta property="og:image" content="/share.png">
        <meta property="og:image:width" content="1200"><meta property="og:image:height" content="630">
        <meta property="og:image:alt" content="Tadpole mascot on a transparent background">
        <meta property="og:site_name" content="Fieldnotes"><meta property="og:type" content="article">
        <meta name="twitter:card" content="summary_large_image">
        <a href="{url}#overview">Read our full guide to planning a useful website and improving the structure of a growing site</a>
        <a href="{url}#details" rel="nofollow sponsored"><img src="/share.png" alt="Tadpole mascot" width="1200" height="630" loading="lazy"></a>
        <a href="https://other.example/{}">External resource</a>
        <img src="/empty.png" alt=""><img src="/missing.png"><img src="data:invalid" alt="Unavailable image">
        <h1>A long heading that should wrap naturally inside the page detail panel without truncation or horizontal scrolling</h1>
        <h2>Detailed recommendations for navigation, metadata, content, accessibility, and internal linking across your website</h2>
        <script type="application/ld+json">{{"@context":"https://schema.org","@type":"Article","headline":"An example article with a deliberately long value to exercise word wrapping in the raw JSON-LD pane","url":"{url}"}}</script>
    "##,
        "external-segment".repeat(15)
    );
    let parsed = tadpole::crawler::parse_html(&url, &html, "demo.example", &CrawlConfig::default());
    let mut page =
        CrawlResult::new_failed(url.clone(), Some(200), None, "Indexable".into(), 0, 120);
    page.title = parsed.title;
    page.h1 = parsed.h1;
    page.h1_count = parsed.h1_count;
    page.h2 = parsed.h2;
    page.h2_count = parsed.h2_count;
    page.outlinks = parsed.outlinks;
    page.link_details = parsed.link_details;
    page.images = parsed.images;
    for image in &mut page.images {
        if image.src.starts_with("https:") {
            image.local_path = Some(image_path.to_string_lossy().into());
        }
    }
    page.og_title = parsed.og_title;
    page.og_description = parsed.og_description;
    page.og_image = parsed.og_image;
    page.og_type = parsed.og_type;
    page.twitter_card = parsed.twitter_card;
    page.social_metadata = parsed.social_metadata;
    page.headings = parsed.headings;
    page.schema_json_ld = parsed.schema_json_ld;
    page.schema_errors = parsed.schema_errors;
    page.markdown = Some(format!(
        "# A useful website\n\nA long URL: {url}\n\n{}",
        "LongUnbrokenMarkdownText".repeat(30)
    ));
    let mut source = page.clone();
    source.url = "https://demo.example/source".into();
    page.inlinks = vec![source.url.clone()];
    let state = CrawlState::new();
    state.insert_result(source);
    state.insert_result(page);
    (state, url, assets)
}

fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();
    let app = adw::Application::builder()
        .application_id("com.tadpole.details.preview")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(|app| {
        tadpole::ui::theme::install();
        let light = std::env::var_os("TADPOLE_PREVIEW_LIGHT").is_some();
        adw::StyleManager::default().set_color_scheme(if light {
            adw::ColorScheme::ForceLight
        } else {
            adw::ColorScheme::ForceDark
        });
        let details = tadpole::ui::details::Details::new();
        let (state, url, assets) = fixtures();
        details.update(&url, &state);
        let width = std::env::var("TADPOLE_PREVIEW_WIDTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024);
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Tadpole — Detail preview (fictional data)")
            .default_width(width)
            .default_height(620)
            .content(details.widget())
            .build();
        window.present();
        // Keep the fixture's local image files alive while the preview is open.
        window.connect_close_request(move |_| {
            let _assets = &assets;
            glib::Propagation::Proceed
        });
    });
    app.run();
}

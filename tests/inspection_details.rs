use tadpole::{
    crawler::parse_html,
    inspection,
    state::{CrawlConfig, CrawlResult, CrawlState, ImageInfo},
};

fn page(url: &str) -> CrawlResult {
    CrawlResult::new_failed(url.into(), Some(200), None, "Indexable".into(), 0, 0)
}

#[test]
fn link_occurrences_keep_anchors_fragments_rel_and_image_alt() {
    let parsed = parse_html(
        "https://example.com/source",
        r##"
        <a href="/target#first" rel="nofollow sponsored">Read <strong>this</strong> guide</a>
        <a href="/target#second"><img src="/logo.png" alt="Company logo"></a>
        <a href="/target"></a><a href="mailto:hello@example.com">Email</a>
    "##,
        "example.com",
        &CrawlConfig::default(),
    );
    assert_eq!(parsed.link_details.len(), 4);
    assert_eq!(parsed.link_details[0].anchor, "Read this guide");
    assert_eq!(parsed.link_details[0].rel, "nofollow sponsored");
    assert_eq!(
        parsed.link_details[0].target,
        "https://example.com/target#first"
    );
    assert_eq!(parsed.link_details[1].anchor, "Company logo");
    assert_eq!(parsed.link_details[2].anchor, "");
    assert_eq!(parsed.outlinks[0], "https://example.com/target");
    assert!(
        !parsed
            .local_queue
            .iter()
            .any(|url| url.starts_with("mailto:"))
    );
    let mut source = page("https://example.com/source");
    source.outlinks = parsed.outlinks;
    source.link_details = parsed.link_details;
    let mut target = page("https://example.com/target");
    target.inlinks.push(source.url.clone());
    let state = CrawlState::new();
    state.insert_result(source);
    let inbound = inspection::inlinks(&target, &state);
    assert_eq!(inbound.len(), 3);
    assert_eq!(inbound[1].target, "https://example.com/target#second");
    assert_eq!(inbound[1].anchor.as_deref(), Some("Company logo"));
}

#[test]
fn image_details_distinguish_declared_dimensions_and_alt_states() {
    let parsed = parse_html(
        "https://example.com/",
        r#"
        <img src="/photo.png" width="1200" height="630" title="Photo title" loading="lazy" alt="A useful description">
        <img src="/decoration.png" width="100%" height="0" alt="">
        <img src="/missing.png"><img src="/blank.png" alt="  ">
    "#,
        "example.com",
        &CrawlConfig::default(),
    );
    assert_eq!(parsed.images[0].width, Some(1200));
    assert_eq!(parsed.images[0].height, Some(630));
    assert_eq!(parsed.images[0].loading.as_deref(), Some("lazy"));
    assert_eq!(parsed.images[0].title.as_deref(), Some("Photo title"));
    assert_eq!(parsed.images[1].width, None);
    assert_eq!(parsed.images[1].height, None);
    assert_eq!(
        inspection::alt_status(parsed.images[1].alt.as_deref()),
        "Empty alt (decorative)"
    );
    assert_eq!(
        inspection::alt_status(parsed.images[2].alt.as_deref()),
        "Missing alt attribute"
    );
    assert_eq!(
        inspection::alt_status(parsed.images[3].alt.as_deref()),
        "Whitespace-only alt"
    );
}

#[test]
fn older_projects_open_without_fabricated_anchor_or_image_data() {
    let mut original = page("https://example.com/source");
    original.outlinks.push("https://example.com/target".into());
    let mut json = serde_json::to_value(&original).unwrap();
    json.as_object_mut().unwrap().remove("link_details");
    json.as_object_mut().unwrap().remove("social_metadata");
    json["images"] = serde_json::json!([{ "src": "/old.png", "alt": null, "local_path": null }]);
    let restored: CrawlResult = serde_json::from_value(json).unwrap();
    assert_eq!(inspection::outlinks(&restored)[0].anchor, None);
    assert_eq!(restored.images[0].width, None);
    let mut target = page("https://example.com/target");
    target.inlinks.push(restored.url.clone());
    let state = CrawlState::new();
    state.insert_result(restored);
    assert_eq!(inspection::inlinks(&target, &state)[0].anchor, None);
    target
        .inlinks
        .push("https://example.com/not-crawled".into());
    assert_eq!(inspection::inlinks(&target, &state).len(), 2);
    let image: ImageInfo =
        serde_json::from_str(r#"{"src":"a","alt":"","local_path":null}"#).unwrap();
    assert_eq!(image.alt.as_deref(), Some(""));
}

#[test]
fn metadata_and_occurrences_survive_project_round_trip() {
    let parsed = parse_html(
        "https://example.com/",
        r#"
        <meta property="og:image" content="/share.png">
        <meta property="og:image:width" content="1200">
        <meta property="og:image:height" content="630">
        <meta property="og:image:alt" content="A share image">
        <meta property="og:site_name" content="Example">
        <meta name="twitter:site" content="@example">
        <a href="/target">A</a><a href="/target">B</a>
    "#,
        "example.com",
        &CrawlConfig::default(),
    );
    let mut result = page("https://example.com/");
    result.link_details = parsed.link_details;
    result.social_metadata = parsed.social_metadata;
    result.images = parsed.images;
    let state = CrawlState::new();
    state.insert_result(result);
    let json = serde_json::to_string(&state.get_project("https://example.com/".into())).unwrap();
    let restored = CrawlState::new();
    restored.load_project(serde_json::from_str(&json).unwrap());
    let page = restored.get_result("https://example.com/").unwrap();
    assert_eq!(page.link_details.len(), 2);
    assert_eq!(page.social_metadata["og:image:width"], "1200");
    assert_eq!(page.social_metadata["twitter:site"], "@example");
    assert_eq!(
        inspection::preview_url(&page.url, "/share.png"),
        Some("https://example.com/share.png".into())
    );
    assert!(inspection::preview_url(&page.url, "javascript:alert(1)").is_none());
    assert!(inspection::preview_url(&page.url, "file:///etc/passwd").is_none());
    assert!(inspection::preview_url(&page.url, "  ").is_none());
}

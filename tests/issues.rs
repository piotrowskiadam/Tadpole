use tadpole::{
    crawler::{CrawlUpdate, Crawler},
    discovery::{DiscoverySnapshot, ResponseEvidence, SitemapSource, UNKNOWN_DEPTH, reconcile},
    filters::{CrawlFilter, DuplicateIndex},
    issues::{
        IssueKind, IssueSource, MARKDOWN_SAMPLE_URL_LIMIT, ScopeCaveatKind, Severity, evaluate,
        evaluate_project, evaluate_state, export_csv, export_issue_csv, export_issue_markdown,
        export_markdown,
    },
    state::{CrawlConfig, CrawlProject, CrawlResult, CrawlState, CrawlStats, ImageInfo},
};

fn indexable_page(url: &str) -> CrawlResult {
    let mut res = CrawlResult::new_failed(url.into(), Some(200), None, "Indexable".into(), 0, 10);
    res.indexable = true;
    res.title = Some(format!("Well-Formed Title For {url}"));
    res.meta_desc = Some(format!(
        "A well-formed meta description for {url} that sits comfortably between seventy and one hundred fifty-five characters."
    ));
    res.h1 = Some(format!("Primary Heading for {url}"));
    res.h1_count = 1;
    res.h2 = Some("Secondary Heading".into());
    res.h2_count = 1;
    res.canonical = Some(url.into());
    res.og_title = Some("OG Title".into());
    res.og_description = Some("OG Description".into());
    res
}

#[test]
fn catalog_spans_all_four_severities_with_metadata_counts_percentages_and_zero_count_issues() {
    let healthy = indexable_page("https://example.com/healthy");

    let mut broken_server = CrawlResult::new_failed(
        "https://example.com/server-error".into(),
        Some(500),
        Some("500 Internal Server Error".into()),
        "Non-200 Status Code".into(),
        1,
        15,
    );
    broken_server.canonical = Some("https://example.com/server-error".into());

    let mut missing_title_and_long_desc = indexable_page("https://example.com/issues");
    missing_title_and_long_desc.title = None;
    missing_title_and_long_desc.meta_desc = Some("x".repeat(170));
    missing_title_and_long_desc.images = vec![ImageInfo {
        src: "https://example.com/img.png".into(),
        alt: None,
        local_path: None,
        width: None,
        height: None,
        title: None,
        loading: None,
    }];

    let mut dup_a = indexable_page("https://example.com/dup-a");
    dup_a.title = Some("Shared Title Across Two Indexable Pages!".into());
    let mut dup_b = indexable_page("https://example.com/dup-b");
    dup_b.title = Some("Shared Title Across Two Indexable Pages!".into());

    let results = [
        healthy,
        broken_server,
        missing_title_and_long_desc,
        dup_a,
        dup_b,
    ];
    let report = evaluate(&results, None, None);

    // Report exposes all 29 catalogued Issues (22 crawl + 7 discovery), ordered Critical -> Low.
    assert_eq!(report.issues.len(), IssueKind::ALL.len());
    assert_eq!(IssueKind::ALL.len(), 29);
    assert!(
        report
            .issues
            .windows(2)
            .all(|pair| pair[0].severity <= pair[1].severity),
        "issues must be ordered by Severity tier (Critical -> High -> Medium -> Low)"
    );

    // Every Issue has non-empty name, rationale, and suggested action.
    for issue in &report.issues {
        assert!(!issue.name.is_empty(), "{:?} missing name", issue.kind);
        assert!(
            !issue.rationale.is_empty(),
            "{:?} missing rationale",
            issue.kind
        );
        assert!(
            !issue.suggested_action.is_empty(),
            "{:?} missing suggested_action",
            issue.kind
        );
        assert_eq!(issue.affected_count, issue.affected_urls.len());
    }

    // Critical: ServerError5xx (1 of 5 crawled = 20%)
    let server_err = report.issue(IssueKind::ServerError5xx).unwrap();
    assert_eq!(server_err.severity, Severity::Critical);
    assert_eq!(server_err.source, IssueSource::Crawl);
    assert_eq!(server_err.affected_count, 1);
    assert!((server_err.affected_percentage - 20.0).abs() < 1e-6);
    assert_eq!(
        server_err.affected_urls[0].url,
        "https://example.com/server-error"
    );
    assert!(server_err.affected_urls[0].evidence.contains("500"));

    // High: MissingTitle (1 of 5 = 20%) & DuplicateTitle (2 of 5 = 40%)
    let missing_title = report.issue(IssueKind::MissingTitle).unwrap();
    assert_eq!(missing_title.severity, Severity::High);
    assert_eq!(missing_title.affected_count, 1);
    assert!((missing_title.affected_percentage - 20.0).abs() < 1e-6);

    let dup_title = report.issue(IssueKind::DuplicateTitle).unwrap();
    assert_eq!(dup_title.severity, Severity::High);
    assert_eq!(dup_title.affected_count, 2);
    assert!((dup_title.affected_percentage - 40.0).abs() < 1e-6);
    assert!(dup_title.affected_urls[0].evidence.contains("2"));

    // Medium: ImagesMissingAlt (1 of 5 = 20%)
    let missing_alt = report.issue(IssueKind::ImagesMissingAlt).unwrap();
    assert_eq!(missing_alt.severity, Severity::Medium);
    assert_eq!(missing_alt.affected_count, 1);
    assert!(missing_alt.affected_urls[0].evidence.contains("1"));

    // Low: DescriptionTooLong (1 of 5 = 20%)
    let long_desc = report.issue(IssueKind::DescriptionTooLong).unwrap();
    assert_eq!(long_desc.severity, Severity::Low);
    assert_eq!(long_desc.affected_count, 1);
    assert!(long_desc.affected_urls[0].evidence.contains("170"));

    // Zero-count Issues remain present and queryable via passed_issues().
    let client_err = report.issue(IssueKind::ClientError4xx).unwrap();
    assert_eq!(client_err.affected_count, 0);
    assert_eq!(client_err.affected_percentage, 0.0);
    assert!(
        report
            .passed_issues()
            .any(|i| i.kind == IssueKind::ClientError4xx)
    );
    assert!(
        !report
            .active_issues()
            .any(|i| i.kind == IssueKind::ClientError4xx)
    );

    // Summary counts reflect active issues per Severity tier.
    assert_eq!(report.crawled_urls, 5);
    assert_eq!(report.indexable_html_urls, 4);
    assert_eq!(report.active_issue_counts.critical, 1);
    assert_eq!(report.active_issue_counts.high, 2);
    assert_eq!(report.active_issue_counts.medium, 1);
    assert_eq!(report.active_issue_counts.low, 1);
}

#[test]
fn content_metadata_social_and_schema_issues_gate_strictly_to_2xx_indexable_html_while_raw_filters_stay_raw() {
    let mut healthy = indexable_page("https://example.com/");
    healthy.title = Some("Shared Title Only On One Indexable Page!".into());
    healthy.h1 = Some("Shared H1 Only On One Indexable Page".into());

    // 3xx redirect with missing metadata, images missing alt, and schema errors
    let mut redirect = CrawlResult::new_failed(
        "https://example.com/old".into(),
        Some(301),
        Some("301 Moved Permanently".into()),
        "Redirect".into(),
        1,
        5,
    );
    redirect.schema_errors = vec!["invalid json".into()];
    redirect.images = vec![ImageInfo {
        src: "https://example.com/r.png".into(),
        alt: None,
        local_path: None,
        width: None,
        height: None,
        title: None,
        loading: None,
    }];

    // 404 page sharing the same title as `healthy` + missing everything else
    let mut not_found = CrawlResult::new_failed(
        "https://example.com/missing".into(),
        Some(404),
        Some("404 Not Found".into()),
        "Non-200 Status Code".into(),
        1,
        5,
    );
    not_found.title = healthy.title.clone();
    not_found.schema_errors = vec!["bad schema on 404".into()];

    // 500 server error with missing everything
    let server_error = CrawlResult::new_failed(
        "https://example.com/error".into(),
        Some(500),
        Some("500 Internal Server Error".into()),
        "Non-200 Status Code".into(),
        1,
        5,
    );

    // 200 Noindex page sharing H1 with `healthy`, missing title/desc/OG, and schema errors
    let mut noindex = CrawlResult::new_failed(
        "https://example.com/private".into(),
        Some(200),
        None,
        "Noindex".into(),
        1,
        5,
    );
    noindex.indexable = false;
    noindex.h1 = healthy.h1.clone();
    noindex.h1_count = 2;
    noindex.schema_errors = vec!["schema on noindex".into()];
    noindex.canonical = Some("https://example.com/private".into());

    // 200 Non-HTML Resource (e.g., PDF)
    let mut pdf = CrawlResult::new_failed(
        "https://example.com/brochure.pdf".into(),
        Some(200),
        None,
        "Non-HTML Resource".into(),
        1,
        5,
    );
    pdf.indexable = false;

    // 200 Canonicalised page (canonical points elsewhere -> indexable = false)
    let mut canonicalised = CrawlResult::new_failed(
        "https://example.com/variant".into(),
        Some(200),
        None,
        "Canonicalised".into(),
        1,
        5,
    );
    canonicalised.indexable = false;
    canonicalised.canonical = Some("https://example.com/".into());

    let results = [
        healthy.clone(),
        redirect.clone(),
        not_found.clone(),
        server_error.clone(),
        noindex.clone(),
        pdf.clone(),
        canonicalised.clone(),
    ];

    let report = evaluate(&results, None, None);
    assert_eq!(report.crawled_urls, 7);
    assert_eq!(report.indexable_html_urls, 1);

    // None of the 17 content/metadata/social/structured-data Issues may fire!
    for issue in &report.issues {
        if issue.kind.requires_indexable_html_2xx() {
            assert_eq!(
                issue.affected_count, 0,
                "gated issue {:?} unexpectedly fired on non-indexable/non-2xx pages: {:?}",
                issue.kind, issue.affected_urls
            );
        }
    }

    // HTTP status, redirect, and canonical Issues DO evaluate across all observed URLs.
    assert_eq!(
        report
            .issue(IssueKind::Redirection3xx)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        vec!["https://example.com/old"]
    );
    assert_eq!(
        report
            .issue(IssueKind::ClientError4xx)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        vec!["https://example.com/missing"]
    );
    assert_eq!(
        report
            .issue(IssueKind::ServerError5xx)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        vec!["https://example.com/error"]
    );
    assert_eq!(
        report
            .issue(IssueKind::Canonicalised)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        vec!["https://example.com/variant"]
    );
    // MissingCanonical fires on redirect, not_found, server_error, pdf (all lack canonical)
    assert_eq!(
        report
            .issue(IssueKind::MissingCanonical)
            .unwrap()
            .affected_count,
        4
    );

    // Raw CrawlFilter / DuplicateIndex remain raw observation filters without regression.
    let raw_dups = DuplicateIndex::from_results(&results);
    assert!(CrawlFilter::TitleMissing.matches(&server_error, &results));
    assert!(CrawlFilter::SchemaErrors.matches(&not_found, &results));
    assert!(raw_dups.matches(CrawlFilter::TitleDuplicate, &healthy));
    assert!(raw_dups.matches(CrawlFilter::TitleDuplicate, &not_found));
    assert!(raw_dups.matches(CrawlFilter::H1Duplicate, &healthy));
    assert!(raw_dups.matches(CrawlFilter::H1Duplicate, &noindex));
}

#[test]
fn discovery_findings_map_all_seven_variants_and_preserve_overlapping_crawl_and_discovery_issues() {
    let config = CrawlConfig::default();
    let mut snapshot = DiscoverySnapshot::new(vec!["https://example.com/".into()], &config);
    snapshot.coverage.finished = true;
    snapshot.collection.sources.push(SitemapSource {
        url: "https://example.com/sitemap.xml".into(),
        parent: None,
        status: "200".into(),
        url_count: 6,
        error: None,
        is_index: Some(false),
    });

    for url in [
        "https://example.com/",
        "https://example.com/orphan-noindex-canon",
        "https://example.com/broken-in-sitemap",
        "https://example.com/redirect-in-sitemap",
        "https://example.com/unchecked-in-sitemap",
    ] {
        snapshot.collection.memberships.insert(
            url.into(),
            ["https://example.com/sitemap.xml".into()].into(),
        );
    }

    let mut root = indexable_page("https://example.com/");
    root.outlinks = vec![
        "https://example.com/absent-from-sitemap".into(),
        "https://example.com/broken-in-sitemap".into(),
        "https://example.com/redirect-in-sitemap".into(),
    ];

    let absent = indexable_page("https://example.com/absent-from-sitemap");

    // Overlapping Discovery findings: Unreached + Noindex + Canonical
    // Overlapping Crawl finding: Canonicalised (non-self canonical)
    let mut orphan_noindex_canon = CrawlResult::new_failed(
        "https://example.com/orphan-noindex-canon".into(),
        Some(200),
        None,
        "Noindex".into(),
        UNKNOWN_DEPTH,
        10,
    );
    orphan_noindex_canon.indexable = false;
    orphan_noindex_canon.canonical = Some("https://example.com/".into());

    // Overlapping Crawl (ClientError4xx) + Discovery (SitemapHttpError)
    let broken = CrawlResult::new_failed(
        "https://example.com/broken-in-sitemap".into(),
        Some(404),
        Some("404 Not Found".into()),
        "Non-200 Status Code".into(),
        1,
        10,
    );

    // Overlapping Crawl (Redirection3xx) + Discovery (SitemapRedirect)
    let redirect = CrawlResult::new_failed(
        "https://example.com/redirect-in-sitemap".into(),
        Some(200),
        None,
        "Redirect".into(),
        1,
        10,
    );
    snapshot.responses.insert(
        "https://example.com/redirect-in-sitemap".into(),
        ResponseEvidence {
            final_url: Some("https://example.com/absent-from-sitemap".into()),
            noindex: false,
            redirected: true,
        },
    );

    let results = [root, absent, orphan_noindex_canon, broken, redirect];
    reconcile(&mut snapshot, &results);

    let report = evaluate(&results, Some(&snapshot), Some(&config));
    // 5 crawled URLs + 1 unfetched sitemap URL = 6 observed URLs.
    assert_eq!(report.crawled_urls, 5);
    assert_eq!(report.observed_urls, 6);
    assert!(report.caveats.is_empty(), "clean finished crawl with readable sitemap should have no caveats: {:?}", report.caveats);

    // Verify all 7 Discovery Issues fired with expected Severities and Evidence.
    let cases = [
        (
            IssueKind::SitemapHttpError,
            Severity::Critical,
            "https://example.com/broken-in-sitemap",
            "404",
        ),
        (
            IssueKind::SitemapNoindex,
            Severity::Critical,
            "https://example.com/orphan-noindex-canon",
            "Noindex",
        ),
        (
            IssueKind::SitemapRedirect,
            Severity::High,
            "https://example.com/redirect-in-sitemap",
            "https://example.com/absent-from-sitemap",
        ),
        (
            IssueKind::SitemapUnreached,
            Severity::High,
            "https://example.com/orphan-noindex-canon",
            "sitemap.xml",
        ),
        (
            IssueKind::SitemapDifferentCanonical,
            Severity::Medium,
            "https://example.com/orphan-noindex-canon",
            "https://example.com/",
        ),
        (
            IssueKind::SitemapUnchecked,
            Severity::Medium,
            "https://example.com/unchecked-in-sitemap",
            "Not fetched",
        ),
        (
            IssueKind::SitemapAbsent,
            Severity::Low,
            "https://example.com/absent-from-sitemap",
            "absent from imported sitemaps",
        ),
    ];

    for (kind, severity, expected_url, expected_evidence_substr) in cases {
        let issue = report.issue(kind).unwrap();
        assert_eq!(issue.severity, severity);
        assert_eq!(issue.source, IssueSource::Discovery);
        let entry = issue
            .affected_urls
            .iter()
            .find(|u| u.url == expected_url)
            .unwrap_or_else(|| panic!("expected {expected_url} in {kind:?}"));
        assert!(
            entry.evidence.contains(expected_evidence_substr),
            "expected evidence for {kind:?} to contain {expected_evidence_substr:?}, got {:?}",
            entry.evidence
        );
    }

    // Verify overlapping Crawl Issues on the exact same URLs.
    assert!(
        report
            .issue(IssueKind::ClientError4xx)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == "https://example.com/broken-in-sitemap")
    );
    let crawl_redirect = report
        .issue(IssueKind::Redirection3xx)
        .unwrap()
        .affected_urls
        .iter()
        .find(|u| u.url == "https://example.com/redirect-in-sitemap")
        .unwrap();
    assert!(
        crawl_redirect
            .evidence
            .contains("https://example.com/absent-from-sitemap")
    );
    assert!(
        report
            .issue(IssueKind::Canonicalised)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == "https://example.com/orphan-noindex-canon")
    );
}

#[test]
fn scope_caveats_flag_stopped_capped_and_missing_sitemap_evidence_including_legacy_projects() {
    // 1. Legacy Project (discovery: None) that also reached max_urls.
    let legacy_config = CrawlConfig {
        max_urls: 1,
        ..Default::default()
    };
    let legacy_project = CrawlProject {
        seed_url: "https://example.com/".into(),
        config: legacy_config,
        results: vec![indexable_page("https://example.com/")],
        stats: CrawlStats::default(),
        discovery: None,
    };
    let legacy_report = evaluate_project(&legacy_project);
    assert!(legacy_report.has_caveat(ScopeCaveatKind::MissingSitemapDiscovery));
    assert!(legacy_report.has_caveat(ScopeCaveatKind::UrlLimitReached));
    assert!(!legacy_report.has_caveat(ScopeCaveatKind::CrawlStopped));

    // 2. Stopped crawl that hit URL limit, depth limit, had only failed sitemap sources, and truncated.
    let config = CrawlConfig::default();
    let mut snapshot = DiscoverySnapshot::new(vec!["https://example.com/".into()], &config);
    snapshot.coverage.finished = true;
    snapshot.coverage.stopped = true;
    snapshot.coverage.url_limit_hit = true;
    snapshot.coverage.depth_limit_hit = true;
    snapshot.collection.truncated = true;
    snapshot.collection.sources.push(SitemapSource {
        url: "https://example.com/sitemap.xml".into(),
        parent: None,
        status: "503".into(),
        url_count: 0,
        error: Some("503 Service Unavailable".into()),
        is_index: None,
    });

    let report = evaluate(&[indexable_page("https://example.com/")], Some(&snapshot), Some(&config));
    assert!(report.has_caveat(ScopeCaveatKind::CrawlStopped));
    assert!(report.has_caveat(ScopeCaveatKind::UrlLimitReached));
    assert!(report.has_caveat(ScopeCaveatKind::DepthLimitReached));
    assert!(report.has_caveat(ScopeCaveatKind::MissingSitemapDiscovery));
    assert!(report.has_caveat(ScopeCaveatKind::SitemapTruncated));
    assert!(!report.has_caveat(ScopeCaveatKind::CrawlInProgress));

    // 3. In-progress live crawl via evaluate_state.
    let state = CrawlState::new();
    state.insert_result(indexable_page("https://example.com/"));
    let in_progress = DiscoverySnapshot::new(vec!["https://example.com/".into()], &config);
    state.set_discovery(Some(in_progress));
    let live_report = evaluate_state(&state);
    assert!(live_report.has_caveat(ScopeCaveatKind::CrawlInProgress));
}

async fn loopback_audit_fixture() -> (String, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let site = origin.clone();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let origin = site.clone();
            tokio::spawn(async move {
                let mut buffer = [0u8; 4096];
                let Ok(n) = socket.read(&mut buffer).await else {
                    return;
                };
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                let (status, content_type, extra_headers, body) = match path {
                    "/robots.txt" => (
                        "200 OK",
                        "text/plain",
                        String::new(),
                        format!("User-agent: *\nAllow: /\nSitemap: {origin}/sitemap.xml\n"),
                    ),
                    "/sitemap.xml" => {
                        let urls = [
                            "/",
                            "/content-issues",
                            "/orphan",
                            "/missing",
                            "/old",
                            "/noindex",
                        ]
                        .iter()
                        .map(|p| format!("<url><loc>{origin}{p}</loc></url>"))
                        .collect::<String>();
                        (
                            "200 OK",
                            "application/xml",
                            String::new(),
                            format!(
                                r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{urls}</urlset>"#
                            ),
                        )
                    }
                    "/" => (
                        "200 OK",
                        "text/html; charset=utf-8",
                        String::new(),
                        format!(
                            r#"<!doctype html><html><head>
                            <title>Home Page With A Proper Length SEO Title</title>
                            <meta name="description" content="A complete home page description that sits comfortably between seventy and one hundred fifty-five characters.">
                            <link rel="canonical" href="{origin}/">
                            <meta property="og:title" content="Home OG Title">
                            <meta property="og:description" content="Home OG Description">
                            </head><body>
                            <h1>Home Primary Heading</h1>
                            <h2>Home Section</h2>
                            <a href="/content-issues">Content</a>
                            <a href="/missing">Missing</a>
                            <a href="/old">Old</a>
                            <a href="/noindex">Noindex</a>
                            <a href="/unlisted">Unlisted</a>
                            </body></html>"#
                        ),
                    ),
                    "/content-issues" => (
                        "200 OK",
                        "text/html; charset=utf-8",
                        String::new(),
                        format!(
                            r#"<!doctype html><html><head>
                            <meta name="description" content="Too short.">
                            <link rel="canonical" href="{origin}/content-issues">
                            <script type="application/ld+json">{{ not valid json }}</script>
                            </head><body>
                            <p>Missing title, missing H1, missing OG tags, bad schema, image missing alt.</p>
                            <img src="/hero.png">
                            </body></html>"#
                        ),
                    ),
                    "/orphan" => (
                        "200 OK",
                        "text/html; charset=utf-8",
                        String::new(),
                        format!(
                            r#"<!doctype html><html><head>
                            <title>Orphan Sitemap Page With A Proper Length Title</title>
                            <meta name="description" content="An orphan page listed in the sitemap with a complete description between seventy and one hundred fifty-five characters.">
                            <link rel="canonical" href="{origin}/orphan">
                            <meta property="og:title" content="Orphan OG">
                            <meta property="og:description" content="Orphan OG Desc">
                            </head><body><h1>Orphan Page Heading</h1><h2>Orphan Section</h2></body></html>"#
                        ),
                    ),
                    "/unlisted" => (
                        "200 OK",
                        "text/html; charset=utf-8",
                        String::new(),
                        format!(
                            r#"<!doctype html><html><head>
                            <title>Unlisted Reachable Page With Proper Title</title>
                            <meta name="description" content="A reachable indexable page that is intentionally absent from the XML sitemap for testing review opportunities.">
                            <link rel="canonical" href="{origin}/unlisted">
                            <meta property="og:title" content="Unlisted OG">
                            <meta property="og:description" content="Unlisted OG Desc">
                            </head><body><h1>Unlisted Heading</h1><h2>Unlisted Section</h2></body></html>"#
                        ),
                    ),
                    "/noindex" => (
                        "200 OK",
                        "text/html; charset=utf-8",
                        String::new(),
                        format!(
                            r#"<!doctype html><html><head>
                            <meta name="robots" content="noindex">
                            <link rel="canonical" href="{origin}/noindex">
                            </head><body><p>Noindex page lacking title, description, H1, and OG tags.</p></body></html>"#
                        ),
                    ),
                    "/old" => (
                        "301 Moved Permanently",
                        "text/plain",
                        format!("Location: {origin}/unlisted\r\n"),
                        "Moved".to_string(),
                    ),
                    "/missing" => (
                        "404 Not Found",
                        "text/html; charset=utf-8",
                        String::new(),
                        "<html><body>Not Found</body></html>".to_string(),
                    ),
                    _ => (
                        "404 Not Found",
                        "text/plain",
                        String::new(),
                        "Not Found".to_string(),
                    ),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    (origin, handle)
}

#[tokio::test]
async fn loopback_http_crawl_unifies_all_severities_gating_overlaps_and_capped_caveats() {
    let (origin, server) = loopback_audit_fixture().await;

    // 1. Full uncapped crawl
    let state = CrawlState::new();
    let config = CrawlConfig {
        max_urls: 50,
        max_concurrency: 2,
        sitemap_enabled: true,
        ..Default::default()
    };
    state.set_config(config);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    state.set_crawling(true);
    let crawler = Crawler::new(state.clone(), tx);
    crawler.start(format!("{origin}/")).await;
    while let Ok(update) = rx.try_recv() {
        if matches!(update, CrawlUpdate::Finished) {
            break;
        }
    }

    let report = evaluate_state(&state);
    assert!(
        report.caveats.is_empty(),
        "complete uncapped crawl with readable sitemap should have no caveats, got {:?}",
        report.caveats
    );

    // Critical: ClientError4xx + SitemapHttpError on /missing; SitemapNoindex on /noindex
    assert!(
        report
            .issue(IssueKind::ClientError4xx)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == format!("{origin}/missing"))
    );
    assert!(
        report
            .issue(IssueKind::SitemapHttpError)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == format!("{origin}/missing"))
    );
    assert!(
        report
            .issue(IssueKind::SitemapNoindex)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == format!("{origin}/noindex"))
    );

    // High: SitemapRedirect on /old, SitemapUnreached on /orphan, MissingTitle + MissingH1 + StructuredDataErrors ONLY on /content-issues
    assert!(
        report
            .issue(IssueKind::SitemapRedirect)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == format!("{origin}/old"))
    );
    assert!(
        report
            .issue(IssueKind::SitemapUnreached)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == format!("{origin}/orphan"))
    );
    for gated_high in [
        IssueKind::MissingTitle,
        IssueKind::MissingH1,
        IssueKind::StructuredDataErrors,
    ] {
        let urls: Vec<&str> = report
            .issue(gated_high)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect();
        assert_eq!(
            urls,
            vec![format!("{origin}/content-issues")],
            "{gated_high:?} must fire only on 2xx indexable /content-issues (never /missing, /old, or /noindex)"
        );
    }

    // Medium: Redirection3xx on /old, ImagesMissingAlt on /content-issues
    assert!(
        report
            .issue(IssueKind::Redirection3xx)
            .unwrap()
            .affected_urls
            .iter()
            .any(|u| u.url == format!("{origin}/old"))
    );
    assert_eq!(
        report
            .issue(IssueKind::ImagesMissingAlt)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        vec![format!("{origin}/content-issues")]
    );

    // Low: DescriptionTooShort + MissingOgTitle + MissingOgDescription only on /content-issues; SitemapAbsent on /unlisted
    assert_eq!(
        report
            .issue(IssueKind::DescriptionTooShort)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        vec![format!("{origin}/content-issues")]
    );
    assert_eq!(
        report
            .issue(IssueKind::SitemapAbsent)
            .unwrap()
            .affected_urls
            .iter()
            .map(|u| u.url.as_str())
            .collect::<Vec<_>>(),
        vec![format!("{origin}/unlisted")]
    );

    // Zero-count check: ServerError5xx did not occur
    assert_eq!(report.issue(IssueKind::ServerError5xx).unwrap().affected_count, 0);

    // 2. URL-limit-capped crawl produces UrlLimitReached caveat + SitemapUnchecked
    let capped_state = CrawlState::new();
    let capped_config = CrawlConfig {
        max_urls: 2,
        max_concurrency: 1,
        sitemap_enabled: true,
        ..Default::default()
    };
    capped_state.set_config(capped_config);
    let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
    capped_state.set_crawling(true);
    Crawler::new(capped_state.clone(), tx2)
        .start(format!("{origin}/"))
        .await;
    while let Ok(update) = rx2.try_recv() {
        if matches!(update, CrawlUpdate::Finished) {
            break;
        }
    }

    let capped_report = evaluate_state(&capped_state);
    assert!(capped_report.has_caveat(ScopeCaveatKind::UrlLimitReached));
    assert!(
        capped_report
            .issue(IssueKind::SitemapUnchecked)
            .unwrap()
            .affected_count
            > 0
    );

    server.abort();
}

#[test]
fn export_markdown_renders_scope_caveats_severity_summary_and_active_issues_omitting_zero_count() {
    let healthy = indexable_page("https://example.com/");

    let mut not_found = CrawlResult::new_failed(
        "https://example.com/missing".into(),
        Some(404),
        Some("404 Not Found".into()),
        "Non-200 Status Code".into(),
        1,
        12,
    );
    not_found.canonical = Some("https://example.com/missing".into());

    let mut missing_title = indexable_page("https://example.com/no-title");
    missing_title.title = None;
    missing_title.meta_desc = Some("x".repeat(170));

    let config = CrawlConfig {
        max_urls: 3,
        ..Default::default()
    };
    let report = evaluate(&[healthy, not_found, missing_title], None, Some(&config));

    let md = export_markdown(&report);
    // Deterministic output
    assert_eq!(md, export_markdown(&report));

    // Scope caveats header present (missing sitemap discovery + URL limit reached)
    assert!(md.contains("Scope Caveats"), "expected Scope Caveats header:\n{md}");
    assert!(md.contains("No sitemap discovery evidence is available"));
    assert!(md.contains("configured URL limit (3 URLs)"));

    // Severity summary table with all 4 fixed tiers and percentages, no health score (ADR-0001)
    assert!(md.contains("## Severity Summary"));
    assert!(md.contains("| Critical | 1 | 1 | 33.3% |"), "md was:\n{md}");
    assert!(md.contains("| High | 1 | 1 | 33.3% |"), "md was:\n{md}");
    assert!(md.contains("| Medium | 0 | 0 | 0.0% |"), "md was:\n{md}");
    assert!(md.contains("| Low | 1 | 1 | 33.3% |"), "md was:\n{md}");
    assert!(!md.to_lowercase().contains("health score"));

    // Active Issues appear in Severity order with Rationale, Suggested Action, Affected URL, and Evidence
    let critical_pos = md.find("Client Error (4xx)").expect("ClientError4xx missing");
    let high_pos = md.find("Missing Title").expect("MissingTitle missing");
    let low_pos = md
        .find("Description > 155 chars")
        .expect("DescriptionTooLong missing");
    assert!(critical_pos < high_pos && high_pos < low_pos);

    assert!(md.contains(IssueKind::ClientError4xx.rationale()));
    assert!(md.contains(IssueKind::ClientError4xx.suggested_action()));
    assert!(md.contains("https://example.com/missing"));
    assert!(md.contains("HTTP 404 (404 Not Found)"));

    assert!(md.contains(IssueKind::MissingTitle.rationale()));
    assert!(md.contains(IssueKind::MissingTitle.suggested_action()));
    assert!(md.contains("https://example.com/no-title"));

    // Zero-count Issues are omitted from active export
    assert!(!md.contains("Server Error (5xx)"));
    assert!(!md.contains("Missing H1"));
}

#[test]
fn export_markdown_and_single_issue_markdown_cap_inline_urls_at_25_with_csv_truncation_note() {
    assert_eq!(MARKDOWN_SAMPLE_URL_LIMIT, 25);

    // 32 indexable pages missing a title + 1 broken 404 page
    let mut results: Vec<CrawlResult> = (1..=32)
        .map(|i| {
            let mut page = indexable_page(&format!("https://example.com/page-{i:02}"));
            page.title = None;
            page
        })
        .collect();
    let mut broken = CrawlResult::new_failed(
        "https://example.com/broken".into(),
        Some(404),
        Some("404 Not Found".into()),
        "Non-200 Status Code".into(),
        1,
        10,
    );
    broken.canonical = Some("https://example.com/broken".into());
    results.push(broken);

    let report = evaluate(&results, None, None);
    let missing_title = report.issue(IssueKind::MissingTitle).unwrap();
    assert_eq!(missing_title.affected_count, 32);

    // Full Markdown export caps MissingTitle at 25 inline rows with truncation note pointing to CSV.
    let full_md = export_markdown(&report);
    assert!(full_md.contains("https://example.com/page-01"));
    assert!(full_md.contains("https://example.com/page-25"));
    assert!(!full_md.contains("https://example.com/page-26"));
    assert!(!full_md.contains("https://example.com/page-32"));
    assert!(full_md.contains("Showing 25 of 32 Affected URLs (7 more omitted). Export to CSV"));

    // Single selected Issue Markdown export includes only MissingTitle, preserves scope caveats,
    // includes Severity summary table, and applies the same 25-URL cap + CSV truncation note.
    let single_md = export_issue_markdown(&report, missing_title);
    assert_eq!(single_md, export_issue_markdown(&report, missing_title));
    assert!(single_md.contains("Scope Caveats"));
    assert!(single_md.contains("## Severity Summary"));
    assert!(single_md.contains("Missing Title"));
    assert!(single_md.contains(IssueKind::MissingTitle.rationale()));
    assert!(single_md.contains(IssueKind::MissingTitle.suggested_action()));
    assert!(single_md.contains("https://example.com/page-01"));
    assert!(single_md.contains("https://example.com/page-25"));
    assert!(!single_md.contains("https://example.com/page-26"));
    assert!(single_md.contains("Showing 25 of 32 Affected URLs (7 more omitted). Export to CSV"));
    // Unselected Issue must not appear in single-Issue export.
    assert!(!single_md.contains("Client Error (4xx)"));
    assert!(!single_md.contains("https://example.com/broken"));

    // Exact boundary of 25 URLs must NOT emit a truncation note, and clean crawl with no caveats omits Scope Caveats.
    let exact_25: Vec<CrawlResult> = (1..=25)
        .map(|i| {
            let mut page = indexable_page(&format!("https://example.com/exact-{i:02}"));
            page.title = None;
            page
        })
        .collect();
    let config = CrawlConfig::default();
    let mut snapshot = DiscoverySnapshot::new(vec!["https://example.com/exact-01".into()], &config);
    snapshot.coverage.finished = true;
    snapshot.collection.sources.push(SitemapSource {
        url: "https://example.com/sitemap.xml".into(),
        parent: None,
        status: "200".into(),
        url_count: 25,
        error: None,
        is_index: Some(false),
    });
    for i in 1..=25 {
        snapshot.collection.memberships.insert(
            format!("https://example.com/exact-{i:02}"),
            ["https://example.com/sitemap.xml".into()].into(),
        );
    }
    let exact_report = evaluate(&exact_25, Some(&snapshot), Some(&config));
    assert!(exact_report.caveats.is_empty());
    let exact_issue = exact_report.issue(IssueKind::MissingTitle).unwrap();
    let exact_md = export_issue_markdown(&exact_report, exact_issue);
    assert!(!exact_md.contains("Scope Caveats"));
    assert!(exact_md.contains("https://example.com/exact-25"));
    assert!(!exact_md.contains("omitted"));
    assert!(!exact_md.contains("Export to CSV"));
}

#[test]
fn export_csv_and_single_issue_csv_are_uncapped_omit_zero_count_preserve_caveats_and_escape_rfc4180() {
    // 30 pages missing a title (exceeds Markdown's 25-URL cap) + 1 page with commas, quotes, and newlines in Evidence.
    let mut results: Vec<CrawlResult> = (1..=30)
        .map(|i| {
            let mut page = indexable_page(&format!("https://example.com/item-{i:02}"));
            page.title = None;
            page
        })
        .collect();

    let mut tricky = indexable_page("https://example.com/tricky,page");
    tricky.schema_errors = vec!["Expected \"@type\", found comma, and\nnewline".into()];
    results.push(tricky);

    let config = CrawlConfig {
        max_urls: 31,
        ..Default::default()
    };
    let report = evaluate(&results, None, Some(&config));
    assert!(report.has_caveat(ScopeCaveatKind::MissingSitemapDiscovery));
    assert!(report.has_caveat(ScopeCaveatKind::UrlLimitReached));

    // Full active Issues CSV
    let full_csv = export_csv(&report).expect("export_csv should succeed");
    assert_eq!(full_csv, export_csv(&report).unwrap());

    // Raw CSV must quote fields containing commas, quotes, or newlines per RFC-4180.
    assert!(full_csv.contains("\"https://example.com/tricky,page\""));
    assert!(full_csv.contains("\"\"@type\"\""));

    let mut reader = csv::Reader::from_reader(full_csv.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .unwrap()
        .iter()
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(
        headers,
        vec![
            "severity",
            "issue",
            "url",
            "evidence",
            "suggested_action",
            "source",
            "scope_caveats",
        ]
    );

    let rows: Vec<csv::StringRecord> = reader.records().map(|r| r.unwrap()).collect();
    // 30 MissingTitle rows + 1 StructuredDataErrors row = 31 rows (uncapped, zero-count Issues omitted).
    assert_eq!(rows.len(), 31);

    let missing_title_rows: Vec<&csv::StringRecord> = rows
        .iter()
        .filter(|r| &r[1] == IssueKind::MissingTitle.name())
        .collect();
    assert_eq!(missing_title_rows.len(), 30);
    assert_eq!(&missing_title_rows[0][0], "High");
    assert_eq!(&missing_title_rows[0][2], "https://example.com/item-01");
    assert_eq!(&missing_title_rows[29][2], "https://example.com/item-30");
    assert_eq!(&missing_title_rows[0][5], "Crawl Results");
    assert!(missing_title_rows[0][6].contains("No sitemap discovery evidence is available"));
    assert!(missing_title_rows[0][6].contains("configured URL limit (31 URLs)"));

    // Verify RFC-4180 round-trip of comma, double-quote, and newline characters.
    let schema_row = rows
        .iter()
        .find(|r| &r[1] == IssueKind::StructuredDataErrors.name())
        .expect("StructuredDataErrors row missing");
    assert_eq!(&schema_row[0], "High");
    assert_eq!(&schema_row[2], "https://example.com/tricky,page");
    assert!(schema_row[3].contains("Expected \"@type\", found comma, and\nnewline"));
    assert_eq!(&schema_row[4], IssueKind::StructuredDataErrors.suggested_action());
    assert_eq!(&schema_row[5], "Crawl Results");

    // Zero-count Issues must not appear in full CSV.
    assert!(!rows.iter().any(|r| &r[1] == IssueKind::ServerError5xx.name()));

    // Single selected Issue CSV (MissingTitle only -> 30 uncapped rows, preserves scope caveats).
    let missing_title = report.issue(IssueKind::MissingTitle).unwrap();
    let single_csv = export_issue_csv(&report, missing_title).expect("export_issue_csv should succeed");
    let mut single_reader = csv::Reader::from_reader(single_csv.as_bytes());
    let single_rows: Vec<csv::StringRecord> = single_reader.records().map(|r| r.unwrap()).collect();
    assert_eq!(single_rows.len(), 30);
    assert!(
        single_rows
            .iter()
            .all(|r| &r[1] == IssueKind::MissingTitle.name())
    );
    assert!(single_rows[0][6].contains("configured URL limit (31 URLs)"));

    // Empty active-Issues report produces header-only CSV and clean Markdown empty state.
    let clean = evaluate(&[indexable_page("https://example.com/clean")], None, None);
    let empty_csv = export_csv(&clean).unwrap();
    let mut empty_reader = csv::Reader::from_reader(empty_csv.as_bytes());
    assert_eq!(empty_reader.records().count(), 0);
    let empty_md = export_markdown(&clean);
    assert!(empty_md.contains("No active Issues found"));
}

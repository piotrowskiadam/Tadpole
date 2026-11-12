use std::io::Write;
use tadpole::{
    crawler::{CrawlUpdate, Crawler},
    discovery::{DiscoverySnapshot, Finding, UNKNOWN_DEPTH, reconcile},
    sitemap,
    state::{CrawlConfig, CrawlMode, CrawlProject, CrawlResult, CrawlState},
};

#[test]
fn parses_sitemap_namespaces_entities_and_gzip_without_image_locations() {
    let xml = br#"<?xml version="1.0"?><s:urlset xmlns:s="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="urn:image"><s:url><s:loc>https://example.com/a?x=1&amp;y=2</s:loc><image:image><image:loc>https://example.com/image.jpg</image:loc></image:image></s:url><s:url><s:loc><![CDATA[https://example.com/b]]></s:loc></s:url></s:urlset>"#;
    let parsed = sitemap::parse(xml, 50_000).unwrap();
    assert_eq!(
        parsed.locations,
        ["https://example.com/a?x=1&y=2", "https://example.com/b"]
    );
    assert!(!parsed.is_index);
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(xml).unwrap();
    let compressed = gz.finish().unwrap();
    assert_eq!(sitemap::decode_body(&compressed, 4096).unwrap(), xml);
    assert!(sitemap::decode_body(&compressed, 20).is_err());
    assert!(sitemap::decode_body(b"\x1f\x8bgarbage", 4096).is_err());
    let truncated = sitemap::parse(xml, 1).unwrap();
    assert!(truncated.truncated);
    assert_eq!(truncated.locations.len(), 1);
    let index = sitemap::parse(
        b"<sitemapindex><sitemap><loc>https://example.com/a.xml</loc></sitemap></sitemapindex>",
        10,
    )
    .unwrap();
    assert!(index.is_index);
}

#[test]
fn rejects_invalid_or_incomplete_xml_instead_of_certifying_an_empty_sitemap() {
    for xml in [
        "",
        "<html/>",
        "<urlset><url><loc>https://example.com/a</loc>",
        "<urlset><url/></urlset>",
        "<urlset><url><loc/></url></urlset>",
        "<urlset/><urlset/>",
        "<!DOCTYPE urlset [<!ENTITY a SYSTEM 'file:///etc/passwd'>]><urlset/>",
        "<urlset><url><loc>https://example.com/a</loc><loc>https://example.com/b</loc></url></urlset>",
        "<urlset><url><loc>&undefined;</loc></url></urlset>",
        "<urlset><url><loc>https://example.com/<b>page</b></loc></url></urlset>",
        "<urlset xmlns:image='urn:image'><url><image:loc>https://example.com/image.jpg</image:loc></url></urlset>",
        "<urlset xmlns='urn:not-a-sitemap'/>",
    ] {
        assert!(
            sitemap::parse(xml.as_bytes(), 50).is_err(),
            "accepted {xml}"
        );
    }
    assert!(sitemap::parse(b"<urlset/>", 50).is_ok());
}

fn page(url: &str, links: &[&str], depth: usize) -> CrawlResult {
    let mut result =
        CrawlResult::new_failed(url.into(), Some(200), None, "Indexable".into(), depth, 1);
    result.indexable = true;
    result.outlinks = links.iter().map(|v| v.to_string()).collect();
    result
}

#[test]
fn disconnected_linked_groups_are_not_roots_and_findings_can_overlap() {
    let config = CrawlConfig::default();
    let mut snapshot = DiscoverySnapshot::new(vec!["https://example.com/".into()], &config);
    for url in ["https://example.com/a", "https://example.com/b"] {
        snapshot.collection.memberships.insert(
            url.into(),
            ["https://example.com/sitemap.xml".into()].into(),
        );
    }
    let root = page("https://example.com/", &["https://example.com/linked"], 0);
    let linked = page("https://example.com/linked", &[], 1);
    let a = page(
        "https://example.com/a",
        &["https://example.com/b"],
        UNKNOWN_DEPTH,
    );
    let mut b = page(
        "https://example.com/b",
        &["https://example.com/a"],
        UNKNOWN_DEPTH,
    );
    b.indexability_status = "Noindex".into();
    b.indexable = false;
    b.canonical = Some("https://example.com/elsewhere".into());
    let results = [root, linked, a, b];
    reconcile(&mut snapshot, &results);
    let a = snapshot
        .rows
        .iter()
        .find(|r| r.url.ends_with("/a"))
        .unwrap();
    assert_eq!(a.inlinks, 1);
    assert_eq!(a.reached_depth, None);
    assert!(a.findings.contains(&Finding::Unreached));
    let b = snapshot
        .rows
        .iter()
        .find(|r| r.url.ends_with("/b"))
        .unwrap();
    assert!(b.findings.contains(&Finding::Unreached));
    assert!(b.findings.contains(&Finding::Noindex));
    assert!(b.findings.contains(&Finding::Canonical));
    let graph = tadpole::visualization::SnapshotBuilder::build_from_results(&results);
    assert_eq!(graph.metrics.max_crawl_depth, 1);
    let mut visualizer = tadpole::visualization::DomainVisualizationModule::new();
    visualizer.load_snapshot(graph);
    let layout = visualizer
        .get_layout(tadpole::visualization::LayoutType::Hierarchical, 1)
        .unwrap();
    assert!(layout.positions.iter().all(|p| p.y.abs() < 1000.0));
    snapshot.coverage.mode = CrawlMode::List;
    reconcile(&mut snapshot, &results);
    assert!(
        snapshot
            .rows
            .iter()
            .all(|r| r.reached_depth.is_none() && !r.findings.contains(&Finding::Unreached))
    );
}

async fn fixture() -> (String, tokio::task::JoinHandle<()>) {
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
                if path == "/slow.xml" {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                }
                let (status, content_type, headers, body) = match path {
                    "/slow.xml" => ("200 OK", "application/xml", "", b"<urlset/>".to_vec()),
                    path if path.starts_with("/depth/") => {
                        let depth: usize = path.trim_start_matches("/depth/").parse().unwrap_or(0);
                        ("200 OK", "application/xml", "", format!("<sitemapindex><sitemap><loc>{origin}/depth/{}</loc></sitemap></sitemapindex>", depth + 1).into_bytes())
                    }
                    "/robots.txt" => (
                        "200 OK",
                        "text/plain",
                        "",
                        format!("User-agent: *\nAllow: /\nSitemap: {origin}/index.xml\n")
                            .into_bytes(),
                    ),
                    "/index.xml" => {
                        let repeated =
                            format!("<sitemap><loc>{origin}/pages.xml</loc></sitemap>").repeat(80);
                        ("200 OK", "application/xml", "", format!("<sitemapindex>{repeated}<sitemap><loc>{origin}/other.xml.gz</loc></sitemap><sitemap><loc>{origin}/broken.xml</loc></sitemap><sitemap><loc>{origin}/index.xml</loc></sitemap></sitemapindex>").into_bytes())
                    }
                    "/pages.xml" => {
                        let entries = [
                            "/",
                            "/linked",
                            "/orphan-a",
                            "/orphan-b",
                            "/noindex",
                            "/old",
                            "/missing",
                        ]
                        .iter()
                        .map(|p| format!("<url><loc>{origin}{p}</loc></url>"))
                        .collect::<String>();
                        (
                            "200 OK",
                            "application/xml",
                            "",
                            format!("<urlset>{entries}</urlset>").into_bytes(),
                        )
                    }
                    "/other.xml.gz" => {
                        let mut gz = flate2::write::GzEncoder::new(
                            Vec::new(),
                            flate2::Compression::default(),
                        );
                        write!(
                            gz,
                            "<urlset><url><loc>{origin}/orphan-a</loc></url></urlset>"
                        )
                        .unwrap();
                        ("200 OK", "application/gzip", "", gz.finish().unwrap())
                    }
                    "/broken.xml" => (
                        "503 Service Unavailable",
                        "text/plain",
                        "",
                        b"Try later".to_vec(),
                    ),
                    "/old" => (
                        "301 Moved Permanently",
                        "text/html",
                        "Location: /destination\r\n",
                        Vec::new(),
                    ),
                    "/missing" => ("404 Not Found", "text/html", "", b"Not found".to_vec()),
                    "/" => (
                        "200 OK",
                        "text/html",
                        "",
                        b"<title>Home</title><a href='/linked'>Linked page</a>".to_vec(),
                    ),
                    "/orphan-a" => (
                        "200 OK",
                        "text/html",
                        "",
                        b"<title>Orphan A</title><a href='/orphan-b'>Orphan B</a>".to_vec(),
                    ),
                    "/orphan-b" => (
                        "200 OK",
                        "text/html",
                        "",
                        b"<title>Orphan B</title><a href='/orphan-a'>Orphan A</a>".to_vec(),
                    ),
                    "/noindex" => (
                        "200 OK",
                        "text/html",
                        "X-Robots-Tag: noindex\r\n",
                        b"<link rel='canonical' href='/elsewhere'><title>Unlisted</title>".to_vec(),
                    ),
                    _ => ("200 OK", "text/html", "", b"<title>A page</title>".to_vec()),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(&body).await;
            });
        }
    });
    (origin, handle)
}

#[tokio::test]
async fn live_crawl_reconciles_index_gzip_redirects_and_partial_source_failure() {
    let (origin, server) = fixture().await;
    let state = CrawlState::new();
    state.set_config(CrawlConfig {
        max_urls: 20,
        max_concurrency: 2,
        ..Default::default()
    });
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    Crawler::new(state.clone(), tx).start(origin.clone()).await;
    let snapshot = state.discovery().unwrap();
    assert!(snapshot.coverage.finished);
    assert!(!snapshot.coverage.stopped);
    assert_eq!(
        snapshot.collection.sources.len(),
        4,
        "cycle must not refetch the index"
    );
    assert!(
        snapshot
            .collection
            .sources
            .iter()
            .any(|s| s.error.is_some())
    );
    assert_eq!(snapshot.collection.memberships.len(), 7);
    let a = snapshot
        .rows
        .iter()
        .find(|r| r.url.ends_with("/orphan-a"))
        .unwrap();
    assert_eq!(a.sitemaps.len(), 2);
    assert_eq!(a.inlinks, 1);
    assert_eq!(a.reached_depth, None);
    assert!(a.findings.contains(&Finding::Unreached));
    assert_eq!(state.get_result(&a.url).unwrap().depth, UNKNOWN_DEPTH);
    let noindex = snapshot
        .rows
        .iter()
        .find(|r| r.url.ends_with("/noindex"))
        .unwrap();
    assert!(noindex.findings.contains(&Finding::Noindex));
    assert!(noindex.findings.contains(&Finding::Canonical));
    let redirect = snapshot
        .rows
        .iter()
        .find(|r| r.url.ends_with("/old"))
        .unwrap();
    assert!(redirect.findings.contains(&Finding::Redirect));
    assert_eq!(
        redirect.response.final_url.as_deref(),
        Some(format!("{origin}/destination").as_str())
    );
    assert!(!state.get_result(&redirect.url).unwrap().indexable);
    assert!(
        snapshot
            .rows
            .iter()
            .find(|r| r.url.ends_with("/missing"))
            .unwrap()
            .findings
            .contains(&Finding::HttpError)
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("audit.seocrawl");
    tadpole::project::save(&path, &state.get_project(origin.clone())).unwrap();
    let project: CrawlProject =
        serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    let restored = CrawlState::new();
    restored.load_project(project);
    assert_eq!(
        restored.discovery().unwrap().rows.len(),
        snapshot.rows.len()
    );
    let mut output = Vec::new();
    tadpole::discovery::export_csv(&mut output, &snapshot, &[a, noindex]).unwrap();
    let csv_rows: Vec<_> = csv::Reader::from_reader(output.as_slice())
        .records()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        csv_rows.len(),
        4,
        "orphan A has one finding, noindex has three including unreachable"
    );
    assert!(
        csv_rows
            .iter()
            .all(|r| r[0].ends_with("/orphan-a") || r[0].ends_with("/noindex"))
    );
    let mut finished = false;
    while let Ok(event) = rx.try_recv() {
        match event {
            CrawlUpdate::Finished => finished = true,
            CrawlUpdate::Crawled(_) => assert!(!finished),
            _ => {}
        }
    }
    assert!(finished);
    server.abort();
}

#[tokio::test]
async fn limited_crawl_keeps_unfetched_sitemap_evidence_without_exceeding_budget() {
    let (origin, server) = fixture().await;
    let state = CrawlState::new();
    state.set_config(CrawlConfig {
        max_urls: 2,
        ..Default::default()
    });
    let (tx, _) = tokio::sync::mpsc::unbounded_channel();
    Crawler::new(state.clone(), tx).start(origin).await;
    assert_eq!(state.get_stats().crawled, 2);
    let snapshot = state.discovery().unwrap();
    assert!(snapshot.coverage.url_limit_hit);
    let a = snapshot
        .rows
        .iter()
        .find(|r| r.url.ends_with("/orphan-a"))
        .unwrap();
    assert_eq!(a.http, None);
    assert_eq!(a.observation, "Not fetched");
    assert!(a.findings.contains(&Finding::Unchecked));
    let mut output = Vec::new();
    tadpole::discovery::export_csv(&mut output, &snapshot, &[a]).unwrap();
    let records: Vec<_> = csv::Reader::from_reader(output.as_slice())
        .records()
        .map(Result::unwrap)
        .collect();
    assert!(
        records.iter().all(|record| record.get(9) == Some("")),
        "unknown noindex observations must not be exported as false"
    );
    server.abort();
}

#[test]
fn old_projects_do_not_claim_sitemap_coverage_and_reset_clears_new_reports() {
    let state = CrawlState::new();
    let mut project =
        serde_json::to_value(state.get_project("https://example.com/".into())).unwrap();
    project.as_object_mut().unwrap().remove("discovery");
    project["config"]
        .as_object_mut()
        .unwrap()
        .remove("sitemap_urls");
    let legacy: CrawlProject = serde_json::from_value(project).unwrap();
    assert!(legacy.discovery.is_none());
    state.set_discovery(Some(DiscoverySnapshot::new(
        vec!["https://example.com/".into()],
        &CrawlConfig::default(),
    )));
    state.load_project(legacy);
    assert!(state.discovery().is_none());
    state.set_discovery(Some(DiscoverySnapshot::new(
        vec!["https://example.com/".into()],
        &CrawlConfig::default(),
    )));
    state.reset();
    assert!(state.discovery().is_none());
}

#[tokio::test]
async fn source_scope_and_recursive_index_depth_are_bounded_and_reported() {
    let (origin, server) = fixture().await;
    let state = CrawlState::new();
    state.set_config(CrawlConfig {
        sitemap_urls: vec![
            "https://outside.example/sitemap.xml".into(),
            format!("{origin}/depth/0"),
        ],
        ..Default::default()
    });
    let (tx, _) = tokio::sync::mpsc::unbounded_channel();
    Crawler::new(state.clone(), tx).start(origin).await;
    let snapshot = state.discovery().unwrap();
    assert!(snapshot.collection.truncated);
    assert!(snapshot.collection.sources.len() <= sitemap::MAX_SOURCES);
    assert!(
        snapshot
            .collection
            .sources
            .iter()
            .any(|s| s.error.as_ref().is_some_and(|e| e.contains("depth limit")))
    );
    assert!(
        snapshot
            .collection
            .sources
            .iter()
            .any(|s| s.url.contains("outside.example")
                && s.error.as_ref().is_some_and(|e| e.contains("crawl site")))
    );
    assert_eq!(
        snapshot.collection.memberships.len(),
        7,
        "successful sibling sources survive errors and recursion limits"
    );
    server.abort();
}

#[tokio::test]
async fn stopping_during_sitemap_fetch_finishes_without_late_results() {
    let (origin, server) = fixture().await;
    let state = CrawlState::new();
    state.set_config(CrawlConfig {
        sitemap_urls: vec![format!("{origin}/slow.xml")],
        ..Default::default()
    });
    let (tx, _) = tokio::sync::mpsc::unbounded_channel();
    let run = tokio::spawn(Crawler::new(state.clone(), tx).start(origin));
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while state.get_stats().crawled < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    state.request_stop();
    tokio::time::timeout(std::time::Duration::from_millis(600), run)
        .await
        .unwrap()
        .unwrap();
    assert!(!state.is_crawling());
    assert!(state.discovery().unwrap().coverage.stopped);
    let snapshot = serde_json::to_string(&state.discovery()).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_eq!(serde_json::to_string(&state.discovery()).unwrap(), snapshot);
    server.abort();
}

#[test]
fn reopening_an_in_progress_project_marks_partial_evidence_instead_of_running_forever() {
    let state = CrawlState::new();
    state.insert_result(page("https://example.com/", &[], 0));
    state.set_discovery(Some(DiscoverySnapshot::new(
        vec!["https://example.com/".into()],
        &CrawlConfig::default(),
    )));
    let saved = serde_json::to_vec(&state.get_project("https://example.com/".into())).unwrap();
    let restored = CrawlState::new();
    restored.load_project(serde_json::from_slice(&saved).unwrap());
    let snapshot = restored.discovery().unwrap();
    assert!(snapshot.coverage.finished && snapshot.coverage.stopped);
    assert_eq!(snapshot.rows.len(), 1);
    assert_eq!(snapshot.rows[0].reached_depth, Some(0));
    assert!(!snapshot.collection.warnings.is_empty());
}

#[test]
fn empty_successful_urlsets_are_distinct_from_missing_sitemap_evidence() {
    let results = [page("https://example.com/", &[], 0)];
    let mut snapshot =
        DiscoverySnapshot::new(vec![results[0].url.clone()], &CrawlConfig::default());
    reconcile(&mut snapshot, &results);
    assert!(!snapshot.rows[0].findings.contains(&Finding::Absent));
    snapshot
        .collection
        .sources
        .push(tadpole::discovery::SitemapSource {
            url: "https://example.com/empty.xml".into(),
            parent: None,
            status: "URL set read".into(),
            url_count: 0,
            error: None,
            is_index: Some(false),
        });
    reconcile(&mut snapshot, &results);
    assert!(snapshot.rows[0].findings.contains(&Finding::Absent));
}

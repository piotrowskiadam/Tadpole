use serde_json::json;
use tadpole::{
    ai::parse_completion,
    crawler::{CrawlUpdate, Crawler, parse_html},
    filters::{CrawlFilter, DuplicateIndex},
    state::{CrawlConfig, CrawlMode, CrawlResult, CrawlState},
};

fn page(url: &str) -> CrawlResult {
    CrawlResult::new_failed(url.into(), Some(200), None, "Indexable".into(), 0, 0)
}

#[test]
fn secrets_are_never_serialized_deserialized_or_debugged() {
    let config = CrawlConfig {
        ai_api_key: Some("secret-ai".into()),
        ..Default::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains("secret-"));
    assert!(!format!("{config:?}").contains("secret-"));
    let legacy: CrawlConfig =
        serde_json::from_value(json!({"ai_api_key":"legacy-secret"})).unwrap();
    assert!(legacy.ai_api_key.is_none());
    assert_eq!(legacy.max_urls, 500);
}

#[test]
fn state_replacement_and_loading_recompute_stats_and_preserve_session_credentials() {
    let state = CrawlState::new();
    let mut config = state.get_config();
    config.ai_api_key = Some("session-key".into());
    state.set_config(config);
    state.insert_result(page("https://example.com/"));
    let mut replacement = page("https://example.com/");
    replacement.status_code = Some(404);
    state.insert_result(replacement);
    assert_eq!(state.get_stats().crawled, 1);
    assert_eq!(state.get_stats().status_2xx, 0);
    assert_eq!(state.get_stats().status_4xx, 1);
    let mut project = state.get_project("https://example.com/".into());
    project.stats.crawled = 900;
    project.config.ai_api_key = None;
    state.set_crawling(true);
    state.set_paused(true);
    state.load_project(project);
    assert_eq!(state.get_stats().crawled, 1);
    assert_eq!(
        state.get_config().ai_api_key.as_deref(),
        Some("session-key")
    );
    assert!(!state.is_crawling());
    assert!(!state.is_paused());
}

#[test]
fn duplicate_index_matches_reference_and_updates() {
    let mut a = page("https://example.com/a");
    a.title = Some("Title".into());
    a.h1 = Some("Header".into());
    let mut b = a.clone();
    b.url = "https://example.com/b".into();
    let results = [a.clone(), b.clone()];
    let mut index = DuplicateIndex::from_results(&results);
    for filter in [
        CrawlFilter::TitleDuplicate,
        CrawlFilter::DescDuplicate,
        CrawlFilter::H1Duplicate,
        CrawlFilter::TitleMissing,
    ] {
        assert_eq!(index.matches(filter, &a), filter.matches(&a, &results));
    }
    index.remove(&b);
    assert!(!index.matches(CrawlFilter::TitleDuplicate, &a));
}

#[test]
fn canonical_and_visible_text_are_normalized() {
    let data = parse_html(
        "https://example.com/a",
        "<head><link rel='canonical' href='/a#fragment'></head><body><script>one two three</script><style>four five</style><p>Hello world</p><a href='mailto:a@b.com'>Mail</a><a href='/next'>Next</a></body>",
        "example.com",
        &CrawlConfig::default(),
    );
    assert_eq!(data.canonical.as_deref(), Some("https://example.com/a"));
    assert_eq!(data.word_count, 4);
    assert_eq!(data.local_queue, ["https://example.com/next"]);
}

#[test]
fn completion_errors_empty_and_truncated_results_are_visible() {
    assert!(parse_completion(&json!({"error":{"message":"private"}})).is_err());
    assert!(parse_completion(&json!({"choices":[{"message":{"content":" "}}]})).is_err());
    assert!(
        parse_completion(
            &json!({"choices":[{"finish_reason":"length","message":{"content":"partial"}}]})
        )
        .is_err()
    );
    assert_eq!(
        parse_completion(&json!({"choices":[{"message":{"content":" Done "}}]})).unwrap(),
        "Done"
    );
}

#[test]
fn exports_do_not_collide_for_same_slug_or_query_variants() {
    use tadpole::project::markdown_filename;
    assert_ne!(
        markdown_filename("https://example.com/a/item"),
        markdown_filename("https://example.com/b/item")
    );
    assert_ne!(
        markdown_filename("https://example.com/a?x=1"),
        markdown_filename("https://example.com/a?x=2")
    );
}

async fn server() -> (String, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let mut buffer = [0; 2048];
                let n = socket.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = request.split_whitespace().nth(1).unwrap_or("/");
                if path.starts_with("/slow") {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                let body = (0..40)
                    .map(|i| format!("<a href='/slow{i}'>Page {i}</a>"))
                    .collect::<String>();
                let (status, headers) = match path {
                    "/redirect" => ("301 Moved Permanently", "Location: /\r\n"),
                    "/noindex" => ("200 OK", "X-Robots-Tag: noindex\r\n"),
                    _ => ("200 OK", ""),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: text/html\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    (address, handle)
}

#[tokio::test]
async fn crawl_limit_finished_order_and_stop_are_strict() {
    let (address, server) = server().await;
    let state = CrawlState::new();
    state.set_config(CrawlConfig {
        max_urls: 5,
        max_concurrency: 4,
        respect_robots: false,
        ..Default::default()
    });
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        Crawler::new(state.clone(), tx).start(address.clone()),
    )
    .await
    .unwrap();
    assert_eq!(state.get_stats().crawled, 5);
    assert_eq!(state.get_stats().discovered, 5);
    let mut finished = false;
    while let Ok(update) = rx.try_recv() {
        match update {
            CrawlUpdate::Finished => finished = true,
            CrawlUpdate::Crawled(_) => assert!(!finished),
            _ => {}
        }
    }
    assert!(finished);
    state.reset();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let crawl = tokio::spawn(Crawler::new(state.clone(), tx).start(format!("{address}/slow")));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    state.request_stop();
    assert!(
        state.is_crawling(),
        "Stop stays active until workers are drained"
    );
    crawl.await.unwrap();
    assert!(!state.is_crawling());
    while let Ok(update) = rx.try_recv() {
        if let CrawlUpdate::Finished = update {
            break;
        }
    }
    state.reset();
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    assert_eq!(state.get_stats().crawled, 0);
    server.abort();
}

#[tokio::test]
async fn redirects_and_x_robots_tag_are_reported() {
    let (address, server) = server().await;
    for (path, status) in [("/redirect", "Redirect"), ("/noindex", "Noindex")] {
        let state = CrawlState::new();
        state.set_config(CrawlConfig {
            crawl_mode: CrawlMode::Url,
            respect_robots: false,
            follow_redirects: false,
            ..Default::default()
        });
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        Crawler::new(state.clone(), tx)
            .start(format!("{address}{path}"))
            .await;
        assert_eq!(state.get_all_results()[0].indexability_status, status);
        assert!(!state.get_all_results()[0].indexable);
    }
    server.abort();
}

#[test]
fn rank_filters_and_layout_cache_follow_snapshot_contents() {
    use tadpole::visualization::snapshot::{FollowState, Indexability};
    use tadpole::visualization::{
        DomainVisualizationModule, LayoutType, PageRankConfig, SnapshotBuilder,
        calculate_internal_rank,
    };
    let mut a = page("https://example.com/a");
    let mut b = page("https://example.com/b");
    a.outlinks = vec![b.url.clone()];
    b.outlinks = vec![a.url.clone()];
    let mut snapshot = SnapshotBuilder::build_from_results(&[a.clone(), b.clone()]);
    let mut edges = snapshot.edges.to_vec();
    edges[0].follow_state = FollowState::Nofollow;
    snapshot.edges = edges.into();
    let ranks = calculate_internal_rank(
        &snapshot,
        &PageRankConfig {
            exclude_nofollow: true,
            ..Default::default()
        },
    );
    assert!(ranks[0] > ranks[1]);
    let mut nodes = snapshot.nodes.to_vec();
    nodes[1].indexability = Indexability::Noindex;
    snapshot.nodes = nodes.into();
    let ranks = calculate_internal_rank(
        &snapshot,
        &PageRankConfig {
            exclude_non_indexable: true,
            ..Default::default()
        },
    );
    assert_eq!(ranks, [1.0, 0.0]);
    assert!(snapshot.find_shortest_path_to_root(99).is_empty());
    let mut module = DomainVisualizationModule::new();
    module.load_snapshot(snapshot);
    let first = module.get_layout(LayoutType::Hierarchical, 1).unwrap();
    b.depth = 3;
    module.load_snapshot(SnapshotBuilder::build_from_results(&[a, b]));
    let second = module.get_layout(LayoutType::Hierarchical, 1).unwrap();
    assert_ne!(first.positions[1].y, second.positions[1].y);
}

#[test]
fn atomic_project_save_replaces_existing_file_without_secrets() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("test.seocrawl");
    let state = CrawlState::new();
    state.insert_result(page("https://example.com/a"));
    tadpole::project::save(&path, &state.get_project("seed".into())).unwrap();
    state.insert_result(page("https://example.com/b"));
    tadpole::project::save(&path, &state.get_project("seed".into())).unwrap();
    let loaded: tadpole::state::CrawlProject =
        serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    assert_eq!(loaded.results.len(), 2);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

use tadpole::{state::CrawlResult, visualization::*};

fn page(url: &str, links: &[&str]) -> CrawlResult {
    let mut p = CrawlResult::new_failed(url.into(), Some(200), None, "Indexable".into(), 0, 20);
    p.indexable = true;
    p.title = Some("Title".into());
    p.outlinks = links.iter().map(|s| s.to_string()).collect();
    p
}

#[test]
fn result_and_snapshot_directory_metrics_agree() {
    let pages = vec![
        page("https://example.test/blog/guide?q=1", &[]),
        page("https://example.test/blog/guide?q=2", &[]),
    ];
    let snapshot = SnapshotBuilder::build_from_results(&pages);
    let a = build_icicle_tree_from_results(&pages, IcicleWeightMetric::IssueCount);
    let b = build_icicle_tree_from_snapshot(&snapshot, IcicleWeightMetric::IssueCount);
    assert_eq!(a.children[0].name, "blog");
    assert_eq!(a.weight_value, b.weight_value);
    assert_eq!(a.children[0].total_pages, 2);
}

#[test]
fn prepared_views_keep_origins_paths_and_link_meanings_distinct() {
    use tadpole::visualization::explorer::*;
    let mut pages = vec![
        page(
            "https://a.test/",
            &["https://a.test/help/", "https://a.test/help/"],
        ),
        page("https://a.test/help/", &["https://a.test/blog/guide"]),
        page("https://a.test/blog/guide", &[]),
        page("https://b.test/blog/guide", &[]),
    ];
    for p in &mut pages[1..] {
        p.depth = 9;
    }
    let mut engine = ExplorerEngine::new(pages, &|| false).unwrap();
    let request = ViewRequest {
        start_url: "https://a.test/".into(),
        ..Default::default()
    };
    let overview = engine.prepare(&request, &|| false).unwrap();
    assert!(overview.ranks.is_none(), "opening must defer rank");
    assert!(std::sync::Arc::ptr_eq(
        &overview,
        &engine.prepare(&request, &|| false).unwrap()
    ));
    assert_eq!(overview.evidence.len(), 4);
    assert_eq!(engine.index.directories[0].children.len(), 2);
    let guide = engine.index.node("https://a.test/blog/guide").unwrap();
    let other = engine.index.node("https://b.test/blog/guide").unwrap();
    assert_eq!(overview.depths.depth[guide as usize], Some(2));
    assert_eq!(overview.depths.depth[other as usize], None);
    let links = engine
        .prepare(
            &ViewRequest {
                view: View::Links,
                selected_url: "https://a.test/".into(),
                ..request.clone()
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(
        links.evidence.len(),
        1,
        "repeated occurrences are one URL pair"
    );
    let paths = engine
        .prepare(
            &ViewRequest {
                view: View::Paths,
                ..request.clone()
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(paths.evidence.len(), 3);
    assert!(engine.prepare(&request, &|| true).is_none());
    let rank = engine
        .prepare(
            &ViewRequest {
                view: View::Rank,
                ..request
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(
        rank.evidence.len(),
        3,
        "unreachable page is not plotted at depth zero"
    );
    assert!((rank.ranks.as_ref().unwrap().iter().sum::<f32>() - 1.).abs() < 0.001);
    for mark in &rank.marks {
        assert!(
            rank.hit_test(
                mark.bounds.x + mark.bounds.w / 2.,
                mark.bounds.y + mark.bounds.h / 2.
            )
            .is_some()
        );
    }
}

#[test]
fn section_other_preserves_every_unique_pair_and_cell_evidence() {
    use tadpole::visualization::explorer::*;
    let urls: Vec<_> = (0..45)
        .map(|i| format!("https://a.test/section-{i}/page"))
        .collect();
    let pages = urls
        .iter()
        .enumerate()
        .map(|(i, url)| page(url, &[&urls[(i + 1) % 45], &urls[(i + 1) % 45]]))
        .collect();
    let mut e = ExplorerEngine::new(pages, &|| false).unwrap();
    let r = ViewRequest {
        view: View::Matrix,
        width: 640,
        height: 320,
        ..Default::default()
    };
    let f = e.prepare(&r, &|| false).unwrap();
    assert_eq!(
        f.marks
            .iter()
            .map(|m| m.title.parse::<usize>().unwrap())
            .sum::<usize>(),
        45
    );
    let cell = f
        .marks
        .iter()
        .find(|m| matches!(m.target, Target::Cell(usize::MAX, usize::MAX)))
        .unwrap();
    let expected = cell.title.parse::<usize>().unwrap();
    let scoped = e
        .prepare(
            &ViewRequest {
                scope: Some(cell.target),
                ..r
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(scoped.evidence.len(), expected);
}

#[test]
fn session_publishes_only_the_newest_request_and_cancel_discards_results() {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use tadpole::visualization::explorer::*;
    let gate = Arc::new(Barrier::new(2));
    let started = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let g = gate.clone();
    let session = VisualizationSession::new(move || {
        if !started.swap(true, Ordering::SeqCst) {
            tx.send(()).unwrap();
            g.wait();
        }
        vec![
            page("https://a.test/apple", &[]),
            page("https://a.test/banana", &[]),
        ]
    });
    session.request(
        ViewRequest {
            query: "apple".into(),
            ..Default::default()
        },
        false,
    );
    rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    let latest = session.request(
        ViewRequest {
            query: "banana".into(),
            ..Default::default()
        },
        false,
    );
    gate.wait();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(event) = session.poll() {
            let SessionEvent::Ready(ready) = event else {
                panic!("worker failed")
            };
            assert_eq!(ready.revision, latest);
            assert_eq!(ready.frame.evidence.len(), 1);
            assert_eq!(
                ready.index.url(ready.frame.evidence[0].page),
                "https://a.test/banana"
            );
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    session.cancel();
    assert!(session.poll().is_none());
    session.close();
}

#[test]
fn shortest_path_ties_use_the_smallest_parent_and_normalize_start() {
    use tadpole::visualization::explorer::*;
    let pages = vec![
        page("https://a.test/", &["https://a.test/a", "https://a.test/b"]),
        page("https://a.test/a", &["https://a.test/z"]),
        page("https://a.test/b", &["https://a.test/y"]),
        page("https://a.test/y", &["https://a.test/goal"]),
        page("https://a.test/z", &["https://a.test/goal"]),
        page("https://a.test/goal", &[]),
    ];
    let mut e = ExplorerEngine::new(pages, &|| false).unwrap();
    let frame = e
        .prepare(
            &ViewRequest {
                view: View::Paths,
                start_url: "a.test".into(),
                ..Default::default()
            },
            &|| false,
        )
        .unwrap();
    let goal = e.index.node("https://a.test/goal").unwrap();
    assert_eq!(
        e.index.url(frame.depths.parent[goal as usize].unwrap()),
        "https://a.test/y"
    );
}

#[test]
fn exported_neighborhood_does_not_duplicate_self_link_node() {
    let snapshot =
        SnapshotBuilder::build_from_results(&[page("https://a.test/", &["https://a.test/"])]);
    let value: serde_json::Value = serde_json::from_str(
        &tadpole::visualization::export::export_subgraph_json(&snapshot, Some(0)),
    )
    .unwrap();
    assert_eq!(value["nodes"].as_array().unwrap().len(), 1);
}

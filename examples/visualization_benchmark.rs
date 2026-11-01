//! Bounded CPU baseline, not a GUI responsiveness benchmark.
//!
//! Run with: cargo run --release --locked --no-default-features --example visualization_benchmark
//! Fixture generation is O(V * links/page); it deliberately avoids the older
//! synthetic generator's repeated linear reverse-link search.
use std::{hint::black_box, time::Instant};
use tadpole::{
    state::{CrawlResult, CrawlState},
    visualization::{
        builder::SnapshotBuilder,
        icicle::{IcicleWeightMetric, build_icicle_tree_from_snapshot},
        pagerank::{PageRankConfig, calculate_internal_rank},
    },
};

fn fixture(count: usize, wide: bool) -> CrawlState {
    let urls: Vec<_> = (0..count)
        .map(|i| {
            if wide {
                format!("https://example.test/flat/page-{i}")
            } else {
                format!(
                    "https://example.test/section-{}/bucket-{}/page-{i}",
                    i % 10,
                    (i / 10) % 100
                )
            }
        })
        .collect();
    let state = CrawlState::new();
    for (i, url) in urls.iter().enumerate() {
        let mut page = CrawlResult::new_failed(
            url.clone(),
            Some(200),
            None,
            "Indexable".into(),
            1 + i % 6,
            50,
        );
        page.indexable = true;
        page.title = Some(format!("Page {i}"));
        page.h1 = page.title.clone();
        page.h1_count = 1;
        page.word_count = 500;
        page.size_bytes = 12_000;
        page.outlinks = std::iter::once(urls[0].clone())
            .chain((1..8).map(|offset| urls[(i + offset) % count].clone()))
            .collect();
        state.insert_result(page);
    }
    state
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let start = Instant::now();
    let result = black_box(f());
    (result, start.elapsed().as_secs_f64() * 1000.0)
}

fn main() {
    let mut observations = Vec::new();
    for (count, wide) in [
        (1_000, false),
        (10_000, false),
        (100_000, false),
        (10_000, true),
        (25_000, true),
    ] {
        let (state, generation_ms) = timed(|| fixture(count, wide));
        for trial in 1..=3 {
            let (results, clone_ms) = timed(|| state.get_all_results());
            let (snapshot, snapshot_ms) = timed(|| SnapshotBuilder::build_from_results(&results));
            let (ranks, rank_ms) =
                timed(|| calculate_internal_rank(&snapshot, &PageRankConfig::default()));
            let (tree, icicle_ms) =
                timed(|| build_icicle_tree_from_snapshot(&snapshot, IcicleWeightMetric::UrlCount));
            assert_eq!(tree.total_pages, count);
            assert_eq!(ranks.len(), count);
            observations.push(serde_json::json!({
                "shape": if wide { "wide" } else { "bucketed" },
                "pages": count, "edges": snapshot.edges.len(), "trial": trial,
                "fixture_generation_ms": generation_ms,
                "clone_ms": clone_ms, "snapshot_ms": snapshot_ms,
                "rank_ms": rank_ms, "icicle_ms": icicle_ms,
            }));
        }
    }
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": 1,
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "scope": "CPU phases only; no GTK, paint, hit testing, disk IO or content bodies; destruction excluded",
        "observations": observations,
    })).unwrap());
}

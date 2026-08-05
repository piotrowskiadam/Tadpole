#[cfg(test)]
mod tests {
    use tadpole::state::CrawlResult;
    use tadpole::visualization::*;

    #[test]
    fn test_snapshot_building_and_csr() {
        let results = vec![
            CrawlResult {
                url: "https://example.com/".to_string(),
                status_code: Some(200),
                error_message: None,
                indexable: true,
                indexability_status: "Indexable".to_string(),
                title: Some("Home".to_string()),
                meta_desc: None,
                h1: Some("Home".to_string()),
                h1_count: 1,
                h2: None,
                h2_count: 0,
                word_count: 500,
                size_bytes: 12000,
                canonical: Some("https://example.com/".to_string()),
                inlinks: vec![],
                outlinks: vec!["https://example.com/blog/".to_string()],
                images: vec![],
                depth: 0,
                response_time_ms: 50,
                og_title: None,
                og_description: None,
                og_image: None,
                og_url: None,
                og_type: None,
                twitter_title: None,
                twitter_description: None,
                twitter_image: None,
                twitter_card: None,
                schema_json_ld: vec![],
                schema_errors: vec![],
                headings: vec![],
                markdown: None,
            },
            CrawlResult {
                url: "https://example.com/blog/".to_string(),
                status_code: Some(200),
                error_message: None,
                indexable: true,
                indexability_status: "Indexable".to_string(),
                title: Some("Blog".to_string()),
                meta_desc: None,
                h1: Some("Blog".to_string()),
                h1_count: 1,
                h2: None,
                h2_count: 0,
                word_count: 800,
                size_bytes: 18000,
                canonical: Some("https://example.com/blog/".to_string()),
                inlinks: vec!["https://example.com/".to_string()],
                outlinks: vec!["https://example.com/".to_string()],
                images: vec![],
                depth: 1,
                response_time_ms: 70,
                og_title: None,
                og_description: None,
                og_image: None,
                og_url: None,
                og_type: None,
                twitter_title: None,
                twitter_description: None,
                twitter_image: None,
                twitter_card: None,
                schema_json_ld: vec![],
                schema_errors: vec![],
                headings: vec![],
                markdown: None,
            },
        ];

        let snapshot = SnapshotBuilder::build_from_results(&results);

        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(snapshot.edges.len(), 2);

        let home_out = snapshot.get_outgoing_neighbors(0);
        assert_eq!(home_out.len(), 1);
        assert_eq!(home_out[0], 1);

        let blog_in = snapshot.get_incoming_neighbors(1);
        assert_eq!(blog_in.len(), 1);
        assert_eq!(blog_in[0], 0);
    }

    #[test]
    fn test_internal_rank_sum_equals_one() {
        let results = synthetic::generate_synthetic_crawl_results(
            synthetic::SyntheticProfile::BalancedSite,
            42,
        );
        let snapshot = SnapshotBuilder::build_from_results(&results);

        let config = PageRankConfig::default();
        let ranks = calculate_internal_rank(&snapshot, &config);

        assert_eq!(ranks.len(), snapshot.nodes.len());
        let sum: f32 = ranks.iter().sum();
        assert!((sum - 1.0).abs() < 1e-3, "PageRank sum should equal ~1.0, got {}", sum);
    }

    #[test]
    fn test_synthetic_1k_graph_performance() {
        let start = std::time::Instant::now();
        let results = synthetic::generate_synthetic_crawl_results(
            synthetic::SyntheticProfile::BalancedSite,
            123,
        );
        let gen_time = start.elapsed();

        let snap_start = std::time::Instant::now();
        let snapshot = SnapshotBuilder::build_from_results(&results);
        let snap_time = snap_start.elapsed();

        let pr_start = std::time::Instant::now();
        let config = PageRankConfig::default();
        let _ranks = calculate_internal_rank(&snapshot, &config);
        let pr_time = pr_start.elapsed();

        println!(
            "1k Nodes: Gen = {:?}, Snapshot = {:?}, PageRank = {:?}",
            gen_time, snap_time, pr_time
        );

        assert!(snap_time.as_millis() < 500);
        assert!(pr_time.as_millis() < 500);
    }

    #[test]
    fn test_icicle_tree_construction() {
        let results = synthetic::generate_synthetic_crawl_results(
            synthetic::SyntheticProfile::BalancedSite,
            999,
        );

        let root = build_icicle_tree_from_results(&results, IcicleWeightMetric::UrlCount);
        assert_eq!(root.name, "/");
        assert!(root.total_pages > 0);
        assert!(!root.children.is_empty());
    }

    #[test]
    fn test_layout_precomputation() {
        let results = synthetic::generate_synthetic_crawl_results(
            synthetic::SyntheticProfile::BalancedSite,
            777,
        );
        let snapshot = SnapshotBuilder::build_from_results(&results);

        let layout = compute_layout(&snapshot, LayoutType::ForceAtlas2, 42);
        assert_eq!(layout.positions.len(), snapshot.nodes.len());
    }

    #[test]
    fn test_filters_isolate() {
        let results = synthetic::generate_synthetic_crawl_results(
            synthetic::SyntheticProfile::BalancedSite,
            555,
        );
        let snapshot = SnapshotBuilder::build_from_results(&results);

        let filter = GraphFilter {
            mode: FilterMode::Isolate,
            search_query: "blog".to_string(),
            ..Default::default()
        };

        let res = filter.apply(&snapshot);
        assert!(!res.matching_node_ids.is_empty());
    }
}

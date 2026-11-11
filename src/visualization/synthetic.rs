use crate::state::CrawlResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticProfile {
    BalancedSite,         // 1,000 nodes, regular hierarchy
    BlogAndCommerce,      // 5,000 nodes, blog linking to product categories
    OrphanIslands,        // 1,000 nodes with several disconnected clusters
    RedirectProblems,     // 1,000 nodes with 301/302 chains and loops
    BoilerplateExplosion, // 2,000 nodes with heavy global nav link density
    LargeSynthetic,       // 10,000 nodes, 100,000 edges
    VeryLargeSynthetic,   // 100,000 nodes, multi-section enterprise site
}

pub fn generate_synthetic_crawl_results(profile: SyntheticProfile, seed: u64) -> Vec<CrawlResult> {
    let count = match profile {
        SyntheticProfile::BalancedSite => 1000,
        SyntheticProfile::BlogAndCommerce => 5000,
        SyntheticProfile::OrphanIslands => 1000,
        SyntheticProfile::RedirectProblems => 1000,
        SyntheticProfile::BoilerplateExplosion => 2000,
        SyntheticProfile::LargeSynthetic => 10000,
        SyntheticProfile::VeryLargeSynthetic => 100000,
    };

    let domain = "https://example-shop.com";
    let sections = ["blog", "products", "category", "about", "help", "account"];
    let mut results = Vec::with_capacity(count);

    let mut rng_state = seed;
    fn next_rand(state: &mut u64) -> f32 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((*state >> 32) as f32) / 4294967295.0
    }

    // Seed root page
    results.push(CrawlResult {
        url: format!("{}/", domain),
        status_code: Some(200),
        error_message: None,
        indexable: true,
        indexability_status: "Indexable".to_string(),
        title: Some("Example Shop Home Page".to_string()),
        meta_desc: Some("Welcome to Example Shop".to_string()),
        h1: Some("Welcome to Example Shop".to_string()),
        h1_count: 1,
        h2: Some("Featured Products".to_string()),
        h2_count: 4,
        word_count: 650,
        size_bytes: 42100,
        canonical: Some(format!("{}/", domain)),
        inlinks: vec![],
        outlinks: vec![],
        images: vec![],
        link_details: vec![],
        social_metadata: Default::default(),
        depth: 0,
        response_time_ms: 120,
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
    });

    for i in 1..count {
        let sec_idx = (next_rand(&mut rng_state) * sections.len() as f32) as usize;
        let sec = sections[sec_idx.min(sections.len() - 1)];
        let url = format!("{}/{}/page-{}", domain, sec, i);

        let is_error =
            profile == SyntheticProfile::RedirectProblems && next_rand(&mut rng_state) < 0.15;
        let is_redirect =
            profile == SyntheticProfile::RedirectProblems && next_rand(&mut rng_state) < 0.20;

        let status_code = if is_error {
            Some(404)
        } else if is_redirect {
            Some(301)
        } else {
            Some(200)
        };

        let indexable = status_code == Some(200);
        let indexability_status = if is_error {
            "Non-200 Status Code".to_string()
        } else if is_redirect {
            "Redirect".to_string()
        } else {
            "Indexable".to_string()
        };

        let depth = (1.0 + (i as f32).log2().floor()) as usize;

        results.push(CrawlResult {
            url: url.clone(),
            status_code,
            error_message: if is_error {
                Some("404 Not Found".to_string())
            } else {
                None
            },
            indexable,
            indexability_status,
            title: Some(format!("Page {} - {}", i, sec)),
            meta_desc: Some(format!("Description for page {} in section {}", i, sec)),
            h1: Some(format!("Page {} Title", i)),
            h1_count: 1,
            h2: None,
            h2_count: 0,
            word_count: (150.0 + next_rand(&mut rng_state) * 800.0) as usize,
            size_bytes: (5000.0 + next_rand(&mut rng_state) * 35000.0) as usize,
            canonical: Some(url),
            inlinks: vec![],
            outlinks: vec![],
            images: vec![],
            link_details: vec![],
            social_metadata: Default::default(),
            depth,
            response_time_ms: (40.0 + next_rand(&mut rng_state) * 250.0) as u32,
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
        });
    }

    // Connect outlinks & inlinks deterministically
    let root_url = format!("{}/", domain);
    let avg_outlinks = match profile {
        SyntheticProfile::BoilerplateExplosion => 40,
        SyntheticProfile::LargeSynthetic => 12,
        SyntheticProfile::VeryLargeSynthetic => 8,
        _ => 10,
    };

    for i in 0..count {
        let num_links = (avg_outlinks as f32 * (0.5 + next_rand(&mut rng_state))) as usize;
        let mut outlinks = Vec::with_capacity(num_links);

        // Always link to root home page
        outlinks.push(root_url.clone());

        for _ in 0..num_links {
            let target_idx = (next_rand(&mut rng_state) * count as f32) as usize;
            let target_url = results[target_idx.min(count - 1)].url.clone();
            outlinks.push(target_url);
        }

        results[i].outlinks = outlinks;
    }

    // Accumulate inlinks
    for i in 0..count {
        let src_url = results[i].url.clone();
        let out = results[i].outlinks.clone();
        for target in out {
            if let Some(target_res) = results.iter_mut().find(|r| r.url == target)
                && !target_res.inlinks.contains(&src_url)
            {
                target_res.inlinks.push(src_url.clone());
            }
        }
    }

    results
}

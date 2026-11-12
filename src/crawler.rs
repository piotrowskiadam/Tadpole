use crate::state::{
    CrawlConfig, CrawlMode, CrawlResult, CrawlState, HeadingEntry, ImageInfo, LinkInfo,
};
use scraper::{Html, Selector};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use url::Url;

#[derive(Debug)]
pub enum CrawlUpdate {
    Discovered(String),
    Crawled(Box<CrawlResult>),
    Discovery,
    Finished,
}

pub struct Crawler {
    state: CrawlState,
    client: reqwest::Client,
    tx: tokio::sync::mpsc::UnboundedSender<CrawlUpdate>,
    include: Option<regex::Regex>,
    exclude: Option<regex::Regex>,
}

/// Immutable settings shared by workers for one crawl; robots cache is scoped to origins.
struct CrawlRun {
    client: reqwest::Client,
    base_domain: String,
    queue_tx: mpsc::UnboundedSender<(String, usize)>,
    browser: Option<Arc<headless_chrome::Browser>>,
    robots_client: reqwest::Client,
    robots: tokio::sync::Mutex<std::collections::HashMap<String, crate::robots::RobotsRules>>,
    config: CrawlConfig,
    catalogue_url: Option<String>,
    limit_hit: AtomicBool,
    depth_hit: AtomicBool,
    root: String,
}

impl CrawlRun {
    async fn robots_rules(&self, target: &str) -> Result<crate::robots::RobotsRules, String> {
        let target = Url::parse(target).map_err(|_| "Invalid robots origin")?;
        let origin = target.origin().ascii_serialization();
        // Serialize cold-cache fetches so concurrent pages on an origin fetch robots once.
        let mut cache = self.robots.lock().await;
        if let Some(rules) = cache.get(&origin) {
            return Ok(rules.clone());
        }
        let response = self
            .robots_client
            .get(format!("{origin}/robots.txt"))
            .send()
            .await
            .map_err(|_| "Cannot fetch robots.txt. Try again or review robots settings.")?;
        let status = response.status();
        if status.is_server_error() || status.as_u16() == 429 {
            return Err(format!(
                "robots.txt returned HTTP {status}; crawling this origin is suspended."
            ));
        }
        let rules = if status.is_success() {
            let body = crate::http::read_body(response, 512 * 1024).await?;
            crate::robots::parse(&String::from_utf8_lossy(&body), &self.config.user_agent)
        } else {
            crate::robots::RobotsRules::default()
        };
        cache.insert(origin, rules.clone());
        Ok(rules)
    }
}

pub fn clean_seed_url(url: &str) -> String {
    let cleaned = url.trim();
    if !cleaned.contains("://") {
        format!("https://{}", cleaned)
    } else {
        cleaned.to_string()
    }
}

fn get_catalogue_url(seed: &str) -> Option<String> {
    if let Ok(mut parsed) = url::Url::parse(seed) {
        parsed.set_query(None);
        parsed.set_fragment(None);

        let (is_empty_or_root, ends_with_slash) = {
            let path = parsed.path();
            (path == "/" || path.is_empty(), path.ends_with('/'))
        };

        if is_empty_or_root || ends_with_slash {
            return Some(parsed.to_string());
        }

        let non_empty_segments: Vec<String> = parsed
            .path_segments()
            .map(|s| {
                s.filter(|seg| !seg.is_empty())
                    .map(|seg| seg.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if non_empty_segments.is_empty() {
            return Some(parsed.to_string());
        }

        let last_segment = non_empty_segments.last().unwrap();
        let has_dot = last_segment.contains('.');

        if let Ok(mut segs) = parsed.path_segments_mut() {
            if has_dot || non_empty_segments.len() > 1 {
                segs.pop();
                segs.push("");
            } else {
                segs.push("");
            }
        }
        Some(parsed.to_string())
    } else {
        None
    }
}

impl Crawler {
    pub fn new(state: CrawlState, tx: tokio::sync::mpsc::UnboundedSender<CrawlUpdate>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("Tadpole/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();

        let config = state.get_config();
        Self {
            state,
            client,
            tx,
            include: config
                .include_regex
                .as_deref()
                .and_then(|v| regex::Regex::new(v).ok()),
            exclude: config
                .exclude_regex
                .as_deref()
                .and_then(|v| regex::Regex::new(v).ok()),
        }
    }

    pub async fn start(self, seed_url_str: String) {
        let mut seed_urls = vec![];
        for line in seed_url_str.lines() {
            let cleaned = line.trim();
            if cleaned.is_empty() {
                continue;
            }
            let cleaned_with_proto = clean_seed_url(cleaned);
            match Url::parse(&cleaned_with_proto) {
                Ok(mut url)
                    if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() =>
                {
                    url.set_fragment(None);
                    seed_urls.push(url);
                }
                _ => {
                    let err_res = CrawlResult::new_failed(
                        cleaned_with_proto.clone(),
                        None,
                        Some("Enter an HTTP or HTTPS website URL".to_string()),
                        "Failed Connection".to_string(),
                        0,
                        0,
                    );
                    if self.state.add_discovered(&cleaned_with_proto) {
                        self.state.insert_result(err_res.clone());
                        let _ = self.tx.send(CrawlUpdate::Crawled(Box::new(err_res)));
                    }
                }
            }
        }

        if seed_urls.is_empty() {
            self.state.set_crawling(false);
            let _ = self.tx.send(CrawlUpdate::Finished);
            return;
        }

        let config = self.state.get_config();
        if config.validate().is_err() {
            self.state.set_crawling(false);
            let _ = self.tx.send(CrawlUpdate::Finished);
            return;
        }

        let mut catalogue_url = None;
        if config.crawl_mode == CrawlMode::Path
            && let Some(cat) = get_catalogue_url(seed_urls[0].as_str())
            && let Ok(cat_parsed) = Url::parse(&cat)
        {
            catalogue_url = Some(cat.clone());
            if !seed_urls.iter().any(|u| u.as_str() == cat) {
                seed_urls.push(cat_parsed);
            }
        }

        let base_domain = seed_urls[0].host_str().unwrap_or("localhost").to_string();
        let roots: Vec<String> = seed_urls.iter().map(ToString::to_string).collect();
        if config.sitemap_enabled {
            self.state
                .set_discovery(Some(crate::discovery::DiscoverySnapshot::new(
                    roots.clone(),
                    &config,
                )));
            let _ = self.tx.send(CrawlUpdate::Discovery);
        }

        if !self.state.is_crawling() {
            self.state.set_crawling(true);
        }
        self.state.set_paused(false);

        // Queue for URLs and depth to process (managed within async tokio task)
        let (queue_tx, mut queue_rx) = mpsc::unbounded_channel::<(String, usize)>();

        // Seed url discovery
        for url in &seed_urls {
            let url_str = url.as_str().to_string();
            if self.state.add_discovered(&url_str) {
                let _ = queue_tx.send((url_str, 0));
            }
        }

        let max_concurrent_tasks = config.max_concurrency.clamp(1, 50);
        let crawler_arc = Arc::new(self);

        // Re-build reqwest client according to settings
        let mut client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(&config.user_agent);
        if !config.follow_redirects {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
        }
        let client = client_builder.build().unwrap_or(crawler_arc.client.clone());

        // JavaScript rendering browser setup
        let browser = if config.js_rendering {
            tokio::task::spawn_blocking(|| {
                let options = headless_chrome::LaunchOptions::default_builder()
                    .headless(true)
                    .build()
                    .ok()?;
                headless_chrome::Browser::new(options).ok().map(Arc::new)
            })
            .await
            .ok()
            .flatten()
        } else {
            None
        };

        let robots_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(&config.user_agent)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_else(|_| client.clone());
        let run = Arc::new(CrawlRun {
            client,
            base_domain,
            queue_tx,
            browser,
            robots_client,
            robots: tokio::sync::Mutex::new(std::collections::HashMap::new()),
            config: config.clone(),
            catalogue_url,
            limit_hit: AtomicBool::new(false),
            depth_hit: AtomicBool::new(false),
            root: roots[0].clone(),
        });

        let source_run = run.clone();
        let source_state = crawler_arc.state.clone();
        let source_tx = crawler_arc.tx.clone();
        let mut sitemap_work = Box::pin(async move {
            let automatic = matches!(
                source_run.config.crawl_mode,
                CrawlMode::Crawl | CrawlMode::Path
            );
            if !source_run.config.sitemap_enabled
                || (!automatic && source_run.config.sitemap_urls.is_empty())
            {
                return crate::discovery::SitemapCollection::default();
            }
            let rules = source_run.robots_rules(&source_run.root).await;
            crate::sitemap::collect(
                &source_run.root,
                &source_run.config,
                rules,
                &source_state,
                |collection| {
                    source_state
                        .update_discovery(|snapshot| snapshot.collection = collection.clone());
                    let _ = source_tx.send(CrawlUpdate::Discovery);
                },
            )
            .await
        });
        let mut sitemaps_done = false;
        let mut auxiliary_started = false;

        // The coordinator owns all workers. Finished is emitted only after every worker
        // has completed or been cancelled; no detached task can modify a later crawl.
        let mut workers = tokio::task::JoinSet::new();
        loop {
            if crawler_arc.state.stop_requested() || !crawler_arc.state.is_crawling() {
                workers.abort_all();
                while workers.join_next().await.is_some() {}
                break;
            }
            if !crawler_arc.state.is_paused() {
                while workers.len() < max_concurrent_tasks
                    && crawler_arc.state.get_stats().crawled + workers.len() < config.max_urls
                {
                    let Ok((target_url, depth)) = queue_rx.try_recv() else {
                        break;
                    };
                    let crawler = crawler_arc.clone();
                    let run = run.clone();
                    workers.spawn(async move { crawler.crawl_url(&target_url, depth, &run).await });
                }
            }
            if workers.is_empty()
                && (queue_rx.is_empty() || crawler_arc.state.check_limit_reached())
            {
                if sitemaps_done && !auxiliary_started {
                    auxiliary_started = true;
                    if config.sitemap_enabled
                        && matches!(config.crawl_mode, CrawlMode::Crawl | CrawlMode::Path)
                    {
                        let snapshot = crawler_arc.state.discovery();
                        if let Some(snapshot) = snapshot {
                            crawler_arc.state.update_discovery(|s| {
                                s.coverage.phase = "Checking sitemap-only URLs".into()
                            });
                            for url in snapshot.collection.memberships.keys() {
                                if crawler_arc.state.get_result(url).is_none()
                                    && crawler_arc.page_allowed(url, &run)
                                {
                                    if crawler_arc.state.add_discovered(url) {
                                        let _ = run
                                            .queue_tx
                                            .send((url.clone(), crate::discovery::UNKNOWN_DEPTH));
                                    } else if !crawler_arc.state.is_discovered(url) {
                                        run.limit_hit.store(true, Ordering::Relaxed);
                                    }
                                }
                            }
                            let _ = crawler_arc.tx.send(CrawlUpdate::Discovery);
                        }
                    }
                    continue;
                }
                if sitemaps_done && auxiliary_started {
                    break;
                }
            }
            tokio::select! {
                collection = &mut sitemap_work, if !sitemaps_done => {
                    crawler_arc.state.update_discovery(|s| s.collection = collection);
                    sitemaps_done = true;
                    let _ = crawler_arc.tx.send(CrawlUpdate::Discovery);
                }
                result = workers.join_next(), if !workers.is_empty() => {
                    if let Some(Ok(result)) = result {
                        crawler_arc.state.insert_result(result.clone());
                        let _ = crawler_arc.tx.send(CrawlUpdate::Crawled(Box::new(result)));
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(50)) => {}
            }
        }

        if let Some(mut snapshot) = crawler_arc.state.discovery() {
            snapshot.coverage.finished = true;
            snapshot.coverage.stopped = crawler_arc.state.stop_requested();
            snapshot.coverage.url_limit_hit = run.limit_hit.load(Ordering::Relaxed);
            snapshot.coverage.depth_limit_hit = run.depth_hit.load(Ordering::Relaxed);
            snapshot.coverage.finished_at = Some(crate::discovery::timestamp());
            snapshot.coverage.phase = "Finished".into();
            let results = crawler_arc.state.get_all_results();
            if let Ok(mut snapshot) = tokio::task::spawn_blocking(move || {
                crate::discovery::reconcile(&mut snapshot, &results);
                snapshot
            })
            .await
            {
                snapshot.coverage.stopped |= crawler_arc.state.stop_requested();
                crawler_arc.state.apply_discovery_depths(&snapshot.rows);
                crawler_arc.state.set_discovery(Some(snapshot));
            }
        }

        crawler_arc.state.set_crawling(false);
        let _ = crawler_arc.tx.send(CrawlUpdate::Finished);
    }

    fn page_allowed(&self, url: &str, run: &CrawlRun) -> bool {
        crate::discovery::same_site(&run.root, url)
            && (run.config.crawl_mode != CrawlMode::Path
                || run
                    .catalogue_url
                    .as_ref()
                    .is_none_or(|root| url.starts_with(root)))
            && self.include.as_ref().is_none_or(|re| re.is_match(url))
            && self.exclude.as_ref().is_none_or(|re| !re.is_match(url))
    }

    async fn crawl_url(&self, url_str: &str, depth: usize, run: &CrawlRun) -> CrawlResult {
        let client = &run.client;
        let config = &run.config;
        let queue_tx = &run.queue_tx;
        let base_domain = &run.base_domain;
        let state = &self.state;
        let browser = &run.browser;
        let start_time = std::time::Instant::now();
        let _parsed_url = match Url::parse(url_str) {
            Ok(url) => url,
            Err(e) => {
                let elapsed = start_time.elapsed().as_millis() as u32;
                return CrawlResult::new_failed(
                    url_str.to_string(),
                    None,
                    Some(format!("URL Parse Error: {}", e)),
                    "Failed Connection".to_string(),
                    depth,
                    elapsed,
                );
            }
        };

        if config.respect_robots {
            match run.robots_rules(url_str).await {
                Ok(rules) if rules.allows(url_str) => {}
                Ok(_) => {
                    return CrawlResult::new_failed(
                        url_str.into(),
                        None,
                        Some("Blocked by robots.txt".into()),
                        "Blocked by Robots.txt".into(),
                        depth,
                        start_time.elapsed().as_millis() as u32,
                    );
                }
                Err(error) => {
                    return CrawlResult::new_failed(
                        url_str.into(),
                        None,
                        Some(error),
                        "Robots Unavailable".into(),
                        depth,
                        start_time.elapsed().as_millis() as u32,
                    );
                }
            }
        }
        if config.js_rendering && browser.is_none() {
            let message = if std::env::var_os("FLATPAK_ID").is_some() {
                "JavaScript rendering requires Chrome/Chromium inside the Flatpak sandbox. Use a compatible packaged browser or disable rendering in Preferences."
            } else {
                "JavaScript rendering requires an installed Chrome/Chromium. Install it or disable rendering in Preferences."
            };
            return CrawlResult::new_failed(
                url_str.into(),
                None,
                Some(message.into()),
                "Rendering Failed".into(),
                depth,
                0,
            );
        }

        // Fetch page
        let response = match client.get(url_str).send().await {
            Ok(res) => res,
            Err(e) => {
                let elapsed = start_time.elapsed().as_millis() as u32;
                return CrawlResult::new_failed(
                    url_str.to_string(),
                    None,
                    Some(format!("Network Error: {}", e)),
                    "Failed Connection".to_string(),
                    depth,
                    elapsed,
                );
            }
        };

        let status = response.status();
        let status_code = status.as_u16();

        // Handle Redirects manually or check response URL
        let final_url = response.url().to_string();
        let redirected = status.is_redirection() || final_url != url_str;
        let destination = if status.is_redirection() {
            response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .and_then(|value| response.url().join(value).ok())
                .map(|u| u.to_string())
                .unwrap_or_else(|| final_url.clone())
        } else {
            final_url.clone()
        };
        let header_noindex = response
            .headers()
            .get("x-robots-tag")
            .and_then(|h| h.to_str().ok())
            .is_some_and(|v| {
                v.to_ascii_lowercase()
                    .split([',', ' '])
                    .any(|v| v == "noindex" || v == "none")
            });
        let mut observed_noindex = header_noindex;
        state.update_discovery(|s| {
            s.responses.insert(
                url_str.into(),
                crate::discovery::ResponseEvidence {
                    final_url: crate::discovery::normalize_url(&destination),
                    noindex: header_noindex,
                    redirected,
                },
            );
        });
        let mut indexability_status = "Indexable".to_string();
        let mut indexable = true;

        if status.is_redirection() {
            indexability_status = "Redirect".to_string();
            indexable = false;
        }

        // We only parse text/html
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let is_html = content_type.contains("text/html");

        let mut title = None;
        let mut meta_desc = None;
        let mut h1 = None;
        let mut h1_count = 0;
        let mut h2 = None;
        let mut h2_count = 0;
        let mut word_count = 0;
        let mut canonical = None;
        let mut outlinks = vec![];
        let mut link_details = vec![];
        let mut social_metadata = Default::default();
        let mut images = vec![];
        let mut body_bytes = vec![];

        let mut og_title = None;
        let mut og_description = None;
        let mut og_image = None;
        let mut og_url = None;
        let mut og_type = None;
        let mut twitter_title = None;
        let mut twitter_description = None;
        let mut twitter_image = None;
        let mut twitter_card = None;
        let mut schema_json_ld = vec![];
        let mut schema_errors = vec![];
        let mut headings: Vec<HeadingEntry> = vec![];
        let mut markdown_content = None;

        if is_html && (status.is_success() || status_code == 200) {
            let bytes = match crate::http::read_body(response, 8 * 1024 * 1024).await {
                Ok(bytes) => bytes,
                Err(error) => {
                    return CrawlResult::new_failed(
                        url_str.into(),
                        Some(status_code),
                        Some(error),
                        "Body Read Error".into(),
                        depth,
                        start_time.elapsed().as_millis() as u32,
                    );
                }
            };
            {
                body_bytes = bytes;

                // Headless JS rendering check
                let mut html_str = String::from_utf8_lossy(&body_bytes).to_string();
                if let Some(browser) = browser {
                    let browser = browser.clone();
                    let target = url_str.to_string();
                    let rendered =
                        tokio::task::spawn_blocking(move || -> Result<String, String> {
                            let tab = browser.new_tab().map_err(|e| e.to_string())?;
                            let result = (|| {
                                tab.navigate_to(&target).map_err(|e| e.to_string())?;
                                tab.wait_until_navigated().map_err(|e| e.to_string())?;
                                tab.get_content().map_err(|e| e.to_string())
                            })();
                            let _ = tab.close(true);
                            result
                        })
                        .await;
                    match rendered {
                        Ok(Ok(html)) if html.len() <= 8 * 1024 * 1024 => html_str = html,
                        _ => {
                            return CrawlResult::new_failed(
                                url_str.into(),
                                Some(status_code),
                                Some(
                                    "Chrome could not render this page within the content limit."
                                        .into(),
                                ),
                                "Rendering Failed".into(),
                                depth,
                                start_time.elapsed().as_millis() as u32,
                            );
                        }
                    }
                }

                let mut parsed = parse_html(&final_url, &html_str, base_domain, config);
                observed_noindex |= parsed.noindex;

                // Download images locally if configured
                if config.download_images
                    && let Some(ref proj_dir) = config.project_dir
                {
                    for img in &mut parsed.images {
                        if let Some(path) = download_image_to_dir(&img.src, client, proj_dir).await
                        {
                            img.local_path = Some(path);
                        }
                    }
                }

                title = parsed.title;
                meta_desc = parsed.meta_desc;
                canonical = parsed.canonical;
                h1 = parsed.h1;
                h1_count = parsed.h1_count;
                h2 = parsed.h2;
                h2_count = parsed.h2_count;
                word_count = parsed.word_count;
                images = parsed.images;
                outlinks = parsed.outlinks;
                link_details = parsed.link_details;
                social_metadata = parsed.social_metadata;

                og_title = parsed.og_title;
                og_description = parsed.og_description;
                og_image = parsed.og_image;
                og_url = parsed.og_url;
                og_type = parsed.og_type;
                twitter_title = parsed.twitter_title;
                twitter_description = parsed.twitter_description;
                twitter_image = parsed.twitter_image;
                twitter_card = parsed.twitter_card;
                schema_json_ld = parsed.schema_json_ld;
                schema_errors = parsed.schema_errors;
                headings = parsed.headings;

                if parsed.noindex || header_noindex {
                    indexability_status = "Noindex".to_string();
                    indexable = false;
                } else if let Some(ref can_url) = canonical
                    && can_url != &final_url
                {
                    indexability_status = "Canonicalised".to_string();
                    indexable = false;
                }

                // Process discovered URLs if in Crawl or Path Mode
                if config.crawl_mode == CrawlMode::Crawl || config.crawl_mode == CrawlMode::Path {
                    let next_depth = depth.saturating_add(1);
                    let under_depth_limit =
                        config.max_depth.is_none_or(|max_d| next_depth <= max_d);
                    for clean_str in parsed.local_queue {
                        if self.page_allowed(&clean_str, run) {
                            state.add_inlink(&clean_str, url_str);
                            if depth == crate::discovery::UNKNOWN_DEPTH {
                                continue;
                            }
                            if !under_depth_limit {
                                run.depth_hit.store(true, Ordering::Relaxed);
                                continue;
                            }
                            if state.add_discovered(&clean_str) {
                                let _ = self.tx.send(CrawlUpdate::Discovered(clean_str.clone()));
                                let _ = queue_tx.send((clean_str, next_depth));
                            } else if !state.is_discovered(&clean_str) {
                                run.limit_hit.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }

                let md = crate::markdown::html_to_markdown(&html_str, config);
                markdown_content = Some(md);
            }
        } else if !status.is_success() && !status.is_redirection() {
            indexable = false;
            indexability_status = "Non-200 Status Code".to_string();
        } else if !is_html && status.is_success() {
            indexable = false;
            indexability_status = "Non-HTML Resource".to_string();
        }

        state.update_discovery(|s| {
            if let Some(response) = s.responses.get_mut(url_str) {
                response.noindex = observed_noindex;
            }
        });
        if redirected {
            indexable = false;
            indexability_status = "Redirect".into();
        }
        CrawlResult {
            url: url_str.to_string(),
            status_code: Some(status_code),
            error_message: if status.is_success() {
                None
            } else {
                Some(status.to_string())
            },
            indexable,
            indexability_status,
            title,
            meta_desc,
            h1,
            h1_count,
            h2,
            h2_count,
            word_count,
            size_bytes: body_bytes.len(),
            canonical,
            inlinks: vec![], // populated dynamically via state.add_inlink
            outlinks,
            link_details,
            social_metadata,
            images,
            depth,
            response_time_ms: start_time.elapsed().as_millis() as u32,
            og_title,
            og_description,
            og_image,
            og_url,
            og_type,
            twitter_title,
            twitter_description,
            twitter_image,
            twitter_card,
            schema_json_ld,
            schema_errors,
            headings,
            markdown: markdown_content,
        }
    }
}

pub fn get_slug_filename(url_str: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        let path = parsed.path().trim_matches('/');
        if path.is_empty() {
            return "index".to_string();
        }

        if let Some(last_segment) = path.split('/').next_back() {
            let segment_no_ext = if let Some(idx) = last_segment.rfind('.') {
                if idx > 0 {
                    &last_segment[..idx]
                } else {
                    last_segment
                }
            } else {
                last_segment
            };

            if segment_no_ext.is_empty() {
                return "index".to_string();
            }

            let mut slug = String::new();
            for c in segment_no_ext.chars() {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    slug.push(c.to_ascii_lowercase());
                } else {
                    slug.push('-');
                }
            }
            return slug;
        }
    }

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    url_str.hash(&mut hasher);
    format!("page_{}", hasher.finish())
}

async fn download_image_to_dir(
    img_url: &str,
    client: &reqwest::Client,
    project_dir: &str,
) -> Option<String> {
    if let Ok(parsed) = Url::parse(img_url) {
        let mut path_segments = parsed.path_segments()?;
        let last_segment = path_segments.next_back().unwrap_or("image.png");
        let ext = std::path::Path::new(last_segment)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("png");

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        img_url.hash(&mut hasher);
        let hash_val = hasher.finish();
        let filename = format!("{}.{}", hash_val, ext);

        let images_dir = std::path::Path::new(project_dir).join("images");
        if std::fs::create_dir_all(&images_dir).is_ok() {
            let dest_path = images_dir.join(&filename);
            if let Ok(res) = client.get(img_url).send().await
                && res.status().is_success()
                && let Ok(bytes) = crate::http::read_body(res, 20 * 1024 * 1024).await
                && std::fs::write(&dest_path, bytes).is_ok()
            {
                return Some(dest_path.to_string_lossy().to_string());
            }
        }
    }
    None
}

pub struct ParsedData {
    pub title: Option<String>,
    pub meta_desc: Option<String>,
    pub h1: Option<String>,
    pub h1_count: usize,
    pub h2: Option<String>,
    pub h2_count: usize,
    pub word_count: usize,
    pub canonical: Option<String>,
    pub noindex: bool,
    pub outlinks: Vec<String>,
    pub link_details: Vec<LinkInfo>,
    pub social_metadata: std::collections::BTreeMap<String, String>,
    pub images: Vec<ImageInfo>,
    pub local_queue: Vec<String>,

    // Social / Open Graph
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_url: Option<String>,
    pub og_type: Option<String>,
    pub twitter_title: Option<String>,
    pub twitter_description: Option<String>,
    pub twitter_image: Option<String>,
    pub twitter_card: Option<String>,
    pub schema_json_ld: Vec<String>,
    pub schema_errors: Vec<String>,
    pub headings: Vec<HeadingEntry>,
}

pub fn parse_html(
    url_str: &str,
    html_str: &str,
    base_domain: &str,
    config: &CrawlConfig,
) -> ParsedData {
    let parsed_url = Url::parse(url_str).unwrap();
    let document = Html::parse_document(html_str);

    // Title
    let title_sel = Selector::parse("title").unwrap();
    let title = document
        .select(&title_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string());

    // Meta description
    let meta_sel = Selector::parse("meta[name=\"description\" i]").unwrap();
    let meta_desc = document
        .select(&meta_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    // Canonical
    let canonical_sel = Selector::parse("link[rel=\"canonical\"]").unwrap();
    let canonical = document
        .select(&canonical_sel)
        .next()
        .and_then(|el| el.attr("href"))
        .and_then(|s| parsed_url.join(s.trim()).ok())
        .map(|mut u| {
            u.set_fragment(None);
            u.to_string()
        });

    // Check robots meta
    let mut noindex = false;
    let robots_sel = Selector::parse("meta[name=\"robots\" i]").unwrap();
    if let Some(robots_el) = document.select(&robots_sel).next()
        && let Some(content) = robots_el.attr("content")
    {
        let content_lower = content.to_lowercase();
        if content_lower
            .split([',', ' '])
            .any(|v| v == "noindex" || v == "none")
        {
            noindex = true;
        }
    }

    // H1
    let h1_sel = Selector::parse("h1").unwrap();
    let h1_nodes: Vec<_> = document.select(&h1_sel).collect();
    let h1_count = h1_nodes.len();
    let h1 = h1_nodes
        .first()
        .map(|el| el.text().collect::<String>().trim().to_string());

    // H2
    let h2_sel = Selector::parse("h2").unwrap();
    let h2_nodes: Vec<_> = document.select(&h2_sel).collect();
    let h2_count = h2_nodes.len();
    let h2 = h2_nodes
        .first()
        .map(|el| el.text().collect::<String>().trim().to_string());

    // All headings H1–H6 in document order
    let all_headings_sel = Selector::parse("h1, h2, h3, h4, h5, h6").unwrap();
    let headings: Vec<HeadingEntry> = document
        .select(&all_headings_sel)
        .filter_map(|el| {
            let tag = el.value().name();
            let level: u8 = match tag {
                "h1" => 1,
                "h2" => 2,
                "h3" => 3,
                "h4" => 4,
                "h5" => 5,
                "h6" => 6,
                _ => return None,
            };
            let text = el.text().collect::<String>();
            let text = text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            Some(HeadingEntry { level, text })
        })
        .collect();

    // Word count
    let mut word_count = 0;
    let body_sel = Selector::parse("body").unwrap();
    if let Some(body) = document.select(&body_sel).next() {
        for node in body.descendants() {
            if let Some(text) = node.value().as_text() {
                let hidden = node.ancestors().any(|ancestor| {
                    ancestor.value().as_element().is_some_and(|el| {
                        matches!(el.name(), "script" | "style" | "noscript" | "template")
                    })
                });
                if !hidden {
                    word_count += text.split_whitespace().count();
                }
            }
        }
    }

    // Images
    let mut images = vec![];
    let img_sel = Selector::parse("img").unwrap();
    for img_el in document.select(&img_sel) {
        if let Some(src) = img_el.attr("src") {
            // Resolve relative src
            let resolved_src = parsed_url
                .join(src)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| src.to_string());
            images.push(ImageInfo {
                src: resolved_src,
                alt: img_el.attr("alt").map(|s| s.to_string()),
                local_path: None,
                width: img_el
                    .attr("width")
                    .and_then(|v| v.trim().parse().ok())
                    .filter(|v| *v > 0),
                height: img_el
                    .attr("height")
                    .and_then(|v| v.trim().parse().ok())
                    .filter(|v| *v > 0),
                title: img_el.attr("title").map(str::to_string),
                loading: img_el.attr("loading").map(str::to_string),
            });
        }
    }

    // Links
    let mut outlinks = vec![];
    let mut link_details = vec![];
    let mut local_queue = vec![];
    let link_sel = Selector::parse("a[href]").unwrap();
    for link_el in document.select(&link_sel) {
        if let Some(href) = link_el.attr("href")
            && let Ok(resolved_url) = parsed_url.join(href)
        {
            // Strip fragment
            let mut clean_url = resolved_url.clone();
            clean_url.set_fragment(None);
            let clean_str = clean_url.to_string();

            // Image links use their alt text when no visible text is available.
            let mut anchor = link_el
                .text()
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if anchor.is_empty() {
                anchor = link_el
                    .select(&img_sel)
                    .filter_map(|img| img.attr("alt"))
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            link_details.push(LinkInfo {
                target: resolved_url.to_string(),
                anchor,
                rel: link_el
                    .attr("rel")
                    .unwrap_or("")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
            });
            outlinks.push(clean_str.clone());

            // Check if internal domain
            if !matches!(resolved_url.scheme(), "http" | "https") {
                continue;
            }
            if let Some(dom) = resolved_url.host_str()
                && dom == base_domain
            {
                local_queue.push(clean_str);
            }
        }
    }

    let mut social_metadata = std::collections::BTreeMap::new();
    for meta in document.select(&Selector::parse("meta[property], meta[name]").unwrap()) {
        let key = meta
            .attr("property")
            .or_else(|| meta.attr("name"))
            .unwrap_or("")
            .to_ascii_lowercase();
        if (key.starts_with("og:") || key.starts_with("twitter:"))
            && let Some(value) = meta.attr("content")
        {
            social_metadata
                .entry(key)
                .or_insert_with(|| value.trim().to_string());
        }
    }

    // Open Graph
    let og_title_sel = Selector::parse("meta[property=\"og:title\" i]").unwrap();
    let og_title = document
        .select(&og_title_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    let og_description_sel = Selector::parse("meta[property=\"og:description\" i]").unwrap();
    let og_description = document
        .select(&og_description_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    let og_image_sel = Selector::parse("meta[property=\"og:image\" i]").unwrap();
    let og_image = document
        .select(&og_image_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    let og_url_sel = Selector::parse("meta[property=\"og:url\" i]").unwrap();
    let og_url = document
        .select(&og_url_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    let og_type_sel = Selector::parse("meta[property=\"og:type\" i]").unwrap();
    let og_type = document
        .select(&og_type_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    // Twitter Cards
    let twitter_title_sel = Selector::parse("meta[name=\"twitter:title\" i]").unwrap();
    let twitter_title = document
        .select(&twitter_title_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    let twitter_description_sel = Selector::parse("meta[name=\"twitter:description\" i]").unwrap();
    let twitter_description = document
        .select(&twitter_description_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    let twitter_image_sel = Selector::parse("meta[name=\"twitter:image\" i]").unwrap();
    let twitter_image = document
        .select(&twitter_image_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    let twitter_card_sel = Selector::parse("meta[name=\"twitter:card\" i]").unwrap();
    let twitter_card = document
        .select(&twitter_card_sel)
        .next()
        .and_then(|el| el.attr("content"))
        .map(|s| s.trim().to_string());

    // JSON-LD Structured Data
    let schema_sel = Selector::parse("script[type=\"application/ld+json\"]").unwrap();
    let mut schema_json_ld = vec![];
    let mut schema_errors = vec![];

    for el in document.select(&schema_sel) {
        let text = el.text().collect::<String>();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        schema_json_ld.push(trimmed.to_string());

        // Validate JSON-LD
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(val) => {
                let schemas = if let Some(arr) = val.as_array() {
                    arr.clone()
                } else if val.is_object() {
                    vec![val]
                } else {
                    schema_errors
                        .push("JSON-LD root must be an Object or Array of Objects.".to_string());
                    continue;
                };

                for schema in schemas {
                    if let Some(obj) = schema.as_object() {
                        let context = obj.get("@context").and_then(|v| v.as_str());
                        let has_type = obj.contains_key("@type");
                        // JSON-LD @graph pattern: root has @context + @graph array,
                        // individual types are declared on the nodes inside @graph.
                        // In this case @type at root level is NOT required.
                        let has_graph = obj.contains_key("@graph");

                        if context.is_none() {
                            schema_errors.push("Missing '@context' property.".to_string());
                        } else if let Some(ctx) = context
                            && !ctx.contains("schema.org")
                        {
                            schema_errors.push(format!(
                                "Non-standard @context: '{}'. Expected schema.org.",
                                ctx
                            ));
                        }

                        // Only require @type when this is NOT an @graph container block.
                        if !has_type && !has_graph {
                            schema_errors.push("Missing '@type' property.".to_string());
                        }
                    } else {
                        schema_errors.push("Schema block in array is not an object.".to_string());
                    }
                }
            }
            Err(e) => {
                schema_errors.push(format!("JSON parsing error: {}", e));
            }
        }
    }

    // Extract Microdata if configured
    if config.extract_microdata {
        let microdata_schemas = parse_microdata_items(&document);
        for m_schema in microdata_schemas {
            schema_json_ld.push(m_schema);
        }
    }

    ParsedData {
        title,
        meta_desc,
        h1,
        h1_count,
        h2,
        h2_count,
        word_count,
        canonical,
        noindex,
        outlinks,
        link_details,
        social_metadata,
        images,
        local_queue,
        og_title,
        og_description,
        og_image,
        og_url,
        og_type,
        twitter_title,
        twitter_description,
        twitter_image,
        twitter_card,
        schema_json_ld,
        schema_errors,
        headings,
    }
}

fn parse_microdata_items(document: &Html) -> Vec<String> {
    let mut items = vec![];
    let itemscope_sel = match Selector::parse("[itemscope]") {
        Ok(sel) => sel,
        Err(_) => return items,
    };

    for el in document.select(&itemscope_sel) {
        let is_top_level = el
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .all(|anc| anc.value().attr("itemscope").is_none());

        if is_top_level {
            let val = parse_itemscope(el);
            if !val.is_null()
                && let Ok(json_str) = serde_json::to_string_pretty(&val)
            {
                items.push(json_str);
            }
        }
    }

    items
}

fn parse_itemscope(element: scraper::ElementRef) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    map.insert(
        "@context".to_string(),
        serde_json::Value::String("https://schema.org".to_string()),
    );

    if let Some(itemtype) = element.value().attr("itemtype") {
        let clean_type = itemtype
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("schema.org/");
        map.insert(
            "@type".to_string(),
            serde_json::Value::String(clean_type.to_string()),
        );
    } else {
        map.insert(
            "@type".to_string(),
            serde_json::Value::String("Thing".to_string()),
        );
    }

    parse_properties(element, &mut map);

    serde_json::Value::Object(map)
}

fn parse_properties(
    element: scraper::ElementRef,
    map: &mut serde_json::Map<String, serde_json::Value>,
) {
    for child in element.children() {
        if let Some(child_el) = scraper::ElementRef::wrap(child) {
            if let Some(itemprop_attr) = child_el.value().attr("itemprop") {
                let prop_name = itemprop_attr.to_string();

                let val = if child_el.value().attr("itemscope").is_some() {
                    parse_itemscope(child_el)
                } else {
                    let text_val = get_microdata_value(child_el);
                    serde_json::Value::String(text_val)
                };

                match map.get_mut(&prop_name) {
                    Some(serde_json::Value::Array(arr)) => {
                        arr.push(val);
                    }
                    Some(existing_val) => {
                        let taken = existing_val.take();
                        map.insert(prop_name, serde_json::Value::Array(vec![taken, val]));
                    }
                    None => {
                        map.insert(prop_name, val);
                    }
                }

                if child_el.value().attr("itemscope").is_none() {
                    parse_properties(child_el, map);
                }
            } else {
                if child_el.value().attr("itemscope").is_none() {
                    parse_properties(child_el, map);
                }
            }
        }
    }
}

fn get_microdata_value(element: scraper::ElementRef) -> String {
    let tag = element.value().name();
    match tag {
        "meta" => element.value().attr("content").unwrap_or("").to_string(),
        "audio" | "embed" | "iframe" | "img" | "source" | "track" | "video" => {
            element.value().attr("src").unwrap_or("").to_string()
        }
        "a" | "area" | "link" => element.value().attr("href").unwrap_or("").to_string(),
        "object" => element.value().attr("data").unwrap_or("").to_string(),
        "data" | "meter" => element.value().attr("value").unwrap_or("").to_string(),
        "time" => element.value().attr("datetime").unwrap_or("").to_string(),
        _ => element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html_basic() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Test Page Title</title>
                <meta name="description" content="This is a test meta description for SEO auditing.">
                <link rel="canonical" href="https://example.com/canonical-url">
                <meta name="robots" content="noindex, nofollow">
            </head>
            <body>
                <h1>Main Heading</h1>
                <h1>Another Heading 1</h1>
                <h2>Sub Heading</h2>
                <p>Hello world. This is a simple word count test with some words.</p>
                <img src="/images/logo.png" alt="Company Logo">
                <img src="https://external.com/photo.jpg">
                <a href="/about">About Us</a>
                <a href="https://external.com/contact">External Link</a>
            </body>
            </html>
        "#;

        let parsed = parse_html(
            "https://example.com/page",
            html,
            "example.com",
            &CrawlConfig::default(),
        );

        assert_eq!(parsed.title, Some("Test Page Title".to_string()));
        assert_eq!(
            parsed.meta_desc,
            Some("This is a test meta description for SEO auditing.".to_string())
        );
        assert_eq!(
            parsed.canonical,
            Some("https://example.com/canonical-url".to_string())
        );
        assert!(parsed.noindex);
        assert_eq!(parsed.h1, Some("Main Heading".to_string()));
        assert_eq!(parsed.h1_count, 2);
        assert_eq!(parsed.h2, Some("Sub Heading".to_string()));
        assert_eq!(parsed.h2_count, 1);

        // Assert links
        assert!(
            parsed
                .outlinks
                .contains(&"https://example.com/about".to_string())
        );
        assert!(
            parsed
                .outlinks
                .contains(&"https://external.com/contact".to_string())
        );

        // Assert local queue
        assert!(
            parsed
                .local_queue
                .contains(&"https://example.com/about".to_string())
        );
        assert!(
            !parsed
                .local_queue
                .contains(&"https://external.com/contact".to_string())
        );

        // Assert images
        assert_eq!(parsed.images.len(), 2);
        assert_eq!(parsed.images[0].src, "https://example.com/images/logo.png");
        assert_eq!(parsed.images[0].alt, Some("Company Logo".to_string()));
        assert_eq!(parsed.images[1].src, "https://external.com/photo.jpg");
        assert_eq!(parsed.images[1].alt, None);
    }

    #[test]
    fn test_clean_seed_url() {
        assert_eq!(clean_seed_url("google.com"), "https://google.com");
        assert_eq!(clean_seed_url("http://example.com"), "http://example.com");
        assert_eq!(clean_seed_url("https://example.com"), "https://example.com");
        assert_eq!(
            clean_seed_url("   example.com/subpage   "),
            "https://example.com/subpage"
        );
    }

    #[test]
    fn test_robots_txt_parsing() {
        let robots = r#"
            User-agent: *
            Disallow: /admin
            Disallow: /private/

            User-agent: Googlebot
            Disallow: /no-google/
        "#;
        let rules = crate::robots::parse(robots, "Tadpole");
        assert!(!rules.allows("https://example.com/admin/settings"));
        assert!(!rules.allows("https://example.com/private/index.html"));
        assert!(rules.allows("https://example.com/no-google/"));
        assert!(rules.allows("https://example.com/blog"));
    }

    #[test]
    fn test_parse_html_social() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Test Page Title</title>
                <meta property="og:title" content="Facebook OG Title">
                <meta property="og:description" content="This is an OG description">
                <meta property="og:image" content="https://example.com/share.jpg">
                <meta property="og:url" content="https://example.com/page">
                <meta property="og:type" content="article">
                <meta name="twitter:title" content="Twitter Card Title">
                <meta name="twitter:description" content="This is a Twitter description">
                <meta name="twitter:image" content="https://example.com/twitter-share.jpg">
                <meta name="twitter:card" content="summary_large_image">
            </head>
            <body>
                <p>Hello world.</p>
            </body>
            </html>
        "#;

        let parsed = parse_html(
            "https://example.com/page",
            html,
            "example.com",
            &CrawlConfig::default(),
        );

        assert_eq!(parsed.og_title, Some("Facebook OG Title".to_string()));
        assert_eq!(
            parsed.og_description,
            Some("This is an OG description".to_string())
        );
        assert_eq!(
            parsed.og_image,
            Some("https://example.com/share.jpg".to_string())
        );
        assert_eq!(parsed.og_url, Some("https://example.com/page".to_string()));
        assert_eq!(parsed.og_type, Some("article".to_string()));
        assert_eq!(parsed.twitter_title, Some("Twitter Card Title".to_string()));
        assert_eq!(
            parsed.twitter_description,
            Some("This is a Twitter description".to_string())
        );
        assert_eq!(
            parsed.twitter_image,
            Some("https://example.com/twitter-share.jpg".to_string())
        );
        assert_eq!(parsed.twitter_card, Some("summary_large_image".to_string()));
    }

    #[test]
    fn test_crawl_filter_social() {
        use crate::filters::CrawlFilter;

        let res_no_social = CrawlResult::new_failed(
            "https://example.com".to_string(),
            Some(200),
            None,
            "Indexable".to_string(),
            0,
            100,
        );

        let mut res_with_social = res_no_social.clone();
        res_with_social.og_title = Some("Title".to_string());
        res_with_social.og_description = Some("Desc".to_string());

        assert!(CrawlFilter::OgTitleMissing.matches(&res_no_social, &[]));
        assert!(CrawlFilter::OgDescMissing.matches(&res_no_social, &[]));

        assert!(!CrawlFilter::OgTitleMissing.matches(&res_with_social, &[]));
        assert!(!CrawlFilter::OgDescMissing.matches(&res_with_social, &[]));
    }

    #[test]
    fn test_parse_html_schema() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Test Page Title</title>
                <script type="application/ld+json">
                {
                    "@context": "https://schema.org",
                    "@type": "Organization",
                    "name": "Local SEO Crawler Corp"
                }
                </script>
                <script type="application/ld+json">
                {
                    "@type": "Product"
                }
                </script>
                <script type="application/ld+json">
                {
                    "invalid_json":
                }
                </script>
            </head>
            <body>
                <p>Hello world.</p>
            </body>
            </html>
        "#;

        let parsed = parse_html(
            "https://example.com/page",
            html,
            "example.com",
            &CrawlConfig::default(),
        );

        assert_eq!(parsed.schema_json_ld.len(), 3);
        assert!(
            parsed
                .schema_errors
                .iter()
                .any(|e| e.contains("Missing '@context' property"))
        );
        assert!(
            parsed
                .schema_errors
                .iter()
                .any(|e| e.contains("JSON parsing error"))
        );
    }

    #[test]
    fn test_crawl_filter_schema() {
        use crate::filters::CrawlFilter;

        let mut res_no_schema = CrawlResult::new_failed(
            "https://example.com".to_string(),
            Some(200),
            None,
            "Indexable".to_string(),
            0,
            100,
        );

        assert!(!CrawlFilter::SchemaPresent.matches(&res_no_schema, &[]));
        assert!(!CrawlFilter::SchemaErrors.matches(&res_no_schema, &[]));

        res_no_schema.schema_json_ld = vec!["{}".to_string()];
        assert!(CrawlFilter::SchemaPresent.matches(&res_no_schema, &[]));
        assert!(!CrawlFilter::SchemaErrors.matches(&res_no_schema, &[]));

        res_no_schema.schema_errors = vec!["Some error".to_string()];
        assert!(CrawlFilter::SchemaErrors.matches(&res_no_schema, &[]));
    }

    #[test]
    fn test_get_catalogue_url() {
        assert_eq!(
            get_catalogue_url("https://example.com/foo/bar"),
            Some("https://example.com/foo/".to_string())
        );
        assert_eq!(
            get_catalogue_url("https://example.com/foo/bar/"),
            Some("https://example.com/foo/bar/".to_string())
        );
        assert_eq!(
            get_catalogue_url("https://example.com"),
            Some("https://example.com/".to_string())
        );
        assert_eq!(
            get_catalogue_url("https://example.com/foo"),
            Some("https://example.com/foo/".to_string())
        );
        assert_eq!(
            get_catalogue_url("https://example.com/foo?param=1#frag"),
            Some("https://example.com/foo/".to_string())
        );
        assert_eq!(
            get_catalogue_url("https://example.com/index.html"),
            Some("https://example.com/".to_string())
        );
        assert_eq!(
            get_catalogue_url("https://example.com/foo/index.html"),
            Some("https://example.com/foo/".to_string())
        );
    }

    #[test]
    fn test_state_inlinks_accumulation() {
        let state = crate::state::CrawlState::new();

        // 1. Add inlink for a page that has not been crawled/inserted yet
        state.add_inlink("https://example.com/target", "https://example.com/source-1");

        // 2. Insert the result for the target page
        let target_res = crate::state::CrawlResult::new_failed(
            "https://example.com/target".to_string(),
            Some(200),
            None,
            "Indexable".to_string(),
            0,
            100,
        );
        state.insert_result(target_res);

        // 3. Verify inlinks were merged during insertion
        let res_1 = state.get_result("https://example.com/target").unwrap();
        assert!(
            res_1
                .inlinks
                .contains(&"https://example.com/source-1".to_string())
        );

        // 4. Add another inlink now that the page exists in the state
        state.add_inlink("https://example.com/target", "https://example.com/source-2");

        // 5. Verify it was appended immediately
        let res_2 = state.get_result("https://example.com/target").unwrap();
        assert!(
            res_2
                .inlinks
                .contains(&"https://example.com/source-1".to_string())
        );
        assert!(
            res_2
                .inlinks
                .contains(&"https://example.com/source-2".to_string())
        );
    }

    #[test]
    fn test_parse_microdata_basic() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <div itemscope itemtype="https://schema.org/Product">
                    <span itemprop="name">Dyson V8 Absolute</span>
                    <div itemprop="offers" itemscope itemtype="https://schema.org/Offer">
                        <span itemprop="price">299.99</span>
                        <meta itemprop="priceCurrency" content="USD" />
                    </div>
                </div>
            </body>
            </html>
        "#;

        let config = CrawlConfig {
            extract_microdata: true,
            ..Default::default()
        };

        let parsed = parse_html("https://example.com/page", html, "example.com", &config);

        assert_eq!(parsed.schema_json_ld.len(), 1);
        let parsed_json: serde_json::Value =
            serde_json::from_str(&parsed.schema_json_ld[0]).unwrap();
        assert_eq!(parsed_json["@type"], "Product");
        assert_eq!(parsed_json["name"], "Dyson V8 Absolute");
        assert_eq!(parsed_json["offers"]["@type"], "Offer");
        assert_eq!(parsed_json["offers"]["price"], "299.99");
        assert_eq!(parsed_json["offers"]["priceCurrency"], "USD");
    }
}

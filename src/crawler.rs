use crate::state::{CrawlResult, CrawlState, ImageInfo, HeadingEntry, CrawlConfig, CrawlMode};
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use url::Url;

#[derive(Debug)]
pub enum CrawlUpdate {
    Discovered(String),
    Crawled(CrawlResult),
    Finished,
}

pub struct Crawler {
    state: CrawlState,
    client: reqwest::Client,
    tx: tokio::sync::mpsc::UnboundedSender<CrawlUpdate>,
}

pub fn clean_seed_url(url: &str) -> String {
    let cleaned = url.trim();
    if !cleaned.starts_with("http://") && !cleaned.starts_with("https://") {
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

        let non_empty_segments: Vec<String> = parsed.path_segments()
            .map(|s| s.filter(|seg| !seg.is_empty()).map(|seg| seg.to_string()).collect::<Vec<_>>())
            .unwrap_or_default();

        if non_empty_segments.is_empty() {
            return Some(parsed.to_string());
        }

        let last_segment = non_empty_segments.last().unwrap();
        let has_dot = last_segment.contains('.');

        if let Ok(mut segs) = parsed.path_segments_mut() {
            if has_dot {
                segs.pop();
                segs.push("");
            } else if non_empty_segments.len() > 1 {
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
            .user_agent("Tadpole/0.1 (Desktop SEO Auditor)")
            .build()
            .unwrap_or_default();

        Self { state, client, tx }
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
                Ok(url) => {
                    seed_urls.push(url);
                }
                Err(e) => {
                    let err_res = CrawlResult::new_failed(
                        cleaned_with_proto.clone(),
                        None,
                        Some(format!("Invalid URL: {}", e)),
                        "Failed Connection".to_string(),
                        0,
                        0,
                    );
                    let _ = self.tx.send(CrawlUpdate::Crawled(err_res));
                }
            }
        }

        if seed_urls.is_empty() {
            let _ = self.tx.send(CrawlUpdate::Finished);
            return;
        }

        let config = self.state.get_config();

        let mut catalogue_url = None;
        if config.crawl_mode == CrawlMode::Path {
            if let Some(cat) = get_catalogue_url(seed_urls[0].as_str()) {
                if let Ok(cat_parsed) = Url::parse(&cat) {
                    catalogue_url = Some(cat.clone());
                    if !seed_urls.iter().any(|u| u.as_str() == cat) {
                        seed_urls.push(cat_parsed);
                    }
                }
            }
        }

        let base_domain = seed_urls[0].domain().unwrap_or("localhost").to_string();

        self.state.set_crawling(true);
        self.state.set_paused(false);

        // Queue for URLs and depth to process (managed within async tokio task)
        let (queue_tx, mut queue_rx) = mpsc::channel::<(String, usize)>(10000);
        let queue_tx = Arc::new(queue_tx);

        // Seed url discovery
        for url in &seed_urls {
            let url_str = url.as_str().to_string();
            self.state.add_discovered(&url_str);
            let _ = self.tx.send(CrawlUpdate::Discovered(url_str.clone()));
            let _ = queue_tx.send((url_str, 0)).await;
        }

        let active_tasks = Arc::new(tokio::sync::Mutex::new(0));
        let max_concurrent_tasks = config.max_concurrency;
        let crawler_arc = Arc::new(self);
        let base_domain = Arc::new(base_domain);

        // Re-build reqwest client according to settings
        let mut client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(&config.user_agent);
        if !config.follow_redirects {
            client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
        }
        let client = client_builder.build().unwrap_or(crawler_arc.client.clone());

        // Respect robots.txt setup
        let mut disallowed_paths = vec![];
        if config.respect_robots {
            if let Ok(robots_url) = seed_urls[0].join("/robots.txt") {
                if let Ok(res) = client.get(robots_url.as_str()).send().await {
                    if res.status().is_success() {
                        if let Ok(text) = res.text().await {
                            disallowed_paths = parse_robots_txt(&text, &config.user_agent);
                        }
                    }
                }
            }
        }
        let disallowed_paths = Arc::new(disallowed_paths);

        // JavaScript rendering browser setup
        let browser = if config.js_rendering {
            let options = headless_chrome::LaunchOptions::default_builder()
                .headless(true)
                .path(Some(std::path::PathBuf::from("/usr/bin/google-chrome")))
                .build()
                .expect("Failed to build LaunchOptions");
            match headless_chrome::Browser::new(options) {
                Ok(b) => Some(Arc::new(b)),
                Err(e) => {
                    eprintln!("Failed to launch headless chrome: {}", e);
                    None
                }
            }
        } else {
            None
        };

        loop {
            // Check if crawling was turned off from the main UI
            if !crawler_arc.state.is_crawling() {
                break;
            }

            // Handle pause state
            if crawler_arc.state.is_paused() {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }

            // Check limit
            if crawler_arc.state.check_limit_reached() {
                break;
            }

            let active = *active_tasks.lock().await;

            if active >= max_concurrent_tasks {
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }

            tokio::select! {
                maybe_task = queue_rx.recv() => {
                    match maybe_task {
                        Some((target_url, depth)) => {
                            let mut count = active_tasks.lock().await;
                            *count += 1;

                            let c_state = crawler_arc.state.clone();
                            let c_client = client.clone();
                            let c_tx = crawler_arc.tx.clone();
                            let c_queue_tx = queue_tx.clone();
                            let c_base_domain = base_domain.clone();
                            let c_active_tasks = active_tasks.clone();
                            let c_crawler = crawler_arc.clone();
                            let c_browser = browser.clone();
                            let c_disallowed = disallowed_paths.clone();
                            let c_config = config.clone();
                            let c_catalogue_url = catalogue_url.clone();

                            tokio::spawn(async move {
                                let result = c_crawler.crawl_url(
                                    &target_url, 
                                    depth,
                                    &c_client, 
                                    &c_base_domain, 
                                    &c_queue_tx, 
                                    &c_state,
                                    c_browser,
                                    &c_disallowed,
                                    &c_config,
                                    c_catalogue_url.as_deref(),
                                ).await;
                                c_state.insert_result(result.clone());
                                let _ = c_tx.send(CrawlUpdate::Crawled(result));

                                let mut count = c_active_tasks.lock().await;
                                *count -= 1;
                            });
                        }
                        None => {
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Timeout check - if active tasks is 0, then we are done
                    if active == 0 {
                        break;
                    }
                }
            }
        }

        crawler_arc.state.set_crawling(false);
        let _ = crawler_arc.tx.send(CrawlUpdate::Finished);
    }

    async fn crawl_url(
        &self,
        url_str: &str,
        depth: usize,
        client: &reqwest::Client,
        base_domain: &str,
        queue_tx: &mpsc::Sender<(String, usize)>,
        state: &CrawlState,
        browser: Option<Arc<headless_chrome::Browser>>,
        disallowed: &[String],
        config: &CrawlConfig,
        catalogue_url: Option<&str>,
    ) -> CrawlResult {
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

        // Respect robots.txt
        if is_disallowed(url_str, disallowed) {
            let elapsed = start_time.elapsed().as_millis() as u32;
            return CrawlResult::new_failed(
                url_str.to_string(),
                None,
                Some("Blocked by robots.txt".to_string()),
                "Blocked by Robots.txt".to_string(),
                depth,
                elapsed,
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
        let _final_url = response.url().to_string();
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

        if is_html && (status.is_success() || status_code == 200) {
            if let Ok(bytes) = response.bytes().await {
                body_bytes = bytes.to_vec();
                
                // Headless JS rendering check
                let mut html_str = String::from_utf8_lossy(&body_bytes).to_string();
                if let Some(ref b) = browser {
                    if let Ok(tab) = b.new_tab() {
                        if tab.navigate_to(url_str).is_ok() {
                            if tab.wait_until_navigated().is_ok() {
                                tokio::time::sleep(Duration::from_millis(800)).await;
                                if let Ok(rendered) = tab.get_content() {
                                    html_str = rendered;
                                }
                            }
                        }
                        let _ = tab.close(true);
                    }
                }

                let mut parsed = parse_html(url_str, &html_str, base_domain);

                // Download images locally if configured
                if config.download_images {
                    if let Some(ref proj_dir) = config.project_dir {
                        for img in &mut parsed.images {
                            if let Some(path) = download_image_to_dir(&img.src, client, proj_dir).await {
                                img.local_path = Some(path);
                            }
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

                if parsed.noindex {
                    indexability_status = "Noindex".to_string();
                    indexable = false;
                } else if let Some(ref can_url) = canonical {
                    if can_url != url_str {
                        indexability_status = "Canonicalised".to_string();
                        indexable = false;
                    }
                }

                // Process discovered URLs if in Crawl or Path Mode
                if config.crawl_mode == CrawlMode::Crawl || config.crawl_mode == CrawlMode::Path {
                    let next_depth = depth + 1;
                    let under_depth_limit = config.max_depth.map_or(true, |max_d| next_depth <= max_d);

                    if under_depth_limit {
                        for clean_str in parsed.local_queue {
                            let mut is_allowed = true;

                            // If Path mode, the URL must belong to the catalogue
                            if config.crawl_mode == CrawlMode::Path {
                                if let Some(cat_url) = catalogue_url {
                                    if !clean_str.starts_with(cat_url) {
                                        is_allowed = false;
                                    }
                                }
                            }

                            if is_allowed {
                                if let Some(ref incl) = config.include_regex {
                                    if let Ok(re) = regex::Regex::new(incl) {
                                        is_allowed = re.is_match(&clean_str);
                                    }
                                }
                                if is_allowed {
                                    if let Some(ref excl) = config.exclude_regex {
                                        if let Ok(re) = regex::Regex::new(excl) {
                                            if re.is_match(&clean_str) {
                                                is_allowed = false;
                                            }
                                        }
                                    }
                                }

                                if is_allowed {
                                    if state.add_discovered(&clean_str) {
                                        let _ = self.tx.send(CrawlUpdate::Discovered(clean_str.clone()));
                                        let _ = queue_tx.send((clean_str.clone(), next_depth)).await;
                                    }
                                    state.add_inlink(&clean_str, url_str);
                                }
                            }
                        }
                    }
                }
            }
        } else if !status.is_success() {
            indexable = false;
            indexability_status = "Non-200 Status Code".to_string();
        }

        CrawlResult {
            url: url_str.to_string(),
            status_code: Some(status_code),
            error_message: if status.is_success() { None } else { Some(status.to_string()) },
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
        }
    }
}

fn parse_robots_txt(content: &str, user_agent: &str) -> Vec<String> {
    let mut disallowed = vec![];
    let mut is_applicable = false;
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() < 2 {
            continue;
        }
        
        let directive = parts[0].trim().to_lowercase();
        let value = parts[1].trim();
        
        if directive == "user-agent" {
            let ua = value.to_lowercase();
            is_applicable = ua == "*" || ua == user_agent.to_lowercase() || user_agent.to_lowercase().contains(&ua);
        } else if directive == "disallow" && is_applicable {
            if !value.is_empty() {
                disallowed.push(value.to_string());
            }
        }
    }
    disallowed
}

fn is_disallowed(url_str: &str, disallowed: &[String]) -> bool {
    if let Ok(parsed) = Url::parse(url_str) {
        let path = parsed.path();
        for dis in disallowed {
            if path.starts_with(dis) {
                return true;
            }
        }
    }
    false
}

async fn download_image_to_dir(img_url: &str, client: &reqwest::Client, project_dir: &str) -> Option<String> {
    if let Ok(parsed) = Url::parse(img_url) {
        let path_segments = parsed.path_segments()?;
        let last_segment = path_segments.last().unwrap_or("image.png");
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
            if let Ok(res) = client.get(img_url).send().await {
                if res.status().is_success() {
                    if let Ok(bytes) = res.bytes().await {
                        if std::fs::write(&dest_path, bytes).is_ok() {
                            return Some(dest_path.to_string_lossy().to_string());
                        }
                    }
                }
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

pub fn parse_html(url_str: &str, html_str: &str, base_domain: &str) -> ParsedData {
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
        .map(|s| s.trim().to_string());

    // Check robots meta
    let mut noindex = false;
    let robots_sel = Selector::parse("meta[name=\"robots\" i]").unwrap();
    if let Some(robots_el) = document.select(&robots_sel).next() {
        if let Some(content) = robots_el.attr("content") {
            let content_lower = content.to_lowercase();
            if content_lower.contains("noindex") {
                noindex = true;
            }
        }
    }

    // H1
    let h1_sel = Selector::parse("h1").unwrap();
    let h1_nodes: Vec<_> = document.select(&h1_sel).collect();
    let h1_count = h1_nodes.len();
    let h1 = h1_nodes.first().map(|el| el.text().collect::<String>().trim().to_string());

    // H2
    let h2_sel = Selector::parse("h2").unwrap();
    let h2_nodes: Vec<_> = document.select(&h2_sel).collect();
    let h2_count = h2_nodes.len();
    let h2 = h2_nodes.first().map(|el| el.text().collect::<String>().trim().to_string());

    // All headings H1–H6 in document order
    let all_headings_sel = Selector::parse("h1, h2, h3, h4, h5, h6").unwrap();
    let headings: Vec<HeadingEntry> = document
        .select(&all_headings_sel)
        .filter_map(|el| {
            let tag = el.value().name();
            let level: u8 = match tag {
                "h1" => 1, "h2" => 2, "h3" => 3,
                "h4" => 4, "h5" => 5, "h6" => 6,
                _ => return None,
            };
            let text = el.text().collect::<String>();
            let text = text.trim().to_string();
            if text.is_empty() { return None; }
            Some(HeadingEntry { level, text })
        })
        .collect();

    // Word count
    let mut word_count = 0;
    let body_sel = Selector::parse("body").unwrap();
    if let Some(body) = document.select(&body_sel).next() {
        for text in body.text() {
            word_count += text.split_whitespace().count();
        }
    }

    // Images
    let mut images = vec![];
    let img_sel = Selector::parse("img").unwrap();
    for img_el in document.select(&img_sel) {
        if let Some(src) = img_el.attr("src") {
            // Resolve relative src
            let resolved_src = parsed_url.join(src).map(|u| u.to_string()).unwrap_or_else(|_| src.to_string());
            images.push(ImageInfo {
                src: resolved_src,
                alt: img_el.attr("alt").map(|s| s.to_string()),
                local_path: None,
            });
        }
    }

    // Links
    let mut outlinks = vec![];
    let mut local_queue = vec![];
    let link_sel = Selector::parse("a[href]").unwrap();
    for link_el in document.select(&link_sel) {
        if let Some(href) = link_el.attr("href") {
            if let Ok(resolved_url) = parsed_url.join(href) {
                // Strip fragment
                let mut clean_url = resolved_url.clone();
                clean_url.set_fragment(None);
                let clean_str = clean_url.to_string();

                outlinks.push(clean_str.clone());

                // Check if internal domain
                if let Some(dom) = resolved_url.domain() {
                    if dom == base_domain {
                        local_queue.push(clean_str);
                    }
                }
            }
        }
    }

    // Open Graph
    let og_title_sel = Selector::parse("meta[property=\"og:title\" i]").unwrap();
    let og_title = document.select(&og_title_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());
    
    let og_description_sel = Selector::parse("meta[property=\"og:description\" i]").unwrap();
    let og_description = document.select(&og_description_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());
    
    let og_image_sel = Selector::parse("meta[property=\"og:image\" i]").unwrap();
    let og_image = document.select(&og_image_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());

    let og_url_sel = Selector::parse("meta[property=\"og:url\" i]").unwrap();
    let og_url = document.select(&og_url_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());

    let og_type_sel = Selector::parse("meta[property=\"og:type\" i]").unwrap();
    let og_type = document.select(&og_type_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());

    // Twitter Cards
    let twitter_title_sel = Selector::parse("meta[name=\"twitter:title\" i]").unwrap();
    let twitter_title = document.select(&twitter_title_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());

    let twitter_description_sel = Selector::parse("meta[name=\"twitter:description\" i]").unwrap();
    let twitter_description = document.select(&twitter_description_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());

    let twitter_image_sel = Selector::parse("meta[name=\"twitter:image\" i]").unwrap();
    let twitter_image = document.select(&twitter_image_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());

    let twitter_card_sel = Selector::parse("meta[name=\"twitter:card\" i]").unwrap();
    let twitter_card = document.select(&twitter_card_sel).next().and_then(|el| el.attr("content")).map(|s| s.trim().to_string());

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
                    schema_errors.push("JSON-LD root must be an Object or Array of Objects.".to_string());
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
                        } else if let Some(ctx) = context {
                            if !ctx.contains("schema.org") {
                                schema_errors.push(format!("Non-standard @context: '{}'. Expected schema.org.", ctx));
                            }
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

        let parsed = parse_html("https://example.com/page", html, "example.com");

        assert_eq!(parsed.title, Some("Test Page Title".to_string()));
        assert_eq!(parsed.meta_desc, Some("This is a test meta description for SEO auditing.".to_string()));
        assert_eq!(parsed.canonical, Some("https://example.com/canonical-url".to_string()));
        assert_eq!(parsed.noindex, true);
        assert_eq!(parsed.h1, Some("Main Heading".to_string()));
        assert_eq!(parsed.h1_count, 2);
        assert_eq!(parsed.h2, Some("Sub Heading".to_string()));
        assert_eq!(parsed.h2_count, 1);
        
        // Assert links
        assert!(parsed.outlinks.contains(&"https://example.com/about".to_string()));
        assert!(parsed.outlinks.contains(&"https://external.com/contact".to_string()));
        
        // Assert local queue
        assert!(parsed.local_queue.contains(&"https://example.com/about".to_string()));
        assert!(!parsed.local_queue.contains(&"https://external.com/contact".to_string()));

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
        assert_eq!(clean_seed_url("   example.com/subpage   "), "https://example.com/subpage");
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
        let rules = parse_robots_txt(robots, "Tadpole");
        assert!(rules.contains(&"/admin".to_string()));
        assert!(rules.contains(&"/private/".to_string()));
        assert!(!rules.contains(&"/no-google/".to_string()));

        assert!(is_disallowed("https://example.com/admin/settings", &rules));
        assert!(is_disallowed("https://example.com/private/index.html", &rules));
        assert!(!is_disallowed("https://example.com/blog", &rules));
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

        let parsed = parse_html("https://example.com/page", html, "example.com");

        assert_eq!(parsed.og_title, Some("Facebook OG Title".to_string()));
        assert_eq!(parsed.og_description, Some("This is an OG description".to_string()));
        assert_eq!(parsed.og_image, Some("https://example.com/share.jpg".to_string()));
        assert_eq!(parsed.og_url, Some("https://example.com/page".to_string()));
        assert_eq!(parsed.og_type, Some("article".to_string()));
        assert_eq!(parsed.twitter_title, Some("Twitter Card Title".to_string()));
        assert_eq!(parsed.twitter_description, Some("This is a Twitter description".to_string()));
        assert_eq!(parsed.twitter_image, Some("https://example.com/twitter-share.jpg".to_string()));
        assert_eq!(parsed.twitter_card, Some("summary_large_image".to_string()));
    }

    #[test]
    fn test_crawl_filter_social() {
        use crate::ui::sidebar::CrawlFilter;

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

        let parsed = parse_html("https://example.com/page", html, "example.com");

        assert_eq!(parsed.schema_json_ld.len(), 3);
        assert!(parsed.schema_errors.iter().any(|e| e.contains("Missing '@context' property")));
        assert!(parsed.schema_errors.iter().any(|e| e.contains("JSON parsing error")));
    }

    #[test]
    fn test_crawl_filter_schema() {
        use crate::ui::sidebar::CrawlFilter;

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
        assert_eq!(get_catalogue_url("https://example.com/foo/bar"), Some("https://example.com/foo/".to_string()));
        assert_eq!(get_catalogue_url("https://example.com/foo/bar/"), Some("https://example.com/foo/bar/".to_string()));
        assert_eq!(get_catalogue_url("https://example.com"), Some("https://example.com/".to_string()));
        assert_eq!(get_catalogue_url("https://example.com/foo"), Some("https://example.com/foo/".to_string()));
        assert_eq!(get_catalogue_url("https://example.com/foo?param=1#frag"), Some("https://example.com/foo/".to_string()));
        assert_eq!(get_catalogue_url("https://example.com/index.html"), Some("https://example.com/".to_string()));
        assert_eq!(get_catalogue_url("https://example.com/foo/index.html"), Some("https://example.com/foo/".to_string()));
    }
}

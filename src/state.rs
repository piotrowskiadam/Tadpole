use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageInfo {
    pub src: String,
    pub alt: Option<String>,
    pub local_path: Option<String>,
}

/// A single heading element (H1–H6) extracted in document order.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadingEntry {
    pub level: u8,    // 1–6
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrawlResult {
    pub url: String,
    pub status_code: Option<u16>,
    pub error_message: Option<String>,
    pub indexable: bool,
    pub indexability_status: String, // "Indexable", "Noindex", "Canonicalised", "Blocked by Robots.txt", "Redirect", "Failed Connection"
    pub title: Option<String>,
    pub meta_desc: Option<String>,
    pub h1: Option<String>,
    pub h1_count: usize,
    pub h2: Option<String>,
    pub h2_count: usize,
    pub word_count: usize,
    pub size_bytes: usize,
    pub canonical: Option<String>,
    pub inlinks: Vec<String>,
    pub outlinks: Vec<String>,
    pub images: Vec<ImageInfo>,
    pub depth: usize,
    pub response_time_ms: u32,
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
    /// All headings H1–H6 in document order
    pub headings: Vec<HeadingEntry>,
    pub markdown: Option<String>,
}

impl CrawlResult {
    pub fn new_failed(
        url: String,
        status_code: Option<u16>,
        error_message: Option<String>,
        indexability_status: String,
        depth: usize,
        response_time_ms: u32,
    ) -> Self {
        Self {
            url,
            status_code,
            error_message,
            indexable: false,
            indexability_status,
            title: None,
            meta_desc: None,
            h1: None,
            h1_count: 0,
            h2: None,
            h2_count: 0,
            word_count: 0,
            size_bytes: 0,
            canonical: None,
            inlinks: vec![],
            outlinks: vec![],
            images: vec![],
            depth,
            response_time_ms,
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
        }
    }
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CrawlStats {
    pub crawled: usize,
    pub discovered: usize,
    pub status_2xx: usize,
    pub status_3xx: usize,
    pub status_4xx: usize,
    pub status_5xx: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CrawlMode {
    Crawl,
    List,
    Path,
    Url,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrawlConfig {
    pub max_urls: usize,
    pub max_concurrency: usize,
    pub user_agent: String,
    pub respect_robots: bool,
    pub follow_redirects: bool,
    pub js_rendering: bool,
    pub include_regex: Option<String>,
    pub exclude_regex: Option<String>,
    
    // AI Settings
    pub ai_provider: String, // "OpenAI" or "OpenRouter"
    pub ai_api_key: Option<String>,
    pub ai_model: String, // e.g. "gpt-4o"
    
    // Image downloads
    pub download_images: bool,
    pub project_dir: Option<String>,
    
    // Advanced/Mode settings
    pub crawl_mode: CrawlMode,
    pub max_depth: Option<usize>,

    // Markdown/Scraping settings
    pub md_only_main_content: bool,
    pub md_ignored_selectors: String,
    pub md_keep_links: bool,
    pub md_ignore_images: bool,
    pub md_clean_whitespace: bool,
    pub md_output_dir: Option<String>,
    pub md_auto_generate: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_urls: 500,
            max_concurrency: 4,
            user_agent: "Tadpole/0.1 (Desktop SEO Auditor)".to_string(),
            respect_robots: true,
            follow_redirects: true,
            js_rendering: false,
            include_regex: None,
            exclude_regex: None,
            ai_provider: "OpenAI".to_string(),
            ai_api_key: None,
            ai_model: "gpt-4o".to_string(),
            download_images: false,
            project_dir: None,
            crawl_mode: CrawlMode::Crawl,
            max_depth: None,
            md_only_main_content: true,
            md_ignored_selectors: "".to_string(),
            md_keep_links: true,
            md_ignore_images: false,
            md_clean_whitespace: true,
            md_output_dir: None,
            md_auto_generate: false,
        }
    }
}

pub struct CrawlStateInner {
    pub urls: HashMap<String, CrawlResult>,
    pub stats: CrawlStats,
    pub discovered_set: HashSet<String>,
    pub is_crawling: bool,
    pub is_paused: bool,
    pub config: CrawlConfig,
}

#[derive(Clone)]
pub struct CrawlState {
    inner: Arc<RwLock<CrawlStateInner>>,
}

impl CrawlState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CrawlStateInner {
                urls: HashMap::new(),
                stats: CrawlStats::default(),
                discovered_set: HashSet::new(),
                is_crawling: false,
                is_paused: false,
                config: CrawlConfig::default(),
            })),
        }
    }

    pub fn reset(&self) {
        let mut inner = self.inner.write();
        inner.urls.clear();
        inner.stats = CrawlStats::default();
        inner.discovered_set.clear();
        inner.is_crawling = false;
        inner.is_paused = false;
    }

    pub fn get_result(&self, url: &str) -> Option<CrawlResult> {
        self.inner.read().urls.get(url).cloned()
    }

    pub fn insert_result(&self, result: CrawlResult) {
        let mut inner = self.inner.write();
        
        // Update stats
        if let Some(code) = result.status_code {
            match code {
                200..=299 => inner.stats.status_2xx += 1,
                300..=399 => inner.stats.status_3xx += 1,
                400..=499 => inner.stats.status_4xx += 1,
                500..=599 => inner.stats.status_5xx += 1,
                _ => inner.stats.errors += 1,
            }
        } else {
            inner.stats.errors += 1;
        }

        inner.stats.crawled += 1;
        inner.urls.insert(result.url.clone(), result);
    }

    pub fn add_discovered(&self, url: &str) -> bool {
        let mut inner = self.inner.write();
        if inner.discovered_set.insert(url.to_string()) {
            inner.stats.discovered += 1;
            true
        } else {
            false
        }
    }

    pub fn check_limit_reached(&self) -> bool {
        let inner = self.inner.read();
        inner.stats.crawled >= inner.config.max_urls
    }

    pub fn add_inlink(&self, target_url: &str, source_url: &str) {
        let mut inner = self.inner.write();
        if let Some(res) = inner.urls.get_mut(target_url) {
            if !res.inlinks.contains(&source_url.to_string()) {
                res.inlinks.push(source_url.to_string());
            }
        }
    }

    pub fn get_stats(&self) -> CrawlStats {
        self.inner.read().stats
    }

    pub fn is_crawling(&self) -> bool {
        self.inner.read().is_crawling
    }

    pub fn set_crawling(&self, val: bool) {
        self.inner.write().is_crawling = val;
    }

    pub fn is_paused(&self) -> bool {
        self.inner.read().is_paused
    }

    pub fn set_paused(&self, val: bool) {
        self.inner.write().is_paused = val;
    }

    pub fn get_limit(&self) -> usize {
        self.inner.read().config.max_urls
    }

    pub fn get_all_results(&self) -> Vec<CrawlResult> {
        self.inner.read().urls.values().cloned().collect()
    }

    pub fn get_config(&self) -> CrawlConfig {
        self.inner.read().config.clone()
    }

    pub fn set_config(&self, config: CrawlConfig) {
        self.inner.write().config = config;
    }

    pub fn load_project(&self, project: CrawlProject) {
        let mut inner = self.inner.write();
        inner.urls.clear();
        inner.discovered_set.clear();
        for res in &project.results {
            inner.urls.insert(res.url.clone(), res.clone());
            inner.discovered_set.insert(res.url.clone());
        }
        inner.stats = project.stats;
        inner.config = project.config;
    }

    pub fn get_project(&self, seed_url: String) -> CrawlProject {
        let inner = self.inner.read();
        CrawlProject {
            seed_url,
            config: inner.config.clone(),
            results: inner.urls.values().cloned().collect(),
            stats: inner.stats,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrawlProject {
    pub seed_url: String,
    pub config: CrawlConfig,
    pub results: Vec<CrawlResult>,
    pub stats: CrawlStats,
}

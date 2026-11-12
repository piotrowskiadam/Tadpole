//! Sitemap provenance and graph reconciliation, independent of the desktop UI.
use crate::state::{CrawlConfig, CrawlMode, CrawlResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// Auxiliary observations have no link depth until a path is observed from a crawl root.
pub const UNKNOWN_DEPTH: usize = usize::MAX;

pub fn normalize_url(value: &str) -> Option<String> {
    let mut url = url::Url::parse(value.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    Some(url.to_string())
}

/// Same hostname and explicit service port. Standard HTTP/HTTPS upgrades are allowed.
pub fn same_site(root: &str, candidate: &str) -> bool {
    match (url::Url::parse(root), url::Url::parse(candidate)) {
        (Ok(a), Ok(b)) => a.host_str() == b.host_str() && a.port() == b.port(),
        _ => false,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SitemapCollection {
    pub sources: Vec<SitemapSource>,
    pub memberships: BTreeMap<String, BTreeSet<String>>,
    pub warnings: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SitemapSource {
    pub url: String,
    pub parent: Option<String>,
    pub status: String,
    pub url_count: usize,
    pub error: Option<String>,
    #[serde(default)]
    pub is_index: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseEvidence {
    pub final_url: Option<String>,
    pub noindex: bool,
    pub redirected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditCoverage {
    pub roots: Vec<String>,
    pub mode: CrawlMode,
    pub max_urls: usize,
    pub max_depth: Option<usize>,
    #[serde(default)]
    pub respect_robots: bool,
    #[serde(default)]
    pub js_rendering: bool,
    pub include: Option<String>,
    pub exclude: Option<String>,
    pub phase: String,
    pub finished: bool,
    pub stopped: bool,
    pub url_limit_hit: bool,
    pub depth_limit_hit: bool,
    pub fetch_gaps: usize,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}

impl AuditCoverage {
    pub fn new(roots: Vec<String>, config: &CrawlConfig) -> Self {
        Self {
            roots,
            mode: config.crawl_mode,
            max_urls: config.max_urls,
            max_depth: config.max_depth,
            respect_robots: config.respect_robots,
            js_rendering: config.js_rendering,
            include: config.include_regex.clone(),
            exclude: config.exclude_regex.clone(),
            phase: "Reading sitemaps and following internal links".into(),
            finished: false,
            stopped: false,
            url_limit_hit: false,
            depth_limit_hit: false,
            fetch_gaps: 0,
            started_at: timestamp(),
            finished_at: None,
        }
    }

    pub fn reachability_available(&self) -> bool {
        matches!(self.mode, CrawlMode::Crawl | CrawlMode::Path) && !self.roots.is_empty()
    }

    pub fn summary(&self) -> String {
        if !self.finished {
            return format!("{} · findings are provisional", self.phase);
        }
        let mut notes = vec![
            if self.stopped {
                "Crawl stopped"
            } else {
                "Crawl finished"
            }
            .to_string(),
        ];
        if self.url_limit_hit {
            notes.push("URL limit reached".into());
        }
        if self.depth_limit_hit {
            notes.push("depth limit reached".into());
        }
        if self.mode == CrawlMode::Path {
            notes.push("path scope only".into());
        }
        if self.include.is_some() || self.exclude.is_some() {
            notes.push("URL filters applied".into());
        }
        if self.fetch_gaps > 0 {
            notes.push(format!(
                "{} {} with fetch/content gaps",
                self.fetch_gaps,
                if self.fetch_gaps == 1 {
                    "page"
                } else {
                    "pages"
                }
            ));
        }
        if !self.reachability_available() {
            notes.push("reachability unavailable in List/URL mode".into());
        }
        notes.push("candidate orphans describe observed scope".into());
        notes.join(" · ")
    }
}

pub fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Finding {
    Unreached,
    HttpError,
    Redirect,
    Noindex,
    Canonical,
    Absent,
    Unchecked,
}

impl Finding {
    pub const ALL: [Self; 7] = [
        Self::Unreached,
        Self::HttpError,
        Self::Redirect,
        Self::Noindex,
        Self::Canonical,
        Self::Absent,
        Self::Unchecked,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Unreached => "Not reached through links",
            Self::HttpError => "HTTP error in sitemap",
            Self::Redirect => "Redirect in sitemap",
            Self::Noindex => "Noindex in sitemap",
            Self::Canonical => "Different canonical",
            Self::Absent => "Not in imported sitemaps",
            Self::Unchecked => "Not evaluated",
        }
    }
    pub fn guidance(self) -> &'static str {
        match self {
            Self::Unreached => {
                "No linking path from a crawl start was observed. Review coverage and scope; add a contextual link if this page should be discoverable. Incoming links from other disconnected pages do not establish reachability."
            }
            Self::HttpError => {
                "Restore the intended page or remove an obsolete URL from the sitemap."
            }
            Self::Redirect => {
                "Review whether the final destination should replace the redirecting URL in the sitemap."
            }
            Self::Noindex => {
                "Review the intended search visibility: remove an unintended noindex directive or remove this URL from the sitemap."
            }
            Self::Canonical => {
                "Verify the intended canonical before changing the page or its sitemap membership."
            }
            Self::Absent => {
                "Review whether this reachable, technically indexable page belongs in the imported sitemaps. Absence alone is not an error; source coverage may be incomplete."
            }
            Self::Unchecked => {
                "Status, directives or page content could not be evaluated. Review the fetch result and crawl budget before drawing a conclusion."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRow {
    pub url: String,
    pub sitemaps: Vec<String>,
    pub reached_depth: Option<usize>,
    pub inlinks: usize,
    pub http: Option<u16>,
    pub observation: String,
    pub canonical: Option<String>,
    pub response: ResponseEvidence,
    pub findings: Vec<Finding>,
}

impl DiscoveryRow {
    pub fn finding_text(&self) -> String {
        if self.findings.is_empty() {
            "No finding in these checks".into()
        } else {
            self.findings
                .iter()
                .map(|f| f.label())
                .collect::<Vec<_>>()
                .join("; ")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySnapshot {
    pub coverage: AuditCoverage,
    pub collection: SitemapCollection,
    pub responses: BTreeMap<String, ResponseEvidence>,
    pub rows: Vec<DiscoveryRow>,
}

impl DiscoverySnapshot {
    pub fn new(roots: Vec<String>, config: &CrawlConfig) -> Self {
        Self {
            coverage: AuditCoverage::new(roots, config),
            collection: SitemapCollection::default(),
            responses: BTreeMap::new(),
            rows: Vec::new(),
        }
    }
}

/// Compute shortest observed paths from ordinary roots, never from sitemap seeds.
/// Redirect destinations are navigation edges, not invented hyperlink inlinks.
pub fn reconcile(snapshot: &mut DiscoverySnapshot, results: &[CrawlResult]) {
    let by_url: HashMap<_, _> = results.iter().map(|p| (p.url.as_str(), p)).collect();
    let keys: BTreeSet<_> = snapshot
        .collection
        .memberships
        .keys()
        .chain(results.iter().map(|p| &p.url))
        .cloned()
        .collect();
    let mut depths = HashMap::new();
    let mut queue = VecDeque::new();
    if snapshot.coverage.reachability_available() {
        for root in &snapshot.coverage.roots {
            depths.insert(root.clone(), 0usize);
            queue.push_back(root.clone());
        }
        while let Some(url) = queue.pop_front() {
            let depth = depths[&url];
            if let Some(page) = by_url.get(url.as_str()) {
                for target in &page.outlinks {
                    if keys.contains(target) && !depths.contains_key(target) {
                        depths.insert(target.clone(), depth.saturating_add(1));
                        queue.push_back(target.clone());
                    }
                }
            }
            if let Some(response) = snapshot.responses.get(&url)
                && response.redirected
                && let Some(final_url) = &response.final_url
                && keys.contains(final_url)
                && !depths.contains_key(final_url)
            {
                depths.insert(final_url.clone(), depth);
                queue.push_back(final_url.clone());
            }
        }
    }
    let mut inlinks: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for page in results {
        for target in &page.outlinks {
            if !keys.contains(target) {
                continue;
            }
            inlinks
                .entry(target.as_str())
                .or_default()
                .insert(page.url.as_str());
        }
    }
    snapshot.coverage.fetch_gaps = results
        .iter()
        .filter(|p| p.error_message.is_some() || p.indexability_status == "Non-HTML Resource")
        .count();
    snapshot.rows = keys
        .into_iter()
        .map(|url| {
            let page = by_url.get(url.as_str()).copied();
            let sitemaps: Vec<_> = snapshot
                .collection
                .memberships
                .get(&url)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            let member = !sitemaps.is_empty();
            let response = snapshot.responses.get(&url).cloned().unwrap_or_default();
            let reached_depth = depths.get(&url).copied();
            let mut findings = Vec::new();
            if member {
                if snapshot.coverage.reachability_available() && reached_depth.is_none() {
                    findings.push(Finding::Unreached);
                }
                if let Some(page) = page {
                    if page.status_code.is_some_and(|s| s >= 400) {
                        findings.push(Finding::HttpError);
                    }
                    if response.redirected
                        || page.indexability_status == "Redirect"
                        || page.status_code.is_some_and(|s| (300..400).contains(&s))
                    {
                        findings.push(Finding::Redirect);
                    }
                    if response.noindex || page.indexability_status == "Noindex" {
                        findings.push(Finding::Noindex);
                    }
                    let final_url = response.final_url.as_deref().unwrap_or(&url);
                    if page.canonical.as_deref().is_some_and(|c| c != final_url) {
                        findings.push(Finding::Canonical);
                    }
                    if page.status_code.is_none()
                        || (page.error_message.is_some()
                            && page.status_code.is_some_and(|s| (200..300).contains(&s)))
                        || page.indexability_status == "Non-HTML Resource"
                    {
                        findings.push(Finding::Unchecked);
                    }
                } else {
                    findings.push(Finding::Unchecked);
                }
            } else if reached_depth.is_some()
                && page.is_some_and(|p| p.indexable)
                && (!snapshot.collection.memberships.is_empty()
                    || snapshot
                        .collection
                        .sources
                        .iter()
                        .any(|source| source.error.is_none() && source.is_index == Some(false)))
            {
                findings.push(Finding::Absent);
            }
            DiscoveryRow {
                inlinks: inlinks.get(url.as_str()).map_or(0, BTreeSet::len),
                url,
                sitemaps,
                reached_depth,
                http: page.and_then(|p| p.status_code),
                observation: page.map_or("Not fetched".into(), |p| p.indexability_status.clone()),
                canonical: page.and_then(|p| p.canonical.clone()),
                response,
                findings,
            }
        })
        .collect();
}

pub fn export_csv(
    writer: impl std::io::Write,
    snapshot: &DiscoverySnapshot,
    rows: &[&DiscoveryRow],
) -> Result<(), csv::Error> {
    let mut csv = csv::Writer::from_writer(writer);
    csv.write_record([
        "URL",
        "Finding",
        "Sitemaps",
        "HTTP",
        "Observation",
        "Observed inlinks",
        "Reachability",
        "Canonical",
        "Final URL",
        "Noindex",
        "Coverage",
        "Source coverage",
        "Suggested action",
    ])?;
    let sources = format!(
        "{} sources; {} unavailable; {} warnings; truncated={}",
        snapshot.collection.sources.len(),
        snapshot
            .collection
            .sources
            .iter()
            .filter(|source| source.error.is_some())
            .count(),
        snapshot.collection.warnings.len(),
        snapshot.collection.truncated
    );
    for row in rows {
        let findings: Vec<Option<Finding>> = if row.findings.is_empty() {
            vec![None]
        } else {
            row.findings.iter().copied().map(Some).collect()
        };
        for finding in findings {
            csv.write_record([
                row.url.clone(),
                finding
                    .map_or("No finding in these checks", Finding::label)
                    .into(),
                row.sitemaps.join("; "),
                row.http.map_or(String::new(), |s| s.to_string()),
                row.observation.clone(),
                row.inlinks.to_string(),
                if !snapshot.coverage.reachability_available() {
                    "Unavailable".into()
                } else {
                    row.reached_depth.map_or("No path observed".into(), |d| {
                        format!("Reached at depth {d}")
                    })
                },
                row.canonical.clone().unwrap_or_default(),
                row.response.final_url.clone().unwrap_or_default(),
                if row.response.noindex {
                    "true".into()
                } else if row.http.is_some_and(|status| (200..300).contains(&status))
                    && !row.findings.contains(&Finding::Unchecked)
                    && row.observation != "Non-HTML Resource"
                {
                    "false".into()
                } else {
                    String::new()
                },
                snapshot.coverage.summary(),
                sources.clone(),
                finding
                    .map_or(
                        "Review observed scope; this does not establish Google indexing.",
                        Finding::guidance,
                    )
                    .into(),
            ])?;
        }
    }
    Ok(csv.flush()?)
}

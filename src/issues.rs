//! Unified, GTK-independent SEO Issues engine combining crawl checks and
//! Sitemap & Discovery findings into a typed `IssueReport`.

use crate::{
    discovery::{DiscoveryRow, DiscoverySnapshot, Finding},
    filters::{CrawlFilter, DuplicateIndex},
    state::{CrawlConfig, CrawlProject, CrawlResult, CrawlState},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// Four fixed Severity tiers expressing contextual audit guidance for
/// prioritization rather than a guaranteed traffic or ranking impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub const ALL: [Self; 4] = [Self::Critical, Self::High, Self::Medium, Self::Low];

    pub fn label(self) -> &'static str {
        match self {
            Self::Critical => "Critical",
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Origin of an Issue for source attribution and workspace navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueSource {
    Crawl,
    Discovery,
}

impl IssueSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Crawl => "Crawl Results",
            Self::Discovery => "Sitemap & Discovery",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Crawl => "crawl",
            Self::Discovery => "discovery",
        }
    }
}

/// Canonical catalog of all unified crawl and Sitemap & Discovery Issues,
/// ordered by Severity tier (`Critical` -> `High` -> `Medium` -> `Low`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueKind {
    // Critical
    ServerError5xx,
    ClientError4xx,
    SitemapHttpError,
    SitemapNoindex,
    // High
    SitemapRedirect,
    SitemapUnreached,
    MissingTitle,
    DuplicateTitle,
    MissingH1,
    StructuredDataErrors,
    // Medium
    Redirection3xx,
    SitemapDifferentCanonical,
    Canonicalised,
    MissingCanonical,
    DuplicateH1,
    MultipleH1s,
    MissingDescription,
    DuplicateDescription,
    ImagesMissingAlt,
    SitemapUnchecked,
    // Low
    TitleTooLong,
    TitleTooShort,
    DescriptionTooLong,
    DescriptionTooShort,
    MissingH2,
    MultipleH2s,
    MissingOgTitle,
    MissingOgDescription,
    SitemapAbsent,
}

impl IssueKind {
    pub const ALL: [Self; 29] = [
        // Critical
        Self::ServerError5xx,
        Self::ClientError4xx,
        Self::SitemapHttpError,
        Self::SitemapNoindex,
        // High
        Self::SitemapRedirect,
        Self::SitemapUnreached,
        Self::MissingTitle,
        Self::DuplicateTitle,
        Self::MissingH1,
        Self::StructuredDataErrors,
        // Medium
        Self::Redirection3xx,
        Self::SitemapDifferentCanonical,
        Self::Canonicalised,
        Self::MissingCanonical,
        Self::DuplicateH1,
        Self::MultipleH1s,
        Self::MissingDescription,
        Self::DuplicateDescription,
        Self::ImagesMissingAlt,
        Self::SitemapUnchecked,
        // Low
        Self::TitleTooLong,
        Self::TitleTooShort,
        Self::DescriptionTooLong,
        Self::DescriptionTooShort,
        Self::MissingH2,
        Self::MultipleH2s,
        Self::MissingOgTitle,
        Self::MissingOgDescription,
        Self::SitemapAbsent,
    ];

    pub fn severity(self) -> Severity {
        match self {
            Self::ServerError5xx
            | Self::ClientError4xx
            | Self::SitemapHttpError
            | Self::SitemapNoindex => Severity::Critical,

            Self::SitemapRedirect
            | Self::SitemapUnreached
            | Self::MissingTitle
            | Self::DuplicateTitle
            | Self::MissingH1
            | Self::StructuredDataErrors => Severity::High,

            Self::Redirection3xx
            | Self::SitemapDifferentCanonical
            | Self::Canonicalised
            | Self::MissingCanonical
            | Self::DuplicateH1
            | Self::MultipleH1s
            | Self::MissingDescription
            | Self::DuplicateDescription
            | Self::ImagesMissingAlt
            | Self::SitemapUnchecked => Severity::Medium,

            Self::TitleTooLong
            | Self::TitleTooShort
            | Self::DescriptionTooLong
            | Self::DescriptionTooShort
            | Self::MissingH2
            | Self::MultipleH2s
            | Self::MissingOgTitle
            | Self::MissingOgDescription
            | Self::SitemapAbsent => Severity::Low,
        }
    }

    pub fn source(self) -> IssueSource {
        match self {
            Self::SitemapHttpError
            | Self::SitemapNoindex
            | Self::SitemapRedirect
            | Self::SitemapUnreached
            | Self::SitemapDifferentCanonical
            | Self::SitemapUnchecked
            | Self::SitemapAbsent => IssueSource::Discovery,
            _ => IssueSource::Crawl,
        }
    }

    pub fn crawl_filter(self) -> Option<CrawlFilter> {
        match self {
            Self::ServerError5xx => Some(CrawlFilter::Status5xx),
            Self::ClientError4xx => Some(CrawlFilter::Status4xx),
            Self::Redirection3xx => Some(CrawlFilter::Status3xx),
            Self::MissingTitle => Some(CrawlFilter::TitleMissing),
            Self::DuplicateTitle => Some(CrawlFilter::TitleDuplicate),
            Self::TitleTooLong => Some(CrawlFilter::TitleTooLong),
            Self::TitleTooShort => Some(CrawlFilter::TitleTooShort),
            Self::MissingDescription => Some(CrawlFilter::DescMissing),
            Self::DuplicateDescription => Some(CrawlFilter::DescDuplicate),
            Self::DescriptionTooLong => Some(CrawlFilter::DescTooLong),
            Self::DescriptionTooShort => Some(CrawlFilter::DescTooShort),
            Self::MissingH1 => Some(CrawlFilter::H1Missing),
            Self::MultipleH1s => Some(CrawlFilter::H1Multiple),
            Self::DuplicateH1 => Some(CrawlFilter::H1Duplicate),
            Self::MissingH2 => Some(CrawlFilter::H2Missing),
            Self::MultipleH2s => Some(CrawlFilter::H2Multiple),
            Self::ImagesMissingAlt => Some(CrawlFilter::ImagesMissingAlt),
            Self::MissingCanonical => Some(CrawlFilter::CanonicalMissing),
            Self::Canonicalised => Some(CrawlFilter::CanonicalOther),
            Self::MissingOgTitle => Some(CrawlFilter::OgTitleMissing),
            Self::MissingOgDescription => Some(CrawlFilter::OgDescMissing),
            Self::StructuredDataErrors => Some(CrawlFilter::SchemaErrors),
            Self::SitemapHttpError
            | Self::SitemapNoindex
            | Self::SitemapRedirect
            | Self::SitemapUnreached
            | Self::SitemapDifferentCanonical
            | Self::SitemapUnchecked
            | Self::SitemapAbsent => None,
        }
    }

    pub fn discovery_finding(self) -> Option<Finding> {
        match self {
            Self::SitemapHttpError => Some(Finding::HttpError),
            Self::SitemapNoindex => Some(Finding::Noindex),
            Self::SitemapRedirect => Some(Finding::Redirect),
            Self::SitemapUnreached => Some(Finding::Unreached),
            Self::SitemapDifferentCanonical => Some(Finding::Canonical),
            Self::SitemapUnchecked => Some(Finding::Unchecked),
            Self::SitemapAbsent => Some(Finding::Absent),
            _ => None,
        }
    }

    /// True for content, metadata, social, and structured-data Issues that
    /// evaluate strictly on 2xx indexable HTML pages.
    pub fn requires_indexable_html_2xx(self) -> bool {
        matches!(
            self,
            Self::MissingTitle
                | Self::DuplicateTitle
                | Self::TitleTooLong
                | Self::TitleTooShort
                | Self::MissingDescription
                | Self::DuplicateDescription
                | Self::DescriptionTooLong
                | Self::DescriptionTooShort
                | Self::MissingH1
                | Self::MultipleH1s
                | Self::DuplicateH1
                | Self::MissingH2
                | Self::MultipleH2s
                | Self::ImagesMissingAlt
                | Self::MissingOgTitle
                | Self::MissingOgDescription
                | Self::StructuredDataErrors
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::ServerError5xx => "Server Error (5xx)",
            Self::ClientError4xx => "Client Error (4xx)",
            Self::SitemapHttpError => "HTTP error in sitemap",
            Self::SitemapNoindex => "Noindex in sitemap",
            Self::SitemapRedirect => "Redirect in sitemap",
            Self::SitemapUnreached => "Not reached through links",
            Self::MissingTitle => "Missing Title",
            Self::DuplicateTitle => "Duplicate Title",
            Self::MissingH1 => "Missing H1",
            Self::StructuredDataErrors => "Structured Data Errors",
            Self::Redirection3xx => "Redirection (3xx)",
            Self::SitemapDifferentCanonical => "Different canonical (sitemap)",
            Self::Canonicalised => "Canonicalised (non-self canonical)",
            Self::MissingCanonical => "Missing Canonical",
            Self::DuplicateH1 => "Duplicate H1",
            Self::MultipleH1s => "Multiple H1s",
            Self::MissingDescription => "Missing Description",
            Self::DuplicateDescription => "Duplicate Description",
            Self::ImagesMissingAlt => "Images Missing Alt Text",
            Self::SitemapUnchecked => "Not evaluated",
            Self::TitleTooLong => "Title > 60 chars",
            Self::TitleTooShort => "Title < 30 chars",
            Self::DescriptionTooLong => "Description > 155 chars",
            Self::DescriptionTooShort => "Description < 70 chars",
            Self::MissingH2 => "Missing H2",
            Self::MultipleH2s => "Multiple H2s",
            Self::MissingOgTitle => "Missing og:title",
            Self::MissingOgDescription => "Missing og:description",
            Self::SitemapAbsent => "Not in imported sitemaps",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::ServerError5xx => "server_error_5xx",
            Self::ClientError4xx => "client_error_4xx",
            Self::SitemapHttpError => "sitemap_http_error",
            Self::SitemapNoindex => "sitemap_noindex",
            Self::SitemapRedirect => "sitemap_redirect",
            Self::SitemapUnreached => "sitemap_unreached",
            Self::MissingTitle => "missing_title",
            Self::DuplicateTitle => "duplicate_title",
            Self::MissingH1 => "missing_h1",
            Self::StructuredDataErrors => "structured_data_errors",
            Self::Redirection3xx => "redirection_3xx",
            Self::SitemapDifferentCanonical => "sitemap_different_canonical",
            Self::Canonicalised => "canonicalised",
            Self::MissingCanonical => "missing_canonical",
            Self::DuplicateH1 => "duplicate_h1",
            Self::MultipleH1s => "multiple_h1",
            Self::MissingDescription => "missing_description",
            Self::DuplicateDescription => "duplicate_description",
            Self::ImagesMissingAlt => "images_missing_alt",
            Self::SitemapUnchecked => "sitemap_unchecked",
            Self::TitleTooLong => "title_too_long",
            Self::TitleTooShort => "title_too_short",
            Self::DescriptionTooLong => "description_too_long",
            Self::DescriptionTooShort => "description_too_short",
            Self::MissingH2 => "missing_h2",
            Self::MultipleH2s => "multiple_h2",
            Self::MissingOgTitle => "missing_og_title",
            Self::MissingOgDescription => "missing_og_description",
            Self::SitemapAbsent => "sitemap_absent",
        }
    }

    pub fn rationale(self) -> &'static str {
        match self {
            Self::ServerError5xx => {
                "5xx responses prevent search engines and visitors from loading the page and can trigger de-indexing if persistent."
            }
            Self::ClientError4xx => {
                "4xx responses represent broken URLs that waste crawl budget and break user navigation when linked internally."
            }
            Self::SitemapHttpError => {
                "Sitemaps should list only live, canonical URLs; including 4xx or 5xx URLs sends conflicting discovery signals."
            }
            Self::SitemapNoindex => {
                "Listing a noindexed URL in an XML sitemap creates a direct contradiction between crawl-priority and exclusion signals."
            }
            Self::SitemapRedirect => {
                "Redirecting URLs in sitemaps add unnecessary hops for crawlers instead of pointing directly to the canonical destination."
            }
            Self::SitemapUnreached => {
                "No internal link path from a crawl root reached this sitemap URL, making it a candidate orphan page with weak internal discovery."
            }
            Self::MissingTitle => {
                "The title element is a primary relevance and SERP display signal; indexable pages without a title rely on fallback generation."
            }
            Self::DuplicateTitle => {
                "Multiple indexable pages sharing the same title make it harder for search engines and users to distinguish distinct URLs."
            }
            Self::MissingH1 => {
                "An indexable page without an H1 lacks a clear primary document heading for users, assistive technology, and content structure."
            }
            Self::StructuredDataErrors => {
                "Invalid JSON-LD or structured data syntax prevents search engines from parsing rich-result markup on the page."
            }
            Self::Redirection3xx => {
                "Internal links pointing to redirecting URLs add latency and dilute link equity through extra redirect hops."
            }
            Self::SitemapDifferentCanonical => {
                "The sitemap lists a URL whose canonical tag points elsewhere, sending mixed signals about which URL should be indexed."
            }
            Self::Canonicalised => {
                "This page declares a canonical pointing to a different URL, signaling that search engines should consolidate indexing to the target."
            }
            Self::MissingCanonical => {
                "Without an explicit canonical tag, parameterized or duplicate URL variants are more likely to be consolidated unpredictably."
            }
            Self::DuplicateH1 => {
                "Indexable pages sharing an identical H1 may indicate templated headings or overlapping page topics."
            }
            Self::MultipleH1s => {
                "Multiple H1 elements on a single page can obscure the primary document topic and weaken heading hierarchy."
            }
            Self::MissingDescription => {
                "Pages without a meta description leave search snippet text entirely to automated extraction."
            }
            Self::DuplicateDescription => {
                "Identical meta descriptions across multiple indexable pages reduce snippet differentiation in search results."
            }
            Self::ImagesMissingAlt => {
                "Images without alt text degrade accessibility for screen readers and omit descriptive context for image search."
            }
            Self::SitemapUnchecked => {
                "Status, directives, or HTML content could not be evaluated for this discovered URL within the observed crawl."
            }
            Self::TitleTooLong => {
                "Titles over 60 characters are frequently truncated in search result snippets, potentially hiding key terms."
            }
            Self::TitleTooShort => {
                "Very short titles under 30 characters often miss descriptive context that helps users evaluate the result."
            }
            Self::DescriptionTooLong => {
                "Meta descriptions over 155 characters are commonly truncated in search results."
            }
            Self::DescriptionTooShort => {
                "Meta descriptions under 70 characters may under-utilize available snippet space to summarize the page."
            }
            Self::MissingH2 => {
                "Adding H2 subheadings helps structure longer content for scannability and semantic sectioning."
            }
            Self::MultipleH2s => {
                "Pages with multiple H2 headings may be normal for long-form content; review whether the heading outline is intentional."
            }
            Self::MissingOgTitle => {
                "Without og:title, social platforms fall back to the document title or heuristics when links are shared."
            }
            Self::MissingOgDescription => {
                "Without og:description, social link previews may omit a summary or extract unintended body text."
            }
            Self::SitemapAbsent => {
                "This reachable, indexable page was not found in the imported sitemaps; absence alone is not an error if sitemap coverage is intentionally selective."
            }
        }
    }

    pub fn suggested_action(self) -> &'static str {
        match self {
            Self::ServerError5xx => {
                "Investigate server logs and application errors, restore a 200 response, or remove broken internal links."
            }
            Self::ClientError4xx => {
                "Restore the missing page, redirect moved URLs to the closest relevant destination, or update internal links."
            }
            Self::SitemapHttpError => Finding::HttpError.guidance(),
            Self::SitemapNoindex => Finding::Noindex.guidance(),
            Self::SitemapRedirect => Finding::Redirect.guidance(),
            Self::SitemapUnreached => Finding::Unreached.guidance(),
            Self::MissingTitle => {
                "Add a unique, descriptive <title> element summarizing the page topic."
            }
            Self::DuplicateTitle => {
                "Write distinct <title> tags for each indexable page or consolidate duplicate pages via canonicals or redirects."
            }
            Self::MissingH1 => {
                "Add a single clear H1 heading that describes the main content of the page."
            }
            Self::StructuredDataErrors => {
                "Fix the JSON-LD syntax or schema validation errors and re-test the structured data output."
            }
            Self::Redirection3xx => {
                "Update internal links to point directly to the final destination URL to avoid redirect hops."
            }
            Self::SitemapDifferentCanonical => Finding::Canonical.guidance(),
            Self::Canonicalised => {
                "Confirm the canonical target is intentional; update internal links to point directly to the canonical URL where appropriate."
            }
            Self::MissingCanonical => {
                "Add a self-referencing <link rel=\"canonical\"> tag to indexable pages to make the preferred URL explicit."
            }
            Self::DuplicateH1 => {
                "Differentiate the H1 heading on each indexable page so it reflects the page's unique subject."
            }
            Self::MultipleH1s => {
                "Keep a single primary H1 for the page title and demote secondary headings to H2 or lower."
            }
            Self::MissingDescription => {
                "Write a concise, unique meta description summarizing the page for search snippets."
            }
            Self::DuplicateDescription => {
                "Customize meta descriptions for each distinct indexable page to improve snippet clarity."
            }
            Self::ImagesMissingAlt => {
                "Add descriptive alt text to informative images, or use an empty alt attribute only for purely decorative assets."
            }
            Self::SitemapUnchecked => Finding::Unchecked.guidance(),
            Self::TitleTooLong => {
                "Tighten the title toward 30–60 characters with primary terms near the start."
            }
            Self::TitleTooShort => {
                "Expand the title with specific, relevant context toward 30–60 characters."
            }
            Self::DescriptionTooLong => {
                "Trim the meta description toward 70–155 characters so the key message is not cut off."
            }
            Self::DescriptionTooShort => {
                "Expand the meta description toward 70–155 characters with a clearer summary of the page."
            }
            Self::MissingH2 => {
                "Introduce H2 subheadings where helpful to break content into scannable sections."
            }
            Self::MultipleH2s => {
                "Review the heading hierarchy to ensure multiple H2 elements represent distinct top-level sections."
            }
            Self::MissingOgTitle => {
                "Add a <meta property=\"og:title\"> tag for consistent titles when the URL is shared."
            }
            Self::MissingOgDescription => {
                "Add a <meta property=\"og:description\"> tag to control the summary shown in social previews."
            }
            Self::SitemapAbsent => Finding::Absent.guidance(),
        }
    }
}

/// A crawled or discovered URL that triggers an Issue, paired with per-URL Evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedUrl {
    pub url: String,
    pub evidence: String,
}

/// A grouped SEO problem category with fixed Severity, Rationale, Suggested Action,
/// Affected URL count and percentage of crawled URLs, and per-URL Evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    pub kind: IssueKind,
    pub severity: Severity,
    pub name: String,
    pub rationale: String,
    pub suggested_action: String,
    pub source: IssueSource,
    pub affected_urls: Vec<AffectedUrl>,
    pub affected_count: usize,
    /// Percentage of crawled URLs (0.0 when no URLs were crawled).
    pub affected_percentage: f64,
}

impl Issue {
    pub fn is_active(&self) -> bool {
        self.affected_count > 0
    }
}

/// Transparent per-Severity counts (no arbitrary 0–100 health score per ADR-0001).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl SeverityCounts {
    pub fn count_for(&self, severity: Severity) -> usize {
        match severity {
            Severity::Critical => self.critical,
            Severity::High => self.high,
            Severity::Medium => self.medium,
            Severity::Low => self.low,
        }
    }

    pub fn total(&self) -> usize {
        self.critical + self.high + self.medium + self.low
    }

    fn increment(&mut self, severity: Severity, amount: usize) {
        match severity {
            Severity::Critical => self.critical += amount,
            Severity::High => self.high += amount,
            Severity::Medium => self.medium += amount,
            Severity::Low => self.low += amount,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScopeCaveatKind {
    CrawlInProgress,
    CrawlStopped,
    UrlLimitReached,
    DepthLimitReached,
    MissingSitemapDiscovery,
    SitemapTruncated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeCaveat {
    pub kind: ScopeCaveatKind,
    pub message: String,
}

/// Unified audit report combining all crawl and Sitemap & Discovery Issues.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IssueReport {
    pub crawled_urls: usize,
    pub indexable_html_urls: usize,
    pub observed_urls: usize,
    pub active_issue_counts: SeverityCounts,
    pub affected_url_counts: SeverityCounts,
    pub caveats: Vec<ScopeCaveat>,
    pub issues: Vec<Issue>,
}

impl IssueReport {
    pub fn active_issues(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|issue| issue.is_active())
    }

    pub fn passed_issues(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|issue| !issue.is_active())
    }

    pub fn issue(&self, kind: IssueKind) -> Option<&Issue> {
        self.issues.iter().find(|issue| issue.kind == kind)
    }

    pub fn has_caveat(&self, kind: ScopeCaveatKind) -> bool {
        self.caveats.iter().any(|caveat| caveat.kind == kind)
    }
}

/// Returns true iff the crawled result is a 2xx indexable HTML page eligible
/// for content, metadata, social, and structured-data Issue evaluation.
pub fn is_indexable_html_2xx(result: &CrawlResult) -> bool {
    result
        .status_code
        .is_some_and(|code| (200..300).contains(&code))
        && result.indexable
        && !result.indexability_status.eq_ignore_ascii_case("Noindex")
        && !result
            .indexability_status
            .eq_ignore_ascii_case("Non-HTML Resource")
        && !result.indexability_status.eq_ignore_ascii_case("Redirect")
}

/// Evaluate all crawl results and optional sitemap discovery evidence from a live `CrawlState`.
pub fn evaluate_state(state: &CrawlState) -> IssueReport {
    let results = state.get_all_results();
    let discovery = state.discovery();
    let config = state.get_config();
    evaluate(&results, discovery.as_ref(), Some(&config))
}

/// Evaluate a saved or loaded `CrawlProject` (including legacy Projects without discovery data).
pub fn evaluate_project(project: &CrawlProject) -> IssueReport {
    evaluate(
        &project.results,
        project.discovery.as_ref(),
        Some(&project.config),
    )
}

/// Core pure evaluation entrypoint. Produces deterministic, Severity-ordered `Issue`
/// entries covering all 29 catalogued crawl + Sitemap & Discovery checks.
pub fn evaluate(
    results: &[CrawlResult],
    discovery: Option<&DiscoverySnapshot>,
    config: Option<&CrawlConfig>,
) -> IssueReport {
    let crawled_urls = results.len();
    let indexable_results: Vec<&CrawlResult> = results
        .iter()
        .filter(|result| is_indexable_html_2xx(result))
        .collect();
    let indexable_html_urls = indexable_results.len();

    let mut observed: BTreeSet<&str> = results.iter().map(|r| r.url.as_str()).collect();
    if let Some(snapshot) = discovery {
        for row in &snapshot.rows {
            observed.insert(row.url.as_str());
        }
    }
    let observed_urls = observed.len();

    // Build DuplicateIndex exclusively over 2xx indexable HTML pages so broken,
    // redirected, or noindexed pages never pollute duplicate counts.
    let indexable_duplicates = DuplicateIndex::from_results(indexable_results.iter().copied());

    let mut affected_by_kind: HashMap<IssueKind, Vec<AffectedUrl>> = HashMap::new();
    for kind in IssueKind::ALL {
        affected_by_kind.insert(kind, Vec::new());
    }

    // 1. Evaluate crawl-sourced Issues.
    for result in results {
        let is_indexable_html = is_indexable_html_2xx(result);
        for kind in IssueKind::ALL {
            if kind.source() != IssueSource::Crawl {
                continue;
            }
            if kind.requires_indexable_html_2xx() && !is_indexable_html {
                continue;
            }
            if matches_crawl_issue(kind, result, &indexable_duplicates) {
                let evidence = crawl_evidence(kind, result, &indexable_duplicates, discovery);
                affected_by_kind
                    .get_mut(&kind)
                    .expect("all IssueKind variants initialized")
                    .push(AffectedUrl {
                        url: result.url.clone(),
                        evidence,
                    });
            }
        }
    }

    // 2. Evaluate Sitemap & Discovery-sourced Issues across all reconciled discovery rows.
    if let Some(snapshot) = discovery {
        for row in &snapshot.rows {
            for &finding in &row.findings {
                if let Some(kind) = issue_kind_for_finding(finding) {
                    let evidence = discovery_evidence(finding, row);
                    affected_by_kind
                        .get_mut(&kind)
                        .expect("all IssueKind variants initialized")
                        .push(AffectedUrl {
                            url: row.url.clone(),
                            evidence,
                        });
                }
            }
        }
    }

    let mut active_issue_counts = SeverityCounts::default();
    let mut affected_url_counts = SeverityCounts::default();
    let mut issues = Vec::with_capacity(IssueKind::ALL.len());

    for kind in IssueKind::ALL {
        let mut affected_urls = affected_by_kind.remove(&kind).unwrap_or_default();
        affected_urls.sort_by(|a, b| a.url.cmp(&b.url));
        let affected_count = affected_urls.len();
        let affected_percentage = if crawled_urls == 0 {
            0.0
        } else {
            (affected_count as f64 / crawled_urls as f64) * 100.0
        };
        let severity = kind.severity();
        if affected_count > 0 {
            active_issue_counts.increment(severity, 1);
            affected_url_counts.increment(severity, affected_count);
        }
        issues.push(Issue {
            kind,
            severity,
            name: kind.name().to_string(),
            rationale: kind.rationale().to_string(),
            suggested_action: kind.suggested_action().to_string(),
            source: kind.source(),
            affected_urls,
            affected_count,
            affected_percentage,
        });
    }

    let caveats = collect_scope_caveats(results, discovery, config);

    IssueReport {
        crawled_urls,
        indexable_html_urls,
        observed_urls,
        active_issue_counts,
        affected_url_counts,
        caveats,
        issues,
    }
}

fn issue_kind_for_finding(finding: Finding) -> Option<IssueKind> {
    match finding {
        Finding::HttpError => Some(IssueKind::SitemapHttpError),
        Finding::Noindex => Some(IssueKind::SitemapNoindex),
        Finding::Redirect => Some(IssueKind::SitemapRedirect),
        Finding::Unreached => Some(IssueKind::SitemapUnreached),
        Finding::Canonical => Some(IssueKind::SitemapDifferentCanonical),
        Finding::Unchecked => Some(IssueKind::SitemapUnchecked),
        Finding::Absent => Some(IssueKind::SitemapAbsent),
    }
}

fn matches_crawl_issue(
    kind: IssueKind,
    result: &CrawlResult,
    duplicates: &DuplicateIndex,
) -> bool {
    match kind {
        IssueKind::Redirection3xx => {
            CrawlFilter::Status3xx.matches(result, &[])
                || result.indexability_status.eq_ignore_ascii_case("Redirect")
        }
        _ => kind
            .crawl_filter()
            .is_some_and(|filter| duplicates.matches(filter, result)),
    }
}

fn truncate_preview(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let count = trimmed.chars().count();
    if count <= max_chars {
        trimmed.to_string()
    } else {
        let prefix: String = trimmed.chars().take(max_chars).collect();
        format!("{prefix}…")
    }
}

fn crawl_evidence(
    kind: IssueKind,
    result: &CrawlResult,
    duplicates: &DuplicateIndex,
    discovery: Option<&DiscoverySnapshot>,
) -> String {
    match kind {
        IssueKind::ServerError5xx | IssueKind::ClientError4xx => match (
            result.status_code,
            result.error_message.as_deref(),
        ) {
            (Some(code), Some(msg)) if !msg.is_empty() => format!("HTTP {code} ({msg})"),
            (Some(code), _) => format!("HTTP {code}"),
            (None, Some(msg)) => msg.to_string(),
            (None, None) => "HTTP error".to_string(),
        },
        IssueKind::Redirection3xx => {
            let final_url = discovery
                .and_then(|d| d.responses.get(&result.url))
                .and_then(|r| r.final_url.as_deref());
            match (result.status_code, final_url) {
                (Some(code), Some(dest)) if dest != result.url => {
                    format!("HTTP {code} redirected to {dest}")
                }
                (None, Some(dest)) if dest != result.url => format!("Redirected to {dest}"),
                (Some(code), _) if (300..400).contains(&code) => format!("HTTP {code} redirect"),
                _ => "Redirect observed".to_string(),
            }
        }
        IssueKind::Canonicalised => match result.canonical.as_deref() {
            Some(target) => format!("Canonical points to {target}"),
            None => "Non-self canonical".to_string(),
        },
        IssueKind::MissingCanonical => match result.status_code {
            Some(code) => format!("No canonical tag found (HTTP {code})"),
            None => "No canonical tag found".to_string(),
        },
        IssueKind::MissingTitle => "Title tag is missing or empty".to_string(),
        IssueKind::DuplicateTitle => {
            let count = duplicates.count(CrawlFilter::TitleDuplicate, result);
            let preview = result
                .title
                .as_deref()
                .map(|t| truncate_preview(t, 60))
                .unwrap_or_default();
            format!("Title \"{preview}\" shared by {count} indexable pages")
        }
        IssueKind::TitleTooLong => {
            let title = result.title.as_deref().unwrap_or("");
            let len = title.chars().count();
            format!(
                "Title length is {len} chars (recommended <= 60): \"{}\"",
                truncate_preview(title, 60)
            )
        }
        IssueKind::TitleTooShort => {
            let title = result.title.as_deref().unwrap_or("");
            let len = title.chars().count();
            format!(
                "Title length is {len} chars (recommended >= 30): \"{}\"",
                truncate_preview(title, 60)
            )
        }
        IssueKind::MissingDescription => "Meta description is missing or empty".to_string(),
        IssueKind::DuplicateDescription => {
            let count = duplicates.count(CrawlFilter::DescDuplicate, result);
            let preview = result
                .meta_desc
                .as_deref()
                .map(|d| truncate_preview(d, 70))
                .unwrap_or_default();
            format!("Meta description shared by {count} indexable pages: \"{preview}\"")
        }
        IssueKind::DescriptionTooLong => {
            let desc = result.meta_desc.as_deref().unwrap_or("");
            let len = desc.chars().count();
            format!("Meta description length is {len} chars (recommended <= 155)")
        }
        IssueKind::DescriptionTooShort => {
            let desc = result.meta_desc.as_deref().unwrap_or("");
            let len = desc.chars().count();
            format!("Meta description length is {len} chars (recommended >= 70)")
        }
        IssueKind::MissingH1 => "H1 heading is missing or empty".to_string(),
        IssueKind::MultipleH1s => format!("{} H1 headings found", result.h1_count),
        IssueKind::DuplicateH1 => {
            let count = duplicates.count(CrawlFilter::H1Duplicate, result);
            let preview = result
                .h1
                .as_deref()
                .map(|h| truncate_preview(h, 60))
                .unwrap_or_default();
            format!("H1 \"{preview}\" shared by {count} indexable pages")
        }
        IssueKind::MissingH2 => "H2 heading is missing or empty".to_string(),
        IssueKind::MultipleH2s => format!("{} H2 headings found", result.h2_count),
        IssueKind::ImagesMissingAlt => {
            let missing = result
                .images
                .iter()
                .filter(|img| img.alt.as_deref().is_none_or(str::is_empty))
                .count();
            let total = result.images.len();
            format!("{missing} of {total} images missing alt text")
        }
        IssueKind::MissingOgTitle => "og:title meta tag is missing or empty".to_string(),
        IssueKind::MissingOgDescription => {
            "og:description meta tag is missing or empty".to_string()
        }
        IssueKind::StructuredDataErrors => {
            let count = result.schema_errors.len();
            match result.schema_errors.first() {
                Some(first) => format!(
                    "{count} schema error{}: {}",
                    if count == 1 { "" } else { "s" },
                    truncate_preview(first, 100)
                ),
                None => "Structured data validation error".to_string(),
            }
        }
        _ => String::new(),
    }
}

fn format_sitemaps(sitemaps: &[String]) -> String {
    if sitemaps.is_empty() {
        "imported sitemaps".to_string()
    } else if sitemaps.len() == 1 {
        sitemaps[0].clone()
    } else {
        format!("{} (+{} more)", sitemaps[0], sitemaps.len() - 1)
    }
}

fn discovery_evidence(finding: Finding, row: &DiscoveryRow) -> String {
    let sitemap_ref = format_sitemaps(&row.sitemaps);
    match finding {
        Finding::HttpError => match row.http {
            Some(code) => format!("HTTP {code}; listed in {sitemap_ref}"),
            None => format!("HTTP error; listed in {sitemap_ref}"),
        },
        Finding::Noindex => format!("Noindex directive observed; listed in {sitemap_ref}"),
        Finding::Redirect => match (row.http, row.response.final_url.as_deref()) {
            (Some(code), Some(dest)) if dest != row.url => {
                format!("HTTP {code} redirects to {dest}; listed in {sitemap_ref}")
            }
            (_, Some(dest)) if dest != row.url => {
                format!("Redirects to {dest}; listed in {sitemap_ref}")
            }
            (Some(code), _) => format!("HTTP {code} redirect; listed in {sitemap_ref}"),
            (None, _) => format!("Redirect observed; listed in {sitemap_ref}"),
        },
        Finding::Unreached => format!(
            "No link path from crawl roots (inlinks: {}); listed in {sitemap_ref}",
            row.inlinks
        ),
        Finding::Canonical => {
            let target = row.canonical.as_deref().unwrap_or("unknown");
            let expected = row.response.final_url.as_deref().unwrap_or(&row.url);
            format!(
                "Canonical points to {target} (expected {expected}); listed in {sitemap_ref}"
            )
        }
        Finding::Unchecked => format!("{}; listed in {sitemap_ref}", row.observation),
        Finding::Absent => match (row.reached_depth, row.http) {
            (Some(depth), Some(code)) => format!(
                "Reachable at depth {depth} (HTTP {code}) but absent from imported sitemaps"
            ),
            (Some(depth), None) => {
                format!("Reachable at depth {depth} but absent from imported sitemaps")
            }
            (None, Some(code)) => {
                format!("Indexable (HTTP {code}) but absent from imported sitemaps")
            }
            (None, None) => "Reachable indexable page absent from imported sitemaps".to_string(),
        },
    }
}

fn collect_scope_caveats(
    results: &[CrawlResult],
    discovery: Option<&DiscoverySnapshot>,
    config: Option<&CrawlConfig>,
) -> Vec<ScopeCaveat> {
    let mut caveats = Vec::new();

    match discovery {
        None => {
            caveats.push(ScopeCaveat {
                kind: ScopeCaveatKind::MissingSitemapDiscovery,
                message: "No sitemap discovery evidence is available for this Project (for example, a legacy Project or crawl run without sitemap discovery). Sitemap Issues could not be evaluated.".to_string(),
            });
            if let Some(cfg) = config
                && cfg.max_urls > 0
                && results.len() >= cfg.max_urls
            {
                caveats.push(ScopeCaveat {
                    kind: ScopeCaveatKind::UrlLimitReached,
                    message: format!(
                        "Crawl reached the configured URL limit ({} URLs); additional pages may not have been inspected.",
                        cfg.max_urls
                    ),
                });
            }
        }
        Some(snapshot) => {
            if !snapshot.coverage.finished {
                caveats.push(ScopeCaveat {
                    kind: ScopeCaveatKind::CrawlInProgress,
                    message: "Crawl is still in progress; Issue counts and discovery findings are provisional.".to_string(),
                });
            }
            if snapshot.coverage.stopped {
                caveats.push(ScopeCaveat {
                    kind: ScopeCaveatKind::CrawlStopped,
                    message: "Crawl was stopped early; Issue counts and orphan/coverage findings reflect only the partial crawl.".to_string(),
                });
            }
            let max_urls = config.map_or(snapshot.coverage.max_urls, |cfg| cfg.max_urls);
            if snapshot.coverage.url_limit_hit
                || (max_urls > 0 && results.len() >= max_urls)
            {
                caveats.push(ScopeCaveat {
                    kind: ScopeCaveatKind::UrlLimitReached,
                    message: format!(
                        "Crawl reached the configured URL limit ({max_urls} URLs); additional pages and sitemap URLs may not have been inspected."
                    ),
                });
            }
            if snapshot.coverage.depth_limit_hit {
                caveats.push(ScopeCaveat {
                    kind: ScopeCaveatKind::DepthLimitReached,
                    message: "Crawl reached its configured maximum link depth; deeper pages may not have been discovered.".to_string(),
                });
            }
            let has_readable_sitemap = !snapshot.collection.memberships.is_empty()
                || snapshot
                    .collection
                    .sources
                    .iter()
                    .any(|source| source.error.is_none());
            if !has_readable_sitemap {
                caveats.push(ScopeCaveat {
                    kind: ScopeCaveatKind::MissingSitemapDiscovery,
                    message: "No readable sitemap sources were imported for this crawl; sitemap reconciliation Issues could not be evaluated.".to_string(),
                });
            }
            if snapshot.collection.truncated {
                caveats.push(ScopeCaveat {
                    kind: ScopeCaveatKind::SitemapTruncated,
                    message: "Sitemap collection hit its source or URL cap; some sitemap declarations were truncated.".to_string(),
                });
            }
        }
    }

    caveats
}

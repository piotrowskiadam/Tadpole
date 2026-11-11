//! Data shared by the detail tables and their CSV exports, including legacy project fallbacks.
use crate::state::{CrawlResult, CrawlState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRow {
    pub source: String,
    pub target: String,
    pub anchor: Option<String>,
    pub rel: Option<String>,
}

pub fn without_fragment(url: &str) -> String {
    url.split('#').next().unwrap_or(url).to_string()
}

pub fn outlinks(page: &CrawlResult) -> Vec<LinkRow> {
    if page.link_details.is_empty() {
        return page
            .outlinks
            .iter()
            .map(|target| LinkRow {
                source: page.url.clone(),
                target: target.clone(),
                anchor: None,
                rel: None,
            })
            .collect();
    }
    page.link_details
        .iter()
        .map(|link| LinkRow {
            source: page.url.clone(),
            target: link.target.clone(),
            anchor: Some(link.anchor.clone()),
            rel: Some(link.rel.clone()),
        })
        .collect()
}

pub fn inlinks(page: &CrawlResult, state: &CrawlState) -> Vec<LinkRow> {
    let mut rows = Vec::new();
    for source in &page.inlinks {
        let matching: Vec<_> = state
            .get_result(source)
            .map(|result| {
                outlinks(&result)
                    .into_iter()
                    .filter(|row| without_fragment(&row.target) == without_fragment(&page.url))
                    .collect()
            })
            .unwrap_or_default();
        if matching.is_empty() {
            rows.push(LinkRow {
                source: source.clone(),
                target: page.url.clone(),
                anchor: None,
                rel: None,
            });
        } else {
            rows.extend(matching);
        }
    }
    rows
}

pub fn alt_status(alt: Option<&str>) -> &'static str {
    match alt {
        None => "Missing alt attribute",
        Some("") => "Empty alt (decorative)",
        Some(text) if text.trim().is_empty() => "Whitespace-only alt",
        Some(_) => "Present",
    }
}

/// Resolve relative social image paths while refusing non-web schemes for network previews.
pub fn preview_url(page: &str, value: &str) -> Option<String> {
    if value.trim().is_empty() {
        return None;
    }
    let url = url::Url::parse(page).ok()?.join(value.trim()).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

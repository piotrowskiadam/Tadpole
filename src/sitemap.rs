//! Bounded sitemap retrieval and streaming XML parsing. No GTK or provider calls.
use crate::discovery::{SitemapCollection, SitemapSource, normalize_url, same_site};
use crate::state::{CrawlConfig, CrawlMode, CrawlState};
use quick_xml::{NsReader, events::Event, name::ResolveResult};
use std::collections::{BTreeSet, VecDeque};
use std::io::Read;
use std::time::Duration;

pub const MAX_SOURCES: usize = 64;
pub const MAX_INDEX_DEPTH: usize = 5;
pub const MAX_URLS: usize = 50_000;
pub const DOWNLOAD_LIMIT: usize = 8 * 1024 * 1024;
pub const DECODED_LIMIT: usize = 50 * 1024 * 1024;

#[derive(Debug)]
pub struct ParsedSitemap {
    pub is_index: bool,
    pub locations: Vec<String>,
    pub truncated: bool,
}

pub fn decode_body(bytes: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    let mut decoded = Vec::new();
    if bytes.starts_with(&[0x1f, 0x8b]) {
        flate2::read::MultiGzDecoder::new(bytes)
            .take(limit as u64 + 1)
            .read_to_end(&mut decoded)
            .map_err(|_| "Invalid gzip sitemap".to_string())?;
    } else {
        decoded.extend_from_slice(&bytes[..bytes.len().min(limit.saturating_add(1))]);
    }
    if decoded.len() > limit {
        return Err("Sitemap exceeds the decoded byte limit".into());
    }
    Ok(decoded)
}

pub fn parse(bytes: &[u8], max_entries: usize) -> Result<ParsedSitemap, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Sitemap must be UTF-8 XML")?;
    let mut reader = NsReader::from_str(text);
    let mut stack = Vec::<Vec<u8>>::new();
    let mut protocol_stack = Vec::new();
    let mut root = None;
    let mut closed = false;
    let mut value = String::new();
    let mut has_loc = false;
    let mut locations = Vec::new();
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|e| format!("Invalid sitemap XML: {e}"))?;
        let protocol = match namespace {
            ResolveResult::Unbound => true,
            ResolveResult::Bound(namespace) => matches!(
                namespace.as_ref(),
                b"http://www.sitemaps.org/schemas/sitemap/0.9"
                    | b"https://www.sitemaps.org/schemas/sitemap/0.9"
            ),
            _ => false,
        };
        let in_loc = stack.len() == 3 && stack[2] == b"loc" && protocol_stack[2];
        match event {
            Event::DocType(_) => {
                return Err("DOCTYPE declarations are not supported in sitemaps".into());
            }
            Event::Start(e) => {
                if in_loc {
                    return Err("Sitemap loc must contain text, not nested elements".into());
                }
                let name = e.local_name().as_ref().to_vec();
                if stack.is_empty() {
                    if root.is_some() || closed {
                        return Err("Sitemap must have one root element".into());
                    }
                    if !protocol || (name != b"urlset" && name != b"sitemapindex") {
                        return Err("Expected urlset or sitemapindex XML".into());
                    }
                    root = Some(name.clone());
                }
                if stack.len() == 1 {
                    let expected: &[u8] = if root.as_deref() == Some(b"urlset") {
                        b"url"
                    } else {
                        b"sitemap"
                    };
                    if !protocol || name != expected {
                        return Err("Unexpected child in sitemap root".into());
                    }
                    has_loc = false;
                }
                if stack.len() == 2 && name == b"loc" {
                    value.clear();
                }
                if stack.len() >= 64 {
                    return Err("XML nesting exceeds the sitemap parser limit".into());
                }
                stack.push(name);
                protocol_stack.push(protocol);
            }
            Event::Empty(e) => {
                let name = e.local_name().as_ref().to_vec();
                if stack.is_empty() {
                    if !protocol || root.is_some() || (name != b"urlset" && name != b"sitemapindex")
                    {
                        return Err("Expected one urlset or sitemapindex root".into());
                    }
                    root = Some(name);
                    closed = true;
                } else if (stack.len() == 2 && name == b"loc" && protocol)
                    || (stack.len() == 1 && (name == b"url" || name == b"sitemap"))
                {
                    return Err("Sitemap entry has an empty or missing loc".into());
                }
            }
            Event::Text(e) if in_loc => {
                value.push_str(&e.decode().map_err(|_| "Invalid loc encoding")?)
            }
            Event::CData(e) if in_loc => {
                value.push_str(&e.decode().map_err(|_| "Invalid loc encoding")?)
            }
            Event::GeneralRef(e) if in_loc => {
                let name = e.decode().map_err(|_| "Invalid loc entity")?;
                value.push_str(
                    &quick_xml::escape::unescape(&format!("&{name};"))
                        .map_err(|_| "Unknown XML entity in loc")?,
                );
            }
            Event::End(_) => {
                if in_loc {
                    let expected_entry: &[u8] = if root.as_deref() == Some(b"urlset") {
                        b"url"
                    } else {
                        b"sitemap"
                    };
                    if stack[1] == expected_entry {
                        if has_loc || value.trim().is_empty() {
                            return Err("Sitemap entry must have one nonempty loc".into());
                        }
                        if locations.len() >= max_entries {
                            return Ok(ParsedSitemap {
                                is_index: root.as_deref() == Some(b"sitemapindex"),
                                locations,
                                truncated: true,
                            });
                        }
                        locations.push(value.trim().to_string());
                        has_loc = true;
                    }
                }
                if stack.len() == 2 {
                    let entry = &stack[1];
                    if (entry == b"url" || entry == b"sitemap") && !has_loc {
                        return Err("Sitemap entry is missing loc".into());
                    }
                }
                if stack.pop().is_none() {
                    return Err("Unexpected XML closing tag".into());
                }
                protocol_stack.pop();
                if stack.is_empty() {
                    closed = true;
                }
            }
            Event::Text(e)
                if stack.is_empty()
                    && !e
                        .decode()
                        .map_err(|_| "Invalid XML text")?
                        .trim()
                        .is_empty() =>
            {
                return Err("Unexpected text outside sitemap root".into());
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if root.is_none() || !closed || !stack.is_empty() {
        return Err("Incomplete sitemap XML".into());
    }
    Ok(ParsedSitemap {
        is_index: root.as_deref() == Some(b"sitemapindex"),
        locations,
        truncated: false,
    })
}

pub async fn collect(
    root: &str,
    config: &CrawlConfig,
    rules: Result<crate::robots::RobotsRules, String>,
    state: &CrawlState,
    mut progress: impl FnMut(&SitemapCollection),
) -> SitemapCollection {
    let mut collection = SitemapCollection::default();
    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    for source in &config.sitemap_urls {
        queue.push_back((source.clone(), None, 0usize));
    }
    if matches!(config.crawl_mode, CrawlMode::Crawl | CrawlMode::Path) {
        match &rules {
            Ok(rules) => {
                for source in &rules.sitemaps {
                    queue.push_back((source.clone(), None, 0));
                }
            }
            Err(error) => collection
                .warnings
                .push(format!("Sitemap discovery: {error}")),
        }
    }
    let mut pending = BTreeSet::new();
    queue.retain(|(url, _, _)| pending.insert(normalize_url(url).unwrap_or_else(|| url.clone())));
    if queue.len() > MAX_SOURCES {
        queue.truncate(MAX_SOURCES);
        collection.truncated = true;
    }
    let boundary = root.to_string();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .user_agent(&config.user_agent)
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 5
                || !same_site(&boundary, attempt.url().as_str())
                || normalize_url(attempt.url().as_str()).is_none()
            {
                attempt.error("Sitemap redirect exceeds the same-site scope or hop limit")
            } else {
                attempt.follow()
            }
        }))
        .build();
    let Ok(client) = client else {
        collection
            .warnings
            .push("Could not initialize sitemap HTTP client".into());
        return collection;
    };
    while let Some((raw, parent, depth)) = queue.pop_front() {
        if state.stop_requested() {
            break;
        }
        while state.is_paused() && !state.stop_requested() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if state.stop_requested() {
            break;
        }
        let key = normalize_url(&raw).unwrap_or_else(|| raw.clone());
        pending.remove(&key);
        if !seen.insert(key.clone()) {
            continue;
        }
        if collection.sources.len() >= MAX_SOURCES {
            collection.truncated = true;
            collection
                .warnings
                .push(format!("Sitemap source limit ({MAX_SOURCES}) reached"));
            break;
        }
        let mut source = SitemapSource {
            url: key.clone(),
            parent,
            status: "Reading".into(),
            url_count: 0,
            error: None,
            is_index: None,
        };
        let allowed = normalize_url(&raw).is_some() && same_site(root, &key);
        let outcome: Result<ParsedSitemap, String> = async {
            if !allowed {
                return Err(
                    "Source must be an HTTP(S) URL on the crawl site without credentials".into(),
                );
            }
            if depth > MAX_INDEX_DEPTH {
                return Err(format!(
                    "Sitemap index depth limit ({MAX_INDEX_DEPTH}) reached"
                ));
            }
            if config.respect_robots {
                match &rules {
                    Ok(rules) if rules.allows(&key) => {}
                    Ok(_) => return Err("Sitemap source blocked by robots.txt".into()),
                    Err(error) => return Err(error.clone()),
                }
            }
            let response = client
                .get(&key)
                .send()
                .await
                .map_err(|_| "Could not fetch sitemap; check the URL and retry")?;
            source.status = format!("HTTP {}", response.status().as_u16());
            if !response.status().is_success() {
                return Err(source.status.clone());
            }
            let body = crate::http::read_body(response, DOWNLOAD_LIMIT).await?;
            tokio::task::spawn_blocking(move || {
                let decoded = decode_body(&body, DECODED_LIMIT)?;
                parse(&decoded, MAX_URLS)
            })
            .await
            .map_err(|_| "Sitemap parser could not finish".to_string())?
        }
        .await;
        match outcome {
            Ok(parsed) => {
                source.is_index = Some(parsed.is_index);
                source.status = if parsed.is_index {
                    "Index read"
                } else {
                    "URL set read"
                }
                .into();
                if parsed.truncated {
                    collection.truncated = true;
                    collection
                        .warnings
                        .push(format!("Entry limit reached in {key}"));
                }
                let mut skipped = 0;
                let mut unique = BTreeSet::new();
                for raw in parsed.locations {
                    let Some(url) = normalize_url(&raw) else {
                        skipped += 1;
                        continue;
                    };
                    if !same_site(root, &url) {
                        skipped += 1;
                        continue;
                    }
                    if parsed.is_index {
                        if !seen.contains(&url)
                            && !pending.contains(&url)
                            && queue.len() < MAX_SOURCES
                        {
                            pending.insert(url.clone());
                            queue.push_back((url, Some(key.clone()), depth + 1));
                        } else if !seen.contains(&url) && !pending.contains(&url) {
                            collection.truncated = true;
                        }
                    } else {
                        if collection.memberships.len() >= MAX_URLS
                            && !collection.memberships.contains_key(&url)
                        {
                            collection.truncated = true;
                            continue;
                        }
                        unique.insert(url.clone());
                        collection
                            .memberships
                            .entry(url)
                            .or_default()
                            .insert(key.clone());
                    }
                }
                source.url_count = unique.len();
                if skipped > 0 {
                    collection.warnings.push(format!(
                        "{key}: skipped {skipped} invalid or out-of-scope locations"
                    ));
                }
            }
            Err(error) => {
                if depth > MAX_INDEX_DEPTH {
                    collection.truncated = true;
                }
                source.error = Some(error);
                source.status = "Unavailable".into();
            }
        }
        collection.sources.push(source);
        progress(&collection);
    }
    if collection.truncated {
        collection
            .warnings
            .push("Sitemap coverage is incomplete because a collection limit was reached".into());
    }
    collection
}

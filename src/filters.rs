use crate::state::CrawlResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlFilter {
    All,
    Status2xx,
    Status3xx,
    Status4xx,
    Status5xx,
    TitleMissing,
    TitleDuplicate,
    TitleTooLong,
    TitleTooShort,
    DescMissing,
    DescDuplicate,
    DescTooLong,
    DescTooShort,
    H1Missing,
    H1Multiple,
    H1Duplicate,
    H2Missing,
    H2Multiple,
    ImagesMissingAlt,
    CanonicalMissing,
    CanonicalOther,
    OgTitleMissing,
    OgDescMissing,
    SchemaPresent,
    SchemaErrors,
}

impl CrawlFilter {
    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "All URLs",
            Self::Status2xx => "Success (2xx)",
            Self::Status3xx => "Redirection (3xx)",
            Self::Status4xx => "Client Error (4xx)",
            Self::Status5xx => "Server Error (5xx)",
            Self::TitleMissing => "Missing Title",
            Self::TitleDuplicate => "Duplicate Title",
            Self::TitleTooLong => "Title > 60 chars",
            Self::TitleTooShort => "Title < 30 chars",
            Self::DescMissing => "Missing Description",
            Self::DescDuplicate => "Duplicate Description",
            Self::DescTooLong => "Description > 155 chars",
            Self::DescTooShort => "Description < 70 chars",
            Self::H1Missing => "Missing H1",
            Self::H1Multiple => "Multiple H1s",
            Self::H1Duplicate => "Duplicate H1",
            Self::H2Missing => "Missing H2",
            Self::H2Multiple => "Multiple H2s",
            Self::ImagesMissingAlt => "Images Missing Alt Text",
            Self::CanonicalMissing => "Missing Canonical",
            Self::CanonicalOther => "Canonicalised",
            Self::OgTitleMissing => "Missing og:title",
            Self::OgDescMissing => "Missing og:description",
            Self::SchemaPresent => "Structured Data Present",
            Self::SchemaErrors => "Structured Data Errors",
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::All => "all_urls",
            Self::Status2xx => "status_2xx",
            Self::Status3xx => "status_3xx",
            Self::Status4xx => "status_4xx",
            Self::Status5xx => "status_5xx",
            Self::TitleMissing => "missing_title",
            Self::TitleDuplicate => "duplicate_title",
            Self::TitleTooLong => "title_over_60",
            Self::TitleTooShort => "title_under_30",
            Self::DescMissing => "missing_description",
            Self::DescDuplicate => "duplicate_description",
            Self::DescTooLong => "description_over_155",
            Self::DescTooShort => "description_under_70",
            Self::H1Missing => "missing_h1",
            Self::H1Multiple => "multiple_h1",
            Self::H1Duplicate => "duplicate_h1",
            Self::H2Missing => "missing_h2",
            Self::H2Multiple => "multiple_h2",
            Self::ImagesMissingAlt => "images_missing_alt",
            Self::CanonicalMissing => "missing_canonical",
            Self::CanonicalOther => "canonicalised",
            Self::OgTitleMissing => "missing_og_title",
            Self::OgDescMissing => "missing_og_description",
            Self::SchemaPresent => "schema_present",
            Self::SchemaErrors => "schema_errors",
        }
    }

    pub fn matches(&self, res: &CrawlResult, all_results: &[CrawlResult]) -> bool {
        match self {
            Self::All => true,
            Self::Status2xx => res.status_code.is_some_and(|c| (200..300).contains(&c)),
            Self::Status3xx => res.status_code.is_some_and(|c| (300..400).contains(&c)),
            Self::Status4xx => res.status_code.is_some_and(|c| (400..500).contains(&c)),
            Self::Status5xx => res.status_code.is_some_and(|c| (500..600).contains(&c)),
            Self::TitleMissing => res.title.is_none() || res.title.as_ref().unwrap().is_empty(),
            Self::TitleDuplicate => {
                if let Some(ref t) = res.title {
                    !t.is_empty()
                        && all_results
                            .iter()
                            .filter(|r| r.title.as_ref() == Some(t))
                            .count()
                            > 1
                } else {
                    false
                }
            }
            Self::TitleTooLong => res.title.as_ref().is_some_and(|t| t.chars().count() > 60),
            Self::TitleTooShort => res
                .title
                .as_ref()
                .is_some_and(|t| !t.is_empty() && t.chars().count() < 30),
            Self::DescMissing => {
                res.meta_desc.is_none() || res.meta_desc.as_ref().unwrap().is_empty()
            }
            Self::DescDuplicate => {
                if let Some(ref d) = res.meta_desc {
                    !d.is_empty()
                        && all_results
                            .iter()
                            .filter(|r| r.meta_desc.as_ref() == Some(d))
                            .count()
                            > 1
                } else {
                    false
                }
            }
            Self::DescTooLong => res
                .meta_desc
                .as_ref()
                .is_some_and(|d| d.chars().count() > 155),
            Self::DescTooShort => res
                .meta_desc
                .as_ref()
                .is_some_and(|d| !d.is_empty() && d.chars().count() < 70),
            Self::H1Missing => res.h1.is_none() || res.h1.as_ref().unwrap().is_empty(),
            Self::H1Multiple => res.h1_count > 1,
            Self::H1Duplicate => {
                if let Some(ref h) = res.h1 {
                    !h.is_empty()
                        && all_results
                            .iter()
                            .filter(|r| r.h1.as_ref() == Some(h))
                            .count()
                            > 1
                } else {
                    false
                }
            }
            Self::H2Missing => res.h2.is_none() || res.h2.as_ref().unwrap().is_empty(),
            Self::H2Multiple => res.h2_count > 1,
            Self::ImagesMissingAlt => res
                .images
                .iter()
                .any(|img| img.alt.is_none() || img.alt.as_ref().unwrap().is_empty()),
            Self::CanonicalMissing => res.canonical.is_none(),
            Self::CanonicalOther => {
                res.canonical.is_some() && res.canonical.as_ref() != Some(&res.url)
            }
            Self::OgTitleMissing => {
                res.og_title.is_none() || res.og_title.as_ref().unwrap().trim().is_empty()
            }
            Self::OgDescMissing => {
                res.og_description.is_none()
                    || res.og_description.as_ref().unwrap().trim().is_empty()
            }
            Self::SchemaPresent => !res.schema_json_ld.is_empty(),
            Self::SchemaErrors => !res.schema_errors.is_empty(),
        }
    }
}

/// Counts are built once per result set; duplicate filters are O(1) per row.
#[derive(Default, Clone)]
pub struct DuplicateIndex {
    titles: std::collections::HashMap<String, usize>,
    descriptions: std::collections::HashMap<String, usize>,
    headings: std::collections::HashMap<String, usize>,
}
impl DuplicateIndex {
    pub fn from_results<'a>(results: impl IntoIterator<Item = &'a CrawlResult>) -> Self {
        let mut index = Self::default();
        for result in results {
            index.add(result);
        }
        index
    }
    pub fn add(&mut self, result: &CrawlResult) {
        for (map, value) in [
            (&mut self.titles, &result.title),
            (&mut self.descriptions, &result.meta_desc),
            (&mut self.headings, &result.h1),
        ] {
            if let Some(value) = value.as_ref().filter(|v| !v.is_empty()) {
                *map.entry(value.clone()).or_default() += 1;
            }
        }
    }
    pub fn remove(&mut self, result: &CrawlResult) {
        for (map, value) in [
            (&mut self.titles, &result.title),
            (&mut self.descriptions, &result.meta_desc),
            (&mut self.headings, &result.h1),
        ] {
            if let Some(count) = value.as_ref().and_then(|v| map.get_mut(v)) {
                *count = count.saturating_sub(1);
            }
        }
    }
    pub fn matches(&self, filter: CrawlFilter, result: &CrawlResult) -> bool {
        let (map, value) = match filter {
            CrawlFilter::TitleDuplicate => (&self.titles, &result.title),
            CrawlFilter::DescDuplicate => (&self.descriptions, &result.meta_desc),
            CrawlFilter::H1Duplicate => (&self.headings, &result.h1),
            _ => return filter.matches(result, &[]),
        };
        value
            .as_ref()
            .and_then(|v| map.get(v))
            .is_some_and(|count| *count > 1)
    }
}

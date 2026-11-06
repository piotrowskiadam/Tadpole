use super::snapshot::GraphSnapshot;
use crate::state::CrawlResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IcicleWeightMetric {
    UrlCount,
    PageSizeBytes,
    WordCount,
    IssueCount,
    InternalRankSum,
    ResponseTimeMs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IcicleColorMetric {
    Section,
    IssueSeverity,
    IndexabilityStatus,
    StatusCodeGroup,
    AvgCrawlDepth,
    AvgResponseTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IcicleTreeNode {
    pub name: String,
    pub full_path: String,
    pub weight_value: f64,
    pub error_count: usize,
    pub total_pages: usize,
    pub indexable_pages: usize,
    pub depth: usize,
    pub children: Vec<IcicleTreeNode>,
    pub sample_urls: Vec<String>,
}

impl IcicleTreeNode {
    pub fn find_by_path<'a>(&'a self, target_path: &str) -> Option<&'a IcicleTreeNode> {
        if self.full_path == target_path {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_path(target_path) {
                return Some(found);
            }
        }
        None
    }
}

pub fn build_icicle_tree_from_snapshot(
    snapshot: &GraphSnapshot,
    weight_metric: IcicleWeightMetric,
) -> IcicleTreeNode {
    use std::collections::HashMap;
    let empty = |name: String, full_path: String, depth| IcicleTreeNode {
        name,
        full_path,
        depth,
        weight_value: 0.0,
        error_count: 0,
        total_pages: 0,
        indexable_pages: 0,
        children: Vec::new(),
        sample_urls: Vec::new(),
    };
    let mut arena = vec![empty("/".into(), "/".into(), 0)];
    let mut children: Vec<Vec<usize>> = vec![Vec::new()];
    let mut lookup = HashMap::new();
    let origins: std::collections::HashSet<_> = snapshot
        .nodes
        .iter()
        .filter_map(|n| url::Url::parse(snapshot.node_url(n)).ok())
        .map(|u| u.origin().ascii_serialization())
        .collect();
    for node in snapshot.nodes.iter() {
        let url = snapshot.node_url(node);
        let weight = match weight_metric {
            IcicleWeightMetric::UrlCount => 1.0,
            IcicleWeightMetric::PageSizeBytes => node.page_size_bytes as f64,
            IcicleWeightMetric::WordCount => node.word_count as f64,
            IcicleWeightMetric::IssueCount => f64::from(node.issue_mask != 0),
            IcicleWeightMetric::InternalRankSum => node.internal_rank as f64,
            IcicleWeightMetric::ResponseTimeMs => node.response_time_ms as f64,
        };
        let path = snapshot.strings.get(node.path_id).unwrap_or("/");
        let mut segments: Vec<String> = path
            .split('/')
            .filter(|s| !s.is_empty())
            .take(64)
            .map(str::to_owned)
            .collect();
        if origins.len() > 1 {
            let origin = url::Url::parse(url)
                .map(|u| u.origin().ascii_serialization())
                .unwrap_or_default();
            segments.insert(0, origin);
        }
        let mut current = 0;
        let mut visited = vec![0];
        for segment in segments {
            let key = (current, segment.clone());
            current = *lookup.entry(key).or_insert_with(|| {
                let index = arena.len();
                let full_path = format!(
                    "{}/{}",
                    arena[current].full_path.trim_end_matches('/'),
                    segment
                );
                arena.push(empty(segment, full_path, arena[current].depth + 1));
                children.push(Vec::new());
                children[current].push(index);
                index
            });
            visited.push(current);
        }
        for index in visited {
            let entry = &mut arena[index];
            entry.weight_value += weight;
            entry.total_pages += 1;
            entry.error_count += usize::from(node.status_code >= 400);
            entry.indexable_pages +=
                usize::from(node.indexability == super::snapshot::Indexability::Indexable);
            if entry.sample_urls.len() < 5 {
                entry.sample_urls.push(url.into());
            }
        }
    }
    let mut built: Vec<Option<IcicleTreeNode>> = arena.into_iter().map(Some).collect();
    for index in (0..built.len()).rev() {
        let mut descendants: Vec<_> = children[index]
            .iter()
            .filter_map(|&i| built[i].take())
            .collect();
        descendants.sort_by(|a, b| a.name.cmp(&b.name));
        built[index].as_mut().unwrap().children = descendants;
    }
    built[0].take().unwrap()
}

pub fn build_icicle_tree_from_results(
    results: &[CrawlResult],
    weight_metric: IcicleWeightMetric,
) -> IcicleTreeNode {
    build_icicle_tree_from_snapshot(
        &super::SnapshotBuilder::build_from_results(results),
        weight_metric,
    )
}

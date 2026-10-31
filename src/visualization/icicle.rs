use super::snapshot::GraphSnapshot;
use crate::state::CrawlResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IcicleWeightMetric {
    UrlCount,
    PageSizeBytes,
    WordCount,
    IssueCount,
    InternalRankSum,
    ResponseTimeMs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    let mut root = IcicleTreeNode {
        name: "/".to_string(),
        full_path: "/".to_string(),
        weight_value: 0.0,
        error_count: 0,
        total_pages: 0,
        indexable_pages: 0,
        depth: 0,
        children: Vec::new(),
        sample_urls: Vec::new(),
    };

    for node in snapshot.nodes.iter() {
        let url = snapshot.node_url(node);
        let path = snapshot.strings.get(node.path_id).unwrap_or("/");

        let weight = match weight_metric {
            IcicleWeightMetric::UrlCount => 1.0,
            IcicleWeightMetric::PageSizeBytes => node.page_size_bytes as f64,
            IcicleWeightMetric::WordCount => node.word_count as f64,
            IcicleWeightMetric::IssueCount => {
                if node.issue_mask != 0 {
                    1.0
                } else {
                    0.0
                }
            }
            IcicleWeightMetric::InternalRankSum => node.internal_rank as f64,
            IcicleWeightMetric::ResponseTimeMs => node.response_time_ms as f64,
        };

        insert_path_into_icicle(
            &mut root,
            path,
            url,
            weight,
            node.status_code >= 400,
            node.indexability == super::snapshot::Indexability::Indexable,
        );
    }

    root
}

pub fn build_icicle_tree_from_results(
    results: &[CrawlResult],
    weight_metric: IcicleWeightMetric,
) -> IcicleTreeNode {
    let mut root = IcicleTreeNode {
        name: "/".to_string(),
        full_path: "/".to_string(),
        weight_value: 0.0,
        error_count: 0,
        total_pages: 0,
        indexable_pages: 0,
        depth: 0,
        children: Vec::new(),
        sample_urls: Vec::new(),
    };

    for res in results {
        let weight = match weight_metric {
            IcicleWeightMetric::UrlCount => 1.0,
            IcicleWeightMetric::PageSizeBytes => res.size_bytes as f64,
            IcicleWeightMetric::WordCount => res.word_count as f64,
            IcicleWeightMetric::IssueCount => {
                if res.status_code.unwrap_or(0) >= 400 || !res.indexable {
                    1.0
                } else {
                    0.0
                }
            }
            IcicleWeightMetric::InternalRankSum => 1.0,
            IcicleWeightMetric::ResponseTimeMs => res.response_time_ms as f64,
        };

        let is_error = res.status_code.unwrap_or(0) >= 400;

        insert_path_into_icicle(
            &mut root,
            &res.url,
            &res.url,
            weight,
            is_error,
            res.indexable,
        );
    }

    root
}

fn insert_path_into_icicle(
    root: &mut IcicleTreeNode,
    path_str: &str,
    full_url: &str,
    weight: f64,
    is_error: bool,
    is_indexable: bool,
) {
    root.weight_value += weight;
    root.total_pages += 1;
    if is_error {
        root.error_count += 1;
    }
    if is_indexable {
        root.indexable_pages += 1;
    }
    if root.sample_urls.len() < 5 {
        root.sample_urls.push(full_url.to_string());
    }

    let segments: Vec<&str> = path_str.split('/').filter(|s| !s.is_empty()).collect();
    let mut curr = root;

    let mut current_path = String::new();
    for (idx, seg) in segments.iter().enumerate() {
        current_path.push('/');
        current_path.push_str(seg);

        let child_idx = if let Some(pos) = curr.children.iter().position(|c| c.name == *seg) {
            pos
        } else {
            let new_child = IcicleTreeNode {
                name: seg.to_string(),
                full_path: current_path.clone(),
                weight_value: 0.0,
                error_count: 0,
                total_pages: 0,
                indexable_pages: 0,
                depth: idx + 1,
                children: Vec::new(),
                sample_urls: Vec::new(),
            };
            curr.children.push(new_child);
            curr.children.len() - 1
        };

        curr = &mut curr.children[child_idx];
        curr.weight_value += weight;
        curr.total_pages += 1;
        if is_error {
            curr.error_count += 1;
        }
        if is_indexable {
            curr.indexable_pages += 1;
        }
        if curr.sample_urls.len() < 5 {
            curr.sample_urls.push(full_url.to_string());
        }
    }
}

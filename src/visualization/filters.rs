use regex::Regex;
use std::collections::HashSet;

use super::snapshot::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FilterMode {
    Focus,   // Dim non-matching nodes
    Hide,    // Hide non-matching nodes but keep layout positions fixed
    Isolate, // Create sub-graph layout for matching nodes only
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GraphFilter {

    pub mode: FilterMode,
    pub section_id: Option<SectionId>,
    pub search_query: String,
    pub include_regex: Option<String>,
    pub exclude_regex: Option<String>,
    pub min_status_code: Option<u16>,
    pub max_status_code: Option<u16>,
    pub only_indexable: bool,
    pub only_orphans: bool,
    pub min_internal_rank: Option<f32>,
    pub max_crawl_depth: Option<u16>,
}

impl Default for FilterMode {
    fn default() -> Self {
        FilterMode::Focus
    }
}

pub struct FilterResult {
    pub matching_node_ids: HashSet<NodeId>,
    pub matching_edge_indices: HashSet<usize>,
}

impl GraphFilter {
    pub fn apply(&self, snapshot: &GraphSnapshot) -> FilterResult {
        let mut matching_node_ids = HashSet::new();

        let inc_re = self
            .include_regex
            .as_ref()
            .and_then(|pat| Regex::new(pat).ok());
        let exc_re = self
            .exclude_regex
            .as_ref()
            .and_then(|pat| Regex::new(pat).ok());
        let query_lower = self.search_query.trim().to_lowercase();

        for node in snapshot.nodes.iter() {
            let url = snapshot.node_url(node);

            if let Some(sec_id) = self.section_id {
                if node.section_id != sec_id {
                    continue;
                }
            }

            if !query_lower.is_empty() && !url.to_lowercase().contains(&query_lower) {
                continue;
            }

            if let Some(ref re) = inc_re {
                if !re.is_match(url) {
                    continue;
                }
            }

            if let Some(ref re) = exc_re {
                if re.is_match(url) {
                    continue;
                }
            }

            if let Some(min_code) = self.min_status_code {
                if node.status_code < min_code {
                    continue;
                }
            }

            if let Some(max_code) = self.max_status_code {
                if node.status_code > max_code {
                    continue;
                }
            }

            if self.only_indexable && node.indexability != Indexability::Indexable {
                continue;
            }

            if self.only_orphans && node.unique_in_degree > 0 {
                continue;
            }

            if let Some(min_rank) = self.min_internal_rank {
                if node.internal_rank < min_rank {
                    continue;
                }
            }

            if let Some(max_d) = self.max_crawl_depth {
                if node.crawl_depth > max_d {
                    continue;
                }
            }

            matching_node_ids.insert(node.id);
        }

        let mut matching_edge_indices = HashSet::new();

        for (idx, edge) in snapshot.edges.iter().enumerate() {
            let src_matches = matching_node_ids.contains(&edge.source);
            let tgt_matches = matching_node_ids.contains(&edge.target);

            match self.mode {
                FilterMode::Focus => {
                    if src_matches || tgt_matches {
                        matching_edge_indices.insert(idx);
                    }
                }
                FilterMode::Hide | FilterMode::Isolate => {
                    if src_matches && tgt_matches {
                        matching_edge_indices.insert(idx);
                    }
                }
            }
        }

        FilterResult {
            matching_node_ids,
            matching_edge_indices,
        }
    }
}

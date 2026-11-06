use std::collections::HashMap;
use std::sync::Arc;

pub type NodeId = u32;
pub type SectionId = u32;
pub type StringId = u32;

/// Interned String Repository to eliminate duplicate string allocations.
#[derive(Debug, Clone, Default)]
pub struct StringTable {
    strings: Vec<String>,
    map: HashMap<String, StringId>,
}

impl StringTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.map.get(s) {
            id
        } else {
            let id = self.strings.len() as StringId;
            self.strings.push(s.to_string());
            self.map.insert(s.to_string(), id);
            id
        }
    }

    pub fn get(&self, id: StringId) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Indexability {
    Indexable,
    Noindex,
    Canonicalised,
    BlockedRobots,
    Redirect,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CanonicalState {
    SelfCanonical,
    CanonicalisedToOther,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ContentType {
    Html,
    Image,
    Pdf,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FollowState {
    Follow,
    Nofollow,
    Sponsored,
    Ugc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LinkPlacement {
    HeaderNav,
    MainContent,
    Footer,
    Sidebar,
    Unknown,
}

/// Compact node data structure representing a single page/URL in the site graph.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SiteNode {
    pub id: NodeId,
    pub url_id: StringId,
    pub path_id: StringId,
    pub status_code: u16,
    pub crawl_depth: u16,
    pub path_depth: u16,
    pub in_degree: u32,
    pub out_degree: u32,
    pub unique_in_degree: u32,
    pub unique_out_degree: u32,
    pub internal_rank: f32,
    pub response_time_ms: u32,
    pub page_size_bytes: u64,
    pub word_count: u32,
    pub section_id: SectionId,
    pub issue_mask: u64,
    pub indexability: Indexability,
    pub canonical_state: CanonicalState,
    pub content_type: ContentType,
}

/// Aggregated graph edge between Source Node -> Target Node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SiteEdge {
    pub source: NodeId,
    pub target: NodeId,
    pub count: u16,
    pub flags: u8,
    pub follow_state: FollowState,
    pub placement: LinkPlacement,
    pub anchor_id: Option<StringId>,
}

/// Domain Section / Directory Group definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SiteSection {
    pub id: SectionId,
    pub name_id: StringId,
    pub path_prefix_id: StringId,
    pub parent_id: Option<SectionId>,
    pub node_count: u32,
    pub total_page_size: u64,
    pub total_word_count: u64,
    pub error_count: u32,
    pub avg_depth: f32,
    pub avg_internal_rank: f32,
}

/// Overall Graph Summary Metrics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GraphMetrics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub indexable_count: usize,
    pub noindex_count: usize,
    pub redirect_count: usize,
    pub error_count: usize,
    pub orphan_count: usize,
    pub max_crawl_depth: usize,
    pub max_in_degree: u32,
    pub max_out_degree: u32,
    pub weakly_connected_components: usize,
}

/// Complete, immutable snapshot of the site graph using Compressed Sparse Row (CSR) representation.
#[derive(Debug, Clone)]
pub struct GraphSnapshot {
    pub nodes: Arc<[SiteNode]>,
    pub edges: Arc<[SiteEdge]>,
    pub outgoing_offsets: Arc<[u32]>,
    pub outgoing_targets: Arc<[NodeId]>,
    pub incoming_offsets: Arc<[u32]>,
    pub incoming_sources: Arc<[NodeId]>,
    pub sections: Arc<[SiteSection]>,
    pub strings: Arc<StringTable>,
    pub metrics: GraphMetrics,
}

impl GraphSnapshot {
    pub fn get_outgoing_neighbors(&self, node_id: NodeId) -> &[NodeId] {
        let idx = node_id as usize;
        if idx + 1 < self.outgoing_offsets.len() {
            let start = self.outgoing_offsets[idx] as usize;
            let end = self.outgoing_offsets[idx + 1] as usize;
            &self.outgoing_targets[start..end]
        } else {
            &[]
        }
    }

    pub fn get_incoming_neighbors(&self, node_id: NodeId) -> &[NodeId] {
        let idx = node_id as usize;
        if idx + 1 < self.incoming_offsets.len() {
            let start = self.incoming_offsets[idx] as usize;
            let end = self.incoming_offsets[idx + 1] as usize;
            &self.incoming_sources[start..end]
        } else {
            &[]
        }
    }

    pub fn node_url(&self, node: &SiteNode) -> &str {
        self.strings.get(node.url_id).unwrap_or("")
    }

    pub fn find_shortest_path_to_root(&self, target_id: NodeId) -> Vec<NodeId> {
        if target_id as usize >= self.nodes.len() {
            return Vec::new();
        }
        if target_id == 0 {
            return vec![target_id];
        }

        use std::collections::VecDeque;
        let mut queue = VecDeque::new();
        let mut parent = vec![None; self.nodes.len()];
        let mut visited = vec![false; self.nodes.len()];

        queue.push_back(target_id);
        visited[target_id as usize] = true;

        let mut found_root = None;

        while let Some(curr) = queue.pop_front() {
            let curr_node = &self.nodes[curr as usize];
            if curr == 0 || curr_node.crawl_depth == 0 {
                found_root = Some(curr);
                break;
            }

            for &inc in self.get_incoming_neighbors(curr) {
                let inc_idx = inc as usize;
                if inc_idx < visited.len() && !visited[inc_idx] {
                    visited[inc_idx] = true;
                    parent[inc_idx] = Some(curr);
                    queue.push_back(inc);
                }
            }
        }

        if let Some(mut curr) = found_root {
            let mut path = vec![curr];
            while let Some(next) = parent[curr as usize] {
                path.push(next);
                curr = next;
            }
            path
        } else {
            vec![target_id]
        }
    }
}

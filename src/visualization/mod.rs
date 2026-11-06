pub mod builder;
pub mod cache;
pub mod explorer;
pub mod export;
pub mod filters;
pub mod icicle;
pub mod layout;
pub mod pagerank;
pub mod protocol;
pub mod renderer;
pub mod sections;
pub mod snapshot;
pub mod synthetic;

use std::sync::Arc;

pub use builder::SnapshotBuilder;
pub use cache::{LayoutCache, compute_graph_fingerprint};
pub use filters::{FilterMode, FilterResult, GraphFilter};
pub use icicle::{
    IcicleTreeNode, IcicleWeightMetric, build_icicle_tree_from_results,
    build_icicle_tree_from_snapshot,
};
pub use layout::{LayoutResult, LayoutType, NodePosition, compute_layout};
pub use pagerank::{PageRankConfig, calculate_internal_rank};
pub use protocol::{PROTOCOL_VERSION, RustToRendererMessage};
pub use renderer::{CairoCanvasRenderer, VisualizationRenderer};
pub use sections::{SectionSummary, SupernodeGraph, build_supernode_graph};
pub use snapshot::{
    GraphMetrics, GraphSnapshot, NodeId, SectionId, SiteEdge, SiteNode, StringTable,
};

pub struct DomainVisualizationModule {
    snapshot: Option<GraphSnapshot>,
    layout_cache: LayoutCache,
    current_filter: GraphFilter,
    selected_node: Option<NodeId>,
}

impl Default for DomainVisualizationModule {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainVisualizationModule {
    pub fn new() -> Self {
        Self {
            snapshot: None,
            layout_cache: LayoutCache::new(),
            current_filter: GraphFilter::default(),
            selected_node: None,
        }
    }

    pub fn load_snapshot(&mut self, snapshot: GraphSnapshot) {
        self.layout_cache.clear();
        self.selected_node = None;
        self.snapshot = Some(snapshot);
    }

    pub fn get_snapshot(&self) -> Option<&GraphSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn compute_internal_rank(&mut self, config: &PageRankConfig) -> Vec<f32> {
        if let Some(ref snapshot) = self.snapshot {
            let ranks = calculate_internal_rank(snapshot, config);

            // If snapshot is mutable or can be cloned, update ranks
            if let Some(mut_snapshot) = self.snapshot.as_mut() {
                let mut nodes: Vec<SiteNode> = mut_snapshot.nodes.iter().cloned().collect();
                for (idx, rank) in ranks.iter().enumerate() {
                    if idx < nodes.len() {
                        nodes[idx].internal_rank = *rank;
                    }
                }
                mut_snapshot.nodes = Arc::from(nodes);
            }

            ranks
        } else {
            Vec::new()
        }
    }

    pub fn get_layout(&mut self, layout_type: LayoutType, seed: u64) -> Option<LayoutResult> {
        let snapshot = self.snapshot.as_ref()?;
        let fp = compute_graph_fingerprint(
            snapshot.nodes.len(),
            snapshot.edges.len(),
            snapshot.metrics.max_crawl_depth,
        );

        if let Some(cached) = self.layout_cache.get(fp, layout_type, seed) {
            return Some(cached.clone());
        }

        let result = compute_layout(snapshot, layout_type, seed);
        self.layout_cache
            .insert(fp, layout_type, seed, result.clone());
        Some(result)
    }

    pub fn apply_filter(&mut self, filter: GraphFilter) -> Option<FilterResult> {
        self.current_filter = filter;
        let snapshot = self.snapshot.as_ref()?;
        Some(self.current_filter.apply(snapshot))
    }

    pub fn set_selected_node(&mut self, node_id: Option<NodeId>) {
        self.selected_node = node_id;
    }

    pub fn selected_node(&self) -> Option<NodeId> {
        self.selected_node
    }
}

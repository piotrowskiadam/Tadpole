use std::collections::HashMap;

use super::layout::{LayoutResult, LayoutType};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayoutCacheKey {
    pub graph_fingerprint: u64,
    pub layout_name: String,
    pub seed: u64,
    pub parameters: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LayoutCache {
    entries: HashMap<String, LayoutResult>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn make_key(fingerprint: u64, layout: LayoutType, seed: u64) -> String {
        format!("{:x}_{:?}_{}", fingerprint, layout, seed)
    }

    pub fn get(&self, fingerprint: u64, layout: LayoutType, seed: u64) -> Option<&LayoutResult> {
        let key = Self::make_key(fingerprint, layout, seed);
        self.entries.get(&key)
    }

    pub fn insert(
        &mut self,
        fingerprint: u64,
        layout: LayoutType,
        seed: u64,
        result: LayoutResult,
    ) {
        let key = Self::make_key(fingerprint, layout, seed);
        self.entries.insert(key, result);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

pub fn compute_graph_fingerprint(num_nodes: usize, num_edges: usize, max_depth: usize) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    num_nodes.hash(&mut hasher);
    num_edges.hash(&mut hasher);
    max_depth.hash(&mut hasher);
    hasher.finish()
}

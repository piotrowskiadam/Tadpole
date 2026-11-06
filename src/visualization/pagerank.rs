use super::snapshot::{FollowState, GraphSnapshot, Indexability};

#[derive(Debug, Clone)]
pub struct PageRankConfig {
    pub damping_factor: f32,
    pub max_iterations: usize,
    pub tolerance: f32,
    pub exclude_nofollow: bool,
    pub exclude_non_indexable: bool,
}
impl Default for PageRankConfig {
    fn default() -> Self {
        Self {
            damping_factor: 0.85,
            max_iterations: 100,
            tolerance: 1e-5,
            exclude_nofollow: false,
            exclude_non_indexable: false,
        }
    }
}

/// Directed, unweighted PageRank. Excluded nodes have zero rank; active ranks sum to one.
/// Crawl snapshots currently mark all parsed edges Follow; the flag is honored for
/// callers that supply enriched snapshots with real rel attributes.
pub fn calculate_internal_rank(snapshot: &GraphSnapshot, config: &PageRankConfig) -> Vec<f32> {
    calculate_internal_rank_cancellable(snapshot, config, &|| false).unwrap()
}

pub fn calculate_internal_rank_cancellable(
    snapshot: &GraphSnapshot,
    config: &PageRankConfig,
    cancelled: &impl Fn() -> bool,
) -> Option<Vec<f32>> {
    let count = snapshot.nodes.len();
    let active: Vec<_> = snapshot
        .nodes
        .iter()
        .map(|node| !config.exclude_non_indexable || node.indexability == Indexability::Indexable)
        .collect();
    let active_count = active.iter().filter(|&&value| value).count();
    if active_count == 0 {
        return Some(vec![0.0; count]);
    }
    let mut adjacency = vec![Vec::new(); count];
    for (i, edge) in snapshot.edges.iter().enumerate() {
        if i % 2048 == 0 && cancelled() {
            return None;
        }
        let source = edge.source as usize;
        let target = edge.target as usize;
        if source < count
            && target < count
            && active[source]
            && active[target]
            && (!config.exclude_nofollow || edge.follow_state == FollowState::Follow)
        {
            adjacency[source].push(target);
        }
    }
    let n = active_count as f32;
    let mut ranks: Vec<f32> = active
        .iter()
        .map(|&a| if a { 1.0 / n } else { 0.0 })
        .collect();
    let mut next = vec![0.0; count];
    let damping = if config.damping_factor.is_finite() {
        config.damping_factor.clamp(0.0, 1.0)
    } else {
        0.85
    };
    for _ in 0..config.max_iterations {
        if cancelled() {
            return None;
        }
        let dangling: f32 = ranks
            .iter()
            .enumerate()
            .filter(|(i, _)| active[*i] && adjacency[*i].is_empty())
            .map(|(_, rank)| rank)
            .sum();
        let base = (1.0 - damping + damping * dangling) / n;
        for (i, value) in next.iter_mut().enumerate() {
            *value = if active[i] { base } else { 0.0 };
        }
        for (source, neighbors) in adjacency
            .iter()
            .enumerate()
            .filter(|(_, neighbors)| !neighbors.is_empty())
        {
            let share = damping * ranks[source] / neighbors.len() as f32;
            for &target in neighbors {
                next[target] += share;
            }
        }
        let difference: f32 = next.iter().zip(&ranks).map(|(a, b)| (a - b).abs()).sum();
        std::mem::swap(&mut ranks, &mut next);
        if difference < config.tolerance {
            break;
        }
    }
    let sum: f32 = ranks.iter().sum();
    if sum > 0.0 {
        for rank in &mut ranks {
            *rank /= sum;
        }
    }
    Some(ranks)
}

use super::snapshot::GraphSnapshot;

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

/// Calculate directed PageRank ("Internal Rank") score for every node in the graph snapshot.
pub fn calculate_internal_rank(snapshot: &GraphSnapshot, config: &PageRankConfig) -> Vec<f32> {
    let num_nodes = snapshot.nodes.len();
    if num_nodes == 0 {
        return Vec::new();
    }

    let n = num_nodes as f32;
    let initial_rank = 1.0 / n;
    let mut current_ranks = vec![initial_rank; num_nodes];
    let mut next_ranks = vec![0.0f32; num_nodes];

    // Compute out-degrees for PageRank calculation
    let mut effective_out_degrees = vec![0u32; num_nodes];

    for (node_id, node) in snapshot.nodes.iter().enumerate() {
        if config.exclude_non_indexable && node.indexability != super::snapshot::Indexability::Indexable {
            continue;
        }

        let neighbors = snapshot.get_outgoing_neighbors(node_id as u32);
        let mut count = 0u32;
        for &target_id in neighbors {
            let target_node = &snapshot.nodes[target_id as usize];
            if config.exclude_non_indexable && target_node.indexability != super::snapshot::Indexability::Indexable {
                continue;
            }
            count += 1;
        }
        effective_out_degrees[node_id] = count;
    }

    let d = config.damping_factor;
    let base_rank = (1.0 - d) / n;

    for _iter in 0..config.max_iterations {
        // Step 1: Calculate total rank accumulated from dangling nodes (out-degree = 0)
        let mut dangling_sum = 0.0f32;
        for i in 0..num_nodes {
            if effective_out_degrees[i] == 0 {
                dangling_sum += current_ranks[i];
            }
        }
        let dangling_contribution = d * (dangling_sum / n);

        // Step 2: Distribute rank to incoming target nodes
        for i in 0..num_nodes {
            next_ranks[i] = base_rank + dangling_contribution;
        }

        for src_id in 0..num_nodes {
            let out_deg = effective_out_degrees[src_id];
            if out_deg > 0 {
                let share = d * (current_ranks[src_id] / out_deg as f32);
                let neighbors = snapshot.get_outgoing_neighbors(src_id as u32);

                for &tgt_id in neighbors {
                    let tgt_idx = tgt_id as usize;
                    if config.exclude_non_indexable && snapshot.nodes[tgt_idx].indexability != super::snapshot::Indexability::Indexable {
                        continue;
                    }
                    next_ranks[tgt_idx] += share;
                }
            }
        }

        // Step 3: Check convergence diff
        let mut diff = 0.0f32;
        for i in 0..num_nodes {
            diff += (next_ranks[i] - current_ranks[i]).abs();
        }

        current_ranks.copy_from_slice(&next_ranks);

        if diff < config.tolerance {
            break;
        }
    }

    // Normalize rank so values sum to 1.0 (or match total node average)
    let total_sum: f32 = current_ranks.iter().sum();
    if total_sum > 0.0 {
        for rank in &mut current_ranks {
            *rank /= total_sum;
        }
    }

    current_ranks
}

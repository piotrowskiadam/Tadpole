use std::collections::HashMap;
use super::snapshot::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LayoutType {
    ForceAtlas2,
    RadialDepth,
    SectionFirst,
    Hierarchical,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodePosition {
    pub node_id: NodeId,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LayoutResult {
    pub layout_type: LayoutType,
    pub positions: Vec<NodePosition>,
}

pub fn compute_layout(snapshot: &GraphSnapshot, layout_type: LayoutType, seed: u64) -> LayoutResult {
    let positions = match layout_type {
        LayoutType::ForceAtlas2 => compute_force_atlas_2(snapshot, seed),
        LayoutType::RadialDepth => compute_radial_depth(snapshot),
        LayoutType::SectionFirst => compute_section_first(snapshot, seed),
        LayoutType::Hierarchical => compute_hierarchical(snapshot),
    };

    LayoutResult {
        layout_type,
        positions,
    }
}

fn compute_force_atlas_2(snapshot: &GraphSnapshot, seed: u64) -> Vec<NodePosition> {
    let num_nodes = snapshot.nodes.len();
    if num_nodes == 0 {
        return Vec::new();
    }

    let mut pos_x = vec![0.0f32; num_nodes];
    let mut pos_y = vec![0.0f32; num_nodes];

    // Deterministic pseudo-random initialization
    let mut rng_state = seed.wrapping_add(0x9E3779B97F4A7C15);
    for i in 0..num_nodes {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let rx = ((rng_state >> 32) as f32 / 4294967295.0) * 800.0 - 400.0;
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let ry = ((rng_state >> 32) as f32 / 4294967295.0) * 800.0 - 400.0;

        pos_x[i] = rx;
        pos_y[i] = ry;
    }

    // ForceAtlas2 iteration loops
    let iterations = if num_nodes > 10000 { 20 } else { 60 };
    let gravity = 0.05f32;
    let k = 50.0f32;

    for _iter in 0..iterations {
        let mut disp_x = vec![0.0f32; num_nodes];
        let mut disp_y = vec![0.0f32; num_nodes];

        // Repulsion between node pairs (sampled for speed on large graphs)
        let step = if num_nodes > 2000 { num_nodes / 500 } else { 1 };

        for i in 0..num_nodes {
            for j in (i + 1..num_nodes).step_by(step) {
                let dx = pos_x[i] - pos_x[j];
                let dy = pos_y[i] - pos_y[j];
                let dist_sq = dx * dx + dy * dy + 0.01;
                let dist = dist_sq.sqrt();
                let force = (k * k) / dist;

                let fx = (dx / dist) * force;
                let fy = (dy / dist) * force;

                disp_x[i] += fx;
                disp_y[i] += fy;
                disp_x[j] -= fx;
                disp_y[j] -= fy;
            }

            // Gravity towards center (0, 0)
            let dist_center = (pos_x[i] * pos_x[i] + pos_y[i] * pos_y[i]).sqrt() + 0.01;
            disp_x[i] -= (pos_x[i] / dist_center) * gravity * dist_center;
            disp_y[i] -= (pos_y[i] / dist_center) * gravity * dist_center;
        }

        // Attraction along edges
        for edge in snapshot.edges.iter() {
            let s = edge.source as usize;
            let t = edge.target as usize;
            if s < num_nodes && t < num_nodes {
                let dx = pos_x[s] - pos_x[t];
                let dy = pos_y[s] - pos_y[t];
                let dist = (dx * dx + dy * dy).sqrt() + 0.01;
                let force = (dist * dist) / k;

                let fx = (dx / dist) * force;
                let fy = (dy / dist) * force;

                disp_x[s] -= fx;
                disp_y[s] -= fy;
                disp_x[t] += fx;
                disp_y[t] += fy;
            }
        }

        // Apply displacement with damping
        let temp = 10.0f32 / ((_iter + 1) as f32).sqrt();
        for i in 0..num_nodes {
            let d_len = (disp_x[i] * disp_x[i] + disp_y[i] * disp_y[i]).sqrt() + 0.01;
            let limit = temp.min(d_len);

            pos_x[i] += (disp_x[i] / d_len) * limit;
            pos_y[i] += (disp_y[i] / d_len) * limit;
        }
    }

    snapshot
        .nodes
        .iter()
        .map(|node| NodePosition {
            node_id: node.id,
            x: pos_x[node.id as usize],
            y: pos_y[node.id as usize],
        })
        .collect()
}

fn compute_radial_depth(snapshot: &GraphSnapshot) -> Vec<NodePosition> {
    let mut depth_groups: HashMap<u16, Vec<NodeId>> = HashMap::new();
    for node in snapshot.nodes.iter() {
        depth_groups.entry(node.crawl_depth).or_default().push(node.id);
    }

    let mut result = Vec::with_capacity(snapshot.nodes.len());
    let base_radius = 120.0f32;

    for (depth, nodes) in depth_groups {
        let radius = depth as f32 * base_radius;
        let count = nodes.len();

        for (idx, &node_id) in nodes.iter().enumerate() {
            let angle = (idx as f32 / count as f32) * std::f32::consts::TAU;
            let x = radius * angle.cos();
            let y = radius * angle.sin();

            result.push(NodePosition { node_id, x, y });
        }
    }

    result.sort_by_key(|p| p.node_id);
    result
}

fn compute_section_first(snapshot: &GraphSnapshot, seed: u64) -> Vec<NodePosition> {
    let num_sections = snapshot.sections.len();
    let mut section_centers = HashMap::new();
    let sec_radius = (num_sections as f32).sqrt() * 300.0;

    for (idx, sec) in snapshot.sections.iter().enumerate() {
        let angle = (idx as f32 / num_sections as f32) * std::f32::consts::TAU;
        let cx = sec_radius * angle.cos();
        let cy = sec_radius * angle.sin();
        section_centers.insert(sec.id, (cx, cy));
    }

    let mut section_node_counts: HashMap<SectionId, u32> = HashMap::new();

    let mut result = Vec::with_capacity(snapshot.nodes.len());

    let mut rng_state = seed;
    for node in snapshot.nodes.iter() {
        let (cx, cy) = section_centers.get(&node.section_id).cloned().unwrap_or((0.0, 0.0));
        let count = section_node_counts.entry(node.section_id).or_insert(0);
        *count += 1;

        let local_radius = (*count as f32).sqrt() * 25.0;
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let angle = ((rng_state >> 32) as f32 / 4294967295.0) * std::f32::consts::TAU;

        let x = cx + local_radius * angle.cos();
        let y = cy + local_radius * angle.sin();

        result.push(NodePosition {
            node_id: node.id,
            x,
            y,
        });
    }

    result.sort_by_key(|p| p.node_id);
    result
}

fn compute_hierarchical(snapshot: &GraphSnapshot) -> Vec<NodePosition> {
    let mut depth_groups: HashMap<u16, Vec<NodeId>> = HashMap::new();
    for node in snapshot.nodes.iter() {
        depth_groups.entry(node.crawl_depth).or_default().push(node.id);
    }

    let mut result = Vec::with_capacity(snapshot.nodes.len());
    let level_height = 150.0f32;

    for (depth, nodes) in depth_groups {
        let y = depth as f32 * level_height;
        let count = nodes.len();
        let total_width = count as f32 * 80.0;
        let start_x = -total_width / 2.0;

        for (idx, &node_id) in nodes.iter().enumerate() {
            let x = start_x + (idx as f32 * 80.0);
            result.push(NodePosition { node_id, x, y });
        }
    }

    result.sort_by_key(|p| p.node_id);
    result
}

use std::collections::HashMap;

use super::snapshot::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SectionSummary {
    pub section_id: SectionId,
    pub name: String,
    pub path_prefix: String,
    pub node_count: u32,
    pub total_page_size: u64,
    pub total_word_count: u64,
    pub error_count: u32,
    pub indexable_count: u32,
    pub noindex_count: u32,
    pub avg_crawl_depth: f32,
    pub avg_internal_rank: f32,
    pub incoming_link_count: u32,
    pub outgoing_link_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupernodeGraph {
    pub sections: Vec<SectionSummary>,
    pub inter_section_edges: Vec<SupernodeEdge>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SupernodeEdge {
    pub source_section: SectionId,
    pub target_section: SectionId,
    pub link_weight: u32,
}

pub fn build_supernode_graph(snapshot: &GraphSnapshot) -> SupernodeGraph {
    let mut summaries: HashMap<SectionId, SectionSummary> = HashMap::new();

    for sec in snapshot.sections.iter() {
        let name = snapshot.strings.get(sec.name_id).unwrap_or("/").to_string();
        let path_prefix = snapshot.strings.get(sec.path_prefix_id).unwrap_or("/").to_string();

        summaries.insert(
            sec.id,
            SectionSummary {
                section_id: sec.id,
                name,
                path_prefix,
                node_count: 0,
                total_page_size: 0,
                total_word_count: 0,
                error_count: 0,
                indexable_count: 0,
                noindex_count: 0,
                avg_crawl_depth: 0.0,
                avg_internal_rank: 0.0,
                incoming_link_count: 0,
                outgoing_link_count: 0,
            },
        );
    }

    let mut depth_sums: HashMap<SectionId, f32> = HashMap::new();
    let mut rank_sums: HashMap<SectionId, f32> = HashMap::new();

    for node in snapshot.nodes.iter() {
        if let Some(summary) = summaries.get_mut(&node.section_id) {
            summary.node_count += 1;
            summary.total_page_size += node.page_size_bytes;
            summary.total_word_count += node.word_count as u64;

            if node.status_code >= 400 {
                summary.error_count += 1;
            }
            if node.indexability == Indexability::Indexable {
                summary.indexable_count += 1;
            } else if node.indexability == Indexability::Noindex {
                summary.noindex_count += 1;
            }

            *depth_sums.entry(node.section_id).or_insert(0.0) += node.crawl_depth as f32;
            *rank_sums.entry(node.section_id).or_insert(0.0) += node.internal_rank;
        }
    }

    for (sec_id, summary) in summaries.iter_mut() {
        if summary.node_count > 0 {
            let n = summary.node_count as f32;
            summary.avg_crawl_depth = depth_sums.get(sec_id).cloned().unwrap_or(0.0) / n;
            summary.avg_internal_rank = rank_sums.get(sec_id).cloned().unwrap_or(0.0) / n;
        }
    }

    // Inter-section edge aggregation
    let mut section_edge_weights: HashMap<(SectionId, SectionId), u32> = HashMap::new();

    for edge in snapshot.edges.iter() {
        let src_node = &snapshot.nodes[edge.source as usize];
        let tgt_node = &snapshot.nodes[edge.target as usize];

        let src_sec = src_node.section_id;
        let tgt_sec = tgt_node.section_id;

        if src_sec != tgt_sec {
            *section_edge_weights.entry((src_sec, tgt_sec)).or_insert(0) += edge.count as u32;

            if let Some(src_sum) = summaries.get_mut(&src_sec) {
                src_sum.outgoing_link_count += edge.count as u32;
            }
            if let Some(tgt_sum) = summaries.get_mut(&tgt_sec) {
                tgt_sum.incoming_link_count += edge.count as u32;
            }
        }
    }

    let inter_section_edges = section_edge_weights
        .into_iter()
        .map(|((source_section, target_section), link_weight)| SupernodeEdge {
            source_section,
            target_section,
            link_weight,
        })
        .collect();

    SupernodeGraph {
        sections: summaries.into_values().collect(),
        inter_section_edges,
    }
}

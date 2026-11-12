use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use url::Url;

use super::snapshot::*;
use crate::state::CrawlResult;

pub struct SnapshotBuilder {
    strings: StringTable,
    url_to_node_id: HashMap<String, NodeId>,
    raw_nodes: Vec<RawNode>,
    raw_edges: Vec<RawEdge>,
    sections_map: HashMap<String, SectionId>,
    sections: Vec<SiteSection>,
}

struct RawNode {
    url: String,
    status_code: u16,
    crawl_depth: u16,
    path_depth: u16,
    response_time_ms: u32,
    page_size_bytes: u64,
    word_count: u32,
    section_id: SectionId,
    issue_mask: u64,
    indexability: Indexability,
    canonical_state: CanonicalState,
    content_type: ContentType,
}

struct RawEdge {
    source: NodeId,
    target: NodeId,
    count: u16,
    follow_state: FollowState,
    placement: LinkPlacement,
    anchor: Option<String>,
}

impl Default for SnapshotBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotBuilder {
    pub fn new() -> Self {
        let mut builder = Self {
            strings: StringTable::new(),
            url_to_node_id: HashMap::new(),
            raw_nodes: Vec::new(),
            raw_edges: Vec::new(),
            sections_map: HashMap::new(),
            sections: Vec::new(),
        };

        // Pre-insert root section
        let root_name_id = builder.strings.intern("/");
        let root_prefix_id = builder.strings.intern("/");
        builder.sections_map.insert("/".to_string(), 0);
        builder.sections.push(SiteSection {
            id: 0,
            name_id: root_name_id,
            path_prefix_id: root_prefix_id,
            parent_id: None,
            node_count: 0,
            total_page_size: 0,
            total_word_count: 0,
            error_count: 0,
            avg_depth: 0.0,
            avg_internal_rank: 0.0,
        });

        builder
    }

    fn get_or_create_section(&mut self, path: &str) -> SectionId {
        let section_name = parse_section_from_path(path);
        if let Some(&id) = self.sections_map.get(&section_name) {
            id
        } else {
            let id = self.sections.len() as SectionId;
            let name_id = self.strings.intern(&section_name);
            let prefix_id = self
                .strings
                .intern(&format!("/{}/", section_name.trim_matches('/')));
            self.sections_map.insert(section_name, id);
            self.sections.push(SiteSection {
                id,
                name_id,
                path_prefix_id: prefix_id,
                parent_id: Some(0), // Default child of root
                node_count: 0,
                total_page_size: 0,
                total_word_count: 0,
                error_count: 0,
                avg_depth: 0.0,
                avg_internal_rank: 0.0,
            });
            id
        }
    }

    pub fn build_from_results(results: &[CrawlResult]) -> GraphSnapshot {
        Self::build_from_results_cancellable(results, &|| false).unwrap()
    }

    pub fn build_from_results_cancellable(
        results: &[CrawlResult],
        cancelled: &impl Fn() -> bool,
    ) -> Option<GraphSnapshot> {
        let mut builder = SnapshotBuilder::new();

        // Pass 1: Assign Node IDs and collect nodes
        for (i, res) in results.iter().enumerate() {
            if i % 256 == 0 && cancelled() {
                return None;
            }
            if builder.url_to_node_id.contains_key(&res.url) {
                continue;
            }

            let node_id = builder.raw_nodes.len() as NodeId;
            builder.url_to_node_id.insert(res.url.clone(), node_id);

            let (path_depth, path_str) = parse_url_path_info(&res.url);
            let section_id = builder.get_or_create_section(&path_str);

            let indexability = match res.indexability_status.as_str() {
                "Indexable" => Indexability::Indexable,
                "Noindex" => Indexability::Noindex,
                "Canonicalised" => Indexability::Canonicalised,
                "Blocked by Robots.txt" => Indexability::BlockedRobots,
                "Redirect" => Indexability::Redirect,
                _ => Indexability::Failed,
            };

            let canonical_state = if let Some(ref can) = res.canonical {
                if can == &res.url {
                    CanonicalState::SelfCanonical
                } else {
                    CanonicalState::CanonicalisedToOther
                }
            } else {
                CanonicalState::Missing
            };

            let mut issue_mask: u64 = 0;
            let status = res.status_code.unwrap_or(0);
            if status >= 400 {
                issue_mask |= 1 << 0; // HTTP Error
            }
            if (300..400).contains(&status) {
                issue_mask |= 1 << 1; // Redirect
            }
            if !res.indexable {
                issue_mask |= 1 << 2; // Non-indexable
            }
            if res.h1_count == 0 {
                issue_mask |= 1 << 3; // Missing H1
            }
            if res.title.is_none() {
                issue_mask |= 1 << 4; // Missing Title
            }
            if res.meta_desc.is_none() {
                issue_mask |= 1 << 5; // Missing Meta Description
            }

            builder.raw_nodes.push(RawNode {
                url: res.url.clone(),
                status_code: status,
                crawl_depth: u16::try_from(res.depth).unwrap_or(u16::MAX),
                path_depth,
                response_time_ms: res.response_time_ms,
                page_size_bytes: res.size_bytes as u64,
                word_count: res.word_count as u32,
                section_id,
                issue_mask,
                indexability,
                canonical_state,
                content_type: ContentType::Html,
            });
        }

        // Pass 2: Aggregate Edges (Source -> Target)
        let mut edge_counts: HashMap<(NodeId, NodeId), u16> = HashMap::new();

        for (i, res) in results.iter().enumerate() {
            if i % 256 == 0 && cancelled() {
                return None;
            }
            if let Some(&src_id) = builder.url_to_node_id.get(&res.url) {
                for (i, outlink) in res.outlinks.iter().enumerate() {
                    if i % 2048 == 0 && cancelled() {
                        return None;
                    }
                    if let Some(&tgt_id) = builder.url_to_node_id.get(outlink) {
                        let count = edge_counts.entry((src_id, tgt_id)).or_insert(0);
                        *count = count.saturating_add(1);
                    }
                }
            }
        }

        let mut sorted_edges: Vec<_> = edge_counts.into_iter().collect();
        sorted_edges.sort_by_key(|(pair, _)| *pair);
        for ((src_id, tgt_id), count) in sorted_edges {
            builder.raw_edges.push(RawEdge {
                source: src_id,
                target: tgt_id,
                count,
                follow_state: FollowState::Follow,
                placement: LinkPlacement::Unknown,
                anchor: None,
            });
        }

        // Pass 3: Build CSR Adjacency arrays & Degrees
        let num_nodes = builder.raw_nodes.len();
        let mut in_degrees = vec![0u32; num_nodes];
        let mut out_degrees = vec![0u32; num_nodes];
        let mut unique_in_degrees = vec![0u32; num_nodes];
        let mut unique_out_degrees = vec![0u32; num_nodes];

        let mut outgoing_adj: Vec<Vec<NodeId>> = vec![Vec::new(); num_nodes];
        let mut incoming_adj: Vec<Vec<NodeId>> = vec![Vec::new(); num_nodes];

        for edge in &builder.raw_edges {
            let src = edge.source as usize;
            let tgt = edge.target as usize;

            if src < num_nodes && tgt < num_nodes {
                outgoing_adj[src].push(edge.target);
                incoming_adj[tgt].push(edge.source);

                out_degrees[src] += edge.count as u32;
                in_degrees[tgt] += edge.count as u32;
                unique_out_degrees[src] += 1;
                unique_in_degrees[tgt] += 1;
            }
        }

        let mut outgoing_offsets = Vec::with_capacity(num_nodes + 1);
        let mut outgoing_targets = Vec::new();
        let mut curr_out_offset = 0u32;

        for neighbors in &outgoing_adj {
            outgoing_offsets.push(curr_out_offset);
            outgoing_targets.extend_from_slice(neighbors);
            curr_out_offset += neighbors.len() as u32;
        }
        outgoing_offsets.push(curr_out_offset);

        let mut incoming_offsets = Vec::with_capacity(num_nodes + 1);
        let mut incoming_sources = Vec::new();
        let mut curr_in_offset = 0u32;

        for sources in &incoming_adj {
            incoming_offsets.push(curr_in_offset);
            incoming_sources.extend_from_slice(sources);
            curr_in_offset += sources.len() as u32;
        }
        incoming_offsets.push(curr_in_offset);

        // Compute Graph Metrics & Update Section Stats
        let mut metrics = GraphMetrics {
            total_nodes: num_nodes,
            total_edges: builder.raw_edges.len(),
            ..Default::default()
        };

        let mut site_nodes = Vec::with_capacity(num_nodes);
        for (idx, raw) in builder.raw_nodes.iter().enumerate() {
            let url_id = builder.strings.intern(&raw.url);
            let path_id = builder.strings.intern(&parse_url_path_info(&raw.url).1);

            match raw.indexability {
                Indexability::Indexable => metrics.indexable_count += 1,
                Indexability::Noindex => metrics.noindex_count += 1,
                Indexability::Redirect => metrics.redirect_count += 1,
                _ => {}
            }

            if raw.status_code == 0 || raw.status_code >= 400 {
                metrics.error_count += 1;
            }

            if unique_in_degrees[idx] == 0 {
                metrics.orphan_count += 1;
            }

            if raw.crawl_depth != u16::MAX && raw.crawl_depth as usize > metrics.max_crawl_depth {
                metrics.max_crawl_depth = raw.crawl_depth as usize;
            }
            if unique_in_degrees[idx] > metrics.max_in_degree {
                metrics.max_in_degree = unique_in_degrees[idx];
            }
            if unique_out_degrees[idx] > metrics.max_out_degree {
                metrics.max_out_degree = unique_out_degrees[idx];
            }

            // Update section counts
            if (raw.section_id as usize) < builder.sections.len() {
                let sec = &mut builder.sections[raw.section_id as usize];
                sec.node_count += 1;
                sec.total_page_size += raw.page_size_bytes;
                sec.total_word_count += raw.word_count as u64;
                if raw.status_code >= 400 {
                    sec.error_count += 1;
                }
            }

            site_nodes.push(SiteNode {
                id: idx as NodeId,
                url_id,
                path_id,
                status_code: raw.status_code,
                crawl_depth: raw.crawl_depth,
                path_depth: raw.path_depth,
                in_degree: in_degrees[idx],
                out_degree: out_degrees[idx],
                unique_in_degree: unique_in_degrees[idx],
                unique_out_degree: unique_out_degrees[idx],
                internal_rank: 1.0 / (num_nodes.max(1) as f32), // Initial uniform rank
                response_time_ms: raw.response_time_ms,
                page_size_bytes: raw.page_size_bytes,
                word_count: raw.word_count,
                section_id: raw.section_id,
                issue_mask: raw.issue_mask,
                indexability: raw.indexability,
                canonical_state: raw.canonical_state,
                content_type: raw.content_type,
            });
        }

        let site_edges: Vec<SiteEdge> = builder
            .raw_edges
            .into_iter()
            .map(|e| SiteEdge {
                source: e.source,
                target: e.target,
                count: e.count,
                flags: 0,
                follow_state: e.follow_state,
                placement: e.placement,
                anchor_id: e.anchor.map(|a| builder.strings.intern(&a)),
            })
            .collect();

        // Calculate Weakly Connected Components count
        metrics.weakly_connected_components = calculate_wcc_count(num_nodes, &site_edges);

        Some(GraphSnapshot {
            nodes: site_nodes.into(),
            edges: site_edges.into(),
            outgoing_offsets: outgoing_offsets.into(),
            outgoing_targets: outgoing_targets.into(),
            incoming_offsets: incoming_offsets.into(),
            incoming_sources: incoming_sources.into(),
            sections: builder.sections.into(),
            strings: Arc::new(builder.strings),
            metrics,
        })
    }
}

fn parse_url_path_info(url_str: &str) -> (u16, String) {
    if let Ok(parsed) = Url::parse(url_str) {
        let path = parsed.path();
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        (segments.len() as u16, path.to_string())
    } else {
        (0, "/".to_string())
    }
}

fn parse_section_from_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}/", segments[0])
    }
}

fn calculate_wcc_count(num_nodes: usize, edges: &[SiteEdge]) -> usize {
    if num_nodes == 0 {
        return 0;
    }
    let mut parent: Vec<usize> = (0..num_nodes).collect();

    fn find(i: usize, parent: &mut [usize]) -> usize {
        let mut root = i;
        while root != parent[root] {
            root = parent[root];
        }
        let mut curr = i;
        while curr != root {
            let nxt = parent[curr];
            parent[curr] = root;
            curr = nxt;
        }
        root
    }

    fn union(i: usize, j: usize, parent: &mut [usize]) {
        let root_i = find(i, parent);
        let root_j = find(j, parent);
        if root_i != root_j {
            parent[root_i] = root_j;
        }
    }

    for edge in edges {
        let s = edge.source as usize;
        let t = edge.target as usize;
        if s < num_nodes && t < num_nodes {
            union(s, t, &mut parent);
        }
    }

    let mut unique_components = HashSet::new();
    for i in 0..num_nodes {
        unique_components.insert(find(i, &mut parent));
    }
    unique_components.len()
}

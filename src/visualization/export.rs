use super::snapshot::{GraphSnapshot, NodeId};
use std::io::Write;

pub fn export_nodes_to_csv(
    snapshot: &GraphSnapshot,
    file_path: &str,
) -> Result<(), std::io::Error> {
    let mut file = std::fs::File::create(file_path)?;
    writeln!(
        file,
        "NodeID,URL,StatusCode,CrawlDepth,PathDepth,InDegree,OutDegree,UniqueInDegree,UniqueOutDegree,InternalRank,Indexability,Section"
    )?;

    for node in snapshot.nodes.iter() {
        let url = snapshot.node_url(node);
        let sec_name = snapshot
            .sections
            .get(node.section_id as usize)
            .and_then(|s| snapshot.strings.get(s.name_id))
            .unwrap_or("/");

        writeln!(
            file,
            "{},\"{}\",{},{},{},{},{},{},{},{:.6},{:?},\"{}\"",
            node.id,
            url.replace('"', "\"\""),
            node.status_code,
            node.crawl_depth,
            node.path_depth,
            node.in_degree,
            node.out_degree,
            node.unique_in_degree,
            node.unique_out_degree,
            node.internal_rank,
            node.indexability,
            sec_name.replace('"', "\"\"")
        )?;
    }

    Ok(())
}

pub fn export_edges_to_csv(
    snapshot: &GraphSnapshot,
    file_path: &str,
) -> Result<(), std::io::Error> {
    let mut file = std::fs::File::create(file_path)?;
    writeln!(
        file,
        "SourceNodeID,SourceURL,TargetNodeID,TargetURL,LinkCount"
    )?;

    for edge in snapshot.edges.iter() {
        let src_node = &snapshot.nodes[edge.source as usize];
        let tgt_node = &snapshot.nodes[edge.target as usize];

        let src_url = snapshot.node_url(src_node);
        let tgt_url = snapshot.node_url(tgt_node);

        writeln!(
            file,
            "{},\"{}\",{},\"{}\",{}",
            edge.source,
            src_url.replace('"', "\"\""),
            edge.target,
            tgt_url.replace('"', "\"\""),
            edge.count
        )?;
    }

    Ok(())
}

pub fn export_subgraph_json(snapshot: &GraphSnapshot, selected_node_id: Option<NodeId>) -> String {
    let nodes_to_export: Vec<_> = if let Some(sel) = selected_node_id {
        let mut ids = vec![sel];
        let outgoing = snapshot.get_outgoing_neighbors(sel);
        ids.extend(outgoing.iter().copied().filter(|&id| id != sel));
        ids
    } else {
        (0..snapshot.nodes.len() as u32).collect()
    };

    let mut nodes_json = Vec::new();
    for &id in &nodes_to_export {
        let Some(node) = snapshot.nodes.get(id as usize) else {
            continue;
        };
        let url = snapshot.node_url(node);
        nodes_json.push(serde_json::json!({
            "id": node.id,
            "url": url,
            "status": node.status_code,
            "crawlDepth": node.crawl_depth,
            "internalRank": node.internal_rank,
            "inDegree": node.in_degree,
            "outDegree": node.out_degree
        }));
    }

    let nodes_to_export: std::collections::HashSet<_> = nodes_to_export.into_iter().collect();
    let mut edges_json = Vec::new();
    for edge in snapshot.edges.iter() {
        if nodes_to_export.contains(&edge.source) && nodes_to_export.contains(&edge.target) {
            edges_json.push(serde_json::json!({
                "source": edge.source,
                "target": edge.target,
                "count": edge.count
            }));
        }
    }

    serde_json::json!({
        "totalNodes": nodes_json.len(),
        "totalEdges": edges_json.len(),
        "nodes": nodes_json,
        "edges": edges_json
    })
    .to_string()
}

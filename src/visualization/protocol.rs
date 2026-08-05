use serde::{Deserialize, Serialize};

use super::snapshot::NodeId;

pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Messages sent from Rust Controller to Embedded Web Renderer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RustToRendererMessage {
    #[serde(rename = "initialize")]
    Initialize {
        theme: String,
        reduced_motion: bool,
        scale_profile: String,
    },
    #[serde(rename = "loadGraphMetadata")]
    LoadGraphMetadata {
        total_nodes: usize,
        total_edges: usize,
        sections: Vec<SectionMeta>,
        strings: Vec<String>,
    },
    #[serde(rename = "loadNodeChunk")]
    LoadNodeChunk {
        chunk_index: usize,
        total_chunks: usize,
        nodes: Vec<NodeWireFormat>,
    },
    #[serde(rename = "loadEdgeChunk")]
    LoadEdgeChunk {
        edges: Vec<EdgeWireFormat>,
    },
    #[serde(rename = "setLayout")]
    SetLayout {
        layout: String,
        positions: Vec<PositionWireFormat>,
    },
    #[serde(rename = "setFilter")]
    SetFilter {
        matching_node_ids: Vec<NodeId>,
        mode: String,
    },
    #[serde(rename = "setSelection")]
    SetSelection {
        selected_node_id: Option<NodeId>,
        focus_camera: bool,
    },
    #[serde(rename = "exportImage")]
    ExportImage {
        format: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionMeta {
    pub id: u32,
    pub name: String,
    pub node_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeWireFormat {
    pub id: NodeId,
    pub url_id: u32,
    pub status_code: u16,
    pub crawl_depth: u16,
    pub path_depth: u16,
    pub in_degree: u32,
    pub out_degree: u32,
    pub internal_rank: f32,
    pub section_id: u32,
    pub indexability: String,
    pub issue_mask: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeWireFormat {
    pub source: NodeId,
    pub target: NodeId,
    pub count: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionWireFormat {
    pub id: NodeId,
    pub x: f32,
    pub y: f32,
}

/// Messages sent from Embedded Web Renderer back to Rust Controller
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RendererToRustMessage {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "nodeHovered")]
    NodeHovered {
        node_id: Option<NodeId>,
    },
    #[serde(rename = "nodeSelected")]
    NodeSelected {
        node_id: Option<NodeId>,
    },
    #[serde(rename = "renderStats")]
    RenderStats {
        fps: f32,
        rendered_nodes: usize,
        rendered_edges: usize,
    },
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
    },
}

impl RustToRendererMessage {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl RendererToRustMessage {
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

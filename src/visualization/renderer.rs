use super::icicle::{IcicleTreeNode, IcicleWeightMetric};
use super::layout::LayoutResult;
use super::snapshot::GraphSnapshot;

pub trait VisualizationRenderer {
    fn render_link_map(&mut self, snapshot: &GraphSnapshot, layout: &LayoutResult);
    fn render_structure_map(&mut self, icicle_root: &IcicleTreeNode, weight_metric: IcicleWeightMetric);
    fn set_selected_node(&mut self, node_id: Option<u32>);
    fn set_hovered_node(&mut self, node_id: Option<u32>);
    fn set_zoom(&mut self, zoom: f64);
    fn set_pan(&mut self, pan_x: f64, pan_y: f64);
}

/// Fallback Native Cairo Canvas Renderer
pub struct CairoCanvasRenderer {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub selected_node: Option<u32>,
    pub hovered_node: Option<u32>,
}

impl Default for CairoCanvasRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl CairoCanvasRenderer {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            selected_node: None,
            hovered_node: None,
        }
    }
}

impl VisualizationRenderer for CairoCanvasRenderer {
    fn render_link_map(&mut self, _snapshot: &GraphSnapshot, _layout: &LayoutResult) {
        // Cairo drawing logic executed via GTK DrawingArea callback
    }

    fn render_structure_map(&mut self, _icicle_root: &IcicleTreeNode, _weight_metric: IcicleWeightMetric) {
        // Icicle plot Cairo rendering logic executed via GTK DrawingArea callback
    }

    fn set_selected_node(&mut self, node_id: Option<u32>) {
        self.selected_node = node_id;
    }

    fn set_hovered_node(&mut self, node_id: Option<u32>) {
        self.hovered_node = node_id;
    }

    fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom;
    }

    fn set_pan(&mut self, pan_x: f64, pan_y: f64) {
        self.pan_x = pan_x;
        self.pan_y = pan_y;
    }
}

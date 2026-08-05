# VISUALIZATION IMPLEMENTATION PLAN — Tadpole SEO Domain Visualization Module

## 1. Executive Summary & Context
This document outlines the architecture, data models, protocol specifications, and step-by-step implementation plan for the new **Domain Visualization Module** in **Tadpole** (a local desktop SEO crawler built with Rust, GTK4, and Libadwaita).

The goal of this module is to transform site visual analysis from a simple graph display into an interactive, high-performance SEO diagnostic suite that enables specialists to instantly identify:
- Key hubs and overloaded hub pages
- Orphaned or weakly linked pages
- Isolated domain sections & weak cluster inter-linking
- Deeply buried critical pages (crawl depth vs path depth discrepancies)
- Internal link PageRank ("Internal Rank") flow across subdomains and URL paths
- SEO issue concentrations and non-indexable content clusters
- Discrepancies between URL directory structure and actual internal linking architecture

---

## 2. Codebase Audit & Data Analysis

### 2.1 Current State of Tadpole Codebase
1. **Crawl Storage (`src/state.rs`)**:
   - `CrawlResult` holds individual page fields: `url`, `status_code`, `indexable`, `indexability_status`, `title`, `meta_desc`, `h1`, `word_count`, `size_bytes`, `canonical`, `inlinks: Vec<String>`, `outlinks: Vec<String>`, `depth`, `response_time_ms`, `headings`, `markdown`.
   - `CrawlState` wraps `RwLock<CrawlStateInner>` managing `urls: HashMap<String, CrawlResult>` and `inlinks_map: HashMap<String, HashSet<String>>`.
   - `CrawlProject` saves/loads seed URL, config, results, and stats to disk.

2. **Current Link Tracking**:
   - `outlinks`: Parsed during HTML crawl (`parse_html` in `src/crawler.rs`) by extracting `a[href]` attributes, stripping fragments, and resolving relative paths.
   - `inlinks`: Dynamically accumulated in `inlinks_map` when target URLs are discovered.

3. **Current Visualizer (`src/ui/site_visualizer.rs`)**:
   - Uses GTK `DrawingArea` and Cairo 2D vector primitives.
   - Implements simple directory trees and basic 3D point projections.
   - Limitations: Cairo canvas degrades on >1,000 nodes, lacks WebGL rendering, semantic zoom, CSR graph indexing, section aggregation, icicle hierarchical trees, or graph metrics like PageRank.

4. **Missing Data in Existing Model**:
   - **Anchor Text**: `a` tag inner text is not saved per outlink.
   - **Follow / Nofollow State**: `rel="nofollow"` / `rel="ugc/sponsored"` attributes are not currently recorded.
   - **Link Placement**: DOM location (Header, Nav, Main Content, Footer/Boilerplate) is not stored.
   - **Aggregated Link Weights**: Multiple links from Page A to Page B are currently stored as flat string arrays rather than weighted unique edges.

---

## 3. Data Architecture & Compact Snapshot (`GraphSnapshot`)

To support fast graph algorithms, multi-level section aggregation, and instant JS/Renderer transfer without memory bloating, the visualization module separates raw crawl results from a compact, indexed `GraphSnapshot`.

### 3.1 Data Structures (`src/visualization/snapshot.rs`)

```rust
pub type NodeId = u32;
pub type SectionId = u32;
pub type StringId = u32;

/// Interned String Repository to eliminate duplicate URL and path strings
pub struct StringTable { ... }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indexability {
    Indexable,
    Noindex,
    Canonicalised,
    BlockedRobots,
    Redirect,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalState {
    SelfCanonical,
    CanonicalisedToOther,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Html,
    Image,
    Pdf,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowState {
    Follow,
    Nofollow,
    Sponsored,
    Ugc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkPlacement {
    HeaderNav,
    MainContent,
    Footer,
    Sidebar,
    Unknown,
}

/// Compact node record (fixed memory size per page)
#[derive(Debug, Clone)]
pub struct SiteNode {
    pub id: NodeId,
    pub url_id: StringId,
    pub path_id: StringId,
    pub status_code: u16,
    pub crawl_depth: u16,
    pub path_depth: u16,
    pub in_degree: u32,
    pub out_degree: u32,
    pub unique_in_degree: u32,
    pub unique_out_degree: u32,
    pub internal_rank: f32,
    pub response_time_ms: u32,
    pub page_size_bytes: u64,
    pub word_count: u32,
    pub section_id: SectionId,
    pub issue_mask: u64,
    pub indexability: Indexability,
    pub canonical_state: CanonicalState,
    pub content_type: ContentType,
}

/// Aggregated graph edge between Source URL -> Target URL
#[derive(Debug, Clone)]
pub struct SiteEdge {
    pub source: NodeId,
    pub target: NodeId,
    pub count: u16,
    pub flags: u8,
    pub follow_state: FollowState,
    pub placement: LinkPlacement,
    pub anchor_id: Option<StringId>,
}

/// Domain Section / Directory Group
#[derive(Debug, Clone)]
pub struct SiteSection {
    pub id: SectionId,
    pub name_id: StringId,
    pub path_prefix_id: StringId,
    pub parent_id: Option<SectionId>,
    pub node_count: u32,
    pub total_page_size: u64,
    pub total_word_count: u64,
    pub error_count: u32,
    pub avg_depth: f32,
    pub avg_internal_rank: f32,
}

/// Complete Immutable Snapshot with Compressed Sparse Row (CSR) adjacency lists
#[derive(Debug, Clone)]
pub struct GraphSnapshot {
    pub nodes: Arc<[SiteNode]>,
    pub edges: Arc<[SiteEdge]>,
    pub outgoing_offsets: Arc<[u32]>,
    pub outgoing_targets: Arc<[NodeId]>,
    pub incoming_offsets: Arc<[u32]>,
    pub incoming_sources: Arc<[NodeId]>,
    pub sections: Arc<[SiteSection]>,
    pub strings: Arc<StringTable>,
    pub metrics: GraphMetrics,
}
```

---

## 4. Algorithms & Graph Metrics (`src/visualization/`)

### 4.1 Internal Rank (Directed PageRank)
- **Implementation**: `src/visualization/pagerank.rs`
- **Method**: Power iteration over directed CSR graph.
- **Parameters**:
  - Damping factor $d = 0.85$ (configurable 0.50 – 0.95).
  - Maximum iterations = 100.
  - Convergence threshold $\epsilon = 10^{-5}$.
- **Dangling Node Handling**: Nodes with out-degree 0 distribute their rank score equally among all nodes in the snapshot.
- **Options**: Ability to exclude `nofollow` edges or non-indexable nodes from rank distribution.
- **Execution**: Computes asynchronously via Tokio threadpool, never blocking the GTK UI loop.

### 4.2 Degree & Depth Metrics
- In-degree, unique in-degree, out-degree, unique out-degree.
- Path Depth vs Crawl Depth gap detection (highlights pages with deep directory URLs that are linked directly from root, or shallow URLs buried 10 clicks deep).
- Weakly Connected Components (WCC) to isolate disconnected site subgraphs.

### 4.3 Section Hierarchy & Aggregation (`sections.rs`)
- Parses normalized URL path segments (`/blog/`, `/products/`, `/category/sub/`).
- Aggregates node counts, issue rates, total byte sizes, word counts, and inter-section link flows into **Supernodes** for large domains.

---

## 5. Renderer Architecture & Protocol

### 5.1 Primary Renderers
1. **Link Map**:
   - **Sigma.js / WebGL / Graphology**: WebGL-accelerated 2D graph renderer for handling up to 100,000+ nodes and edges smoothly.
   - ForceAtlas2 layout with deterministic seeds, precomputed spatial quadtree bounds, and semantic zoom.
2. **Structure Map**:
   - **D3.js / SVG / Canvas**: Hierarchical Icicle Plot (primary view) showing nested URL directory structures, section sizes, and issue distributions. Treemap available as secondary view mode.
3. **Native GTK Fallback**:
   - High-performance Cairo/GTK canvas adapter ensuring complete fallback capability on systems without WebGL/WebView dependencies.

### 5.2 Embedded Webview Bridge (`src/visualization/webview.rs` & `renderer.rs`)
- Uses `wry` crate on supported platforms (WebKitGTK on Linux, WebView2 on Windows).
- Provides a clean abstract trait `VisualizationRenderer` so UI controller code is completely decoupled from rendering implementation.
- All web resources (Sigma.js, Graphology, D3.js, custom bundle scripts, CSS) are embedded directly into the binary with `include_str!` / `include_bytes!` — **Zero CDN, Zero HTTP server, Zero external network requests**.

---

## 6. User Interface & GNOME HIG Integration (`src/ui/visualization/`)

The visualization view integrates cleanly into Tadpole's GTK4 / Libadwaita layout:

- **Top Bar**: View selector switcher (Link Map | Structure Map | Section Flow), Layout combo (ForceAtlas2, Radial Depth, Section-First, Hierarchical), Metric sizing (Internal Rank, In-Degree, Page Size, Issues), Search bar, Reset Camera, Export button.
- **Left Sidebar (Collapsible)**: Filter controls (Status code, indexability, depth, section, issue type, regex).
- **Center Canvas**: Interactive WebGL / Canvas visualization area with pan, zoom, hover tooltips, and click selection.
- **Right Sidebar (Collapsible)**: Selected Node/Section details (URL, title, metrics, top incoming/outgoing links with anchor texts, issues, quick actions).
- **Bottom Status Bar**: Active filter summary, visible node/edge counter, LOD scale profile indicator.
- **Theme Synchronization**: Automatically syncs light / dark palette with Libadwaita style manager.

---

## 7. Scale Profiles & Domain Performance

| Scale Profile | URL Count | Default View | Edge Mode | Label Priority Strategy |
|---|---|---|---|---|
| **Small** | 1 – 1,000 | Individual Nodes | All Edges | Smart collision grid, high density |
| **Medium** | 1,000 – 10,000 | Individual Nodes | LOD filtered | Priority-based top 100 + selection |
| **Large** | 10,000 – 100,000 | Supernodes / Aggregated | Section Bridges | Section labels + selection focus |
| **Very Large** | 100,000+ | Section Directory Hierarchy | Aggregated Flows | Section headers + search focus |

---

## 8. Verification Plan & Test Strategy

### 8.1 Automated Unit & Integration Tests
- `tests/visualization_tests.rs`:
  - `test_graph_snapshot_building`: Verifies CSR adjacency offsets and interned strings.
  - `test_internal_rank_convergence`: Validates PageRank calculation on simple 5-node test graph and verify sum of rank = 1.0.
  - `test_section_hierarchy_parsing`: Verifies URL path parsing into section trees.
  - `test_synthetic_data_generator`: Validates test graph generation for 1k, 10k, and 100k node graphs.
- `cargo test` execution.

### 8.2 Manual & UI Verification
- Verify Link Map pan, zoom, click selection, search filtering, and detail panel updates.
- Verify Structure Map Icicle plot drill-down and breadcrumb navigation.
- Verify Light/Dark theme switching.
- Verify export functionality (PNG image export and CSV URL selection export).

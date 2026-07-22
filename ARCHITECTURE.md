# Tadpole Architecture & Developer Guide

This document provides a comprehensive technical overview of Tadpole's codebase architecture, data model, multi-threading patterns, UI state management, and visualization engine for AI agents and human developers.

---

## 1. Project Overview & Tech Stack

- **Language**: Rust (Edition 2024)
- **GUI Framework**: GTK4 (`gtk4` crate v0.11.3) + Libadwaita (`libadwaita` crate v0.9.1)
- **Async Runtime**: `tokio` (v1.52.3)
- **2D/3D Rendering Canvas**: `cairo-rs` (v0.22)
- **HTML Parsing & Extraction**: `scraper` (v0.27.0) + `ego-tree`
- **HTTP Client**: `reqwest` (v0.13.4, async with JSON support)

---

## 2. Codebase Directory Structure

```
Tadpole/
├── src/
│   ├── main.rs               # Application entrypoint & GTK Application initialization
│   ├── crawler.rs            # Multi-threaded async Tokio crawler & HTTP fetcher
│   ├── state.rs              # Shared CrawlState, CrawlResult, CrawlConfig, & issue tracking
│   ├── parser.rs             # HTML parsing, Schema JSON-LD extraction, OG tags, Headings tree
│   ├── markdown.rs           # Body text extraction & HTML-to-Markdown GFM converter
│   ├── link_opportunities.rs# Internal link opportunity & N-gram density analyzer
│   └── ui/
│       ├── mod.rs            # UI module declaration
│       ├── window.rs         # Main GTK window, HeaderBar, async file loader, ListStore streaming
│       ├── sidebar.rs        # Filter sidebar & crawl config settings drawer
│       ├── details.rs        # Bottom details notebook (SEO info, Social, Schema, Headings, Markdown)
│       ├── table.rs          # GTK ColumnView / ListStore for live crawl grid
│       ├── summary.rs        # SEO Health Score dashboard & depth distribution chart
│       ├── row_data.rs       # GObject wrapper (CrawlRowData) for GTK ListStore binding
│       └── site_visualizer.rs# Standalone 2D/3D Site Visualizer window (Directory Tree & TensorFlow 3D Projector)
├── snap/                     # Snapcraft configuration & snapcraft.yaml
├── .github/workflows/        # CI/CD pipelines (release.yml for AppImage, DEB, RPM, Snap, EXE)
└── Cargo.toml                # Dependencies & package metadata
```

---

## 3. Data Flow & Multi-Threading Model

```
 ┌────────────────┐     mpsc channel      ┌─────────────────┐
 │ Tokio Crawler  ├──────────────────────►│ GTK Main Loop   │
 │ (crawler.rs)   │  CrawlUpdate events   │ (glib::idle_add)│
 └───────┬────────┘                       └────────┬────────┘
         │                                         │
         ▼                                         ▼
 ┌────────────────┐                       ┌─────────────────┐
 │   CrawlState   │                       │  gio::ListStore │
 │ (parking_lot)  │◄──────────────────────┤ (gtk::ColumnView│
 └────────────────┘   Thread-safe read    └─────────────────┘
```

1. **Crawler Execution**: The Tokio crawler (`crawler.rs`) runs asynchronously in background tasks, spawning workers bounded by `CrawlConfig::max_concurrent`.
2. **State Mutability**: Results are inserted into `CrawlState` wrapped in `Arc<RwLock<CrawlState>>` (using `parking_lot::RwLock`).
3. **UI Streaming**: Live updates travel to the GTK UI thread via `tokio::sync::mpsc::unbounded_channel()`. The main window receives updates using `glib::idle_add_local` or Tokio async handlers, pushing batched chunks into `gio::ListStore` with `tokio::task::yield_now().await` to prevent UI freezing.

---

## 4. Visualizer Architecture (`site_visualizer.rs`)

The **Visual Site Map** is rendered inside a standalone `gtk::Window` (`VisualizerWindow`) containing a `gtk::DrawingArea` with a Cairo 2D/3D render func.

### Modes
1. **DirectoryTree (Recursive N-Level Tree)**:
   - Parses URLs into a recursive `DirectoryTreeNode` hierarchy.
   - Calculates horizontal column $X = \text{depth} \times 240.0$.
   - Draws organic tree branch connectors using Cairo cubic Bezier curves (`cr.curve_to`).
   - Supports interactive folder branch expansion/collapsing (`expanded_folders: HashSet<String>`).
2. **ThreeDConstellation (TensorFlow 3D Embedding Projector)**:
   - Projects 3D spatial coordinates $(X, Y, Z)$ to 2D screen coordinates using a 3D perspective camera matrix:
     $$x_{screen} = \frac{f \cdot x'}{z' + d} + \text{center}_x, \quad y_{screen} = \frac{f \cdot y'}{z' + d} + \text{center}_y$$
   - Supports 3D camera orbit rotation (mouse drag adjusts `yaw` and `pitch` angles) and dolly zoom (mouse scroll).
   - Applies Painter's Algorithm ($Z$-sorting back-to-front) and depth attenuation fog.

### Smart Level-of-Detail (LOD) Label Culling
To maintain 60+ FPS performance and crystal-clear legibility:
- **Major Hubs**: Only primary directory folders ($>3\%$ of total site pages) display text labels by default in `SmartAuto` mode.
- **3D Camera Proximity**: In 3D mode, text labels are rendered only for nodes positioned in the front camera plane (`proj_scale > 0.82`).
- **Hover & Selection**: Hovering over or clicking any node forces its text label to render immediately.

---

## 5. Non-Blocking File Opening Pattern

When opening saved `.seocrawl` JSON project files in `window.rs`:
- JSON deserialization executes off the main thread via `tokio::task::spawn_blocking`.
- UI updates are streamed into `gio::ListStore` in small batches (50 rows at a time) with `tokio::task::yield_now().await`, giving GTK time to process events and render UI frames smoothly without triggering OS "Not Responding" alerts.

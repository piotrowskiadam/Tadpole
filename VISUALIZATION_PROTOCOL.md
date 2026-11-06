> Historical design proposal, not a description of the shipped UI. The application currently uses a native Cairo icicle view; embedded web rendering and its IPC are not wired into the desktop application. See [ARCHITECTURE.md](ARCHITECTURE.md).

# VISUALIZATION PROTOCOL — Rust ↔ Renderer Communication Specification

## 1. Overview & Security Rules
This document specifies the bidirectional message protocol between the Rust backend (Tadpole Controller) and the embedded web frontend (Sigma.js / D3.js Renderer).

### Security Requirements:
1. **Zero External Requests**: No external CDNs or remote URLs are fetched. All assets are loaded from embedded local memory.
2. **Untrusted Data Handling**: URLs, Titles, Anchor text, and Headings are treated as untrusted text and strictly escaped / sanitized.
3. **Structured Messages**: All IPC communication occurs via structured JSON objects containing protocol version numbers. String-based JS `eval` calls are prohibited.

---

## 2. Protocol Version
`PROTOCOL_VERSION = "1.0.0"`

---

## 3. Rust → Renderer Messages

### 3.1 `initialize`
Sent when the renderer webview is loaded.
```json
{
  "protocol": "1.0.0",
  "type": "initialize",
  "payload": {
    "theme": "dark",
    "reducedMotion": false,
    "scaleProfile": "medium"
  }
}
```

### 3.2 `loadGraphMetadata`
Provides high-level graph metrics and section definitions before sending chunks.
```json
{
  "protocol": "1.0.0",
  "type": "loadGraphMetadata",
  "payload": {
    "totalNodes": 1250,
    "totalEdges": 8420,
    "sections": [
      { "id": 0, "name": "/", "nodeCount": 10 },
      { "id": 1, "name": "/blog/", "nodeCount": 450 }
    ],
    "strings": ["https://example.com/", "https://example.com/blog/"]
  }
}
```

### 3.3 `loadNodeChunk`
Sends a chunk of compact node representations.
```json
{
  "protocol": "1.0.0",
  "type": "loadNodeChunk",
  "payload": {
    "chunkIndex": 0,
    "totalChunks": 1,
    "nodes": [
      {
        "id": 0,
        "urlId": 0,
        "pathId": 1,
        "statusCode": 200,
        "crawlDepth": 1,
        "pathDepth": 1,
        "inDegree": 42,
        "outDegree": 15,
        "internalRank": 0.0452,
        "sectionId": 0,
        "indexability": "Indexable",
        "issueMask": 0
      }
    ]
  }
}
```

### 3.4 `loadEdgeChunk`
Sends graph edges in compact numeric ID pairs.
```json
{
  "protocol": "1.0.0",
  "type": "loadEdgeChunk",
  "payload": {
    "edges": [
      { "source": 0, "target": 1, "count": 3, "follow": true }
    ]
  }
}
```

### 3.5 `setLayout`
Changes the active visual layout mode.
```json
{
  "protocol": "1.0.0",
  "type": "setLayout",
  "payload": {
    "layout": "forceAtlas2",
    "params": {
      "iterations": 100,
      "gravity": 1.0,
      "barnesHut": true
    }
  }
}
```

### 3.6 `setFilter`
Applies filter criteria to highlight or isolate subgraphs.
```json
{
  "protocol": "1.0.0",
  "type": "setFilter",
  "payload": {
    "mode": "focus",
    "sectionId": 1,
    "searchQuery": "seo",
    "minInternalRank": 0.001,
    "statusCodes": [200, 301, 404]
  }
}
```

### 3.7 `setSelection`
Updates the selected node or section from GTK UI.
```json
{
  "protocol": "1.0.0",
  "type": "setSelection",
  "payload": {
    "selectedNodeId": 42,
    "focusCamera": true
  }
}
```

---

## 4. Renderer → Rust Messages

### 4.1 `ready`
Sent by JS frontend after initialization scripts complete.
```json
{
  "protocol": "1.0.0",
  "type": "ready"
}
```

### 4.2 `nodeHovered`
Sent when the user hovers over a node.
```json
{
  "protocol": "1.0.0",
  "type": "nodeHovered",
  "payload": {
    "nodeId": 42,
    "url": "https://example.com/blog/seo-guide"
  }
}
```

### 4.3 `nodeSelected`
Sent when a node is clicked in the visualization canvas.
```json
{
  "protocol": "1.0.0",
  "type": "nodeSelected",
  "payload": {
    "nodeId": 42,
    "url": "https://example.com/blog/seo-guide"
  }
}
```

### 4.4 `renderStats`
Reports FPS and rendering diagnostics.
```json
{
  "protocol": "1.0.0",
  "type": "renderStats",
  "payload": {
    "fps": 58.4,
    "renderedNodes": 1250,
    "renderedEdges": 3400
  }
}
```

### 4.5 `error`
Reports rendering or IPC errors back to Rust.
```json
{
  "protocol": "1.0.0",
  "type": "error",
  "payload": {
    "code": "LAYOUT_TIMEOUT",
    "message": "ForceAtlas2 calculation exceeded max iteration timeout."
  }
}
```

# Visualization design and loading plan

Status: **proposed**, 2026-11-01. [Open the self-contained design study](index.html). This change supplies research, six diagrams and a CPU benchmark harness; implementation of the proposed views and loading model is a subsequent step.

## Evidence and recommendation

[Eight first-party vendor sources](../research/seo-visualizations.md) distinguish URL containment, crawl paths and actual internal links. Screaming Frog and Sitebulb provide the strongest map precedents; Ahrefs and Semrush show the value of directory aggregates and direct chart/table drill-down. Public documentation does not establish their layout algorithms or transferable latency guarantees. Sitebulb's older/newer documentation conflicts on pruning and indexability; the research records both descriptions.

Keep the current icicle as one directory presentation. Introduce complementary views around specific questions, with one shared URL selection and explicit evidence tables. A section matrix and a depth/rank distribution are Tadpole proposals inferred from these workflows, not verified vendor features. Whole-site 3D and automatic global force layouts have lower priority than useful, bounded detail.

| View | Question | Relationship / measure | Initial display strategy |
|---|---|---|---|
| Directory area | Where are URLs or bytes concentrated? | Origin + URL-path containment; additive area weight | Squarified treemap alongside icicle; aggregate the small tail |
| Link-depth tree | How many links from the start reach a URL? | One deterministic BFS parent per reachable node | Three levels, expandable branches, collapsed counts |
| URL neighborhood | What links to/from this URL? | Directed actual source/target URL pairs | One hop; proposed 250-node / 500-edge cap |
| Section matrix | Which sections connect poorly? | Unique source/target URL pairs by source and target group | Proposed maximum 40 × 40 cells, remainder in Other |
| Depth + internal rank | Which influential pages are relatively deep? | Shortest-path depth and local rank share | Small-site points; bounded density bins for large crawls |

The static study uses 960 × 600 inline SVG canvases. Small example datasets keep each topology legible (eight tree nodes, seven neighborhood nodes); production display caps above are provisional engineering choices, not validated promises. The loading figure uses the Diagram Design fan-in/bottleneck pattern: three request sources, one pending slot, one active worker, and accepted/stale outcomes. Every diagram includes a textual description, a separate bottom legend and a local horizontal scroller. Its skin follows the user's choice to match Tadpole; [the style receipt](style-guide.md) records tokens and review status.

## Data contracts before drawing

- **Scope and coverage:** include origin, start URL(s), crawl generation, timestamp, crawl limits and active filters. Path-only keys would merge identical paths on different hosts. Show known totals, shown totals and groups hidden by display limits separately. A partial crawl is not a map of the whole website.
- **Directory containment:** split a parsed URL path, never the complete URL string. Keep direct pages/self weight explicit so parent totals reconcile with children. Query variants remain separate URLs unless the crawl's normalization policy says otherwise. Represent root pages, an empty path and groups with zero selected weight explicitly.
- **Crawl paths:** the current `CrawlResult` retains a depth, but no discovery-parent relationship. Reconstruct shortest paths from the final known directed graph and label the view as BFS. Tie-break parents by normalized URL. Persisting actual discovery parents would be a separate crawler change. Unreachable pages are a separate component/list; unknown start URLs produce an explanation rather than an invented tree. Recorded crawl depth must not silently substitute for shortest-path depth.
- **Links:** default to unique source/target pairs, with repeated occurrences available only as a separately named metric. Self-links and within-section links have explicit inclusion rules. The existing builder marks links `Follow` with `Unknown` placement; it cannot support truthful follow/nofollow or content/navigation filters until the crawler preserves that evidence. Do not expose unsupported filters as working controls.
- **Matrix normalization:** optional pairs per *crawled source page* uses the source row's denominator and states whether it is before/after filtering. It is not a fraction of pages and may exceed one. Other groups include all omitted row/column memberships without double counting. A displayed zero differs from missing data.
- **Rank:** Tadpole's directed, unweighted link-graph score is not Google's PageRank. Specify graph scope, damping/exclusion configuration and whether rank is computed on the whole crawl or the filtered subgraph. Recommended default: whole-crawl rank, filtered display. Show incomplete/preparing states, never zero while a calculation is pending.
- **Orphans:** zero in-degree in a partial crawl is insufficient evidence. Seed URLs, List-mode input, imported URLs and scope exclusions need context. Use “No incoming links found in this crawl” until comparison data supports a stronger conclusion.

## Shared interaction model

Use a native mode selector in the visualization toolbar, contextual metric controls and the existing GTK table/details pattern. Preserve selected URL, filters and origin when switching views. Directory focus, tree expansion and graph neighborhood focus are distinct view state; do not rewrite a URL selection into a folder silently. Back/Up restores the previous focus without recomputing the crawl.

Selection is reachable through the virtualized URL table and a keyboard-accessible search/results list. Tab exposes controls and the evidence list; arrows move within the current visible group, Enter selects/expands, Escape clears a temporary selection, and Back/Up returns to its parent. Show focus separately from selection. Cairo diagrams must have a companion accessible result list and counts; a tooltip cannot be the only access to a URL or metric.

Render labels for the selected page and a limited set of important items. Expose the complete URL as selectable plain text in details. Counts, statuses and line patterns supplement color. Theme changes recolor the prepared frame; font/scale changes invalidate affected label geometry. Search can debounce worker requests (initial proposal: 150 ms), but selecting a result must respond immediately from the prepared index.

Loading states: window chrome → preparing summary → ready aggregate → optional detailed analysis. Keep the previous usable frame while a new request is prepared, with a clear “Updating…” indication so it cannot be mistaken for current results. Cancel preserves it; errors offer retry; an empty filtered result differs from an uncrawled project. Include progress only when actual stage totals are known. During an active crawl, state the snapshot time and provide explicit refresh; avoid a complete graph rebuild after each page.

## Current hot paths

These observations refer to production commit `ee3733bee328661cdca39ceafda167d82776f15d`:

| Location | Observed work | Consequence / remedy |
|---|---|---|
| `src/ui/site_visualizer.rs`, `VisualizerWindow::new` | Full `get_all_results()` clone, snapshot build and rank computation before the window is usable | Move compact capture/analysis to a worker; show chrome before analysis; rank on demand |
| Same file, `draw_structure_map` and pointer/click handlers | Reconstructs the complete icicle tree during painting and hit testing | Cache tree, rectangles and hit index by request; pointer movement does no graph construction |
| `src/visualization/icicle.rs`, `insert_path_into_icicle` | Linear `children.iter().position` for each segment | Indexed path construction; expected O(total path segments), then deterministic ordering; guard extreme depth |
| Same file, result-based vs snapshot-based builders | Complete URL passed as a path in one builder; issue-count definitions differ | One parsed-path implementation and explicit metric definitions |
| `src/visualization/layout.rs` | Synchronous layouts, cloned layout results; force mode has sampled pairwise work | Share immutable prepared frames; defer layout; bound the visible neighborhood before considering force placement |
| `src/visualization/synthetic.rs` | Reverse-link generation repeatedly scans the result list | Use an indexed generator for larger benchmarks; fixture generation must not be mistaken for layout time |

The existing immutable `GraphSnapshot` and CSR adjacency are useful foundations. Extend their use instead of creating a second graph representation per mode. Stable URL identity and a real snapshot revision are needed; node/edge counts alone are not a content identity.

## Measured baseline

Raw observations and environment: [baseline.json](baseline.json). Harness: [`examples/visualization_benchmark.rs`](../../examples/visualization_benchmark.rs). Release build, desktop feature disabled, three sequential trials per case on Intel Core i7-8550U / 32 GB RAM, Linux 7.2.13, Rust 1.99.1. Values below are phase medians in milliseconds; they are not percentiles or end-to-end GTK timings.

| Shape | URLs | Unique edges | Full-result clone | Snapshot | Rank | One icicle rebuild |
|---|---:|---:|---:|---:|---:|---:|
| Bucketed | 1,000 | 7,993 | 0.96 | 5.81 | 0.82 | 2.55 |
| Bucketed | 10,000 | 79,993 | 13.58 | 101.05 | 10.56 | 16.05 |
| Bucketed | 100,000 | 799,993 | 190.61 | 1,218.74 | 205.67 | 366.03 |
| One wide folder | 10,000 | 79,993 | 8.75 | 69.90 | 10.64 | 548.17 |
| One wide folder | 25,000 | 199,993 | 33.74 | 221.96 | 35.90 | 6,035.52 |

The wide 10k tree rebuild is approximately 34× the bucketed case despite equal URL/edge counts. This matches the linear child-search risk; the measurement does not isolate every allocator/cache effect or establish a precise scaling exponent. The renderer calls this same rebuilding function from input and drawing handlers, making cached geometry a first-order requirement. No production optimization or speedup is claimed by this design change.

The timed process finished in 28.81 s and peaked at 468,660 KiB RSS across all cases. This includes the state, result copies, graph and allocator retention; it is not a per-view or incremental-memory figure. Fixture generation is outside the phases. Fixtures have eight outgoing entries per page, no Markdown/images/schema bodies, no realistic reverse-link metadata, and simple status/content fields. Their clone costs are optimistic for real crawls. The PageRank phase is graph-dependent and omits the module's subsequent node-array copy. Drop costs, disk loading, geometry creation, Cairo paint, hit testing and screen latency are excluded. There is no warm-up exclusion or confidence interval; all three raw observations are retained. No GUI profiler trace was collected.

Reproduce:

```sh
cargo build --release --locked --no-default-features --example visualization_benchmark
/usr/bin/time -v target/release/examples/visualization_benchmark > /tmp/tadpole-baseline.json
```

## Proposed module boundary

Introduce one `VisualizationSession` that owns the immutable index, derived analyses, bounded request queue and prepared-frame cache. Its small interface accepts a `ViewRequest` and publishes `Preparing`, `Ready(Arc<PreparedView>)`, `Cancelled` or recoverable `Failed` events. GTK owns widgets, focus and painting. The core owns normalization, aggregation, BFS, rank, layout, geometry and hit queries; UI callbacks consume prepared data.

Capture only visualization facts from `CrawlState` under a short read lock: URL, link targets and required scalar fields. Build/sort/index after releasing that lock; clone no Markdown, images or schema bodies. The capture still has a cost and needs measurement. Keep the same source revision throughout preparation. Existing project deserialization is a separate cold-start phase, not hidden inside a claimed visualization time.

Use one active analysis job and one replaceable newest pending request, not one thread/job per input event. A revision/cancellation token prevents stale publication. Check it within bounded construction/layout batches and between rank iterations, rather than merely ignoring an expensive result after completion. Split reusable graph analyses from viewport-specific layout; resizing should not rerun BFS or rank.

Cache identity includes source revision, start URL(s), view type, metric, focus, filter identity, relevant algorithm configuration, logical viewport and font/scale revision. Selection changes that only affect highlighting do not regenerate layout. Use shared immutable buffers, avoid returning vector clones, and start with a provisional 32 MiB LRU budget for derived view geometry. Snapshot/source/result storage is additional memory and must be measured separately. Theme colors need not invalidate topology.

This follows [GTK's threading guidance](https://docs.gtk.org/gtk4/section-threading.html): long-running work belongs on workers, and UI work returns to the main context. The immutable Rust graph can cross that boundary; GTK widgets remain on the main thread. `DrawingArea`'s [draw callback](https://docs.gtk.org/gtk4/class.DrawingArea.html) should consume ready geometry rather than construct the graph.

## Implementation sequence and acceptance

1. **Repair the current path first.** Indexed directory construction, unified metric/path semantics, cached immutable tree + visible geometry/hit index, background snapshot preparation and deferred rank. Add meaningful wide/deep-tree parity and cache-invalidation tests through the session interface. Verify opening does not synchronously analyze the full crawl.
2. **Section matrix and URL neighborhood.** Both reuse CSR and have bounded visible output. Ship source/target evidence tables, known/shown counts, explicit expansion, empty/error states and keyboard access with the view. Benchmark selection/filter/resize, not only layout generation.
3. **Link-depth tree and rank distribution.** Establish start-URL, unreachable-component and rank-scope contracts. Add deterministic BFS, density-bin accounting and cancellation tests. Persist discovery parents only if a later requirement calls for actual discovery history.
4. **Directory treemap.** Reuse the same directory index and native selection, including explicit self/root weights and Other aggregation. Keep the existing icicle as an alternate presentation.

Provisional budgets on an agreed reference device: window chrome paints within 100 ms; a cached mode/selection change becomes useful within 100 ms; main-thread paint/input tasks remain below 8 ms at the 95th percentile; a cold 10k overview targets 1 s from already-loaded crawl facts. These are acceptance targets, **not current measurements or guarantees**. Set a 100k cold-view target after the corrected pipeline is profiled. A 16.7 ms frame budget cannot accommodate any full-crawl rebuild in a pointer handler.

Measure cold/warm window-open → first useful view, result-capture lock time, each worker stage, queue delay/cancellation, paint, hit test, search/filter/resize and peak/incremental memory. Use wide, deep, cyclic, disconnected, dense-navigation and long-URL fixtures at 1k/10k/100k, plus sanitized real crawls. Record hardware/power mode, compiler/features, graph counts, repeated-run distributions and profiler traces. Validate under active crawling and rapid view/filter changes; the newest accepted revision must win. Loading performance is an implementation gate before raising display caps.

## Verification of this design artifact

- Diagram Design offline self-check: passes; one self-contained HTML file, six accessible SVGs, no scripts or remote assets.
- Pango/Adwaita Sans text measurement: 151 text elements checked for canvas bounds and text overlap; no failures. Orthogonal connector segments checked against node interiors and shared strokes; no failures. This is geometric validation, not a browser screenshot review.
- Treemap area shares: maximum relative error 1.31% after snapping/gutters. Matrix includes all 25 declared cells; scatter positions use the declared linear axes.
- Calculated contrast in both themes: normal text ≥ 4.5:1, essential outlines ≥ 3:1. [Style receipt](style-guide.md) has values and constraints.
- Release benchmark completed all 15 observations and its count assertions. The harness compiles headlessly; application behavior was not changed.
- Live browser review was blocked by the browser security policy for local `file:` URLs. No alternate browser route was attempted. **Visual/style approval is pending user inspection.**

Please open [the study](index.html) and verify palette, typography, contrast and overall rendering in your system's light/dark modes. Reply `approved` or describe changes. A reusable named Tadpole style profile can be saved after that review; the installed Diagram Design defaults were not modified.

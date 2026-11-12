# Tadpole architecture

The library owns crawling, audit data, provider integrations and graph algorithms. The desktop binary initializes Tokio and the Libadwaita application, then builds the UI from that library. `desktop` is the default Cargo feature; disabling it removes GTK, GDK, GLib, Gio, Libadwaita and Cairo dependencies from the core build.

## Module map

| Module | Responsibility |
| --- | --- |
| `src/main.rs` | Runtime/application startup and platform-specific loader setup |
| `src/state.rs` | Shared `CrawlState`, results, config, counters and portable project format |
| `src/crawler.rs` | HTTP scheduling, HTML extraction, optional browser rendering and resource discovery |
| `src/robots.rs` | Robots groups, agent specificity, longest-match Allow/Disallow and global sitemap declarations |
| `src/sitemap.rs` | Bounded same-site XML/index/gzip ingestion with source provenance and failures |
| `src/discovery.rs` | Reachability, sitemap reconciliation, coverage, project snapshot and CSV export |
| `src/ui/discovery.rs` | Native Sitemap & Discovery report, sources, filters and selected evidence |
| `src/http.rs` | Response allocation limits, independent of Content-Length |
| `src/markdown.rs` | HTML-to-Markdown conversion and extraction settings |
| `src/filters.rs` | GTK-independent audit filters and duplicate indexes |
| `src/project.rs` | Atomic project writes and stable Markdown filenames |
| `src/ai.rs` | Bounded OpenRouter/OpenAI completions, response validation and page context |
| `src/ui/window.rs` | Crawl workspace, preferences and GTK event handling |
| `src/ui/table.rs`, `row_data.rs` | Virtualized results model, sorting, filtering, CSV/clipboard exports |
| `src/ui/details.rs` | Selected-page diagnostics and metadata suggestions |
| `src/ui/sidebar.rs`, `summary.rs` | Filters and local health/progress summaries |
| `src/ui/link_opportunities.rs` | Cancellable background internal-link and n-gram analysis |
| `src/ui/theme.rs` | Shared semantic Libadwaita styling |
| `src/ui/site_visualizer.rs` | Native Cairo URL icicle UI |
| `src/visualization/` | CSR graph snapshots, layouts, PageRank, sections, filtering and exports |

## Crawl lifecycle

1. The UI validates config and seed input, confirms replacement of existing results, resets state and marks the crawl active before spawning the crawler.
2. The coordinator normalizes/deduplicates seeds and maintains a frontier. Discovery admission is bounded by `max_urls`; concurrency is clamped to 1–50.
3. Every worker belongs to a Tokio `JoinSet`. Workers fetch/parse pages and discover links. The coordinator inserts completed results and emits `Crawled` events.
4. Scheduling reserves capacity for in-flight pages, so the URL limit is strict. Pause suspends new work; already running requests finish.
5. Stop requests cancellation while retaining the active flag, then aborts and drains owned workers. Only then does the coordinator clear active state and emit `Finished`. A completed old worker cannot insert results into a new crawl.
6. GTK consumes events on its main context. Aggregate sidebar/dashboard work is limited to roughly four refreshes per second, with a final refresh when crawling ends. GTK retains its own active/busy state through event delivery and row population; Start/Open cannot replace data during GLib yields.

The discovery/event channels are unbounded channel types; the admitted URL count bounds the frontier. This avoids a bounded frontier deadlock where every HTTP worker waits to enqueue links while the coordinator waits for workers to finish. `CrawlState` uses `parking_lot::RwLock`; no guard may cross an `.await`.

## UI cost model

`ColumnView` virtualizes rows. `CrawlRowData` owns an `Rc<CrawlResult>` so sorting, filtering and cell binding do not clone full Markdown/images/link vectors. A `DuplicateIndex` supports constant-time duplicate checks per row; sidebar/dashboard rebuild their index once per refresh. Text searches are case-insensitive. GTK widgets stay on the main thread; project serialization and Markdown file writes use background work. Loading saved rows yields through a GLib timeout, rather than a Tokio yield which does not guarantee time for GTK to paint.

## Project compatibility and secrets

`.seocrawl` remains JSON with `seed_url`, `config`, `results`, `stats`, and an optional `discovery` snapshot. Older projects load without fabricated discovery evidence. An unfinished snapshot reopens as stopped, partial evidence. Missing config fields use `CrawlConfig::default`, allowing older files to open. Loading reconstructs counters from unique result URLs and resets active/paused state. Session API credentials are retained when opening a project.

Credentials have `#[serde(skip)]` and config uses a custom redacted `Debug` implementation. Old embedded keys are ignored when loading. They still exist in old files on disk until the owner removes them. Atomic saves write/sync a private temporary file beside the destination and replace it only after successful serialization. A compact format reduces file size without changing the schema.

## External providers

The AI module has no GTK dependency. Page-level suggestion actions supply session config and a prompt with crawled page context; the module owns provider-specific URLs, bounded HTTP requests, status validation and response interpretation. No standalone research workspace or search-data provider client is included.

- Completions: 90-second total timeout, 10-second connect timeout, 2 MiB response limit, 30k-character prompt limit and 2,500 output tokens. HTTP bodies are omitted from errors to avoid echoing secrets/context. Empty, failed and truncated completions are rejected.

## Visualization truth

The shipped UI is a Cairo icicle view. Compact graph algorithms and JSON message types exist, but there is no embedded Sigma.js/D3/WebGL renderer. `CairoCanvasRenderer` is an interface placeholder; actual drawing happens in `site_visualizer.rs`. The older implementation plan and protocol are design references, not a list of shipped capabilities. Layout caches are cleared on snapshot replacement; equal node/edge counts do not identify equal topology.

See [development](docs/DEVELOPMENT.md) for operational limits and [audit](docs/AUDIT.md) for findings and remaining work.

## Sitemap discovery lifecycle

Sitemap collection is an owned future polled alongside the ordinary page workers. The ordinary seed(s) remain the only graph roots. After link traversal and source collection finish, sitemap-only pages are inspected within the remaining admitted URL limit. Source records and declared URLs have independent bounds; source failures retain successful siblings. Stop drains page workers and cancels source collection before publishing final evidence.

Pure reconciliation runs off the GTK thread and distinguishes actual hyperlink inlinks, observed root paths, sitemap membership and page observations. An auxiliary result uses `UNKNOWN_DEPTH` until an observed path is recovered. It is never presented as depth zero. CSV and native filters use the same typed finding set, including overlapping findings and unknown observations. Effective mode/scope, limits, timestamps, source coverage and response destinations remain attached to the snapshot.

The main notebook contains the complete Crawl Results workspace, Sitemap & Discovery and Link Opportunities. Each report owns its data-specific filters and evidence; the existing crawl sidebar/details/summary remain inside Crawl Results. Explicit linking handoff selects one target; analysis generations prevent cancelled/replaced runs from publishing stale results.

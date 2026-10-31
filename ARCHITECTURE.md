# Tadpole architecture

The library owns crawling, audit data, provider integrations and graph algorithms. The desktop binary initializes Tokio and the Libadwaita application, then builds the UI from that library. `desktop` is the default Cargo feature; disabling it removes GTK, GDK, GLib, Gio, Libadwaita and Cairo dependencies from the core build.

## Module map

| Module | Responsibility |
| --- | --- |
| `src/main.rs` | Runtime/application startup and platform-specific loader setup |
| `src/state.rs` | Shared `CrawlState`, results, config, counters and portable project format |
| `src/crawler.rs` | HTTP scheduling, HTML extraction, optional browser rendering and resource discovery |
| `src/robots.rs` | Robots groups, agent specificity and longest-match Allow/Disallow rules |
| `src/http.rs` | Response allocation limits, independent of Content-Length |
| `src/markdown.rs` | HTML-to-Markdown conversion and extraction settings |
| `src/filters.rs` | GTK-independent audit filters and duplicate indexes |
| `src/project.rs` | Atomic project writes and stable Markdown filenames |
| `src/integrations.rs` | OpenRouter/OpenAI completions, DataForSEO SERP, response validation and editorial prompts |
| `src/ui/window.rs` | Crawl workspace, preferences and GTK event handling |
| `src/ui/table.rs`, `row_data.rs` | Virtualized results model, sorting, filtering, CSV/clipboard exports |
| `src/ui/details.rs` | Selected-page diagnostics and metadata suggestions |
| `src/ui/sidebar.rs`, `summary.rs` | Filters and local health/progress summaries |
| `src/ui/link_opportunities.rs` | Cancellable background internal-link and n-gram analysis |
| `src/ui/workbench.rs` | Separate SEO Studio window and explicit paid actions |
| `src/ui/theme.rs` | Shared semantic Libadwaita styling |
| `src/ui/site_visualizer.rs` | Native Cairo URL icicle UI |
| `src/visualization/` | CSR graph snapshots, layouts, PageRank, sections, filtering and exports |

## Crawl lifecycle

1. The UI validates config and seed input, confirms replacement of existing results, resets state and marks the crawl active before spawning the crawler.
2. The coordinator normalizes/deduplicates seeds and maintains a frontier. Discovery admission is bounded by `max_urls`; concurrency is clamped to 1–50.
3. Every worker belongs to a Tokio `JoinSet`. Workers fetch/parse pages and discover links. The coordinator inserts completed results and emits `Crawled` events.
4. Scheduling reserves capacity for in-flight pages, so the URL limit is strict. Pause suspends new work; already running requests finish.
5. Stop aborts and drains owned workers. Only then does the coordinator clear active state and emit `Finished`. A completed old worker cannot insert results into a new crawl.
6. GTK consumes events on its main context. Aggregate sidebar/dashboard work is limited to roughly four refreshes per second, with a final refresh when crawling ends.

The discovery/event channels are unbounded channel types; the admitted URL count bounds the frontier. This avoids a bounded frontier deadlock where every HTTP worker waits to enqueue links while the coordinator waits for workers to finish. `CrawlState` uses `parking_lot::RwLock`; no guard may cross an `.await`.

## UI cost model

`ColumnView` virtualizes rows. `CrawlRowData` owns an `Rc<CrawlResult>` so sorting, filtering and cell binding do not clone full Markdown/images/link vectors. A `DuplicateIndex` supports constant-time duplicate checks per row; sidebar/dashboard rebuild their index once per refresh. Text searches are case-insensitive. GTK widgets stay on the main thread; project serialization and Markdown file writes use background work. Loading saved rows yields through a GLib timeout, rather than a Tokio yield which does not guarantee time for GTK to paint.

## Project compatibility and secrets

`.seocrawl` remains JSON with `seed_url`, `config`, `results`, and `stats`. Missing config fields use `CrawlConfig::default`, allowing older files to open. Loading reconstructs counters from unique result URLs and resets active/paused state. Session API credentials are retained when opening a project.

Credentials have `#[serde(skip)]` and config uses a custom redacted `Debug` implementation. Old embedded keys are ignored when loading. They still exist in old files on disk until the owner removes them. Atomic saves write/sync a private temporary file beside the destination and replace it only after successful serialization. A compact format reduces file size without changing the schema.

## External providers

The integration module has no GTK dependency. The UI supplies session config and an explicit topic/prompt; the module owns provider-specific URLs, bounded HTTP requests, status validation and response interpretation. It never crawls competitor URLs returned by SERP.

- Completions: 90-second total timeout, 10-second connect timeout, 2 MiB response limit, 30k-character prompt limit and 2,500 output tokens. HTTP bodies are omitted from errors to avoid echoing secrets/context. Empty, failed and truncated completions are rejected.
- DataForSEO: one Google Organic Live Advanced task, depth 10, Basic Auth, both envelope and task status must equal 20000. Organic items and task cost are extracted; missing results are errors.
- Studio: keyword/market changes invalidate cached evidence; active requests lock those inputs. Report state is per window and is not stored in crawl projects. Requests require explicit buttons and do not automatically retry.

## Visualization truth

The shipped UI is a Cairo icicle view. Compact graph algorithms and JSON message types exist, but there is no embedded Sigma.js/D3/WebGL renderer. `CairoCanvasRenderer` is an interface placeholder; actual drawing happens in `site_visualizer.rs`. The older implementation plan and protocol are design references, not a list of shipped capabilities. Layout caches are cleared on snapshot replacement; equal node/edge counts do not identify equal topology.

See [development](docs/DEVELOPMENT.md) for operational limits and [audit](docs/AUDIT.md) for findings and remaining work.

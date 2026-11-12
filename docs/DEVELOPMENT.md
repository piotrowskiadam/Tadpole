# Development and validation

## Commands

```sh
cargo fmt --all --check
cargo test --locked --no-default-features
cargo clippy --locked --no-default-features --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Core tests need no display or browser; some bind an ephemeral loopback HTTP port. A sandbox must permit local networking. GTK tests currently verify compilation and core behavior, not screen-reader behavior. The default build needs GTK 4.12+ and Libadwaita 1.5+. Use Cargo.lock for application builds. CI runs on pushes and PRs; version-tag packaging remains a separate workflow.

The regression suite covers credential serialization/debugging/legacy import, state replacement/loading, duplicate-index parity, normalized canonicals and word count, provider error envelopes, bounded crawl/stop completion, redirect/header indexability and Markdown filenames. Extend these public module tests when changing behavior.

## Visual previews

`examples/ui_preview.rs` opens the real GTK widgets in an isolated application ID. It uses no production crawl data and makes no paid API calls. Compile it with:

```sh
cargo build --locked --example ui_preview
```

For isolated testing, start `gtk4-broadwayd :8 --port=8088 --address=127.0.0.1`, connect a browser to `http://127.0.0.1:8088`, then run:

```sh
GDK_BACKEND=broadway BROADWAY_DISPLAY=:8 GSK_RENDERER=broadway target/debug/examples/ui_preview
```

The crawl window has an 800px minimum to keep native header controls reachable. Set `TADPOLE_PREVIEW_WIDTH=1024` for a compact crawl window and `TADPOLE_PREVIEW_LIGHT=1` to force the light appearance in the preview process. The backend needs its browser client connected before rendering. Alternatively run the example in a graphical test session. Inspect/capture the actual rendered browser client: attempting to export Broadway-specific GSK nodes through a Cairo renderer produced invalid colors in this environment, so that PNG helper was removed. These previews are a visual smoke check; they do not replace keyboard/screen-reader or Windows checks.

The detail-tab fixture exercises wrapping with long URLs, repeated anchors, missing/empty alt attributes, local image previews, social fallback values, JSON-LD and Markdown. It uses fictional data and bundled images, with no external requests or AI calls:

```sh
cargo run --locked --example details_preview
TADPOLE_PREVIEW_WIDTH=800 TADPOLE_PREVIEW_LIGHT=1 cargo run --locked --example details_preview
```

For Broadway, use the same connected-client setup above and run `target/debug/examples/details_preview` instead. Inspect the browser client directly; Broadway render trees cannot reliably be exported using the native PNG helper.

Detail tables preserve each link occurrence, including anchor text, `rel` and fragment targets. Older projects still show their URL lists; recrawl source pages to collect missing anchor/HTML image details. Image previews fetch HTTP(S) images on demand (or use downloaded local files), with four concurrent requests, a 12-second timeout, an 8 MiB body limit and a 64-thumbnail memory cache. Preview resolution/size is measured from the image file; exported image width/height are explicitly the declared HTML attributes. Social previews are illustrative, show Twitter-to-OG fallbacks and retain additional OG/Twitter metadata such as image alt, dimensions and site name.

Manual release checklist:

- Start, pause, resume and stop a crawl; start another crawl and verify that old results do not arrive.
- Open/save an older project; use an unwritable destination; verify error recovery and credential exclusion.
- Check filter/search/sort and filtered CSV, including late incoming links after completion.
- At 1024px and 1200px widths, inspect toolbar, filter panel, dashboard, details tabs; check system light/dark themes and larger text.
- Check keyboard focus, field names and icon descriptions with the platform accessibility inspector/screen reader.

## Media capture

Use the opt-in offline media mode for store screenshots and short frame recordings. See [Media assets and recording](MEDIA.md) for launch commands, limitations, and GIF/MP4 export. Normal application startup is unchanged.

## Packaging

- `build_appimage.sh` stages the binary/desktop file/icon. It currently requires host GTK/Libadwaita; it is not a fully self-contained runtime bundle.
- Snap uses the GNOME extension and core24 base. Chromium discovery and sandbox permissions must be tested separately; Chrome is not bundled.
- Windows packaging uses MSYS2 CLANG64, the Rust gnullvm target and Inno Setup. Verify loader DLLs, schemas, icons and launcher behavior on a real Windows installation.
- Keep Cargo.toml, snap version and installer AppVersion aligned when preparing a release. No package release is implied by a development PR.

## Operational limits and next steps

- HTML downloads are capped at 8 MiB, images at 20 MiB, provider replies at 2 MiB. Download timeouts are 10 seconds for crawl HTTP and 90 seconds for provider calls. These are current implementation constants.
- Raw page results, Markdown and links remain in memory; full project load/save still uses an in-memory snapshot. For very large crawls, consider a disk-backed result store and incremental aggregate state before raising advertised limits.
- Robots handling supports agent groups, wildcard rules and longest-match Allow precedence. It is not a formal RFC conformance certification. Sitemap declarations are collected independently of agent groups. Crawl-delay and full percent-encoding normalization remain work for a dedicated crawler-policy module.
- Automatic redirect following reports the final HTTP status; it does not preserve full hop-by-hop redirect history. Indexability is a local heuristic, not a guarantee of search-engine indexing.
- Link-opportunity analysis is a cancellable background operation, but its pairwise source/target scan remains expensive for very large sites.
- JSON-LD validation is intentionally limited. Metadata recommendations and text-only alt suggestions require human review.
- Rendering/layout/graph modules include experimental groundwork. No web renderer, persisted layout cache, rank-aware UI or 3D constellation is currently shipped in the main visualizer.
- Store credentials in an OS keyring in a future opt-in persistence feature; do not reintroduce secrets into CrawlConfig serialization.
- API parser tests use fixtures. End-to-end billable calls and provider model availability require owner-supplied credentials and are not part of CI.
- Localization, accessible canvas alternatives, fully bundled AppImages and real OS package validation are separate deliverables.

## Provider references

Contracts checked against primary documentation on 2026-10-31:

- [OpenRouter API overview](https://openrouter.ai/docs/api/reference/overview): endpoint, messages/model, output limit, attribution headers, completion text.
- [OpenRouter error handling](https://openrouter.ai/docs/api_reference/errors-and-debugging): HTTP/provider error cases.

## Sitemap and discovery audits

`src/sitemap.rs` parses UTF-8 XML URL sets and indexes (including namespaces, entity references and gzip) with quick-xml and flate2. DOCTYPEs and malformed entries are rejected. Same-site boundaries, deduplication, index-depth/source/byte/catalogue limits and source-specific errors apply independently of the page budget. Limits are 64 source files, five nested index levels, 50,000 unique declared URLs, 8 MiB downloaded and 50 MiB decoded per source. These are application resource limits, not a claim to accept every maximum-size protocol file. [Sitemap protocol](https://www.sitemaps.org/protocol.html).

The coordinator polls sitemap collection alongside ordinary crawl workers, prioritizes ordinary link traversal, then uses remaining URL capacity for sitemap-only inspections. The collection future is owned by the coordinator and is dropped on stop. Auxiliary inspections record actual links without inserting sitemap edges or roots. `discovery::UNKNOWN_DEPTH` represents an auxiliary page without an observed root path; UI/CSV show a dash, summary depth charts exclude it, and graph layouts keep unknown nodes in a separate display band. Reconciliation can subsequently recover a real observed depth.

`src/discovery.rs` owns graph reachability, overlapping findings, coverage and shared CSV export. Discovery collection, response observations and reconciled rows are saved in an optional project snapshot. Legacy projects show “not collected”; a snapshot saved during a crawl reopens as stopped, partial evidence. Missing sitemap membership is a review opportunity; no report claims Google indexing. List/URL modes do not establish ordinary root reachability.

`src/ui/discovery.rs` presents live source progress, filtered/sortable URLs, selected-page evidence, source setup and export. Source configuration applies to the next crawl; **Apply and recrawl** uses the existing crawl replacement flow. The linking handoff runs explicitly for the selected target and uses reachable, indexable source pages where discovery evidence exists. Clear/replacement cancels and invalidates older link-analysis updates.

For an isolated real-workspace check, start a local fixture server, connect a GTK Broadway client, then run the `ui_preview` example with `TADPOLE_PREVIEW_URL` set to that fixture. This explicit preview variable starts a real crawl; it does not inject results. `TADPOLE_PREVIEW_THEME=light|dark`, `TADPOLE_PREVIEW_WIDTH` and `TADPOLE_PREVIEW_HEIGHT` control the fixture view. Ordinary application startup does not read these preview variables.

`tests/discovery.rs` covers namespaces/entities, gzip limits, malformed XML, disconnected linked groups, duplicate/cyclic/recursive indexes, partial source failure, same-site scope, redirects/noindex/canonical findings, a shared page budget, CSV unknown values, cancellation without late updates, legacy persistence and interrupted saved crawls. Run it with the core or full suite; its integration cases bind a local HTTP port.

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

Set `TADPOLE_PREVIEW_STUDIO=1` to open Studio, `TADPOLE_PREVIEW_WIDTH=640` for a compact window, and `TADPOLE_PREVIEW_LIGHT=1` to force the light appearance in the preview process. The backend needs its browser client connected before rendering. Alternatively run the example in a graphical test session. Inspect/capture the actual rendered browser client: attempting to export Broadway-specific GSK nodes through a Cairo renderer produced invalid colors in this environment, so that PNG helper was removed. These previews are a visual smoke check; they do not replace keyboard/screen-reader or Windows checks.

Manual release checklist:

- Start, pause, resume and stop a crawl; start another crawl and verify that old results do not arrive.
- Open/save an older project; use an unwritable destination; verify error recovery and credential exclusion.
- Check filter/search/sort and filtered CSV, including late incoming links after completion.
- At 1024px and 1200px widths, inspect toolbar, filter panel, dashboard, details tabs and Studio reports; check system light/dark themes and larger text.
- In Studio, check empty topic/missing credentials, successful/failed SERP, a brief with/without page/evidence, keyword/market invalidation, and both report exports. Use test fixtures for routine tests; any real provider calls cost money.
- Check keyboard focus, field names and icon descriptions with the platform accessibility inspector/screen reader.

## Packaging

- `build_appimage.sh` stages the binary/desktop file/icon. It currently requires host GTK/Libadwaita; it is not a fully self-contained runtime bundle.
- Snap uses the GNOME extension and core24 base. Chromium discovery and sandbox permissions must be tested separately; Chrome is not bundled.
- Windows packaging uses MSYS2 MinGW64 and Inno Setup. Verify loader DLLs, schemas, icons and launcher behavior on a real Windows installation.
- Keep Cargo.toml, snap version and installer AppVersion aligned when preparing a release. No package release is implied by a development PR.

## Operational limits and next steps

- HTML downloads are capped at 8 MiB, images at 20 MiB, provider replies at 2 MiB. Download timeouts are 10 seconds for crawl HTTP and 90 seconds for provider calls. These are current implementation constants.
- Raw page results, Markdown and links remain in memory; full project load/save still uses an in-memory snapshot. For very large crawls, consider a disk-backed result store and incremental aggregate state before raising advertised limits.
- Robots handling supports agent groups, wildcard rules and longest-match Allow precedence. It is not a formal RFC conformance certification. Sitemap ingestion, crawl-delay and full percent-encoding normalization remain work for a dedicated crawler-policy module.
- Automatic redirect following reports the final HTTP status; it does not preserve full hop-by-hop redirect history. Indexability is a local heuristic, not a guarantee of search-engine indexing.
- Link-opportunity analysis is a cancellable background operation, but its pairwise source/target scan remains expensive for very large sites.
- JSON-LD validation is intentionally limited. Metadata recommendations and text-only alt suggestions require human review.
- Rendering/layout/graph modules include experimental groundwork. No web renderer, persisted layout cache, rank-aware UI or 3D constellation is currently shipped in the main visualizer.
- Store credentials in an OS keyring in a future opt-in persistence feature; do not reintroduce secrets into CrawlConfig serialization.
- API parser tests use fixtures. End-to-end billable calls and provider model availability require owner-supplied credentials and are not part of CI.
- Localization, broader DataForSEO market selection, accessible canvas alternatives, fully bundled AppImages and real OS package validation are separate deliverables.

## Provider references

Contracts checked against primary documentation on 2026-10-31:

- [OpenRouter API overview](https://openrouter.ai/docs/api/reference/overview): endpoint, messages/model, output limit, attribution headers, completion text.
- [OpenRouter error handling](https://openrouter.ai/docs/api_reference/errors-and-debugging): HTTP/provider error cases.
- [DataForSEO Google Organic Live Advanced](https://docs.dataforseo.com/v3/serp/google/organic/live/advanced/): Basic Auth, keyword/location/language/depth, organic items, task cost.
- [DataForSEO common errors](https://dataforseo.com/help-center/common-first-errors-with-dataforseo-api-and-how-to-fix-them): task status can fail inside HTTP 200.

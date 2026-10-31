# Tadpole modernization audit — 2026-10-31

Scope: all tracked Rust modules, integration tests, build/release scripts and Markdown documentation, starting from `be686f5`. UI evidence included the older committed screenshot and a source review of GTK widgets. Changes preserve the native application and add the owner-selected content-brief/SERP workflow. This is a code audit, not a certification of SEO accuracy, accessibility, packaging or provider availability.

## Implementation integrity

The core product is coherent: local crawling, a virtualized audit table, diagnostics, export and a native structure visualizer. The biggest mismatch was documentation advertising web/3D rendering and untraceable benchmark numbers. Historical proposals are now labelled as such; README and architecture describe the code that actually ships. Native platform controls remain the visual authority.

## Findings and disposition

| Priority | Finding / evidence in the baseline | Impact | Disposition |
| --- | --- | --- | --- |
| P1 | `CrawlConfig.ai_api_key` derived Serialize/Debug | Saved/shared project files and diagnostics could reveal keys | All provider secrets skip serialization/deserialization; redacted config Debug; session keys survive project opening; regression tests |
| P1 | Detached crawl workers, count polling, early Finished | Late results could cross into the next crawl; limits could be exceeded | One owned JoinSet, strict reservations and abort/drain before Finished; local HTTP tests |
| P1 | Bounded frontier filled from all workers | Seed lists over 10k or high fan-out could deadlock | Admission bounded by configured URL count; nonblocking discovery frontier |
| P1 | Table filtering cloned every result for every row; row access cloned full page content | Search/sort/filter scaled poorly and increased allocation | Shallow Rc row access and incremental duplicate index; indexed aggregate counts and throttled refresh |
| P1 | Relative canonical comparison, overwritten redirect status, ignored X-Robots-Tag, scripts counted as words | Misleading indexability/content metrics | Resolved/fragment-free canonicals; redirect classification retained; header directives recognized; script/style/template text excluded |
| P1 | Provider requests lacked bounded time/output and robust response validation | Hung actions, unknown costs, misleading success on failed tasks | Shared provider module, limits/timeouts, empty/truncated/error handling, both DataForSEO status levels checked |
| P1 | Slug-only Markdown names, synchronous project writes, ignored save errors | Silent overwrites, blocked UI and uncertain save success | URL-hash names, background writes, atomic project replacement and visible errors |
| P2 | Startup rewrote user launchers/icons and ran external cache tools each time | Unnecessary startup I/O and installation side effects | Removed; package installation owns launcher/icon registration |
| P2 | UI titlebar held a fixed 450px URL entry; dashboard consumed fixed space | Compact windows crowded primary controls | Separate expanding crawl toolbar, hideable panels and native width breakpoints |
| P2 | Fixed semantic hex colors and fixed data-table font size | Dark/light and large-text inconsistency | Libadwaita semantic colors, inherited font; custom canvas/syntax palettes remain a follow-up |
| P2 | Opening a project during crawling could replace shared state; saved counters trusted | Mixed projects and incorrect counters | Active-crawl guards, reset lifecycle flags, derive counters from unique results |
| P2 | Tokio yield used to “yield to GTK” during file loading | No reliable opportunity for GTK frames | GLib timeout yields in bounded row batches |
| P2 | Robots groups/Allow patterns ignored and first host rules reused for lists | Incorrect crawl permissions | Agent specificity, longest Allow/Disallow/wildcards and per-origin cache; unavailable/5xx robots blocks origin |
| P2 | Hardcoded Chrome path and synchronous browser calls inside async tasks | Linux-only assumption and blocked runtime workers | Automatic discovery; blocking pool for browser operations; explicit rendering failure instead of silent HTTP fallback |
| P2 | Equal-count graphs reused cached layout; u16 edge weights overflowed; nofollow flag ignored | Incorrect graph results or panic | Clear cache on replacement, saturating edge count, sorted edges, honor rank filtering; regression tests |
| P2 | `main.rs` compiled the same modules again; core tests required all GTK packages | Extra build work and poor CI isolation | Binary imports library; optional desktop feature; core CI independent of GTK |
| P2 | No push/PR quality workflow, stale Snap icon source | Regressions reached release pipelines; Snap install source missing | New quality CI, locked builds, corrected tracked icon source |

Severity denotes user impact in the baseline. No P0 data-loss catastrophe was demonstrated. The table groups related defects; it is not a count of every stylistic warning. Mechanical Rust idiom/formatting fixes accompany the behavioral changes.

## UI/UX assessment

Impeccable's native audit dimensions were adapted to GTK/Libadwaita desktop, rather than applying iOS/Android touch/inset rules. Web-only detectors/ARIA/mobile viewport scores do not apply to this application. Scores below are source-review estimates; full screen-reader/contrast measurements have not been performed.

| Dimension | Baseline estimate | Main evidence / change |
| --- | --- | --- |
| Accessibility | 2/4 | Native GTK controls and selectable text; URL/mode/Studio inputs gain explicit labels; canvas keyboard/screen-reader parity remains unverified |
| Performance | 1/4 | Nested full-result clones/scans and synchronous persistence; indexing/background work now address the verified hot paths |
| Appearance/theming | 2/4 | Native system styling was overridden by fixed status/font values; semantic tokens restored, custom syntax/canvas palettes remain |
| Platform conformance | 3/4 | Native Libadwaita navigation/controls; removed startup installation side effects and retained standard focus/selection behavior |
| Adaptivity | 1/4 | Fixed URL entry + always-visible panels; expanding toolbar and hideable/adaptive panels improve compact desktop use |
| **Total baseline estimate** | **9/20** | Significant work needed; no unmeasured “after” score is claimed |

Positive foundations: Rust ownership, an async HTTP client, native virtualized ColumnView, explicit data models, structured Serde JSON and compact CSR graph snapshots. Keep these. The modernization does not require a rewrite to a web stack.

SEO Studio makes the new feature discoverable next to the crawl workspace. It explains costs/transmissions before action, keeps evidence and suggestions in separate tabs, works with no crawl and supports copy/save. Market/topic changes invalidate evidence; failed requests leave prior reports available. Output is editable outside the app, and content remains selectable plain text.

## Verification

Validation commands and final results are recorded in [VALIDATION.md](VALIDATION.md). Core/provider tests use local fixtures and a loopback HTTP server; no billable production requests or user credentials are used. Graph test timings are debug smoke data, not release guarantees. See [VISUALIZATION_BENCHMARKS.md](../VISUALIZATION_BENCHMARKS.md).

## Remaining work, with practical limits

- For very large crawls: disk-backed storage, incremental aggregate snapshots, indexed link-opportunity search and chunked CSV generation. The current in-memory result model remains the main scaling ceiling.
- For crawler completeness: redirect-hop history, sitemap ingestion, rate/crawl-delay policy, formal robots percent-encoding cases, exact handling of all X-Robots-Tag agent scopes, nofollow/placement/anchor extraction.
- For accessibility/theming: screen-reader/keyboard canvas alternatives, semantic syntax colors, light/dark canvas palettes and testing at increased text scale.
- For packaging: real Windows/Snap installation tests and a fully bundled AppImage. No package release is claimed from local source checks.
- For product growth: OS keyring opt-in, UI localization, more markets and report persistence. Credentials remain session-only by design.
- For API assurance: fixture tests cover known success/failure shapes; billable end-to-end provider calls require real account keys and an owner's test budget.

Recommended next Impeccable passes: `$impeccable harden` for remaining error/export paths, `$impeccable adapt` for large text/keyboard/canvas use, `$impeccable optimize` when a measured large crawl supplies evidence, then `$impeccable polish` after those functional changes. These are follow-up scopes, not claims that this audit verified every possible runtime state.

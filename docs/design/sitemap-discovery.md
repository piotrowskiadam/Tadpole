# Sitemap & Discovery

Status: implemented in the production crawl workspace on 12 November 2026. The original clickable concept below remains an offline design reference; its data is fictional. The production report uses real crawl evidence.

## Implementation status

The live **Sitemap & Discovery** tab now collects same-site XML indexes and gzip sitemaps, tracks source provenance and failures, checks auxiliary URLs within the shared page budget, reconciles graph reachability and overlapping findings, saves report snapshots, and exports filtered CSV with coverage. Source changes apply with **Apply and recrawl**. The explicit linking handoff analyzes the selected target using reachable source pages when reachability evidence exists.

The module and type names in the implementation proposal below are now backed by `src/sitemap.rs`, `src/discovery.rs`, `src/state.rs` and `src/ui/discovery.rs`. The retained concept example remains useful for discussing visual states; its simulated controls are not part of the production report. Current operational limits and test coverage are documented in [DEVELOPMENT.md](../DEVELOPMENT.md#sitemap-and-discovery-audits).

## Production verification

[View the live native report](sitemap-discovery-live.jpg). This capture shows production GTK widgets after a real crawl of a local HTTP fixture, with embedded origin metadata. The fixture's content and displayed timings are test evidence, not a performance claim or a customer crawl.

- **Finish review:** the reviewer inspected the live light-theme window at 940px and compact dark-theme window at 800px (`.impeccable/review/discovery-live/desktop-light.jpg` and `compact-dark.jpg`). The native shell, coverage-first hierarchy, finding navigation, results table, selected-page evidence and explicit linking handoff received a **ship** disposition with no material fixes requested. This extends the existing design conventions; no replacement visual system was introduced.
- **Behavior and checks:** all 53 tests passed with `cargo test --locked -- --test-threads=1`. Ten discovery tests cover XML namespaces/entities/gzip, malformed and empty versus missing sources, disconnected groups and overlapping findings, duplicate/cyclic/depth-limited indexes, partial source errors, bounds/shared page budget/cancellation, CSV unknowns and legacy/interrupted project persistence. Formatting, diff checks, release build and strict core Clippy (`cargo clippy --locked --no-default-features --all-targets -- -D warnings`) passed. An earlier parallel test run during a concurrent release build hit an unchanged visualization timing threshold; the serialized run passed after the build was idle.
- **Implementation review:** source changes apply to the next crawl through the existing recrawl flow. Selected-target link analysis uses reachable, indexable sources when discovery evidence exists; clearing or replacing a crawl invalidates older analysis generations. Source management and CSV export are implemented, but individual native controls and file dialogs were not reliably exercised through browser input.
- **Remaining validation:** native Windows runtime, keyboard traversal, assistive-technology access and the manual save-dialog workflow have not been certified. The visual review and automated checks do not replace those release checks.

## The job

An SEO professional should finish a crawl knowing which submitted URLs have technical problems and which URLs have no observed linking path from the site's ordinary crawl start. The report should carry enough evidence to decide whether to add links, repair a page, update a sitemap, or leave an intentional configuration alone.

The design extends Tadpole's existing native workspace. It inherits GTK4/Libadwaita typography, controls, themes, mascot, notebook navigation, dense results table and lower details pane. It introduces no external account requirement.

## Open the concept

```sh
cargo run --locked --example discovery_preview
```

This opens an isolated application (`com.tadpole.discovery.preview`) without crawling a website or loading a real project. It is also usable through a connected GTK Broadway session as described in [DEVELOPMENT.md](../DEVELOPMENT.md).

[View the native concept screenshot](sitemap-discovery-preview.jpg). This is a full local capture of the GTK preview, copied from `.impeccable/review/discovery/desktop-light.jpg`; it uses fictional data. The captured light-theme window is 940px wide. Screenshot image dimensions include the surrounding capture surface and do not describe the window width.

Optional environment variables:

- `TADPOLE_PREVIEW_WIDTH=800` for the compact desktop layout (default 1200).
- `TADPOLE_PREVIEW_THEME=light` or `dark`; without this, follow the system preference.

Try: select another finding, search a URL or HTTP status, choose a result, open **Manage sitemaps**, enter an invalid and then valid sample URL, choose **Find linking pages**, export the filtered sample CSV, and switch **Preview state**. The preview's link suggestions and rerun are explicitly illustrative. Source entries are validated locally and do not fetch or alter sample findings. Export is a real local save action.

## Layout and interaction

Add **Sitemap & Discovery** beside **Crawl Results** and **Link Opportunities**. Keep the existing crawl toolbar and project actions. In production this is a new report in the current crawl, not a second application or independent crawl session. The isolated example recreates the surrounding shell only to demonstrate placement.

The report has four useful layers:

1. **Coverage and sources.** A compact statement explains whether the link crawl completed, which limits affected it, and whether sitemap ingestion succeeded. Ordinary inline totals show URLs in imported sitemaps, URLs reached through links, and the deduplicated union of known URLs. These sets overlap; totals are not additive. **Manage sitemaps** reveals sources in place, without losing the selected finding.
2. **Finding navigation.** A left-hand list provides All known URLs, Not reached through links, HTTP errors, Redirects, Noindex, Different canonical, Not in sitemaps, and No finding. The production report counts distinct URLs per finding; one URL can appear in several findings. Scope and source completeness qualify every count.
3. **Results.** A searchable, virtualized table shows URL, HTTP status, observed inlinks and finding. Keep the full URL available for copying and in a tooltip. Horizontal scrolling handles long URLs; split panes are resizable. Selection updates the evidence pane without a modal or a new window.
4. **Evidence and next step.** Show exact sitemap membership, homepage reachability, observed inlinks, HTTP/directive observations and canonical or redirect destination. Explain why the selected finding matters and how to review it. A candidate orphan can continue to **Find linking pages**, carrying the selected target into Tadpole's existing link analysis.

At compact desktop widths keep the data type readable and allow native scrolling. Put verbose evidence in the lower pane instead of requiring a permanent right inspector. The minimum width follows the incumbent 800px workspace. Use system appearance, semantic colors and standard focus behavior; status always has a text label.

The prototype's `Preview state` selector belongs to the demonstration only. It does not belong in the production user flow.

## Report language

| Observation | Report label | User decision |
| --- | --- | --- |
| Sitemap member, no observed root-to-URL path | Not reached through links / Candidate orphan | Decide whether the page deserves a contextual link. Review crawl scope first. |
| Sitemap member returns HTTP error | HTTP error in sitemap | Restore the page or remove an obsolete submission. |
| Sitemap member redirects | Redirect in sitemap | Review whether the final URL should replace it. |
| Sitemap member declares noindex | Noindex in sitemap | Resolve the conflict with the intended search visibility. |
| Sitemap member canonicalizes elsewhere | Different canonical | Verify the intended canonical and sitemap choice. |
| Reachable, technically indexable, absent from imported sources | Not in imported sitemaps | Review inclusion; absence is not automatically an error. |
| No applicable finding | No finding | These checks found no issue; this is not proof of Google indexing. |

Severity is contextual. An intentional noindex page may simply need to leave the sitemap. Avoid invented traffic-impact scores and the blanket phrase “SEO error” for every configuration difference.

“Observed inlinks” is independent of reachability. Two disconnected guides can link to one another while neither has a path from the homepage. The sample deliberately includes this case.

## States

| State | Display and available action |
| --- | --- |
| No crawl | Explain the comparison; offer sitemap setup and an ordinary crawl. No zero-error score. |
| Discovering/parsing sitemaps | Show each source's state, unique URLs collected and cancellation. Counts are provisional. |
| Link crawl running | Retain useful results with provisional labels. Do not finalize orphan findings. |
| Complete within configured scope | Show scope, sources and observed findings; use candidate language for reachability conclusions. |
| URL/depth limit, stop, fetch or rendering gaps | State the concrete gap and retain evidence. Distinguish not fetched from not reached. |
| No sitemap found | Offer manual URL entry; keep the crawl results available. |
| Source malformed/unavailable | Show source-specific parse/HTTP error and retry. Partial source success remains visible. |
| No matching rows | Keep the search/filter context and offer clearing the search. |
| No findings | Show what was checked and its coverage, without certifying indexing. |
| Legacy saved project | Say discovery data was not collected. Offer a new crawl; do not fabricate sitemap absence. |

The preview implements complete, depth-limited, missing-source and source-failure variants plus source validation, no-match search and simulated running states. These are available concept states, not a claim that every scenario has been manually tested. Other states above are production requirements.

## Original implementation proposal

The following records the concept-stage plan. For implemented behavior and current limits, use the implementation status and production verification above and the developer guide.

### 1. Add independent data collection and reconciliation modules

Keep parsing, networking and report rules outside GTK.

- Proposed `src/sitemap.rs`: discover `Sitemap:` declarations, accept explicitly supplied URLs, parse URL sets and sitemap indexes, support gzip, track child-source membership and return source-specific failures. Use a maintained streaming XML parser; choose the dependency during implementation against its current documentation. Do not parse XML with regular expressions.
- Proposed `src/discovery.rs`: reconcile a known-URL catalogue, sitemap membership, page observations, the observed internal-link graph and run coverage. Return typed, deterministic findings that filters, export and UI share.
- Existing `src/robots.rs`: expose sitemap declarations independently of agent-specific allow/disallow matching. Sitemap discovery must not change crawl permission rules.
- Existing `src/state.rs`: store catalogue, source provenance and coverage for the current run, alongside existing page results. Keep derived report logic out of this state container.

Suggested conceptual types (names are proposals):

```text
SitemapSource
  requested URL, parent source, discovery method,
  fetch/parse outcome, unique URL count, truncation reason

UrlDiscovery
  normalized URL key,
  membership in zero or more sitemap sources,
  discovery sources: ordinary seed / internal link / sitemap / manual list,
  page observation: fetched / blocked / failed / not fetched

Reachability
  reached from ordinary root (shortest observed link depth)
  no path observed in the collected graph
  unavailable (no appropriate root/graph)

AuditCoverage
  crawl roots and mode, effective scope/settings,
  complete / stopped / URL limit / depth limit,
  source coverage, fetch/render gaps, start/completion timestamps

DiscoveryFinding
  URL key, finding type, evidence, review guidance
```

A URL may belong to multiple sitemaps and have multiple findings. Findings should reference evidence rather than copy whole `CrawlResult` records. Treat the catalogue as bounded data: known membership can be collected without pretending every URL was fetched.

### 2. Preserve link discovery while extending the crawl

Use a staged first implementation:

1. Discover and parse permitted sitemap sources into a bounded catalogue while the ordinary link crawl runs.
2. Prioritize ordinary link traversal, preserving the configured start URL and its graph.
3. After that traversal, inspect sitemap URLs that lack a page observation, using the remaining page-request budget and the same scope/robots rules.
4. Reconcile the combined observed graph and catalogue when work completes. If there is no remaining budget, expose those URLs as **not fetched**, with unknown status/indexability.

Sitemap membership is not an internal link. Do not insert a sitemap edge in `inlinks_map`, assign sitemap candidates depth zero, or turn every auxiliary URL into a homepage root. Breadth-first traversal from the ordinary root over observed internal links determines reachability; disconnected groups remain disconnected even if their pages link to one another. Recompute when auxiliary fetches add genuine edges.

Respect one explicit total page-fetch limit across ordinary and auxiliary work. Separately bound sitemap request counts, index depth, compressed/decompressed bytes, catalogue entries and redirects; surface truncation instead of hiding it. Deduplicate index cycles and repeated URL memberships. Record child-source errors without discarding successful siblings. Validate cross-origin child sources against the user's crawl scope; a listed source is not permission to expand that scope silently.

In List mode, sitemap membership and technical checks remain useful, but orphan conclusions need a declared ordinary root and its link crawl. Mark reachability unavailable where that evidence does not exist. A depth-limited Path crawl cannot claim to describe whole-site connectivity.

### 3. Use consistent URL and response evidence

Use the same URL identity rules in discovery, results and graph building. Remove fragments and normalize parser-defined scheme/host differences; preserve meaningful path case, queries and trailing-slash distinctions. Retain declared URLs and requested/final response URLs as evidence. Do not silently conflate a redirecting URL with its destination.

The current crawler follows redirects and has limited redirect history. Verify its redirect indicator when adding sitemap rules, and persist requested/final URL observations so a final 200 is not mislabeled as a direct sitemap success. Full hop-by-hop redirect auditing can remain a later feature. Reuse existing noindex/canonical checks and qualify observations where fetch or rendering failed.

### 4. Save, filter and export the same report

Extend `CrawlProject` with an optional, defaulted discovery snapshot. Old `.seocrawl` files must still open with “discovery not collected”, rather than an empty-sitemap claim. Reuse the current atomic project save path. Do not add provider credentials or require an account.

CSV should include URL, finding, source sitemap(s), observed HTTP/directive/canonical data, observed inlinks, reachability, coverage/completion and review guidance. Export the selected filter/search view with full URL values. When several findings apply, export one explicit URL/finding record per row or document a stable multi-value format.

### 5. Integrate the native report

Proposed `src/ui/discovery.rs` owns presentation and selection only. Add its tab in `src/ui/window.rs`, reuse `ColumnView` and current detail-table conventions, and connect to shared crawl progress/cancellation. Compute graph/reconciliation work in the background. Update aggregate counters in bounded batches using the existing UI refresh approach. Selection must not start a new crawl, provider request or link scan automatically.

The linking action should pass a target URL into existing link-opportunity analysis, run it explicitly, and preserve the report selection on return. Suggestions require source-page context and human review.

## Delivery sequence and acceptance

1. **Sitemap ingestion and provenance:** offline fixtures for URL sets, namespace variants, indexes, gzip, duplicates, cycles, malformed XML, source errors and enforced limits. Show the collected sources before adding conclusions.
2. **Pure reconciliation:** meaningful tests for disconnected linked groups, auxiliary URLs staying out of root/depth/inlink accounting, limited and stopped crawls, not-fetched records, multiple sitemap memberships and overlapping findings.
3. **Persistence and UI:** legacy-project round trips, new snapshot round trips, filtered CSV parity, stable selection, source retry/cancel and running-to-complete transitions. Render light/dark and 800/1200px layouts; manually check keyboard and assistive-technology access.
4. **Release validation:** run the repository's required core and desktop checks and validate the real Windows package. Do not advertise large-site scalability without measurement; existing in-memory storage remains a constraint.

A useful first release is complete when a user can load a sitemap index, run a bounded crawl, inspect reliable evidence for every finding, export the chosen results, reopen the saved report and understand its coverage. Search Console import, historic comparisons and comprehensive redirect-hop reporting can follow independently.

## Historical concept verification

The original prototype was a local extension of the visual system recorded in [DESIGN.md](../../DESIGN.md) and the product constraints in [PRODUCT.md](../../PRODUCT.md). That prototype changed no production behavior and required no new visual-world or system files. These checks describe the isolated concept only; production verification is recorded above.

- **Native visual evidence:** reviewed the light-theme concept at an actual 940px window width and the dark-theme compact layout at an actual 800px window width. The compact capture is `.impeccable/review/discovery/compact-dark.jpg`. Finding filtering, local source-entry validation and the selected-target handoff to Link Opportunities were exercised visually. This native GTK concept has no web-detector result.
- **Finish review:** the reviewer identified low-contrast secondary labels and table headings. The preview now scopes secondary labels to the theme's window foreground at 0.85 opacity and table headings to the view foreground at full opacity, preserving native focus and selection styling. The follow-up reviewer marked that contrast fix resolved with a **ship** disposition. That verdict covers the fix, not comprehensive product validation.
- **Code checks at the concept stage:** the example build, Rust formatting and diff checks passed. The then-current normal Clippy run reported no new example diagnostics and 80 pre-existing library warnings; strict `-D warnings` was blocked at that stage. Current production checks are recorded above.
- **Limits:** sitemap fetching, audit execution and link suggestions are simulated. CSV export implements a real local save through the native file dialog, but the saved-file workflow has not been smoke-tested. Windows runtime behavior, keyboard traversal and assistive-technology access have not been certified; they remain implementation and release checks above.

## Evidence

This proposal builds on the [competitive research](../research/next-feature-competitive-research.md) and current [architecture](../../ARCHITECTURE.md), [product brief](../../PRODUCT.md), [design conventions](../../DESIGN.md), `src/state.rs`, `src/crawler.rs`, `src/robots.rs` and `src/ui/link_opportunities.rs`. It is a design recommendation, not validated demand or a claim about SEO uplift.

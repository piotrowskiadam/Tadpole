# Tadpole releases

## 0.6.3 — Smart Seed Redirects & Crawl Recovery Suggestions

- **Automatic `apex ↔ www.` & `http → https` seed redirect resolution:** when a seed URL in `Crawl` or `Path` mode redirects between apex and `www.` (or upgrades `http` to `https`) with *Follow Redirects* enabled, Tadpole records the original seed URL as a `Redirect` row and automatically switches the crawl scope and sitemap discovery root to the canonical host instead of stopping after 1 URL.
- **Non-modal inline crawl recovery banner:** when a crawl ends on a single blocked, failing, or cross-domain redirecting seed URL, Tadpole probes recovery options off the UI thread and displays a dismissible banner directly below the address bar with a one-click action:
  - **Reachable `www.` / apex alternate** (e.g. `starwars.com` returning HTTP 403 while `www.starwars.com` returns 200) → **Crawl `www.starwars.com`**.
  - **Cross-domain redirect** (e.g. `old.com` → `new.com`) → **Crawl `new.com`** without silently switching domains.
  - **WAF / crawler User-Agent block (`401`/`403`/`406`/`429`)** → **Retry with Browser UA** when a standard browser User-Agent succeeds.

## 0.6.2 — Link Opportunities Rule Filters & N-gram Sync

- **Live rule filters in `Link Opportunities`:** filter analyzed link opportunities immediately without re-running analysis across five fields (`Either URL (Source or Target)`, `Source Page`, `Target Page`, `Anchor Keyword`, `Context Snippet`) and six case-insensitive operators (`Contains`, `Does not contain`, `Starts with`, `Ends with`, `Equals`, `Does not equal`) with comma-separated multi-value patterns, `Match all (AND)` / `Match any (OR)` combinators, and default quick-filter rows for URL and anchor exclusions.
- **`Top N-grams` sync & click-to-filter:** anchor exclusion rules automatically filter the bottom `Top N-grams` table, and activating an N-gram row filters the top `Link Opportunities` table by that phrase.
- **Filtered CSV export & wrapped URL columns:** `Export CSV...` in `Link Opportunities` now exports only the currently filtered rows, and the `Source Page`, `Target Page (Suggested)`, and `Anchor Keyword` columns use wider default widths with resizable dividers, word wrapping, and full-text tooltips.

## 0.6.1 — Crawl Results Table & Issues Navigation Fixes

- **Resizable, wrapping Crawl Results columns:** set sensible default widths across all 8 Crawl Results columns, enable column resizing (`set_resizable(true)`) with visible column separators, and wrap long URLs, indexability labels, and titles (`WordChar` with full-text tooltips) so long addresses no longer push the rest of the table off-screen.
- **Synced `Issues` → `Crawl Results` filter jump:** clicking **Open in Crawl Results** from an Issue now clears stale search queries and automatically selects the matching filter in **SEO Audit Filters** before highlighting and scrolling to the Affected URL.
- **Stable window height & scrolling:** fix a GTK4 height-for-width layout bug where wrapping labels in hidden notebook tabs could push the bottom **Details** panel off-screen and disable scrolling in **SEO Audit Filters** after a crawl completed.

## 0.6.0 — Prioritized Issues, `.tadpole` Projects, and Platform Hardening

- **Dedicated `Issues` workspace:** triage crawl and Sitemap & Discovery findings together in a native GNOME/Libadwaita tab, grouped into named Issues across four fixed Severity tiers (**Critical**, **High**, **Medium**, **Low**).
- **Transparent audit summary (no arbitrary 0–100 health score):** review exact Severity counts, Affected URL percentages, explicit scope-honesty caveats (stopped crawls, URL-capped runs, or disabled/missing sitemaps), per-Issue Rationale, Suggested Actions, and per-URL Evidence.
- **2xx-indexable content gating:** content, metadata, social, and structured-data Issues evaluate only on 2xx indexable HTML pages so 4xx, 5xx, 3xx, and noindexed URLs do not pollute lower-severity counts.
- **Developer-ready exports & cross-tab navigation:** export full-audit or single-Issue reports to Markdown (with a 25-URL inline sample cap and one-click clipboard copy) or uncapped CSV, and jump directly from any Affected URL into **Crawl Results** or **Sitemap & Discovery**.
- **`.tadpole` Project branding & direct open:** save new Projects as `.tadpole` files with Linux MIME/desktop and Windows installer file associations, open Projects directly via OS double-click or CLI arguments (with active-crawl/replacement guards), and continue opening legacy `.seocrawl` Projects seamlessly.
- **Platform hardening:** Windows packaging audit and reserved-filename hardening with a clean-machine [Windows validation checklist](https://github.com/piotrowskiadam/Tadpole/blob/main/docs/WINDOWS_VALIDATION.md), Flatpak document-portal save hardening and [sandbox smoke-test checklist](https://github.com/piotrowskiadam/Tadpole/blob/main/docs/FLATHUB_READINESS.md), and refreshed Snap Store metadata.

Existing `.seocrawl` Projects remain readable via the Open dialog and CLI. New saves default to `.tadpole`. Severity tiers express contextual audit guidance for prioritization rather than a guaranteed traffic or ranking impact.

## 0.5.0 — Sitemap & Discovery

- Collect XML sitemap URL sets, indexes and gzip files from robots.txt or manual same-site sources, with bounded downloads and visible source errors.
- Compare declarations with real link reachability; inspect candidate orphan pages, sitemap HTTP errors, redirects, noindex directives, canonical differences and URLs absent from imported sources.
- Keep unknown observations explicit, preserve crawl limits and cancellation, and save discovery evidence with `.seocrawl` projects. Legacy projects remain readable.
- Review findings in a native searchable/sortable report, export coverage-aware CSV, and explicitly analyze reachable linking sources for a selected candidate target.
- Preserve the full Crawl Results workspace inside its tab. Unknown auxiliary depth is displayed as unknown throughout tables, export and graph metrics.

Existing `.seocrawl` projects remain readable. Recrawl to collect discovery evidence for older projects. Candidate orphan findings describe observed crawl scope, and missing sitemap membership is a review opportunity; neither establishes Google indexing. Source changes apply to a new crawl through **Manage sitemaps → Apply and recrawl**.

Validation includes 53 passing tests in a serialized run, strict core Clippy, an optimized Linux build and native light/dark review against a real local HTTP fixture. Windows packages are built by CI; clean-machine Windows runtime, accessibility and native file-dialog operation have not been independently validated. Snap Store publication is separate from the GitHub release.

## 0.4.2 — Richer page inspection

- **Inlinks and Outlinks:** inspect each link occurrence in wrapping tables with source URL, target URL, anchor text and `rel`. Fragment targets and repeated links are retained. Activate a row to select the related crawled page, or export the richer data to CSV.
- **Images:** see thumbnails, measured image resolution and file size, alt text and missing/empty-alt status, declared HTML dimensions, image title, loading mode and local file details. Previews load on demand with bounded downloads and a thumbnail cache.
- **Social / OG:** inspect illustrative Open Graph and Twitter/X preview cards alongside the original metadata, including image dimensions, alt text and site information. Twitter previews identify fallback to available Open Graph values.
- **Word wrapping:** read long URLs and text throughout the bottom detail panels, including headings, JSON-LD and Markdown, without extensive horizontal scrolling.
- **Documentation:** refreshed README, app-view banners, support links and an offline detail-tab preview for development.

Existing `.seocrawl` projects remain compatible. Recrawl source pages to collect anchor text, HTML image attributes and additional social metadata that older versions did not save. Image CSV dimensions are the declared HTML attributes; measured resolution and file size are shown in the on-demand preview. Social previews are illustrative and may differ from the platforms' rendering.

The upstream Flatpak bundle is distributed through GitHub and is not a Flathub submission. This release also retains the native site visualizer, local SEO audit tools and explicitly initiated page-level AI suggestions.

## AI development disclosure

Tadpole is developed primarily by orchestrating AI coding agents. An experienced SEO professional sets the direction and checks each update. Read the [AI development disclosure](https://github.com/piotrowskiadam/Tadpole/blob/main/AI_DISCLOSURE.md) for details.

## Before upgrading

API credentials are no longer saved in project files and must be entered again for each session. Older project files may still contain previously saved keys: remove those keys and rotate them if the files were shared. The former standalone SEO Studio workspace has been removed.

Linux packages include DEB, RPM, AppImage, Snap and Flatpak; Windows packages include a ZIP and installer. The AppImage requires compatible GTK 4.12+ and Libadwaita 1.5+ libraries on the host. Windows packaging has not been independently validated on a clean Windows machine. The Snap package in this GitHub release does not imply an updated Snap Store listing.

See the [README](https://github.com/piotrowskiadam/Tadpole#readme) and [development documentation](https://github.com/piotrowskiadam/Tadpole/blob/main/docs/DEVELOPMENT.md) for requirements and usage.

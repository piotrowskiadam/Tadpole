# Tadpole releases

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

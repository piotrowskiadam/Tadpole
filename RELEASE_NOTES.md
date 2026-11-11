# Tadpole releases

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

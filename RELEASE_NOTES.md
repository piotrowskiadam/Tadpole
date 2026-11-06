# Tadpole releases

Tadpole includes a native site-visualization workspace and local SEO audit tools. Version-specific changes are listed in the automatically generated notes below.

- Explore directory treemaps and icicles, shortest-path trees, URL link neighborhoods, section-link matrices, and depth versus internal rank.
- Prepare visualizations in the background, cache drawing geometry, and inspect results in a searchable, paginated evidence list.
- Audit crawl status, indexability, metadata, structured data, images, and internal links; export filtered results and Markdown.
- Use optional page-level OpenRouter or OpenAI metadata suggestions. Paid AI requests are initiated explicitly.
- Benefit from crawler lifecycle, persistence, accessibility, documentation, and validation improvements.

## AI development disclosure

Tadpole is developed primarily by orchestrating AI coding agents. An experienced SEO professional sets the direction and checks each update. Read the [AI development disclosure](https://github.com/piotrowskiadam/Tadpole/blob/main/AI_DISCLOSURE.md) for details.

## Before upgrading

API credentials are no longer saved in project files and must be entered again for each session. Older project files may still contain previously saved keys: remove those keys and rotate them if the files were shared. The former standalone SEO Studio workspace has been removed.

Linux packages include DEB, RPM, AppImage, Snap and Flatpak; Windows packages include a ZIP and installer. The AppImage requires compatible GTK 4.12+ and Libadwaita 1.5+ libraries on the host. Windows packaging has not been independently validated on a clean Windows machine. The Snap package in this GitHub release does not imply an updated Snap Store listing.

See the [README](https://github.com/piotrowskiadam/Tadpole#readme) and [development documentation](https://github.com/piotrowskiadam/Tadpole/blob/main/docs/DEVELOPMENT.md) for requirements and usage.

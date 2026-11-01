<p align="center"><img src="tadpolelogonobg.png" alt="Tadpole" width="140"></p>

# Tadpole

A local desktop SEO crawler built with Rust, GTK4 and Libadwaita. Audit a website, inspect its internal links and structured data, export page content and generate page-level metadata suggestions.

[Downloads](https://github.com/piotrowskiadam/Tadpole/releases) · [Issue tracker](https://github.com/piotrowskiadam/Tadpole/issues) · [Developer guide](docs/DEVELOPMENT.md) · [Architecture](ARCHITECTURE.md) · [Modernization audit](docs/AUDIT.md)

## Website audits

- Crawl a site, a path, one URL, or an explicit list of URLs.
- Configure URL/depth limits, concurrency, include/exclude regular expressions, redirects and robots.txt handling.
- Inspect HTTP status, titles, descriptions, canonical URLs, indexability, headings, images, Open Graph/Twitter metadata and JSON-LD syntax.
- Search and filter results; export the filtered table to CSV or copy it to a spreadsheet.
- Inspect Markdown extracted from the main page content and export it to files with collision-resistant names.
- Find internal-link opportunities and recurring phrases in titles/headings.
- Explore URL directory structure in the native Cairo icicle visualizer.
- Save and reopen `.seocrawl` projects. API credentials are excluded from these files.

SEO health is a local heuristic, not a Google score. Schema diagnostics check syntax and selected fields; they do not certify rich-result eligibility. A crawl cannot discover truly orphaned pages that have no links from the seed; provide those URLs in List mode.

## Optional AI metadata suggestions

In **Preferences → AI Assistant**, choose OpenRouter or OpenAI, enter your API key and select a model (or enter a custom model ID). The selected page's details offer title, description and image alt-text suggestions based on crawled page content.

**Privacy and cost:** ordinary crawling stays local apart from requests to crawled websites. Clicking a page-level AI action sends the page URL, metadata, headings and up to 12,000 characters of page text to the chosen provider. The provider bills your account. Requests have timeouts and input/output limits; there are no automatic paid requests or application-level retries. API credentials stay in memory for the session and must be entered again after restarting. They are excluded from project files and config debug output; embedded legacy project keys are ignored. Older files may still contain keys saved by previous versions—remove those keys and rotate them if the files were shared.

Review generated suggestions before using them. Image alt-text suggestions are text-only and do not inspect image pixels.

## Build and run

Install a current stable Rust toolchain (edition 2024), GTK **4.12+**, Libadwaita **1.5+**, a C compiler and pkg-config. A graphical desktop is required for the application.

Ubuntu 24.04:

```sh
sudo apt-get install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
cargo run --locked
```

Fedora:

```sh
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel
cargo run --locked
```

A headless core build/test needs no GTK development packages:

```sh
cargo test --locked --no-default-features
```

Optional JavaScript rendering needs an installed Chrome/Chromium discoverable by `headless_chrome`. It is not bundled. See the [developer guide](docs/DEVELOPMENT.md) for checks, preview rendering, packaging and known limitations.

## Packages

Release workflows run on version tags and build Linux DEB/RPM/AppImage/Snap and Windows ZIP/Inno Setup installers. Package availability depends on successful release jobs; see the actual [release assets](https://github.com/piotrowskiadam/Tadpole/releases).

- Linux: `cargo build --release --locked`, then `cargo deb`, `cargo generate-rpm`, `./build_appimage.sh`, or `snapcraft` as appropriate.
- Windows: install MSYS2 and Inno Setup, then run `build_windows.ps1` from the repository directory.
- The current AppImage script packages the application and icon; it **does not bundle GTK/Libadwaita**, so it requires compatible host libraries.

## Contributing

Read [DEVELOPMENT.md](docs/DEVELOPMENT.md) before changing execution or persistence behavior. Include a regression test for fixes to crawling, parsing, provider responses or stored data. Keep paid-provider tests offline with fixtures or a local HTTP server. Do not commit credentials, real crawl files or generated packages.

MIT licensed. Built with AI assistance.

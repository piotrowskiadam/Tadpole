<p align="center"><img src="tadpolelogonobg.png" alt="Tadpole" width="140"></p>

# Tadpole

A local desktop SEO crawler built with Rust, GTK4 and Libadwaita. Audit a website, inspect its internal links and structured data, export page content, or research a topic in SEO Studio.

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

## SEO Studio: content briefs and Google SERP

Choose **SEO Studio** in the header. It works independently of a crawl.

1. In **Preferences → AI Assistant**, choose OpenRouter or OpenAI, enter your key, and select a model (or enter a custom model ID).
2. Optionally enter your **DataForSEO API login and password** on the same preferences page. These differ from some account login credentials; use [API access](https://app.dataforseo.com/api-access).
3. Enter a topic/keyword and choose Poland/Polish, United States/English, or United Kingdom/English.
4. **Fetch Google SERP** sends one DataForSEO Live Advanced request with depth 10. The report includes organic results, snippets, URLs and the task cost returned by DataForSEO.
5. **Generate content brief** sends one completion request to the chosen AI provider. It can use the SERP evidence and, if selected, content from the page selected when Studio opened. Without SERP evidence, competitive/search-intent suggestions are explicitly treated as hypotheses.
6. Review the result and copy it or save it as Markdown. The brief includes intent, audience, metadata, outline, questions, differentiation, internal-link suggestions and a fact-check checklist.

Changing the keyword or market invalidates the SERP evidence. Reports remain separate: generating a brief never overwrites the source SERP report. Brief output follows the selected market language. To include a different page, select it in the crawl table and reopen Studio.

**Privacy and cost:** ordinary crawling stays local apart from requests to the crawled websites. Clicking an AI action sends the displayed topic/context to your selected provider; fetching SERP sends the keyword/market to DataForSEO. Providers bill your account. There are no automatic paid requests or application-level retries. Completion input/output and response sizes are bounded, but token limits do not guarantee a fixed monetary cost. Credentials are held in memory for the session and must be entered again after restarting. They are never serialized, imported from old project keys, or included in config debug output. Older project files created by previous versions may already contain keys; remove those keys and rotate them if the files were shared.

Existing title and description suggestions now use crawled content as context. Image alt-text suggestions remain text-only suggestions based on image URL/page context; they do not inspect image pixels.

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

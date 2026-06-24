# Tadpole

Tadpole is a native desktop SEO crawler and site auditor built in Rust and GTK4/Libadwaita. It provides a simple, fast, and visual way to crawl websites, inspect SEO parameters, run audits, and analyze structured schema data on Linux and Windows.

<img src="tadpolelogonobg.png" alt="Tadpole Logo" width="160">

---

## Download

You can download the latest pre-compiled binaries and installers for all supported platforms directly from the [Releases](https://github.com/piotrowskiadam/Tadpole/releases) page.

### Available Packages

Tadpole release assets are built and published automatically for the following formats:

[![Get it from the Snap Store](https://snapcraft.io/en/dark/install.svg)](https://snapcraft.io/tadpole)

| OS | Format | Type | Installation / Run Command |
| :--- | :--- | :--- | :--- |
| **Linux** | **AppImage** (`.AppImage`) | Portable Standalone Binary | `chmod +x Tadpole-*.AppImage && ./Tadpole-*.AppImage` |
| **Linux** | **Debian** (`.deb`) | Native Package (Debian/Ubuntu) | `sudo dpkg -i tadpole_*.deb` |
| **Linux** | **RPM** (`.rpm`) | Native Package (Fedora/RHEL) | `sudo rpm -i tadpole-*.rpm` |
| **Linux** | **Snap** (`.snap`) | Strictly Confined Snap | `sudo snap install tadpole_*.snap --dangerous` |
| **Windows** | **Setup Installer** (`.exe`) | Executable Installer | Run `TadpoleSetup.exe` to install and create shortcuts |
| **Windows** | **Standalone Archive** (`.zip`) | Standalone Folder | Unzip `tadpole-windows.zip` and run `tadpole.exe` |

---

## Key Features

### 1. Multi-Mode Crawling
- **Crawl Mode**: Recursively crawls and discovers internal links starting from a seed URL.
- **List Mode**: Audits a predefined list of static URLs entered manually or loaded from a local text file.
- **Path Mode**: Restricts crawls only to pages belonging to the same subfolder category (prefix path) as the seed URL.
- **URL Mode**: Single URL inspection; crawls only the seed page and stops.

### 2. Live SEO Diagnostics Grid
- **Comprehensive Auditing**: Inspect status codes, indexability, canonical URLs, page titles, meta descriptions, H1/H2 tags, word counts, and page sizes.
- **Advanced Metrics**: Tracks both **Crawl Depth** (number of hops from seed) and **Response Time (ms)**.
- **Search & Filter**: Find matches instantly across all crawled columns using a live search entry box, or apply left-sidebar filters (such as showing only pages missing canonical tags or structured data errors).

### 3. SEO Health Dashboard
- **SEO Health Score**: Computes a live health rating based on issue frequencies.
- **Response Metrics**: Average response times, page sizes, and word counts.
- **Depth Distribution**: Visual distribution graph rendered dynamically using Cairo.

### 4. Interactive Visual Site Map
- **Force-Directed Graph**: Opens a separate canvas visualizing crawled pages as nodes and directories as structural hubs.
- **Interactive Controls**:
  - Hover or click nodes to display page titles and paths.
  - Click directory nodes to collapse/expand entire folders (collapsed folders display child counts).
  - Use toolbar controls for zoom, centering, expanding, or collapsing the entire site map.

### 5. Social & Open Graph Audit
- **Metadata Extraction**: Parsed Open Graph (`og:title`, `og:description`, `og:image`, `og:url`, `og:type`) and Twitter Card metadata.
- **Diagnostic tab**: Shows social tags, active warning indicators (such as mismatches between standard titles/descriptions and their social equivalents), and missing essentials.

### 6. Schema & Structured Data Checker
- **JSON-LD Schema Extraction**: Extracts nested JSON-LD schema blocks from pages.
- **Syntax Highlighting**: Monospace code editor with real-time character-offset syntax highlighting for pretty-printed raw JSON.
- **Combined View**: Visually separated "Combined View" section that merges all JSON-LD schemas into a single array for overall inspection, alongside a list of individual schema blocks.
- **Validation Errors**: Evaluates structures for `@context` validity and `@type` presence.

### 7. Headings Outline Tab
- **Hierarchy Tree**: Displays headings (`H1` to `H6`) in document order.
- **Copy Outline**: A toolbar button that copies the structured heading tree to the clipboard with indentation. Includes visual "Copied!" feedback.

### 8. AI Metadata Assistant
- **Optimized Suggestions**: Integrates with OpenAI and OpenRouter to analyze page content and suggest optimized metadata.
- **Dynamic Model Selector**: Dropdown selection displaying active provider models with a background refresh mechanism, search/filter entry, and custom model inputs.

### 9. Markdown Scraping & Exporter
- **Page Content Conversion**: Extracts core article/body text (filtering headers, footers, navigation, sidebars, and custom CSS selectors) and converts it to clean GitHub Flavored Markdown (GFM).
- **Tab Preview**: Shows a monospace preview of the page's Markdown content under the details panel, with options to copy to clipboard or save to a file.
- **Batch Directory Export**: Click the export dropdown in the header bar to export all crawled pages to a selected folder as separate `<slug>.md` files.

### 10. Comprehensive SEO CSV Audit
- **34 Columns of Data**: Export complete details for every single crawled page, including:
  - Meta titles and descriptions, along with their character lengths.
  - All internal links pointing to a page (`Inlinks`) and all links pointing outward (`Outlinks`).
  - Image paths paired with their `alt` text.
  - Social sharing (Open Graph and Twitter) tags.
  - A formatted tree of headings in document order (e.g., `H1: text | H2: text`).
  - Merged raw JSON-LD schemas and schema validation errors.
  - Word count, response times, sizes, depths, indexability status, and canonical URLs.

---

## System Requirements

### Linux
- GTK4 (`libgtk-4-dev`)
- Libadwaita (`libadwaita-1-dev`)
- Rust toolchain (2024 edition)

### Windows
- MSYS2 (with MinGW64 toolchain)

---

## How to Build and Run

### Running Locally (Linux / Development)
Ensure GTK4 and Libadwaita development libraries are installed, then run:

```bash
cargo run
```

### Packaging for Linux

Tadpole supports multiple native package formats on Linux:

#### Snap Package
Ensure `snapcraft` is installed on your Linux system, then build the snap package locally using:
```bash
snapcraft
```

#### Debian Package (.deb)
Install `cargo-deb` (e.g. `cargo install cargo-deb`), then build the package using:
```bash
cargo deb
```
The resulting `.deb` package will be saved in `target/debian/`.

#### RPM Package (.rpm)
Install `cargo-generate-rpm` (e.g. `cargo install cargo-generate-rpm`), then compile and build the package using:
```bash
cargo build --release
cargo generate-rpm
```
The resulting `.rpm` package will be saved in `target/generate-rpm/`.

#### AppImage (.AppImage)
To package Tadpole as a portable, standalone Linux binary:
```bash
./build_appimage.sh
```
The script structures the AppDir and compiles the final `Tadpole-x86_64.AppImage` automatically (downloading `appimagetool` internally if not present on your system).

---

### Packaging for Windows

Tadpole is compiled on Windows inside the MSYS2 environment. It bundles the required DLLs and schemas automatically, and compiles a single-file executable installer using Inno Setup.

1. Install [MSYS2](https://www.msys2.org/) to `C:\msys64`.
2. Install [Inno Setup](https://jrsoftware.org/isinfo.php) on your Windows machine to generate the installer.
3. Open PowerShell in the project root directory.
4. Run the packaging script:
   ```powershell
   .\build_windows.ps1
   ```
5. The standalone folder, the zip package, and the single-file setup installer (`TadpoleSetup.exe`) will be generated inside the `dist/` directory.

---

### Automated CI/CD Releases (GitHub Actions)
A GitHub Actions workflow is configured in `.github/workflows/release.yml`. Whenever you push a version tag (e.g. `v0.1.0`), the pipeline automatically builds the Snap, Debian, RPM, AppImage, ZIP, and Inno Setup EXE installer packages, publishing all of them directly as release assets.

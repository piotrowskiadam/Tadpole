# Clean-Machine Windows Validation Checklist

Run this checklist on a clean Windows 10/11 x64 VM or machine (without MSYS2, Rust, or GTK installed) before cutting a Tadpole release.

## 0. Build artifacts & pre-flight

- [ ] Download `TadpoleSetup.exe` and `tadpole-windows.zip` from the `Build Windows Packages` CI job (or local `build_windows.ps1` / `build_windows.sh` run).
- [ ] Confirm `setup.iss` `AppVersion` matches `Cargo.toml` version.
- [ ] Verify the portable `tadpole-windows.zip` extracts cleanly and launches `tadpole.exe` with no missing DLL errors and no stray console window (`#![windows_subsystem = "windows"]`).

## 1. Install & uninstall

- [ ] Run `TadpoleSetup.exe` (test both default all-users install and, optionally, per-user install via the privileges dialog).
- [ ] Confirm Start Menu entry, Desktop shortcut, and app icon render using `tadpole.ico`.
- [ ] Confirm `tadpole.exe` launches cleanly from the Start Menu and renders all toolbar/sidebar SVG icons (confirms bundled `loaders.cache` and `Adwaita` icon theme work).
- [ ] After completing sections 2–6 below, uninstall Tadpole from **Settings → Apps → Installed apps**.
- [ ] Verify the install directory, Start Menu/Desktop shortcuts, and `.tadpole` / `Tadpole.Project` registry keys under `HKLM\Software\Classes` (or `HKCU\Software\Classes` for per-user install) are cleanly removed.

## 2. Double-click `.tadpole` open & OS file association

- [ ] Save a Project to a folder whose path contains spaces (for example `C:\Users\<user>\Desktop\Client Audits\example.com.tadpole`).
- [ ] Confirm Explorer displays the `.tadpole` file with the Tadpole icon and type `Tadpole Project`.
- [ ] Close Tadpole, then double-click the `.tadpole` file in Explorer:
  - [ ] Tadpole launches cold and loads the saved Project (URLs, settings, and discovery data if present).
- [ ] With Tadpole still open and displaying crawl data, double-click a second `.tadpole` file:
  - [ ] Tadpole brings the existing window forward and prompts with **Discard Current Crawl Results?** before replacing workspace data.
- [ ] Start a live crawl and double-click a `.tadpole` file while crawling:
  - [ ] Tadpole blocks replacement and shows the status message asking you to stop the crawl first.
- [ ] From Command Prompt or PowerShell, run `"C:\Program Files (x86)\Tadpole\tadpole.exe" "C:\path with spaces\project.tadpole"` (or the per-user install path) and confirm it opens the Project.

## 3. Legacy `.seocrawl` open compatibility

- [ ] Click **Open** and confirm the native Windows file dialog defaults to `Tadpole Projects (*.tadpole;*.seocrawl)` and displays both `.tadpole` and legacy `.seocrawl` files.
- [ ] Open a legacy `.seocrawl` Project via the Open dialog and confirm all crawled URLs and settings load cleanly.
- [ ] Open a legacy `.seocrawl` Project via CLI (`tadpole.exe legacy.seocrawl`) and confirm it loads cleanly.
- [ ] Click **Save** and confirm the Save dialog defaults to `project.tadpole` with `Tadpole Projects (*.tadpole)` selected as the active filter.

## 4. Crawl, `Issues` workflow & Project save edge cases

- [ ] Run a small crawl against a live or local HTTP site (for example 10–30 URLs).
- [ ] Open the `Issues` tab and confirm:
  - [ ] Severity counts (`Critical`, `High`, `Medium`, `Low`) and Affected URL percentages populate without UI stalls.
  - [ ] Selecting an Issue displays Rationale, Suggested Action, and per-URL Evidence.
  - [ ] Clicking an Affected URL jumps to the corresponding row in **Crawl Results** or **Sitemap & Discovery**.
- [ ] Save a Project typing a domain name without `.tadpole` (for example `example.com`) and verify Tadpole saves `example.com.tadpole` (not a `.com` file).
- [ ] Save again over the same existing `.tadpole` path and verify the overwrite succeeds cleanly with updated data and no stray temporary files left in the folder.

## 5. Export dialogs & Windows filename safety

- [ ] Export **Crawl Results** CSV (both unfiltered and filtered): verify the native Save dialog defaults to `CSV Files (*.csv)` with a `.csv` filename and writes a valid CSV readable in Excel/Notepad.
- [ ] Export **Markdown** to a folder: verify folder picker works and writes `.md` files (including pages whose URL path ends in a Windows reserved name like `/con` or `/aux` if tested).
- [ ] Export **Issues** Markdown and CSV (full audit and single selected Issue) and test **Copy Markdown** to the Windows clipboard (paste into Notepad/VS Code).
- [ ] Export **Sitemap & Discovery** CSV, **Link Opportunities** CSV, **Details** CSV/Markdown, and **Site Visualizer** evidence (`nodes.csv` / `edges.csv` / `neighborhood.json`) and confirm each dialog shows its expected default filename and filter.

## 6. Accessibility & theme smoke checks

- [ ] Switch Windows between **Light** and **Dark** mode (**Settings → Personalization → Colors**) and verify Tadpole updates cleanly with legible text, borders, and Severity badges.
- [ ] Test at 125% or 150% Windows display scaling and verify icons, tables, and dialogs remain sharp without clipped controls.
- [ ] Navigate the header bar, workspace tabs, and `Issues` list using `Tab`, `Shift+Tab`, arrow keys, and `Enter` / `Space`, confirming visible keyboard focus indicators.

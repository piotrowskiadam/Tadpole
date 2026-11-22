# Tadpole Coding Standards

Judgement-call rules enforced during `/code-review`. Mechanical checks (formatting, core tests, strict core Clippy, version alignment, AppStream/desktop validation) live in `scripts/check.sh`.

## GTK4 / Libadwaita UI

- **Windows file-dialog filters**: Every `gtk::FileDialog` that attaches file filters must call both `set_filters` and `set_default_filter(Some(&filter))` so native Win32 file dialogs activate the expected filter dropdown.
- **Plain text for crawled and external data**: Never render crawled HTML metadata, URLs, API responses, or AI output as GTK/Pango markup. Keep user-facing values as selectable plain text.
- **Native GNOME HIG**: Use Libadwaita semantic colors and native controls (`DESIGN.md`). Avoid custom fixed hex palettes or web-style chrome in desktop widgets.
- **No blocking on the GTK main thread**: Keep file I/O, project serialization, CSV/Markdown exports, and HTTP/crawl work off the main thread.

## Documentation & Domain Integrity

- **Single source of truth in `ARCHITECTURE.md`**: When adding, moving, or renaming modules or export formats, verify every module path and exported CSV/Markdown column list in `ARCHITECTURE.md` matches the code 1:1.
- **Domain vocabulary and ADRs**: Use canonical terms from `GLOSSARY.md` and honor `docs/adr/` decisions without silently overriding them.
- **Flathub policy**: Never author or modify a Flathub submission manifest or submission PR (`docs/FLATHUB_READINESS.md`).

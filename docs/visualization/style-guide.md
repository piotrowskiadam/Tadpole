# Tadpole diagram style receipt

Scope: `index.html` in this directory, selected by the user as **Match Tadpole's existing style**. Status: **pending user verification**. This is a project-local record, not an installed Diagram Design profile. No global skill file or profile marker was overwritten.

## Sources and mapping

- `DESIGN.md`: native GTK4/Libadwaita controls, system typography, semantic colors and system light/dark preference.
- `src/ui/theme.rs`: inherited typography and platform foreground/accent/status roles.
- Local desktop font setting: `Adwaita Sans 11`; `fc-match` confirms Adwaita Sans and Cantarell are installed. Local monospace resolution is Noto Sans Mono.
- The HTML colors below are explicit Adwaita-like approximations for the static study. The production GTK UI should resolve its actual theme tokens; a custom desktop theme may use different colors. These are not asserted to be an extraction of GTK's computed palette.

| Diagram role | Light | Dark | Application |
|---|---|---|---|
| paper | `#fafafa` | `#242424` | Canvas/page |
| paper-2 | `#ededed` | `#303030` | Neutral node surfaces |
| ink / ink-strong | `#2e3436` | `#f6f5f4` | Main text |
| muted / soft | `#5e5c64` | `#c0bfbc` | Supporting text, no opacity reduction |
| rule | `#d5d3d7` | `#4c4c4c` | Decorative separators/grid |
| rule-solid / essential edge | `#77767b` | `#9a9996` | Topology, outlines |
| accent / link | `#1c71d8` | `#78aeed` | One selected item, text links |
| accent-tint | `#dfebf8` | `#2e3540` | Selected surfaces |

Compared with Diagram Design defaults, this replaces the editorial gray/tangerine palette with native-like neutral/blue roles. It replaces Instrument Serif titles and Geist families with system sans typography. Topology, accessibility and geometry conventions stay in place. No status rainbow or multi-series palette is used.

## Typography

Font source: **system**, with no network font request. Sans stack: `'Adwaita Sans', Cantarell, 'Noto Sans', system-ui, sans-serif`. Technical stack: `'Noto Sans Mono', ui-monospace, monospace`. On a machine without these named fonts, the system fallback applies; exact glyph metrics can differ.

| Role | Family | Size | Weight |
|---|---|---:|---:|
| Page title | Sans | 32 px desktop / 28 px narrow | 700 |
| Figure/section title | Sans | 24 px | 600–650 |
| Node/row/column name | Sans | 14 px | 600 |
| Supporting text | Sans | 14 px | 400 |
| URL / compact request key | Mono | 12 px | 400 |
| Legend / axis tick | Sans | 12 px | 400 |

Native-style readability takes precedence over the skill's smaller default type ramp. Monospace is reserved for technical strings. No fallback font is represented as a fetched brand font.

## Geometry and contrast

960 × 600 canvases; 4 px structural grid; 6 px native-like node corners; rounded orthogonal elbows with 8 px radii. No shadows or decorative dot field. Every SVG has unique title/description IDs and a bottom legend. The local scroller preserves intrinsic SVG width; print overrides it to prevent clipping.

Contrast ratios, calculated from the actual sRGB tokens:

| Pair | Light | Dark |
|---|---:|---:|
| Ink / paper | 12.12 | 14.26 |
| Muted / paper | 6.31 | 8.44 |
| Muted / maximum 16% ink ramp | 4.71 | 5.18 |
| Accent/link / paper | 4.57 | 6.71 |
| Accent border / accent tint | 3.95 | 5.34 |
| Essential edge / paper | 4.32 | 5.45 |

Small text uses ink/muted on selected tints; accent is not used as small text on its tint. Neutral data fills are at most 16% ink, with literal numeric values and essential outlines providing independent evidence. Decorative rules do not carry unique meaning. Focus outlines use the accent. The final user check must include actual rendering and any different local font/theme configuration.

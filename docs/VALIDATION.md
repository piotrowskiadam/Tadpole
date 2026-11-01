# Validation record

Local environment: Linux x86_64, Rust 1.99.1, GTK 4.22.5, Libadwaita 1.9.4. Work started 2026-10-31 and final checks completed 2026-11-01. GitHub CI/package results are separate.

## Final checks

| Command | Result |
| --- | --- |
| `cargo test --locked --no-default-features` | 29 tests passed: 13 module tests, 10 core regressions, 6 graph tests |
| `cargo test --locked` | The same 29 tests passed with the desktop feature; binary/doc targets also passed |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --locked --no-default-features --all-targets -- -D warnings` | Passed without warnings |
| `cargo check --locked` | Desktop compilation passed |
| `cargo build --locked --example ui_preview` | Native preview compiled |
| `git diff --check` | Passed |

The local HTTP regression tests require loopback permission. Their initial sandbox bind failures were environment errors; authorized runs passed. Paid production endpoints are not used by the tests.

## Visual evidence and limits

The real crawl window and SEO Studio were rendered through GTK's isolated Broadway backend and inspected in a browser. The crawl view was checked in the dark appearance at a 1024px window width; Studio was inspected in light appearance, with explicit cost/context information, separate report tabs and disabled initial copy/save controls. A compact Studio view was used for the final layout check. Studio uses vertical scrolling to keep actions reachable in short windows; essential transmission/cost copy uses normal foreground text.

Direct PNG exports through the Cairo renderer produced invalid colors for Broadway-specific render nodes, so that export helper was removed. The retained preview example opens the real widgets for inspection. Browser forwarding did not provide a reliable automated interaction test in this environment; no claim is made that keyboard, screen-reader, clipboard or file-picker behavior was exhaustively tested.

Two independent code-review axes confirmed their findings resolved; see [REVIEW.md](REVIEW.md). No live paid-provider request, release-profile performance comparison, Windows/Snap installation or package release was performed. See [DEVELOPMENT.md](DEVELOPMENT.md) for the remaining release checks.

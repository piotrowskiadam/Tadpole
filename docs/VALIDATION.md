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
| `cargo build --locked` | Desktop application rebuilt after removal of the standalone workspace |
| `git diff --check` | Passed |

The local HTTP regression tests require loopback permission. Their initial sandbox bind failures were environment errors; authorized runs passed. Paid production endpoints are not used by the tests.

## Visual evidence and limits

The real crawl window was rendered through GTK's isolated Broadway backend and inspected in a browser, including the dark appearance at a 1024px window width. The standalone research workspace and its preview options have since been removed at the owner's request; earlier visual checks of that workspace do not apply to the current product. The remaining native preview opens only the crawl window.

Direct PNG exports through the Cairo renderer produced invalid colors for Broadway-specific render nodes, so that export helper was removed. The retained preview example opens the real widgets for inspection. Browser forwarding did not provide a reliable automated interaction test in this environment; no claim is made that keyboard, screen-reader, clipboard or file-picker behavior was exhaustively tested.

Two independent code-review axes confirmed their findings resolved; see [REVIEW.md](REVIEW.md). No live paid-provider request, release-profile performance comparison, Windows/Snap installation or package release was performed. See [DEVELOPMENT.md](DEVELOPMENT.md) for the remaining release checks.

After the owner requested removal of the standalone research workspace, both 29-test suites, core Clippy with `-D warnings`, formatting, desktop build and diff checks were rerun successfully. Obsolete workspace/provider tests and settings were removed; the completion-response and credential tests remain.

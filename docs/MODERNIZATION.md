# Modernization scope

Source: the owner's modernization request on 2026-10-31. The owner subsequently requested removal of the standalone research workspace; additional provider workflows are deferred pending a concrete feature specification.

- Inspect all tracked application modules, tests, packaging and documentation; record verifiable findings with priorities and limitations.
- Preserve the native Rust/GTK product and existing local crawl/export workflows.
- Correct high-impact lifecycle, data, privacy and performance defects identified in the audit.
- Improve UI readability, configuration guidance, error feedback and compact-window use using the incumbent Libadwaita system.
- Retain the existing page-level AI metadata suggestions through OpenRouter/OpenAI.
- Keep API secrets out of files/debug output; document transmissions/cost, use timeouts and bounded requests. Test without paid production requests.
- Add reproducible core tests/CI and update architecture/build documentation. Distinguish measured evidence from proposals.
- Push verified changes to a GitHub branch and open a reviewable PR; do not implicitly publish packages or tag a release.

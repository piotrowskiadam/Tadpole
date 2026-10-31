# Modernization scope

Source: the owner's request on 2026-10-31, with the selected feature direction “content briefs and SERP analysis”.

- Inspect all tracked application modules, tests, packaging and documentation; record verifiable findings with priorities and limitations.
- Preserve the native Rust/GTK product and existing local crawl/export workflows.
- Correct high-impact lifecycle, data, privacy and performance defects identified in the audit.
- Improve UI readability, configuration guidance, error feedback and compact-window use using the incumbent Libadwaita system.
- Add a separate SEO research workflow: manual Google organic SERP via DataForSEO and an editorial brief via OpenRouter (retain OpenAI support).
- Allow a brief without a crawl or SERP; use selected page content/evidence only when available and requested. Keep source evidence separate from AI output.
- Keep API secrets out of files/debug output; document transmissions/cost, use timeouts and bounded requests. Test without paid production requests.
- Add reproducible core tests/CI and update architecture/build documentation. Distinguish measured evidence from proposals.
- Push verified changes to a GitHub branch and open a reviewable PR; do not implicitly publish packages or tag a release.

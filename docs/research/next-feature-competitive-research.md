# Tadpole next-feature comparison

Researched 12 November 2026 against official product pages and tutorials. This is a product recommendation, not a demand survey or implementation plan. Documentation establishes capabilities, not their prevalence of use or ranking impact. No competitor application was tested.

## Recommendation

Build **XML sitemap discovery and crawl reconciliation**, presented as a practical sitemap/indexability audit. It should answer: which declared URLs have technical problems, which cannot be reached through the site's internal links, and which crawled indexable URLs are absent from the supplied sitemaps?

This is a judgment based on Tadpole's current scope. Sitemap ingestion remains explicitly unfinished in [the developer guide](../DEVELOPMENT.md). Tadpole already has metadata/indexability checks, internal-link opportunities, graph analysis, visualizations and project persistence; see [README](../../README.md) and `src/ui/link_opportunities.rs`. Discovering URLs that a link-following crawl misses extends the evidence available to those existing features. Adding another presentation of already discovered pages has less immediate diagnostic breadth.

This recommendation closes a capability gap. Sitemap auditing and orphan discovery are established competitor features; novelty would come from clarity, local operation and connecting findings to Tadpole's existing link analysis.

## Competitive evidence

| Official source | Verified capability | Implication for Tadpole |
| --- | --- | --- |
| [Screaming Frog: finding orphan pages](https://www.screamingfrog.co.uk/seo-spider/tutorials/find-orphan-pages/) | Combines sitemap, Analytics and Search Console URL sources with the internal crawl. Defines orphan status by absence of an observed path from the starting page; disconnected pages can link to each other. Source and orphan reports are exportable. | Track provenance and reachability separately from inlink count. A sitemap fetch alone is not an orphan audit. |
| [Ahrefs Site Audit](https://ahrefs.com/site-audit) | Reports sitemap membership/parsing problems, issue severity, internal-link suggestions, and bulk issue exports with fixing instructions. | Aim for an actionable result list with evidence and a clear next action. |
| [Sitebulb: prioritized Hints](https://sitebulb.com/features/prioritized-hints/) | Hints carry Critical/High/Medium/Low importance, explanations, and the percentage of URLs affected. | Accompany sitemap findings with severity and affected counts; don't bury them among cosmetic metadata warnings. |
| [Screaming Frog: crawl comparison](https://www.screamingfrog.co.uk/seo-spider/tutorials/how-to-compare-crawls/) | Compares issue changes, titles, depth and internal links; distinguishes URLs present in only one crawl; supports mapping staging URLs to production. | Strong next candidate for validation after deployments and migrations. Missing from a crawl must be distinguished from a verified HTTP failure. |
| [Sitebulb: audit comparison](https://sitebulb.com/features/audit-comparison/) | Shows historical metric trends and comparisons between any two audits. | Comparison supports recurring auditing and communicating changes to clients. |

## Small useful first release

1. Discover sitemap declarations from robots.txt and allow an explicit sitemap URL. Support sitemap indexes, ordinary URL sets and compressed sitemaps within clear scope/size limits.
2. Preserve discovery source: internal link, sitemap, manually supplied seed/list. Keep sitemap membership available in filters and exports.
3. Reconcile with the internal-link crawl. Report candidate unreachable URLs, HTTP failures/redirects in sitemaps, noindex pages and canonical mismatches. Canonical mismatches need context, not automatic claims of incorrect configuration.
4. Show crawled indexable URLs absent from the imported sitemaps as a review opportunity, not a universal error.
5. Export URL, sitemap source, observed result, reason and suggested action; offer the existing internal-link analysis when a candidate page needs links.

A sitemap-seeded URL must not gain an invented internal link or homepage depth. Compute reachability from the ordinary start URL across observed internal links, including disconnected groups discovered through auxiliary sources. Limit the orphan claim to the observed crawl: exclusions, robots rules, crawl limits, incomplete rendering and fetch failures can prevent discovery. Use “not reached in this crawl” or “candidate orphan” with completion/scope information.

This is **not a Google index coverage report**. A crawler's indexability observations and sitemap membership do not establish whether Google indexed a page. Search Console can later add search-performance context and additional URL sources; the first release can deliver value without OAuth or a third-party account.

## Sequence after that

1. **Crawl comparison:** compare two saved projects; show new/changed/not-observed URLs and before/after status, indexability, canonical, title and link depth. Preserve scope/settings/completion so users can judge comparability. This becomes the first priority instead if user research shows Tadpole is mainly used for repeat release checks rather than initial audits.
2. **Prioritized issue workflow:** grouped findings, explicit severity rationale, affected URLs, evidence, suggested action and developer-friendly export. Treat severity as contextual guidance, not an estimate of traffic uplift.
3. **Search Console enrichment:** initially consider a local CSV import for URL clicks/impressions, then native authorization if demand warrants it. Imported performance data covers a time window and is not an exhaustive URL or index inventory.

These rankings are reasoned product judgments. Validate them with a few working SEO professionals using real audit tasks before committing to a larger roadmap.

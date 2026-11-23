# Tadpole

Desktop SEO crawler and site-auditing application for inspecting crawl results, indexability, internal linking, and sitemap reconciliation.

## Language

**Project**:
A saved crawl snapshot containing crawl settings, collected page observations, and optional sitemap discovery evidence, stored as a `.tadpole` file (with legacy `.seocrawl` files still readable).
_Avoid_: Session, workspace file, dump

**Issue**:
A named SEO problem category (such as *Missing H1* or *Sitemap URL returns 4xx*) with a fixed Severity, Rationale, and Suggested Action, grouping all matching Affected URLs.
_Avoid_: Hint, warning, alert, filter

**Affected URL**:
A crawled or discovered URL that triggers an Issue, paired with its per-URL Evidence.
_Avoid_: Hit, violation, row

**Evidence**:
The concrete observed facts on an Affected URL that explain why it triggered an Issue (such as HTTP status code, canonical target, sitemap source, or measured length).
_Avoid_: Proof, detail, payload

**Severity**:
One of four fixed tiers — **Critical**, **High**, **Medium**, **Low** — expressing contextual audit guidance for prioritization rather than a guaranteed traffic or ranking impact.
_Avoid_: Priority, score, impact level

**Link Opportunity**:
A suggested internal hyperlink from a crawled source page to an indexable target page where the source page's visible text already contains the target page's candidate Anchor Keyword without an existing link to that target.
_Avoid_: Missing link, backlink suggestion

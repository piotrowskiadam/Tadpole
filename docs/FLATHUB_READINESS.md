# Flathub readiness (investigation, not a submission)

This document was prepared with AI assistance. It records facts and open work; it is not a Flathub manifest, submission description, review, or approval. Check Flathub's current [requirements and generative AI policy](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy) directly before submitting.

## Policy and provenance

Flathub prohibits AI-generated or AI-assisted content in its manifests. A person must author the submission manifest without AI assistance and must personally handle the submission pull request, its commit messages, description and review interactions. Do not ask an AI agent to review the Flathub submission. Other AI-generated application material requires disclosure identifying the affected areas and approximate extent; reviewers decide whether to accept it.

Known AI-assisted work in this repository during the 2026 modernization includes crawler and persistence changes, GTK interface work, the visualization module, tests, documentation, and GitHub release packaging. The comparison from `be686f5` (the original baseline) to the 0.4.0 merge commit `689ad947960eed7f22c5d96a35a82f3686130df2` spans approximately 70 files, 7,851 inserted lines and 2,629 deleted lines. These are **whole-diff figures, not a measurement of AI authorship**: the diff includes at least one independently made README badge change, deletions, and unchanged context is excluded. The human submitter should inspect the history and account for any other prior AI-assisted work before preparing their own disclosure. This readiness document and the later application-identity changes were also AI-assisted and are not included in those 0.4.0 diff figures.

## Application identity

The existing native packages use `com.tadpole.seo`. Its corresponding domain ownership has not been established. A human should independently choose an appropriate ID under [Flathub's application-ID rules](https://docs.flathub.org/docs/for-app-authors/requirements#application-id), and confirm ownership and suitability. This branch makes the application use the ID provided by Flatpak in `FLATPAK_ID`, retaining the original ID for the existing desktop packages. The icon follows the application ID. The human-authored packaging must install a matching desktop file and icon, and use the same ID in the AppStream metadata. No claim of Flathub verification is made.

The identity change is **after** GitHub release 0.4.0. A Flathub submission must pin a stable upstream tag containing it, such as v0.4.1 or later. The separate, AI-assisted upstream Flatpak bundle has been built locally and in GitHub Actions; it is not a Flathub submission and its manifest must not be reused for one.

## Functional work to validate

- Ordinary crawling and optional AI requests need network access. Review and justify the minimum sandbox permissions under Flathub's rules.
- Open/save workflows and image or Markdown exports need testing through the desktop file chooser and document portal. Check whether selected directories continue to work for later automatic exports without broad filesystem access.
- JavaScript rendering launches an external Chrome/Chromium process. A browser installed on the host is not automatically available inside a Flatpak sandbox. Test a compatible sandbox-accessible browser or document a supported limitation; do not assume host Chrome is usable. The app now gives a sandbox-specific error if no browser is available. Never weaken the browser sandbox or grant host access without an explicit, reviewed design.
- The file chooser, clipboard, launch/icon behavior, URLs, non-JavaScript crawl, optional AI action, project persistence and error paths need a real sandbox smoke test. Include a clean system and, if applicable, aarch64.
- Flatpak builds on Flathub run without network access. A person should follow the official [flatpak-cargo-generator documentation](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo) and Flathub's source rules when independently preparing the build inputs and human-authored manifest. The GNOME SDK and Flatpak Builder are present locally, but this does not establish that the matching Rust SDK extension or current Flathub runtime is installed or sufficient.
- AppStream metadata (`flatpak/com.tadpole.seo.metainfo.xml`) and MIME metadata (`flatpak/com.tadpole.seo.mime.xml`) are validated offline with `appstreamcli validate --no-net` and `xmllint --noout`. Keep descriptions, optional paid AI disclosures, content ratings, and release notes aligned with [Flathub's metadata guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines).
- Project save paths preserve exact XDG document-portal filenames (`/run/user/<uid>/doc/<id>/<file>`) and fall back to serialized direct writes if a single-file portal mount denies sibling temporary files.
- Flathub requires at least one real, representative screenshot, available from an immutable public URL. The repository's older screenshot predates the current UI and should not be represented as an up-to-date capture. OBS Studio 33.0.1 is installed as a Flatpak on the development machine; a human can capture the updated window and review it for private data. The agent session did not have native-window capture access.

## Sandbox smoke-test checklist

Run this checklist inside an installed Flatpak sandbox (with no native Tadpole instance running) before any release or human-owned Flathub submission:

- [ ] **Launch & identity**: Run `flatpak run com.tadpole.seo` (or the human-chosen Flathub ID), confirm the window icon and desktop entry appear properly, and confirm no host `--filesystem=home` or `--filesystem=host` override is needed.
- [ ] **Crawl & `Issues` workspace**: Crawl a small site (for example 10–30 URLs), switch to the `Issues` tab, verify Severity counts (`Critical`, `High`, `Medium`, `Low`) and Affected URL percentages populate, and click an Affected URL to jump into **Crawl Results** or **Sitemap & Discovery**.
- [ ] **Document-portal open/save (`.tadpole` and `.seocrawl`)**:
  - [ ] Save a new `.tadpole` Project via the portal Save dialog (both accepting the default `project.tadpole` name and typing a custom name) and confirm the file is written cleanly on the host.
  - [ ] Re-save over the existing `.tadpole` file through the portal and confirm it updates without permission errors or stray temporary files.
  - [ ] Open both the saved `.tadpole` Project and a legacy `.seocrawl` Project via the portal Open dialog and confirm crawl results, settings, and discovery Evidence load cleanly.
- [ ] **Portal exports (`Issues`, Crawl Results, Sitemap & Discovery)**:
  - [ ] Export the full `Issues` report to Markdown and CSV via the FileChooser portal; verify both files appear at the chosen host paths with expected content.
  - [ ] Export a single selected Issue to Markdown and CSV via the portal.
  - [ ] Export Crawl Results CSV and Markdown folder (`select_folder`) through the portal and verify written files.
- [ ] **Clipboard copy**: Click **Copy Markdown** in the `Issues` workspace (and copy rows from **Crawl Results**), then paste into a host text editor to confirm Wayland/X11 clipboard handoff works.
- [ ] **Browser-missing sandbox message**: Enable JavaScript rendering in **Preferences** without a sandbox-accessible Chrome/Chromium, start a crawl, and confirm Tadpole surfaces the Flatpak-specific sandbox error message without crashing or attempting to weaken the sandbox.
- [ ] **Light/dark UI & keyboard navigation**: Toggle the desktop appearance between light and dark styles and navigate tabs, filters, and the `Issues` list with keyboard focus.

## Human-owned submission

After a human authors and validates the manifest, creates current screenshots and prepares the required disclosure, they should follow the official [submission and validation procedure](https://docs.flathub.org/docs/for-app-authors/submission), including local build, sandbox testing and the Flathub linter. The submission goes to Flathub's `new-pr` branch, not `master`. Acceptance and publication are decisions for Flathub's reviewers.

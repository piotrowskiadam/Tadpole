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
- AppStream metadata must validate, accurately describe the optional paid AI requests, and include an appropriate content rating and release information. Follow the current [metadata guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines).
- Flathub requires at least one real, representative screenshot, available from an immutable public URL. The repository's older screenshot predates the current UI and should not be represented as an up-to-date capture. OBS Studio 33.0.1 is installed as a Flatpak on the development machine; a human can capture the updated window and review it for private data. The agent session did not have native-window capture access.

## Human-owned submission

After a human authors and validates the manifest, creates current screenshots and prepares the required disclosure, they should follow the official [submission and validation procedure](https://docs.flathub.org/docs/for-app-authors/submission), including local build, sandbox testing and the Flathub linter. The submission goes to Flathub's `new-pr` branch, not `master`. Acceptance and publication are decisions for Flathub's reviewers.

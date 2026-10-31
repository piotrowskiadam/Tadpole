# Tadpole interface conventions

Preserve the existing GTK4/Libadwaita identity. This is a task-oriented desktop tool for SEO specialists, developers and editors. Use native controls, platform typography, standard keyboard/focus behavior and the user's system light/dark preference.

- Keep crawl controls in a separate expanding toolbar; reserve the header for file/preferences/workspace actions.
- Dense data belongs in a virtualized ColumnView with horizontal scrolling. Filters and summary can be hidden to make room.
- Use semantic Libadwaita foreground/accent/error/warning/success colors. Do not replace platform focus or selection styling.
- Preserve document text as selectable plain text. Do not interpret crawled metadata, API results or AI output as GTK markup.
- Label fields and icon actions; show recovery guidance for missing credentials and failed requests.
- SEO Studio is a nonmodal window: topic, market/language, optional selected-page context, explicit request actions, then separate evidence/brief reports. Report actions have disabled/loading/ready/error states.
- Paid requests are explicit, never triggered by selection, startup, or project loading. State the data being sent next to the action.
- Keep content copy in English to match the incumbent app; brief output follows the chosen market language. Full UI localization is future work.

The committed old screenshot depicts an earlier release. Current visual verification uses `examples/ui_preview.rs` and the platform's own renderer; see `docs/DEVELOPMENT.md`.

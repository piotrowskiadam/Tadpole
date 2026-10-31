pub fn install() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        "
            .bold { font-weight: bold; }

            .error { color: @error_color; font-weight: bold; }
            .warning { color: @warning_color; }
            .success { color: @success_color; }
            .accent { color: @accent_color; }
            .status-bar { border-top: 1px solid @borders; }
            .numeric { font-variant-numeric: tabular-nums; }
            .data-table { font-size: 1em; }
            progressbar.score-good > trough > progress { background: @success_color; }
            progressbar.score-warn > trough > progress { background: @warning_color; }
            progressbar.score-poor > trough > progress { background: @error_color; }
        ",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

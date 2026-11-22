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
            .discovery-report .dim-label { color: @window_fg_color; opacity: 0.85; }
            .discovery-report columnview header,
            .discovery-report columnview header button,
            .discovery-report columnview header label { color: @view_fg_color; opacity: 1; }
            .issues-workspace .dim-label { color: @window_fg_color; opacity: 0.82; }
            .issues-workspace columnview header,
            .issues-workspace columnview header button,
            .issues-workspace columnview header label { color: @view_fg_color; opacity: 1; }
            .severity-badge {
                font-size: 0.85em;
                font-weight: 700;
                padding: 2px 8px;
                border-radius: 999px;
            }
            .severity-critical {
                color: @error_color;
                background-color: alpha(@error_color, 0.14);
            }
            .severity-high {
                color: @warning_color;
                background-color: alpha(@warning_color, 0.16);
            }
            .severity-medium {
                color: @accent_color;
                background-color: alpha(@accent_color, 0.14);
            }
            .severity-low {
                color: @window_fg_color;
                background-color: alpha(@window_fg_color, 0.10);
            }
            .issues-caveat-banner {
                background-color: alpha(@warning_color, 0.12);
                border: 1px solid alpha(@warning_color, 0.35);
                border-radius: 8px;
                padding: 8px 12px;
            }
            .issues-severity-btn {
                padding: 8px 12px;
            }
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

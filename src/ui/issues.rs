//! Native GNOME / Libadwaita `Issues` workspace tab powered by the unified
//! GTK-independent `crate::issues` engine.

use crate::{
    issues::{self, AffectedUrl, Issue, IssueKind, IssueReport, IssueSource, Severity},
    state::CrawlState,
};
use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

const LIVE_REFRESH_INTERVAL: Duration = Duration::from_millis(500);

type AffectedData = Rc<RefCell<Vec<AffectedUrl>>>;
type JumpCallback = Rc<dyn Fn(String, IssueKind, IssueSource)>;

fn label(value: &str, css: &str) -> gtk::Label {
    let widget = gtk::Label::new(Some(value));
    widget.set_xalign(0.0);
    if !css.is_empty() {
        widget.add_css_class(css);
    }
    widget
}

fn plain_paragraph(value: &str) -> gtk::Label {
    let widget = label(value, "");
    widget.set_wrap(true);
    widget.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    widget.set_width_chars(28);
    widget.set_xalign(0.0);
    widget.set_selectable(true);
    widget
}

fn padded_box(orientation: gtk::Orientation, spacing: i32, margin: i32) -> gtk::Box {
    let widget = gtk::Box::new(orientation, spacing);
    widget.set_margin_start(margin);
    widget.set_margin_end(margin);
    widget.set_margin_top(margin);
    widget.set_margin_bottom(margin);
    widget
}

fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn clear_list_box(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn severity_badge_class(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "severity-critical",
        Severity::High => "severity-high",
        Severity::Medium => "severity-medium",
        Severity::Low => "severity-low",
    }
}

fn severity_text_class(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "error",
        Severity::High => "warning",
        Severity::Medium => "accent",
        Severity::Low => "dim-label",
    }
}

fn severity_badge(severity: Severity) -> gtk::Label {
    let badge = gtk::Label::new(Some(severity.label()));
    badge.add_css_class("severity-badge");
    badge.add_css_class(severity_badge_class(severity));
    badge.set_halign(gtk::Align::Start);
    badge.set_valign(gtk::Align::Center);
    badge
}

pub struct IssuesPanel {
    overlay: adw::ToastOverlay,
    state: CrawlState,
    report: Rc<RefCell<IssueReport>>,
    filtered_kinds: Rc<RefCell<Vec<IssueKind>>>,
    selected_kind: Cell<Option<IssueKind>>,
    severity_filter: Cell<Option<Severity>>,
    show_passed: Cell<bool>,
    tab_active: Cell<bool>,
    dirty: Cell<bool>,
    last_refresh: Cell<Option<Instant>>,
    syncing_controls: Cell<bool>,

    // Summary header
    scope_label: gtk::Label,
    severity_buttons: [gtk::ToggleButton; 4],
    severity_issue_labels: [gtk::Label; 4],
    severity_url_labels: [gtk::Label; 4],
    copy_all_btn: gtk::Button,
    export_all_btn: gtk::MenuButton,

    // Scope caveat banner
    caveat_banner: gtk::Box,
    caveat_list: gtk::Box,

    // Main body
    empty_page: adw::StatusPage,
    split: gtk::Paned,

    // Left pane controls
    search_entry: gtk::SearchEntry,
    severity_dropdown: gtk::DropDown,
    list_count_label: gtk::Label,
    issue_list: gtk::ListBox,

    // Right pane detail controls
    detail_content: gtk::Box,
    detail_empty: gtk::Box,
    detail_empty_title: gtk::Label,
    detail_empty_body: gtk::Label,
    detail_badge: gtk::Label,
    detail_title: gtk::Label,
    detail_meta: gtk::Label,
    detail_rationale: gtk::Label,
    detail_action: gtk::Label,
    affected_heading_count: gtk::Label,
    jump_btn: gtk::Button,

    // Virtualized Affected URLs table
    affected_data: AffectedData,
    affected_model: gtk::StringList,
    affected_selection: gtk::SingleSelection,
    selected_url_label: gtk::Label,
    selected_evidence_label: gtk::Label,
    selected_url_bar: gtk::Box,

    on_jump: JumpCallback,
}

impl IssuesPanel {
    pub fn new(
        state: CrawlState,
        on_jump: impl Fn(String, IssueKind, IssueSource) + 'static,
    ) -> Rc<Self> {
        let overlay = adw::ToastOverlay::new();
        overlay.set_vexpand(true);
        overlay.set_hexpand(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("issues-workspace");
        root.set_vexpand(true);
        root.set_hexpand(true);
        overlay.set_child(Some(&root));

        // --- Header & Transparent Severity Summary ---
        let header = padded_box(gtk::Orientation::Vertical, 10, 14);
        let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let title_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        title_box.set_hexpand(true);
        title_box.append(&label("Issues", "title-2"));
        let scope_label = plain_paragraph("No crawled URLs yet.");
        scope_label.add_css_class("dim-label");
        scope_label.add_css_class("numeric");
        title_box.append(&scope_label);
        title_row.append(&title_box);

        let copy_all_btn = gtk::Button::builder()
            .label("Copy Markdown")
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy developer-ready Markdown report for all active Issues")
            .valign(gtk::Align::Center)
            .sensitive(false)
            .build();
        title_row.append(&copy_all_btn);

        let export_all_btn = gtk::MenuButton::builder()
            .label("Export All")
            .icon_name("folder-download-symbolic")
            .tooltip_text("Export all active Issues as Markdown or CSV")
            .valign(gtk::Align::Center)
            .sensitive(false)
            .build();
        let export_all_popover = gtk::Popover::new();
        let export_all_box = padded_box(gtk::Orientation::Vertical, 4, 6);
        let export_all_md_btn = gtk::Button::builder()
            .label("Export All as Markdown (.md)")
            .css_classes(vec!["flat".to_string()])
            .halign(gtk::Align::Fill)
            .build();
        let export_all_csv_btn = gtk::Button::builder()
            .label("Export All as CSV (.csv)")
            .css_classes(vec!["flat".to_string()])
            .halign(gtk::Align::Fill)
            .build();
        export_all_box.append(&export_all_md_btn);
        export_all_box.append(&export_all_csv_btn);
        export_all_popover.set_child(Some(&export_all_box));
        export_all_btn.set_popover(Some(&export_all_popover));
        title_row.append(&export_all_btn);

        header.append(&title_row);

        // Transparent Severity summary strip (Critical, High, Medium, Low — NO 0-100 score per ADR-0001)
        let summary_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        summary_row.set_homogeneous(true);

        let mut sev_buttons: Vec<gtk::ToggleButton> = Vec::with_capacity(4);
        let mut sev_issue_labels: Vec<gtk::Label> = Vec::with_capacity(4);
        let mut sev_url_labels: Vec<gtk::Label> = Vec::with_capacity(4);

        for severity in Severity::ALL {
            let btn = gtk::ToggleButton::new();
            btn.add_css_class("issues-severity-btn");
            btn.set_tooltip_text(Some(&format!(
                "Filter Issues by {} severity (click again to show all)",
                severity.label()
            )));
            btn.update_property(&[gtk::accessible::Property::Label(&format!(
                "{} severity filter",
                severity.label()
            ))]);

            let content = gtk::Box::new(gtk::Orientation::Vertical, 3);
            let top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let name = label(severity.label(), "heading");
            name.add_css_class(severity_text_class(severity));
            name.set_hexpand(true);
            top.append(&name);

            let issue_count = label("0 Issues", "numeric");
            issue_count.add_css_class("bold");
            top.append(&issue_count);
            content.append(&top);

            let url_count = label("0 URLs · 0.0%", "dim-label");
            url_count.add_css_class("numeric");
            url_count.add_css_class("caption");
            content.append(&url_count);

            btn.set_child(Some(&content));
            summary_row.append(&btn);
            sev_buttons.push(btn);
            sev_issue_labels.push(issue_count);
            sev_url_labels.push(url_count);
        }

        let severity_buttons: [gtk::ToggleButton; 4] = sev_buttons.try_into().unwrap();
        let severity_issue_labels: [gtk::Label; 4] = sev_issue_labels.try_into().unwrap();
        let severity_url_labels: [gtk::Label; 4] = sev_url_labels.try_into().unwrap();
        header.append(&summary_row);

        // Scope Caveat Banner (hidden when no caveats apply)
        let caveat_banner = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        caveat_banner.add_css_class("issues-caveat-banner");
        caveat_banner.set_visible(false);
        let caveat_icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
        caveat_icon.add_css_class("warning");
        caveat_icon.set_valign(gtk::Align::Start);
        caveat_icon.set_margin_top(2);
        caveat_banner.append(&caveat_icon);
        let caveat_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
        caveat_list.set_hexpand(true);
        caveat_banner.append(&caveat_list);
        header.append(&caveat_banner);

        root.append(&header);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // --- Empty state (before crawl or Project load) ---
        let empty_page = adw::StatusPage::builder()
            .icon_name("system-search-symbolic")
            .title("No audit data yet")
            .description(
                "Start a crawl or open a saved .tadpole Project to inspect prioritized SEO Issues.",
            )
            .vexpand(true)
            .build();
        root.append(&empty_page);

        // --- Main Split View ---
        let split = gtk::Paned::new(gtk::Orientation::Horizontal);
        split.set_position(340);
        split.set_resize_start_child(false);
        split.set_shrink_start_child(false);
        split.set_shrink_end_child(false);
        split.set_wide_handle(true);
        split.set_vexpand(true);
        split.set_visible(false);
        root.append(&split);

        // Left Pane: Search, Severity filter, Show passed checks, grouped Issue list
        let left_pane = padded_box(gtk::Orientation::Vertical, 8, 12);
        left_pane.set_width_request(260);

        let search_entry = gtk::SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search Issues or Affected URLs…"));
        search_entry.update_property(&[gtk::accessible::Property::Label(
            "Search Issues or Affected URLs",
        )]);
        left_pane.append(&search_entry);

        let filter_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let severity_dropdown =
            gtk::DropDown::from_strings(&["All Severities", "Critical", "High", "Medium", "Low"]);
        severity_dropdown.set_hexpand(true);
        severity_dropdown
            .update_property(&[gtk::accessible::Property::Label("Filter by Severity")]);
        filter_row.append(&severity_dropdown);
        left_pane.append(&filter_row);

        let toggle_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let show_passed_check = gtk::CheckButton::with_label("Show passed checks");
        show_passed_check.set_active(false);
        show_passed_check.set_hexpand(true);
        toggle_row.append(&show_passed_check);
        let list_count_label = label("0 Issues", "dim-label");
        list_count_label.add_css_class("numeric");
        list_count_label.add_css_class("caption");
        toggle_row.append(&list_count_label);
        left_pane.append(&toggle_row);

        let issue_list = gtk::ListBox::new();
        issue_list.add_css_class("navigation-sidebar");
        issue_list.set_selection_mode(gtk::SelectionMode::Single);
        let list_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&issue_list)
            .build();
        left_pane.append(&list_scroll);
        split.set_start_child(Some(&left_pane));

        // Right Pane: Selected Issue Detail + Virtualized Affected URLs Table
        let right_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        right_root.set_width_request(420);
        right_root.set_vexpand(true);
        right_root.set_hexpand(true);

        let detail_empty = padded_box(gtk::Orientation::Vertical, 8, 24);
        detail_empty.set_vexpand(true);
        detail_empty.set_valign(gtk::Align::Center);
        let detail_empty_title = label("Select an Issue", "title-3");
        detail_empty_title.set_halign(gtk::Align::Center);
        let detail_empty_body = plain_paragraph(
            "Choose an Issue from the list to inspect its Rationale, Suggested Action, and Affected URLs.",
        );
        detail_empty_body.set_halign(gtk::Align::Center);
        detail_empty_body.add_css_class("dim-label");
        detail_empty.append(&detail_empty_title);
        detail_empty.append(&detail_empty_body);
        right_root.append(&detail_empty);

        let detail_content = padded_box(gtk::Orientation::Vertical, 12, 16);
        detail_content.set_vexpand(true);
        detail_content.set_hexpand(true);
        detail_content.set_visible(false);

        // Issue detail header row
        let detail_header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let detail_title_col = gtk::Box::new(gtk::Orientation::Vertical, 4);
        detail_title_col.set_hexpand(true);

        let title_badge_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let detail_badge = severity_badge(Severity::Critical);
        title_badge_row.append(&detail_badge);
        let detail_title = label("", "title-2");
        detail_title.set_wrap(true);
        detail_title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        detail_title.set_width_chars(20);
        detail_title.set_selectable(true);
        detail_title.set_hexpand(true);
        title_badge_row.append(&detail_title);
        detail_title_col.append(&title_badge_row);

        let detail_meta = plain_paragraph("");
        detail_meta.add_css_class("dim-label");
        detail_meta.add_css_class("numeric");
        detail_title_col.append(&detail_meta);
        detail_header.append(&detail_title_col);

        let copy_issue_btn = gtk::Button::builder()
            .label("Copy Issue Markdown")
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy Markdown report for the selected Issue")
            .valign(gtk::Align::Start)
            .build();
        detail_header.append(&copy_issue_btn);

        let export_issue_btn = gtk::MenuButton::builder()
            .label("Export Issue")
            .icon_name("document-save-symbolic")
            .tooltip_text("Export the selected Issue as Markdown or CSV")
            .valign(gtk::Align::Start)
            .build();
        let export_issue_popover = gtk::Popover::new();
        let export_issue_box = padded_box(gtk::Orientation::Vertical, 4, 6);
        let export_issue_md_btn = gtk::Button::builder()
            .label("Export Issue as Markdown (.md)")
            .css_classes(vec!["flat".to_string()])
            .halign(gtk::Align::Fill)
            .build();
        let export_issue_csv_btn = gtk::Button::builder()
            .label("Export Issue as CSV (.csv)")
            .css_classes(vec!["flat".to_string()])
            .halign(gtk::Align::Fill)
            .build();
        export_issue_box.append(&export_issue_md_btn);
        export_issue_box.append(&export_issue_csv_btn);
        export_issue_popover.set_child(Some(&export_issue_box));
        export_issue_btn.set_popover(Some(&export_issue_popover));
        detail_header.append(&export_issue_btn);
        detail_content.append(&detail_header);

        // Rationale & Suggested Action
        let guidance_grid = gtk::Grid::builder()
            .column_spacing(16)
            .row_spacing(6)
            .build();
        let rationale_lbl = label("Rationale", "heading");
        rationale_lbl.set_valign(gtk::Align::Start);
        let detail_rationale = plain_paragraph("");
        detail_rationale.set_hexpand(true);
        guidance_grid.attach(&rationale_lbl, 0, 0, 1, 1);
        guidance_grid.attach(&detail_rationale, 1, 0, 1, 1);

        let action_lbl = label("Suggested Action", "heading");
        action_lbl.set_valign(gtk::Align::Start);
        let detail_action = plain_paragraph("");
        detail_action.set_hexpand(true);
        guidance_grid.attach(&action_lbl, 0, 1, 1, 1);
        guidance_grid.attach(&detail_action, 1, 1, 1, 1);
        detail_content.append(&guidance_grid);

        detail_content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // Affected URLs section header
        let affected_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let affected_heading = label("Affected URLs", "heading");
        affected_header.append(&affected_heading);
        let affected_heading_count = label("0", "dim-label");
        affected_heading_count.add_css_class("numeric");
        affected_heading_count.set_hexpand(true);
        affected_header.append(&affected_heading_count);

        let jump_btn = gtk::Button::builder()
            .label("Open in Crawl Results")
            .icon_name("go-jump-symbolic")
            .tooltip_text("Select and reveal this URL in Crawl Results or Sitemap & Discovery")
            .sensitive(false)
            .build();
        affected_header.append(&jump_btn);
        detail_content.append(&affected_header);

        // Virtualized ColumnView of Affected URLs + Evidence
        let affected_data: AffectedData = Rc::new(RefCell::new(Vec::new()));
        let affected_model = gtk::StringList::new(&[]);
        let affected_table = gtk::ColumnView::new(None::<gtk::SelectionModel>);
        affected_table.add_css_class("data-table");
        affected_table.set_show_row_separators(true);
        let sorted_model =
            gtk::SortListModel::new(Some(affected_model.clone()), affected_table.sorter());
        let affected_selection = gtk::SingleSelection::new(Some(sorted_model));
        affected_table.set_model(Some(&affected_selection));

        affected_table.append_column(&affected_column(
            "Affected URL",
            360,
            true,
            AffectedColumn::Url,
            affected_data.clone(),
        ));
        affected_table.append_column(&affected_column(
            "Evidence",
            380,
            true,
            AffectedColumn::Evidence,
            affected_data.clone(),
        ));

        let table_scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&affected_table)
            .build();
        detail_content.append(&table_scroll);

        // Selected Affected URL plain-text inspection strip (selectable + word-wrapped)
        let selected_url_bar = gtk::Box::new(gtk::Orientation::Vertical, 4);
        selected_url_bar.set_visible(false);
        let selected_url_label = plain_paragraph("");
        selected_url_label.add_css_class("bold");
        selected_url_label.set_lines(2);
        selected_url_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let selected_evidence_label = plain_paragraph("");
        selected_evidence_label.add_css_class("dim-label");
        selected_evidence_label.set_lines(2);
        selected_evidence_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        selected_url_bar.append(&selected_url_label);
        selected_url_bar.append(&selected_evidence_label);
        detail_content.append(&selected_url_bar);

        right_root.append(&detail_content);
        split.set_end_child(Some(&right_root));

        let initial_report = issues::evaluate(&[], None, None);

        let panel = Rc::new(Self {
            overlay,
            state,
            report: Rc::new(RefCell::new(initial_report)),
            filtered_kinds: Rc::new(RefCell::new(Vec::new())),
            selected_kind: Cell::new(None),
            severity_filter: Cell::new(None),
            show_passed: Cell::new(false),
            tab_active: Cell::new(false),
            dirty: Cell::new(true),
            last_refresh: Cell::new(None),
            syncing_controls: Cell::new(false),
            scope_label,
            severity_buttons,
            severity_issue_labels,
            severity_url_labels,
            copy_all_btn: copy_all_btn.clone(),
            export_all_btn: export_all_btn.clone(),
            caveat_banner,
            caveat_list,
            empty_page,
            split,
            search_entry: search_entry.clone(),
            severity_dropdown: severity_dropdown.clone(),
            list_count_label,
            issue_list: issue_list.clone(),
            detail_content,
            detail_empty,
            detail_empty_title,
            detail_empty_body,
            detail_badge,
            detail_title,
            detail_meta,
            detail_rationale,
            detail_action,
            affected_heading_count,
            jump_btn: jump_btn.clone(),
            affected_data,
            affected_model,
            affected_selection: affected_selection.clone(),
            selected_url_label,
            selected_evidence_label,
            selected_url_bar,
            on_jump: Rc::new(on_jump),
        });

        // --- Wire Severity summary toggle buttons ---
        for (idx, severity) in Severity::ALL.into_iter().enumerate() {
            let p = Rc::downgrade(&panel);
            panel.severity_buttons[idx].connect_toggled(move |btn| {
                let Some(p) = p.upgrade() else { return };
                if p.syncing_controls.get() {
                    return;
                }
                let next = if btn.is_active() {
                    Some(severity)
                } else {
                    None
                };
                p.set_severity_filter(next);
            });
        }

        // --- Wire Severity dropdown ---
        severity_dropdown.connect_selected_notify({
            let p = Rc::downgrade(&panel);
            move |dropdown| {
                let Some(p) = p.upgrade() else { return };
                if p.syncing_controls.get() {
                    return;
                }
                let next = match dropdown.selected() {
                    1 => Some(Severity::Critical),
                    2 => Some(Severity::High),
                    3 => Some(Severity::Medium),
                    4 => Some(Severity::Low),
                    _ => None,
                };
                p.set_severity_filter(next);
            }
        });

        // --- Wire Search & Show Passed toggle ---
        search_entry.connect_search_changed({
            let p = Rc::downgrade(&panel);
            move |_| {
                if let Some(p) = p.upgrade() {
                    p.rebuild_issue_list();
                }
            }
        });

        show_passed_check.connect_toggled({
            let p = Rc::downgrade(&panel);
            move |btn| {
                if let Some(p) = p.upgrade() {
                    p.show_passed.set(btn.is_active());
                    p.rebuild_issue_list();
                }
            }
        });

        // --- Wire Issue List selection ---
        issue_list.connect_row_selected({
            let p = Rc::downgrade(&panel);
            move |_, row| {
                let Some(p) = p.upgrade() else { return };
                if let Some(row) = row {
                    let idx = row.index() as usize;
                    let kind = p.filtered_kinds.borrow().get(idx).copied();
                    p.selected_kind.set(kind);
                    p.render_selected_issue();
                } else if p.filtered_kinds.borrow().is_empty() {
                    p.selected_kind.set(None);
                    p.render_selected_issue();
                }
            }
        });

        // --- Wire Affected URLs table selection & activation (jump-to-URL) ---
        affected_selection.connect_selected_notify({
            let p = Rc::downgrade(&panel);
            move |_| {
                if let Some(p) = p.upgrade() {
                    p.update_selected_affected_bar();
                }
            }
        });

        affected_table.connect_activate({
            let p = Rc::downgrade(&panel);
            move |_, pos| {
                let Some(p) = p.upgrade() else { return };
                p.affected_selection.set_selected(pos);
                p.trigger_jump_for_selected();
            }
        });

        jump_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            move |_| {
                if let Some(p) = p.upgrade() {
                    p.trigger_jump_for_selected();
                }
            }
        });

        // --- Wire Copy & Export All ---
        copy_all_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            move |_| {
                let Some(p) = p.upgrade() else { return };
                let markdown = issues::export_markdown(&p.report.borrow());
                copy_to_clipboard(&markdown);
                p.show_toast("Copied Markdown report for all active Issues to clipboard.");
            }
        });

        export_all_md_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            let popover = export_all_popover.clone();
            move |_| {
                popover.popdown();
                let Some(p) = p.upgrade() else { return };
                p.export_all_markdown();
            }
        });

        export_all_csv_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            let popover = export_all_popover.clone();
            move |_| {
                popover.popdown();
                let Some(p) = p.upgrade() else { return };
                p.export_all_csv();
            }
        });

        // --- Wire Copy & Export Selected Issue ---
        copy_issue_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            move |_| {
                let Some(p) = p.upgrade() else { return };
                let Some(kind) = p.selected_kind.get() else {
                    return;
                };
                let report = p.report.borrow();
                let Some(issue) = report.issue(kind) else {
                    return;
                };
                let markdown = issues::export_issue_markdown(&report, issue);
                let name = issue.name.clone();
                drop(report);
                copy_to_clipboard(&markdown);
                p.show_toast(&format!("Copied \"{name}\" Markdown to clipboard."));
            }
        });

        export_issue_md_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            let popover = export_issue_popover.clone();
            move |_| {
                popover.popdown();
                let Some(p) = p.upgrade() else { return };
                p.export_selected_markdown();
            }
        });

        export_issue_csv_btn.connect_clicked({
            let p = Rc::downgrade(&panel);
            let popover = export_issue_popover.clone();
            move |_| {
                popover.popdown();
                let Some(p) = p.upgrade() else { return };
                p.export_selected_csv();
            }
        });

        panel.refresh();
        panel
    }

    pub fn widget(&self) -> &adw::ToastOverlay {
        &self.overlay
    }

    pub fn show_toast(&self, message: &str) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(3);
        self.overlay.add_toast(toast);
    }

    /// Track whether the `Issues` tab is currently the active notebook page.
    /// Switching to the tab triggers an immediate refresh if new crawl data arrived.
    pub fn set_tab_active(self: &Rc<Self>, active: bool) {
        self.tab_active.set(active);
        if active && self.dirty.get() {
            self.refresh();
        }
    }

    /// Called during live crawls. Marks Issues dirty and performs a throttled
    /// recomputation ONLY while the `Issues` tab is visible.
    pub fn notify_crawl_progress(self: &Rc<Self>) {
        self.dirty.set(true);
        if !self.tab_active.get() {
            return;
        }
        let should_refresh = self
            .last_refresh
            .get()
            .is_none_or(|t| t.elapsed() >= LIVE_REFRESH_INTERVAL);
        if should_refresh {
            self.refresh();
        }
    }

    /// Guaranteed full recomputation from `CrawlState` (used on crawl finish,
    /// Project load, crawl reset, or switching to a dirty `Issues` tab).
    pub fn refresh(self: &Rc<Self>) {
        let report = issues::evaluate_state(&self.state);
        self.apply_report(report);
    }

    /// Apply a pre-computed `IssueReport` directly (useful for offline previews).
    pub fn apply_report(&self, report: IssueReport) {
        self.dirty.set(false);
        self.last_refresh.set(Some(Instant::now()));

        let has_data = report.crawled_urls > 0 || report.observed_urls > 0;
        self.empty_page.set_visible(!has_data);
        self.split.set_visible(has_data);

        self.copy_all_btn.set_sensitive(has_data);
        self.export_all_btn.set_sensitive(has_data);

        if has_data {
            self.scope_label.set_text(&format!(
                "{} crawled URLs · {} indexable HTML · {} observed URLs · {} active Issues",
                report.crawled_urls,
                report.indexable_html_urls,
                report.observed_urls,
                report.active_issue_counts.total()
            ));
        } else {
            self.scope_label.set_text("No crawled URLs yet.");
        }

        // Update transparent Severity summary strip (NO 0-100 score per ADR-0001)
        let summaries = report.severity_summaries();
        for (idx, summary) in summaries.iter().enumerate() {
            let issue_word = if summary.active_issues == 1 {
                "Issue"
            } else {
                "Issues"
            };
            let url_word = if summary.unique_affected_urls == 1 {
                "URL"
            } else {
                "URLs"
            };
            self.severity_issue_labels[idx]
                .set_text(&format!("{} {issue_word}", summary.active_issues));
            self.severity_url_labels[idx].set_text(&format!(
                "{} {url_word} · {:.1}%",
                summary.unique_affected_urls, summary.affected_percentage
            ));
        }

        // Update Scope Caveat Banner
        clear_box(&self.caveat_list);
        if has_data && !report.caveats.is_empty() {
            for caveat in &report.caveats {
                let msg = plain_paragraph(&caveat.message);
                self.caveat_list.append(&msg);
            }
            self.caveat_banner.set_visible(true);
        } else {
            self.caveat_banner.set_visible(false);
        }

        *self.report.borrow_mut() = report;
        self.rebuild_issue_list();
    }

    fn set_severity_filter(&self, severity: Option<Severity>) {
        self.severity_filter.set(severity);
        self.syncing_controls.set(true);
        for (idx, tier) in Severity::ALL.into_iter().enumerate() {
            self.severity_buttons[idx].set_active(severity == Some(tier));
        }
        let dropdown_idx = match severity {
            None => 0,
            Some(Severity::Critical) => 1,
            Some(Severity::High) => 2,
            Some(Severity::Medium) => 3,
            Some(Severity::Low) => 4,
        };
        self.severity_dropdown.set_selected(dropdown_idx);
        self.syncing_controls.set(false);
        self.rebuild_issue_list();
    }

    fn rebuild_issue_list(&self) {
        let prev_kind = self.selected_kind.get();
        let query = self.search_entry.text().to_string();
        let severity = self.severity_filter.get();
        let show_passed = self.show_passed.get();

        let report = self.report.borrow();
        let matching = report.filter_issues(severity, show_passed, &query);
        let kinds: Vec<IssueKind> = matching.iter().map(|issue| issue.kind).collect();

        clear_list_box(&self.issue_list);
        for issue in &matching {
            self.issue_list.append(&build_issue_row(issue));
        }

        let count_word = if matching.len() == 1 {
            "Issue"
        } else {
            "Issues"
        };
        self.list_count_label
            .set_text(&format!("{} {count_word}", matching.len()));

        drop(report);
        *self.filtered_kinds.borrow_mut() = kinds.clone();

        if let Some(pos) = prev_kind.and_then(|k| kinds.iter().position(|item| *item == k)) {
            if let Some(row) = self.issue_list.row_at_index(pos as i32) {
                self.issue_list.select_row(Some(&row));
            }
            self.selected_kind.set(prev_kind);
        } else if let Some(&first_kind) = kinds.first() {
            if let Some(row) = self.issue_list.row_at_index(0) {
                self.issue_list.select_row(Some(&row));
            }
            self.selected_kind.set(Some(first_kind));
        } else {
            self.selected_kind.set(None);
        }

        self.render_selected_issue();
    }

    fn render_selected_issue(&self) {
        let Some(kind) = self.selected_kind.get() else {
            self.detail_content.set_visible(false);
            self.detail_empty.set_visible(true);
            let report = self.report.borrow();
            if report.active_issue_counts.total() == 0 && !self.show_passed.get() {
                self.detail_empty_title.set_text("No active Issues found");
                self.detail_empty_body.set_text(
                    "All evaluated checks passed for this crawl. Enable \"Show passed checks\" to review all 29 checks.",
                );
            } else {
                self.detail_empty_title.set_text("No matching Issues");
                self.detail_empty_body.set_text(
                    "Clear the search query, choose All Severities, or enable \"Show passed checks\".",
                );
            }
            return;
        };

        let report = self.report.borrow();
        let Some(issue) = report.issue(kind) else {
            self.detail_content.set_visible(false);
            self.detail_empty.set_visible(true);
            return;
        };

        self.detail_empty.set_visible(false);
        self.detail_content.set_visible(true);

        // Update severity badge class and text
        for tier in Severity::ALL {
            self.detail_badge
                .remove_css_class(severity_badge_class(tier));
        }
        self.detail_badge.set_text(issue.severity.label());
        self.detail_badge
            .add_css_class(severity_badge_class(issue.severity));

        self.detail_title.set_text(&issue.name);

        let url_word = if issue.affected_count == 1 {
            "Affected URL"
        } else {
            "Affected URLs"
        };
        self.detail_meta.set_text(&format!(
            "{} · {} {url_word} ({:.1}% of crawled URLs)",
            issue.source.label(),
            issue.affected_count,
            issue.affected_percentage
        ));
        self.detail_rationale.set_text(&issue.rationale);
        self.detail_action.set_text(&issue.suggested_action);

        let jump_label = match issue.source {
            IssueSource::Crawl => "Open in Crawl Results",
            IssueSource::Discovery => "Open in Sitemap & Discovery",
        };
        self.jump_btn.set_label(jump_label);

        // Filter Affected URLs if search query specifically matches URLs/evidence
        // (if the query matches the Issue name/rationale itself, show all Affected URLs for that Issue).
        let query = self.search_entry.text().to_string();
        let trimmed = query.trim();
        let issue_meta_matches = trimmed.is_empty()
            || issue.name.to_lowercase().contains(&trimmed.to_lowercase())
            || issue
                .rationale
                .to_lowercase()
                .contains(&trimmed.to_lowercase())
            || issue
                .suggested_action
                .to_lowercase()
                .contains(&trimmed.to_lowercase());
        let rows: Vec<AffectedUrl> = if issue_meta_matches {
            issue.affected_urls.clone()
        } else {
            issue
                .filter_affected_urls(trimmed)
                .into_iter()
                .cloned()
                .collect()
        };

        let shown = rows.len();
        let total = issue.affected_count;
        if shown == total {
            self.affected_heading_count.set_text(&format!("{total}"));
        } else {
            self.affected_heading_count
                .set_text(&format!("{shown} matching of {total}"));
        }

        let ids: Vec<String> = (0..rows.len()).map(|i| i.to_string()).collect();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        *self.affected_data.borrow_mut() = rows;
        self.affected_model
            .splice(0, self.affected_model.n_items(), &id_refs);

        if !id_refs.is_empty() {
            self.affected_selection.set_selected(0);
        }
        drop(report);
        self.update_selected_affected_bar();
    }

    fn selected_affected_url(&self) -> Option<AffectedUrl> {
        let id = self
            .affected_selection
            .selected_item()
            .and_downcast::<gtk::StringObject>()
            .and_then(|s| s.string().parse::<usize>().ok())?;
        self.affected_data.borrow().get(id).cloned()
    }

    fn update_selected_affected_bar(&self) {
        if let Some(affected) = self.selected_affected_url() {
            self.jump_btn.set_sensitive(true);
            self.selected_url_label.set_text(&affected.url);
            self.selected_evidence_label.set_text(&affected.evidence);
            self.selected_url_bar.set_visible(true);
        } else {
            self.jump_btn.set_sensitive(false);
            self.selected_url_bar.set_visible(false);
        }
    }

    fn trigger_jump_for_selected(&self) {
        let Some(kind) = self.selected_kind.get() else {
            return;
        };
        let Some(affected) = self.selected_affected_url() else {
            return;
        };
        (self.on_jump)(affected.url, kind, kind.source());
    }

    fn export_all_markdown(self: &Rc<Self>) {
        let markdown = issues::export_markdown(&self.report.borrow());
        self.save_export_file(
            "Export All Issues (Markdown)",
            "tadpole-issues.md",
            "Markdown Files",
            "*.md",
            move |path| std::fs::write(path, markdown).map_err(|e| e.to_string()),
            "Exported all active Issues as Markdown.",
        );
    }

    fn export_all_csv(self: &Rc<Self>) {
        let report = self.report.borrow().clone();
        self.save_export_file(
            "Export All Issues (CSV)",
            "tadpole-issues.csv",
            "CSV Files",
            "*.csv",
            move |path| {
                let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
                issues::write_csv(file, &report).map_err(|e| e.to_string())
            },
            "Exported all active Issues as CSV.",
        );
    }

    fn export_selected_markdown(self: &Rc<Self>) {
        let Some(kind) = self.selected_kind.get() else {
            return;
        };
        let report = self.report.borrow();
        let Some(issue) = report.issue(kind) else {
            return;
        };
        let markdown = issues::export_issue_markdown(&report, issue);
        let initial_name = format!("tadpole-issue-{}.md", kind.slug());
        let toast_msg = format!("Exported \"{}\" as Markdown.", issue.name);
        drop(report);
        self.save_export_file(
            "Export Issue (Markdown)",
            &initial_name,
            "Markdown Files",
            "*.md",
            move |path| std::fs::write(path, markdown).map_err(|e| e.to_string()),
            &toast_msg,
        );
    }

    fn export_selected_csv(self: &Rc<Self>) {
        let Some(kind) = self.selected_kind.get() else {
            return;
        };
        let report = self.report.borrow().clone();
        let Some(issue) = report.issue(kind).cloned() else {
            return;
        };
        let initial_name = format!("tadpole-issue-{}.csv", kind.slug());
        let toast_msg = format!("Exported \"{}\" as CSV.", issue.name);
        self.save_export_file(
            "Export Issue (CSV)",
            &initial_name,
            "CSV Files",
            "*.csv",
            move |path| {
                let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
                issues::write_issue_csv(file, &report, &issue).map_err(|e| e.to_string())
            },
            &toast_msg,
        );
    }

    fn save_export_file(
        self: &Rc<Self>,
        title: &str,
        initial_name: &str,
        filter_name: &str,
        pattern: &str,
        write_fn: impl FnOnce(std::path::PathBuf) -> Result<(), String> + Send + 'static,
        success_message: &str,
    ) {
        let parent = self.overlay.root().and_downcast::<gtk::Window>();
        let dialog = gtk::FileDialog::builder()
            .title(title)
            .initial_name(initial_name)
            .build();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(filter_name));
        filter.add_pattern(pattern);
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&filter));

        let panel = Rc::downgrade(self);
        let success_message = success_message.to_string();
        let write_fn = RefCell::new(Some(write_fn));
        dialog.save(parent.as_ref(), None::<&gio::Cancellable>, move |result| {
            let (Some(panel), Ok(file)) = (panel.upgrade(), result) else {
                return;
            };
            let Some(path) = file.path() else {
                panel.show_toast("Choose a local file for this export.");
                return;
            };
            let Some(write_fn) = write_fn.borrow_mut().take() else {
                return;
            };
            let (tx, rx) = tokio::sync::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(write_fn(path));
            });
            glib::MainContext::default().spawn_local(async move {
                match rx.await {
                    Ok(Ok(())) => panel.show_toast(&success_message),
                    Ok(Err(error)) => {
                        panel.show_toast(&format!("Export failed: {error}"));
                    }
                    Err(_) => panel.show_toast("Export could not finish. Retry the export."),
                }
            });
        });
    }
}

fn copy_to_clipboard(text: &str) {
    if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(text);
    }
}

fn build_issue_row(issue: &Issue) -> gtk::Box {
    let row = padded_box(gtk::Orientation::Vertical, 4, 8);

    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let badge = severity_badge(issue.severity);
    top.append(&badge);

    let name = label(
        &issue.name,
        if issue.is_active() {
            "bold"
        } else {
            "dim-label"
        },
    );
    name.set_wrap(true);
    name.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    name.set_hexpand(true);
    top.append(&name);

    let count_text = if issue.is_active() {
        issue.affected_count.to_string()
    } else {
        "Passed".to_string()
    };
    let count = label(&count_text, "numeric");
    if issue.is_active() {
        count.add_css_class("bold");
    } else {
        count.add_css_class("success");
    }
    count.set_valign(gtk::Align::Center);
    top.append(&count);
    row.append(&top);

    let bottom = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let source = label(issue.source.label(), "caption");
    source.add_css_class("dim-label");
    source.set_hexpand(true);
    bottom.append(&source);

    if issue.is_active() {
        let pct = label(&format!("{:.1}%", issue.affected_percentage), "caption");
        pct.add_css_class("dim-label");
        pct.add_css_class("numeric");
        bottom.append(&pct);
    }
    row.append(&bottom);

    row
}

#[derive(Clone, Copy)]
enum AffectedColumn {
    Url,
    Evidence,
}

fn affected_column(
    title: &str,
    width: i32,
    expand: bool,
    column_kind: AffectedColumn,
    data: AffectedData,
) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let item = object.downcast_ref::<gtk::ListItem>().unwrap();
        let text = label("", "");
        text.set_margin_start(10);
        text.set_margin_end(10);
        text.set_margin_top(8);
        text.set_margin_bottom(8);
        text.set_wrap(true);
        text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        item.set_child(Some(&text));
    });
    factory.connect_bind({
        let data = data.clone();
        move |_, object| {
            let item = object.downcast_ref::<gtk::ListItem>().unwrap();
            let Some(id) = item
                .item()
                .and_downcast::<gtk::StringObject>()
                .and_then(|s| s.string().parse::<usize>().ok())
            else {
                return;
            };
            let rows = data.borrow();
            let Some(row) = rows.get(id) else {
                return;
            };
            let value = match column_kind {
                AffectedColumn::Url => row.url.as_str(),
                AffectedColumn::Evidence => row.evidence.as_str(),
            };
            let text = item.child().and_downcast::<gtk::Label>().unwrap();
            text.set_text(value);
            text.set_tooltip_text(Some(value));
        }
    });

    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column.set_fixed_width(width);
    column.set_expand(expand);
    column.set_resizable(true);

    let sorter = gtk::CustomSorter::new(move |a, b| {
        let rows = data.borrow();
        let get = |obj: &glib::Object| {
            obj.downcast_ref::<gtk::StringObject>()
                .and_then(|s| s.string().parse::<usize>().ok())
                .and_then(|id| rows.get(id))
        };
        let ordering = match (get(a), get(b)) {
            (Some(a), Some(b)) => match column_kind {
                AffectedColumn::Url => a.url.cmp(&b.url),
                AffectedColumn::Evidence => a.evidence.cmp(&b.evidence),
            },
            _ => std::cmp::Ordering::Equal,
        };
        match ordering {
            std::cmp::Ordering::Less => gtk::Ordering::Smaller,
            std::cmp::Ordering::Equal => gtk::Ordering::Equal,
            std::cmp::Ordering::Greater => gtk::Ordering::Larger,
        }
    });
    column.set_sorter(Some(&sorter));
    column
}

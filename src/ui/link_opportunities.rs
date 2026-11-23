pub use crate::link_opportunities::{
    FilterCombinator, FilterOperator, NgramInfo, OpportunityFilterField, OpportunityFilterRule,
    OpportunityFilterSet, OpportunityInfo, strip_context_markup,
};
use crate::state::CrawlState;
use adw::prelude::*;
use glib::Object;
use glib::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

enum AnalysisUpdate {
    Progress {
        processed: usize,
        total: usize,
        opp_count: usize,
        ngram_count: usize,
    },
    Finished {
        opps: Vec<OpportunityInfo>,
        ngrams: Vec<NgramInfo>,
    },
    Cancelled,
}

// ==========================================
// 1. GObject Subclass for Opportunity Row
// ==========================================

glib::wrapper! {
    pub struct OpportunityRowData(ObjectSubclass<imp::OpportunityRowData>);
}

impl OpportunityRowData {
    pub fn new(info: OpportunityInfo) -> Self {
        let obj: Self = Object::builder().build();
        *obj.imp().info.borrow_mut() = Some(info);
        obj
    }

    pub fn get_info(&self) -> Option<OpportunityInfo> {
        self.imp().info.borrow().clone()
    }
}

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct OpportunityRowData {
        pub info: RefCell<Option<OpportunityInfo>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for OpportunityRowData {
        const NAME: &'static str = "OpportunityRowData";
        type Type = super::OpportunityRowData;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for OpportunityRowData {}
}

// ==========================================
// 2. GObject Subclass for N-gram Row
// ==========================================

glib::wrapper! {
    pub struct NgramRowData(ObjectSubclass<imp_ngram::NgramRowData>);
}

impl NgramRowData {
    pub fn new(info: NgramInfo) -> Self {
        let obj: Self = Object::builder().build();
        *obj.imp().info.borrow_mut() = Some(info);
        obj
    }

    pub fn get_info(&self) -> Option<NgramInfo> {
        self.imp().info.borrow().clone()
    }
}

mod imp_ngram {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct NgramRowData {
        pub info: RefCell<Option<NgramInfo>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NgramRowData {
        const NAME: &'static str = "NgramRowData";
        type Type = super::NgramRowData;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for NgramRowData {}
}

// ==========================================
// 3. Main Link Opportunities UI Panel
// ==========================================

struct RuleRowWidgets {
    row_box: gtk::Box,
    field_dropdown: gtk::DropDown,
    operator_dropdown: gtk::DropDown,
    value_entry: gtk::Entry,
    remove_button: gtk::Button,
}

fn default_placeholder_for_field(
    field: OpportunityFilterField,
    operator: FilterOperator,
) -> &'static str {
    match (field, operator) {
        (OpportunityFilterField::EitherUrl, FilterOperator::NotContains) => {
            "Exclude URLs containing (e.g. /tag/, /author/, ?page=)..."
        }
        (OpportunityFilterField::AnchorKeyword, FilterOperator::NotContains) => {
            "Exclude anchors containing (e.g. brand, author)..."
        }
        (OpportunityFilterField::AnchorKeyword, _) => {
            "Comma-separated anchor keywords (e.g. seo audit, internal links)..."
        }
        (OpportunityFilterField::ContextSnippet, _) => "Comma-separated context phrases...",
        _ => "Comma-separated URL patterns (e.g. /blog/, /guides/)...",
    }
}

pub struct LinkOpportunitiesPanel {
    widget: gtk::Box,
    opp_store: gio::ListStore,
    ngram_store: gio::ListStore,
    status_label: gtk::Label,
    opp_sel: gtk::SingleSelection,
    _export_button: gtk::Button,
    target_entry: gtk::Entry,
    analyze_button: gtk::Button,
    cancel_button: gtk::Button,
    cancel_token: Rc<RefCell<Option<Arc<AtomicBool>>>>,
    analysis_epoch: Rc<Cell<u64>>,
    clear_filters_btn: gtk::Button,
}

impl LinkOpportunitiesPanel {
    pub fn new(state: CrawlState) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 10);
        widget.set_margin_start(10);
        widget.set_margin_end(10);
        widget.set_margin_top(10);
        widget.set_margin_bottom(10);

        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 12);

        let analyze_button = gtk::Button::builder()
            .label("Analyze Link Opportunities")
            .css_classes(vec!["suggested-action".to_string()])
            .build();
        toolbar.append(&analyze_button);

        let cancel_button = gtk::Button::builder()
            .label("Cancel")
            .css_classes(vec!["destructive-action".to_string()])
            .sensitive(false)
            .build();
        toolbar.append(&cancel_button);

        let export_button = gtk::Button::builder()
            .label("Export CSV...")
            .icon_name("document-save-symbolic")
            .sensitive(false)
            .build();
        toolbar.append(&export_button);
        let target_entry = gtk::Entry::builder()
            .placeholder_text("All target URLs (or enter one URL)")
            .hexpand(true)
            .build();
        target_entry
            .update_property(&[gtk::accessible::Property::Label("Link analysis target URL")]);
        toolbar.append(&target_entry);

        let status_label = gtk::Label::new(Some("Ready (click Analyze to process crawled pages)"));
        status_label.add_css_class("dim-label");
        toolbar.append(&status_label);

        widget.append(&toolbar);

        // --- Ahrefs-style Live Rule Filter Bar ---
        let filter_bar = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let filter_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);

        let filter_title = gtk::Label::new(Some("Rules:"));
        filter_title.add_css_class("heading");
        filter_header.append(&filter_title);

        let combinator_labels: Vec<&str> =
            FilterCombinator::ALL.iter().map(|c| c.label()).collect();
        let combinator_dropdown = gtk::DropDown::from_strings(&combinator_labels);
        combinator_dropdown.set_selected(FilterCombinator::And.to_index());
        combinator_dropdown.set_tooltip_text(Some("Combine active filter rules with AND or OR"));
        filter_header.append(&combinator_dropdown);

        let add_filter_btn = gtk::Button::builder()
            .label("+ Add filter")
            .tooltip_text("Add another filter rule")
            .build();
        filter_header.append(&add_filter_btn);

        let clear_filters_btn = gtk::Button::builder()
            .label("Clear filters")
            .tooltip_text("Reset filter rules to default quick exclusions")
            .build();
        filter_header.append(&clear_filters_btn);

        let filter_count_label = gtk::Label::new(Some("Showing 0 of 0 link opportunities"));
        filter_count_label.add_css_class("dim-label");
        filter_count_label.add_css_class("numeric");
        filter_count_label.set_hexpand(true);
        filter_count_label.set_halign(gtk::Align::End);
        filter_header.append(&filter_count_label);

        filter_bar.append(&filter_header);

        let rules_container = gtk::Box::new(gtk::Orientation::Vertical, 6);
        filter_bar.append(&rules_container);
        widget.append(&filter_bar);

        // Split pane: Top = Opportunities table, Bottom = N-grams table
        let split_pane = gtk::Paned::new(gtk::Orientation::Vertical);
        split_pane.set_position(350);
        split_pane.set_wide_handle(true);
        split_pane.set_vexpand(true);
        split_pane.set_hexpand(true);
        widget.append(&split_pane);

        // Shared filter state
        let filter_set = Rc::new(RefCell::new(OpportunityFilterSet::default()));
        let rule_rows: Rc<RefCell<Vec<RuleRowWidgets>>> = Rc::new(RefCell::new(Vec::new()));
        let syncing_rules = Rc::new(Cell::new(false));

        // --- Top Pane: Opportunities Table ---
        let opp_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        opp_box.set_vexpand(true);
        let opp_title = gtk::Label::builder()
            .label("Internal Link Opportunities")
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading".to_string()])
            .build();
        opp_box.append(&opp_title);

        let opp_scrolled = gtk::ScrolledWindow::new();
        opp_scrolled.set_vexpand(true);
        opp_scrolled.set_hexpand(true);

        let opp_store = gio::ListStore::new::<OpportunityRowData>();
        let filter_set_for_opp = filter_set.clone();
        let opp_custom_filter = gtk::CustomFilter::new(move |obj| {
            obj.downcast_ref::<OpportunityRowData>()
                .and_then(|row| row.get_info())
                .is_some_and(|info| filter_set_for_opp.borrow().matches(&info))
        });
        let opp_filter_model =
            gtk::FilterListModel::new(Some(opp_store.clone()), Some(opp_custom_filter.clone()));
        let opp_sel = gtk::SingleSelection::new(Some(opp_filter_model.clone()));
        let opp_view = gtk::ColumnView::new(Some(opp_sel.clone()));
        opp_view.add_css_class("data-table");
        opp_view.set_show_column_separators(true);
        opp_scrolled.set_child(Some(&opp_view));
        opp_box.append(&opp_scrolled);
        split_pane.set_start_child(Some(&opp_box));

        // Column setup for opportunities
        let col_source =
            Self::create_opp_column("Source Page", false, |info| info.source_url.clone());
        col_source.set_fixed_width(310);
        col_source.set_resizable(true);
        opp_view.append_column(&col_source);

        let col_target = Self::create_opp_column("Target Page (Suggested)", false, |info| {
            info.target_url.clone()
        });
        col_target.set_fixed_width(310);
        col_target.set_resizable(true);
        opp_view.append_column(&col_target);

        let col_keyword =
            Self::create_opp_column("Anchor Keyword", false, |info| info.keyword.clone());
        col_keyword.set_fixed_width(180);
        col_keyword.set_resizable(true);
        opp_view.append_column(&col_keyword);

        let col_context =
            Self::create_opp_column("Context Snippet", true, |info| info.context.clone());
        col_context.set_fixed_width(360);
        col_context.set_expand(true);
        col_context.set_resizable(true);
        opp_view.append_column(&col_context);

        // --- Bottom Pane: N-grams Table ---
        let ngram_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        ngram_box.set_vexpand(true);
        let ngram_title = gtk::Label::builder()
            .label("Top N-grams (Common Phrases Across Site Titles/Headings — double-click to filter Anchor Keyword)")
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading".to_string()])
            .build();
        ngram_box.append(&ngram_title);

        let ngram_scrolled = gtk::ScrolledWindow::new();
        ngram_scrolled.set_vexpand(true);
        ngram_scrolled.set_hexpand(true);

        let ngram_store = gio::ListStore::new::<NgramRowData>();
        let filter_set_for_ngram = filter_set.clone();
        let ngram_custom_filter = gtk::CustomFilter::new(move |obj| {
            obj.downcast_ref::<NgramRowData>()
                .and_then(|row| row.get_info())
                .is_some_and(|info| filter_set_for_ngram.borrow().matches_ngram(&info))
        });
        let ngram_filter_model =
            gtk::FilterListModel::new(Some(ngram_store.clone()), Some(ngram_custom_filter.clone()));
        let ngram_sel = gtk::SingleSelection::new(Some(ngram_filter_model.clone()));
        let ngram_view = gtk::ColumnView::new(Some(ngram_sel));
        ngram_view.add_css_class("data-table");
        ngram_view.set_show_column_separators(true);
        ngram_scrolled.set_child(Some(&ngram_view));
        ngram_box.append(&ngram_scrolled);
        split_pane.set_end_child(Some(&ngram_box));

        // Column setup for n-grams
        let col_phrase = Self::create_ngram_column("Phrase", true, |info| info.phrase.clone());
        col_phrase.set_fixed_width(260);
        col_phrase.set_expand(true);
        col_phrase.set_resizable(true);
        ngram_view.append_column(&col_phrase);

        let col_length =
            Self::create_ngram_column("Word Count", false, |info| info.length.to_string());
        col_length.set_fixed_width(110);
        col_length.set_resizable(true);
        ngram_view.append_column(&col_length);

        let col_freq =
            Self::create_ngram_column("Occurrences", false, |info| info.frequency.to_string());
        col_freq.set_fixed_width(110);
        col_freq.set_resizable(true);
        ngram_view.append_column(&col_freq);

        // Live filter evaluation helper
        let refresh_filters: Rc<dyn Fn()> = {
            let filter_set = filter_set.clone();
            let rule_rows = rule_rows.clone();
            let combinator_dropdown = combinator_dropdown.clone();
            let opp_custom_filter = opp_custom_filter.clone();
            let ngram_custom_filter = ngram_custom_filter.clone();
            let opp_filter_model = opp_filter_model.clone();
            let opp_store = opp_store.clone();
            let filter_count_label = filter_count_label.clone();
            let export_button = export_button.clone();
            let analyze_button = analyze_button.clone();
            Rc::new(move || {
                {
                    let rows = rule_rows.borrow();
                    let can_remove = rows.len() > 1;
                    let mut rules = Vec::with_capacity(rows.len());
                    for r in rows.iter() {
                        r.remove_button.set_sensitive(can_remove);
                        let field = OpportunityFilterField::from_index(r.field_dropdown.selected());
                        let operator = FilterOperator::from_index(r.operator_dropdown.selected());
                        r.value_entry
                            .set_placeholder_text(Some(default_placeholder_for_field(
                                field, operator,
                            )));
                        rules.push(OpportunityFilterRule {
                            field,
                            operator,
                            raw_value: r.value_entry.text().to_string(),
                        });
                    }
                    let mut set = filter_set.borrow_mut();
                    set.combinator = FilterCombinator::from_index(combinator_dropdown.selected());
                    set.rules = rules;
                }
                opp_custom_filter.changed(gtk::FilterChange::Different);
                ngram_custom_filter.changed(gtk::FilterChange::Different);
                let visible = opp_filter_model.n_items();
                let total = opp_store.n_items();
                filter_count_label.set_text(&format!(
                    "Showing {} of {} link opportunities",
                    visible, total
                ));
                if analyze_button.is_sensitive() {
                    export_button.set_sensitive(visible > 0);
                }
            })
        };

        // Helper to append a rule row
        let append_rule_row: Rc<
            dyn Fn(OpportunityFilterField, FilterOperator, &str, Option<&str>),
        > = {
            let rules_container = rules_container.clone();
            let rule_rows = rule_rows.clone();
            let refresh_filters = refresh_filters.clone();
            let syncing_rules = syncing_rules.clone();

            Rc::new(move |field, operator, initial_value, custom_placeholder| {
                let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);

                let field_labels: Vec<&str> = OpportunityFilterField::ALL
                    .iter()
                    .map(|f| f.label())
                    .collect();
                let field_dropdown = gtk::DropDown::from_strings(&field_labels);
                field_dropdown.set_selected(field.to_index());

                let op_labels: Vec<&str> = FilterOperator::ALL.iter().map(|o| o.label()).collect();
                let operator_dropdown = gtk::DropDown::from_strings(&op_labels);
                operator_dropdown.set_selected(operator.to_index());

                let placeholder = custom_placeholder
                    .unwrap_or_else(|| default_placeholder_for_field(field, operator));
                let value_entry = gtk::Entry::builder()
                    .placeholder_text(placeholder)
                    .text(initial_value)
                    .hexpand(true)
                    .build();

                let remove_button = gtk::Button::builder()
                    .icon_name("list-remove-symbolic")
                    .tooltip_text("Remove filter rule")
                    .build();

                row_box.append(&field_dropdown);
                row_box.append(&operator_dropdown);
                row_box.append(&value_entry);
                row_box.append(&remove_button);
                rules_container.append(&row_box);

                let refresh_on_field = refresh_filters.clone();
                let syncing_on_field = syncing_rules.clone();
                field_dropdown.connect_selected_notify(move |_| {
                    if !syncing_on_field.get() {
                        refresh_on_field();
                    }
                });

                let refresh_on_op = refresh_filters.clone();
                let syncing_on_op = syncing_rules.clone();
                operator_dropdown.connect_selected_notify(move |_| {
                    if !syncing_on_op.get() {
                        refresh_on_op();
                    }
                });

                let refresh_on_val = refresh_filters.clone();
                let syncing_on_val = syncing_rules.clone();
                value_entry.connect_changed(move |_| {
                    if !syncing_on_val.get() {
                        refresh_on_val();
                    }
                });

                let rules_container_for_rm = rules_container.clone();
                let rule_rows_for_rm = rule_rows.clone();
                let refresh_on_rm = refresh_filters.clone();
                let row_box_for_rm = row_box.clone();
                remove_button.connect_clicked(move |_| {
                    {
                        let mut rows = rule_rows_for_rm.borrow_mut();
                        if rows.len() <= 1 {
                            return;
                        }
                        if let Some(pos) = rows.iter().position(|r| r.row_box == row_box_for_rm) {
                            rows.remove(pos);
                            rules_container_for_rm.remove(&row_box_for_rm);
                        }
                    }
                    refresh_on_rm();
                });

                rule_rows.borrow_mut().push(RuleRowWidgets {
                    row_box,
                    field_dropdown,
                    operator_dropdown,
                    value_entry,
                    remove_button,
                });
            })
        };

        let seed_default_rules: Rc<dyn Fn()> = {
            let rules_container = rules_container.clone();
            let rule_rows = rule_rows.clone();
            let combinator_dropdown = combinator_dropdown.clone();
            let append_rule_row = append_rule_row.clone();
            let refresh_filters = refresh_filters.clone();
            let syncing_rules = syncing_rules.clone();
            Rc::new(move || {
                syncing_rules.set(true);
                for r in rule_rows.borrow_mut().drain(..) {
                    rules_container.remove(&r.row_box);
                }
                combinator_dropdown.set_selected(FilterCombinator::And.to_index());
                append_rule_row(
                    OpportunityFilterField::EitherUrl,
                    FilterOperator::NotContains,
                    "",
                    Some("Exclude URLs containing (e.g. /tag/, /author/, ?page=)..."),
                );
                append_rule_row(
                    OpportunityFilterField::AnchorKeyword,
                    FilterOperator::NotContains,
                    "",
                    Some("Exclude anchors containing (e.g. brand, author)..."),
                );
                syncing_rules.set(false);
                refresh_filters();
            })
        };

        seed_default_rules();

        {
            let refresh_on_comb = refresh_filters.clone();
            let syncing_on_comb = syncing_rules.clone();
            combinator_dropdown.connect_selected_notify(move |_| {
                if !syncing_on_comb.get() {
                    refresh_on_comb();
                }
            });
        }

        {
            let append_rule_row = append_rule_row.clone();
            let refresh_filters = refresh_filters.clone();
            add_filter_btn.connect_clicked(move |_| {
                append_rule_row(
                    OpportunityFilterField::EitherUrl,
                    FilterOperator::Contains,
                    "",
                    None,
                );
                refresh_filters();
            });
        }

        {
            let seed_default_rules = seed_default_rules.clone();
            clear_filters_btn.connect_clicked(move |_| {
                seed_default_rules();
            });
        }

        // Activating a row in Top N-grams sets or adds an Anchor Keyword + Contains rule
        {
            let ngram_filter_model = ngram_filter_model.clone();
            let rule_rows = rule_rows.clone();
            let append_rule_row = append_rule_row.clone();
            let refresh_filters = refresh_filters.clone();
            let syncing_rules = syncing_rules.clone();
            ngram_view.connect_activate(move |_, pos| {
                let Some(phrase) = ngram_filter_model
                    .item(pos)
                    .and_downcast::<NgramRowData>()
                    .and_then(|row| row.get_info())
                    .map(|info| info.phrase)
                else {
                    return;
                };

                syncing_rules.set(true);
                let updated_existing = {
                    let rows = rule_rows.borrow();
                    if let Some(existing) = rows.iter().find(|r| {
                        OpportunityFilterField::from_index(r.field_dropdown.selected())
                            == OpportunityFilterField::AnchorKeyword
                            && FilterOperator::from_index(r.operator_dropdown.selected())
                                == FilterOperator::Contains
                    }) {
                        existing.value_entry.set_text(&phrase);
                        true
                    } else if let Some(empty_row) =
                        rows.iter().find(|r| r.value_entry.text().trim().is_empty())
                    {
                        empty_row
                            .field_dropdown
                            .set_selected(OpportunityFilterField::AnchorKeyword.to_index());
                        empty_row
                            .operator_dropdown
                            .set_selected(FilterOperator::Contains.to_index());
                        empty_row.value_entry.set_text(&phrase);
                        true
                    } else {
                        false
                    }
                };
                if !updated_existing {
                    append_rule_row(
                        OpportunityFilterField::AnchorKeyword,
                        FilterOperator::Contains,
                        &phrase,
                        None,
                    );
                }
                syncing_rules.set(false);
                refresh_filters();
            });
        }

        // Wire up analyze action
        let state_clone = state.clone();
        let opp_store_clone = opp_store.clone();
        let ngram_store_clone = ngram_store.clone();
        let status_lbl_clone = status_label.clone();
        let button_clone = analyze_button.clone();
        let cancel_button_clone = cancel_button.clone();
        let export_button_clone = export_button.clone();

        let current_cancel_token = Rc::new(RefCell::new(None::<Arc<AtomicBool>>));
        let cancel_token_for_panel = current_cancel_token.clone();
        let analysis_epoch = Rc::new(std::cell::Cell::new(0u64));
        let epoch_for_analysis = analysis_epoch.clone();
        let cancel_token_for_btn = current_cancel_token.clone();

        cancel_button.connect_clicked(move |_| {
            if let Some(token) = &*cancel_token_for_btn.borrow() {
                token.store(true, Ordering::Relaxed);
            }
        });

        // Wire up export action (iterates over filtered opportunities!)
        let opp_filter_for_export = opp_filter_model.clone();
        export_button.connect_clicked(move |btn| {
            let root = btn.root();
            let parent_window = root.as_ref().and_then(|r| r.downcast_ref::<gtk::Window>());

            let file_dialog = gtk::FileDialog::new();
            file_dialog.set_title("Export Link Opportunities to CSV");
            file_dialog.set_initial_name(Some("tadpole_link_opportunities.csv"));

            let filter = gtk::FileFilter::new();
            filter.set_name(Some("CSV Files"));
            filter.add_pattern("*.csv");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            file_dialog.set_filters(Some(&filters));
            file_dialog.set_default_filter(Some(&filter));

            let opp_filter_inner = opp_filter_for_export.clone();
            file_dialog.save(parent_window, None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let mut wtr = match csv::Writer::from_path(&path) {
                            Ok(w) => w,
                            Err(e) => {
                                eprintln!("Error creating CSV file: {:?}", e);
                                return;
                            }
                        };

                        let _ = wtr.write_record(&[
                            "Source Page",
                            "Target Page (Suggested)",
                            "Anchor Keyword",
                            "Context Snippet",
                        ]);

                        for i in 0..opp_filter_inner.n_items() {
                            if let Some(item) = opp_filter_inner.item(i) {
                                if let Some(row_data) = item.downcast_ref::<OpportunityRowData>() {
                                    if let Some(info) = row_data.get_info() {
                                        let plain_context = strip_context_markup(&info.context);
                                        let _ = wtr.write_record(&[
                                            &info.source_url,
                                            &info.target_url,
                                            &info.keyword,
                                            &plain_context,
                                        ]);
                                    }
                                }
                            }
                        }
                        let _ = wtr.flush();
                    }
                }
            });
        });

        let export_button_inner = export_button.clone();
        let target_for_analysis = target_entry.clone();
        let analyze_button_for_panel = analyze_button.clone();
        let cancel_button_for_panel = cancel_button.clone();
        let refresh_after_analysis = refresh_filters.clone();
        analyze_button.connect_clicked(move |_| {
            button_clone.set_sensitive(false);
            cancel_button_clone.set_sensitive(true);
            export_button_clone.set_sensitive(false);
            status_lbl_clone.set_text("Analyzing website structure in background...");

            let results = state_clone.get_all_results();
            let target_filter = target_for_analysis.text().trim().to_string();
            let reachable_sources: Option<std::collections::HashSet<String>> = if target_filter.is_empty() { None } else {
                state_clone.discovery().filter(|s| s.coverage.reachability_available()).map(|s| s.rows.into_iter().filter(|r| r.reached_depth.is_some()).map(|r| r.url).collect())
            };
            if results.is_empty() {
                status_lbl_clone.set_text("No crawled pages to analyze. Please run a crawl first.");
                button_clone.set_sensitive(true);
                cancel_button_clone.set_sensitive(false);
                return;
            }

            let token = Arc::new(AtomicBool::new(false));
            let generation = epoch_for_analysis.get().wrapping_add(1);
            epoch_for_analysis.set(generation);
            let epoch_for_updates = epoch_for_analysis.clone();
            *current_cancel_token.borrow_mut() = Some(token.clone());
            target_for_analysis.set_sensitive(false);
            let target_entry_inner = target_for_analysis.clone();

            let status_lbl_inner = status_lbl_clone.clone();
            let opp_store_inner = opp_store_clone.clone();
            let ngram_store_inner = ngram_store_clone.clone();
            let button_inner = button_clone.clone();
            let cancel_inner = cancel_button_clone.clone();
            let export_inner = export_button_inner.clone();
            let refresh_on_finish = refresh_after_analysis.clone();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AnalysisUpdate>();

            // Receive streaming updates on the main UI thread
            glib::MainContext::default().spawn_local(async move {
                while let Some(update) = rx.recv().await {
                    if epoch_for_updates.get() != generation { break; }
                    match update {
                        AnalysisUpdate::Progress { processed, total, opp_count, ngram_count } => {
                            let dots = match processed % 3 {
                                0 => ".",
                                1 => "..",
                                _ => "...",
                            };
                            status_lbl_inner.set_text(&format!(
                                "Analyzing website structure{} {}/{} pages (found {} opportunities, {} phrases)",
                                dots, processed, total, opp_count, ngram_count
                            ));
                        }
                        AnalysisUpdate::Finished { opps, ngrams } => {
                            opp_store_inner.remove_all();
                            let opp_count = opps.len();
                            for opp in opps {
                                opp_store_inner.append(&OpportunityRowData::new(opp));
                            }

                            ngram_store_inner.remove_all();
                            let ngram_count = ngrams.len();
                            for ngram in ngrams {
                                ngram_store_inner.append(&NgramRowData::new(ngram));
                            }

                            status_lbl_inner.set_text(&format!(
                                "Analysis complete! Found {} link opportunities and {} common phrases.",
                                opp_count, ngram_count
                            ));
                            button_inner.set_sensitive(true);
                            target_entry_inner.set_sensitive(true);
                            cancel_inner.set_sensitive(false);
                            export_inner.set_sensitive(opp_count > 0);
                            refresh_on_finish();
                        }
                        AnalysisUpdate::Cancelled => {
                            status_lbl_inner.set_text("Analysis cancelled by user.");
                            target_entry_inner.set_sensitive(true);
                            button_inner.set_sensitive(true);
                            cancel_inner.set_sensitive(false);
                            export_inner.set_sensitive(opp_store_inner.n_items() > 0);
                            refresh_on_finish();
                        }
                    }
                }
            });

            // Run CPU-heavy analysis step-by-step in a background worker thread
            std::thread::spawn(move || {
                let mut opportunities = vec![];
                let mut ngram_counts = HashMap::new();

                let mut target_pages = vec![];
                for res in &results {
                    if !target_filter.is_empty() && res.url != target_filter { continue; }
                    if res.status_code.map_or(false, |c| c >= 200 && c < 300) && res.indexable {
                        let title = res.title.clone().unwrap_or_default();
                        let h1 = res.h1.clone().unwrap_or_default();
                        let candidates = get_candidate_keywords(&title, &h1);

                        let kw_chars: Vec<(String, Vec<char>)> = candidates
                            .into_iter()
                            .map(|kw| {
                                let chars: Vec<char> = kw.to_lowercase().chars().collect();
                                (kw, chars)
                            })
                            .collect();

                        target_pages.push((res.url.clone(), kw_chars));
                    }
                }

                let total = results.len();
                for (idx, source) in results.iter().enumerate() {
                    if token.load(Ordering::Relaxed) {
                        let _ = tx.send(AnalysisUpdate::Cancelled);
                        return;
                    }

                    if !source.status_code.map_or(false, |c| c >= 200 && c < 300) {
                        continue;
                    }
                    if !target_filter.is_empty() && (!source.indexable || reachable_sources.as_ref().is_some_and(|sources| !sources.contains(&source.url))) { continue; }

                    let source_title = source.title.clone().unwrap_or_default();
                    let source_h1 = source.h1.clone().unwrap_or_default();
                    let source_md = source.markdown.clone().unwrap_or_default();
                    let stripped_md = strip_markdown(&source_md);

                    let mut text_block = format!("{} {} ", source_title, source_h1);
                    for heading in &source.headings {
                        text_block.push_str(&heading.text);
                        text_block.push(' ');
                    }
                    text_block.push_str(&stripped_md);

                    let source_ngrams = generate_page_ngrams(&text_block);
                    let text_chars_lower: Vec<char> = text_block.to_lowercase().chars().collect();
                    let text_chars_original: Vec<char> = text_block.chars().collect();

                    for (target_url, keywords) in &target_pages {
                        if *target_url == source.url {
                            continue;
                        }

                        if source.outlinks.contains(target_url) {
                            continue;
                        }

                        for (kw_original, kw_lower_chars) in keywords {
                            if source_ngrams.contains(kw_original) {
                                let matches = find_word_matches(&text_chars_lower, kw_lower_chars);
                                if !matches.is_empty() {
                                    let snippet = get_context_snippet(&text_chars_original, matches[0], kw_lower_chars.len());
                                    opportunities.push(OpportunityInfo {
                                        source_url: source.url.clone(),
                                        target_url: target_url.clone(),
                                        keyword: kw_original.clone(),
                                        context: snippet,
                                    });
                                    break;
                                }
                            }
                        }
                    }

                    // Collect N-grams
                    let mut ngram_text = format!("{} ", source_title);
                    if !source_h1.is_empty() {
                        ngram_text.push_str(&source_h1);
                        ngram_text.push(' ');
                    }
                    for heading in &source.headings {
                        ngram_text.push_str(&heading.text);
                        ngram_text.push(' ');
                    }

                    let words = clean_and_tokenize(&ngram_text);
                    let n = words.len();
                    for len in 1..=4 {
                        if n >= len {
                            for i in 0..=(n - len) {
                                let gram = &words[i..i+len];
                                if is_stop_word(&gram[0]) || is_stop_word(&gram[len - 1]) {
                                    continue;
                                }
                                if gram.iter().any(|w| w.len() < 2) {
                                    continue;
                                }
                                let phrase = gram.join(" ");
                                if phrase.len() >= 3 {
                                    *ngram_counts.entry(phrase).or_insert(0) += 1;
                                }
                            }
                        }
                    }

                    // Send progress update (counts only to keep UI extremely fast)
                    if idx % 10 == 0 || idx == total - 1 {
                        let _ = tx.send(AnalysisUpdate::Progress {
                            processed: idx + 1,
                            total,
                            opp_count: opportunities.len(),
                            ngram_count: ngram_counts.len(),
                        });
                    }
                }

                let mut final_ngrams: Vec<NgramInfo> = ngram_counts
                    .into_iter()
                    .map(|(phrase, count)| {
                        let length = phrase.split_whitespace().count() as u8;
                        NgramInfo {
                            phrase,
                            length,
                            frequency: count,
                        }
                    })
                    .filter(|n| n.frequency > 1)
                    .collect();
                final_ngrams.sort_by(|a, b| b.frequency.cmp(&a.frequency));

                // Truncate to top 200 items to keep UI update instantaneous
                final_ngrams.truncate(200);

                let _ = tx.send(AnalysisUpdate::Finished {
                    opps: opportunities,
                    ngrams: final_ngrams,
                });
            });
        });

        Self {
            widget,
            opp_store,
            ngram_store,
            status_label,
            opp_sel,
            _export_button: export_button,
            target_entry,
            analyze_button: analyze_button_for_panel,
            cancel_button: cancel_button_for_panel,
            cancel_token: cancel_token_for_panel,
            analysis_epoch,
            clear_filters_btn,
        }
    }

    pub fn analyze_target(&self, url: &str) {
        if !self.analyze_button.is_sensitive() {
            self.status_label
                .set_text("Cancel the current analysis before analyzing another target.");
            return;
        }
        self.target_entry.set_text(url);
        self.analyze_button.emit_clicked();
    }

    pub fn connect_selection_changed<F>(&self, callback: F)
    where
        F: Fn(Option<String>) + 'static,
    {
        let selection_model = self.opp_sel.clone();
        selection_model.connect_selected_item_notify(move |model| {
            let item = model.selected_item();
            if let Some(item) = item {
                if let Some(row_data) = item.downcast_ref::<OpportunityRowData>() {
                    if let Some(info) = row_data.get_info() {
                        callback(Some(info.source_url));
                    }
                }
            } else {
                callback(None);
            }
        });
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }

    pub fn clear(&self) {
        self.analysis_epoch
            .set(self.analysis_epoch.get().wrapping_add(1));
        if let Some(token) = self.cancel_token.borrow_mut().take() {
            token.store(true, Ordering::Relaxed);
        }
        self.analyze_button.set_sensitive(true);
        self.cancel_button.set_sensitive(false);
        self.target_entry.set_text("");
        self.target_entry.set_sensitive(true);
        self._export_button.set_sensitive(false);
        self.opp_store.remove_all();
        self.ngram_store.remove_all();
        self.clear_filters_btn.emit_clicked();
        self.status_label
            .set_text("Ready (click Analyze to process crawled pages)");
    }

    fn create_opp_column<F>(title: &str, use_markup: bool, select_fn: F) -> gtk::ColumnViewColumn
    where
        F: Fn(&OpportunityInfo) -> String + 'static,
    {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, obj| {
            let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_halign(gtk::Align::Fill);
            label.set_xalign(0.0);
            label.set_valign(gtk::Align::Center);
            label.set_hexpand(true);
            label.set_wrap(true);
            label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            label.set_margin_start(10);
            label.set_margin_end(10);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            label.set_selectable(true);
            list_item.set_child(Some(&label));
        });
        factory.connect_bind(move |_, obj| {
            let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
            let row_data = list_item
                .item()
                .and_downcast::<OpportunityRowData>()
                .unwrap();
            if let Some(info) = row_data.get_info() {
                let text = select_fn(&info);
                if use_markup {
                    label.set_use_markup(true);
                    label.set_markup(&text);
                    let plain = strip_context_markup(&text);
                    label.set_tooltip_text(if plain.is_empty() { None } else { Some(&plain) });
                } else {
                    label.set_text(&text);
                    label.set_tooltip_text(if text.is_empty() { None } else { Some(&text) });
                }
            }
        });
        gtk::ColumnViewColumn::new(Some(title), Some(factory))
    }

    fn create_ngram_column<F>(title: &str, wrap: bool, select_fn: F) -> gtk::ColumnViewColumn
    where
        F: Fn(&NgramInfo) -> String + 'static,
    {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, obj| {
            let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_halign(gtk::Align::Fill);
            label.set_xalign(0.0);
            label.set_valign(gtk::Align::Center);
            label.set_hexpand(true);
            label.set_wrap(wrap);
            if wrap {
                label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            }
            label.set_margin_start(10);
            label.set_margin_end(10);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            list_item.set_child(Some(&label));
        });
        factory.connect_bind(move |_, obj| {
            let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();
            let row_data = list_item.item().and_downcast::<NgramRowData>().unwrap();
            if let Some(info) = row_data.get_info() {
                let text = select_fn(&info);
                label.set_text(&text);
                if wrap && !text.is_empty() {
                    label.set_tooltip_text(Some(&text));
                } else {
                    label.set_tooltip_text(None);
                }
            }
        });
        gtk::ColumnViewColumn::new(Some(title), Some(factory))
    }
}

// ==========================================
// 4. Algorithm & Text Processing Helpers
// ==========================================

fn is_stop_word(word: &str) -> bool {
    static STOP_WORDS: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
        std::sync::LazyLock::new(|| {
            [
                "the",
                "a",
                "an",
                "and",
                "or",
                "but",
                "if",
                "then",
                "else",
                "to",
                "in",
                "on",
                "at",
                "by",
                "for",
                "with",
                "about",
                "of",
                "is",
                "are",
                "was",
                "were",
                "be",
                "been",
                "have",
                "has",
                "had",
                "this",
                "that",
                "these",
                "those",
                "from",
                "our",
                "your",
                "their",
                "my",
                "its",
                "i",
                "you",
                "we",
                "they",
                "he",
                "she",
                "it",
                "i",
                "w",
                "na",
                "z",
                "do",
                "o",
                "u",
                "dla",
                "ze",
                "za",
                "po",
                "oraz",
                "jest",
                "sa",
                "są",
                "wraz",
                "pod",
                "nad",
                "przed",
                "miedzy",
                "między",
                "przez",
                "od",
                "takze",
                "także",
                "lub",
                "albo",
                "czy",
                "bo",
                "poniewaz",
                "ponieważ",
                "gdyz",
                "gdyż",
                "ale",
                "lecz",
                "jednak",
                "a",
                "ze",
                "że",
                "to",
                "sie",
                "się",
                "go",
                "mu",
                "ich",
                "je",
                "ją",
                "jej",
                "jego",
                "nimi",
                "nim",
                "by",
                "być",
                "byc",
                "o",
            ]
            .iter()
            .cloned()
            .collect()
        });

    STOP_WORDS.contains(word)
}

fn clean_text(text: &str) -> String {
    let mut clean = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                clean.push(lc);
            }
        } else {
            clean.push(' ');
        }
    }
    clean
}

fn clean_and_tokenize(text: &str) -> Vec<String> {
    clean_text(text)
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn get_candidate_keywords(title: &str, h1: &str) -> Vec<String> {
    let mut candidates = std::collections::HashSet::new();

    for source in &[title, h1] {
        let words = clean_and_tokenize(source);
        let n = words.len();

        // 1-gram candidate if it is the whole phrase and not a stop word
        if n == 1 && !is_stop_word(&words[0]) && words[0].len() >= 4 {
            candidates.insert(words[0].clone());
        }

        // 2 to 4-grams
        for len in 2..=4 {
            if n >= len {
                for i in 0..=(n - len) {
                    let gram = &words[i..i + len];
                    if is_stop_word(&gram[0]) || is_stop_word(&gram[len - 1]) {
                        continue;
                    }
                    if gram.iter().any(|w| w.len() < 2) {
                        continue;
                    }
                    let phrase = gram.join(" ");
                    if phrase.len() >= 4 {
                        candidates.insert(phrase);
                    }
                }
            }
        }
    }

    candidates.into_iter().collect()
}

fn strip_markdown(md: &str) -> String {
    let mut plain = String::new();
    let mut chars = md.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut link_text = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == ']' {
                    chars.next();
                    break;
                }
                link_text.push(chars.next().unwrap());
            }
            if let Some(&'(') = chars.peek() {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    if nc == ')' {
                        chars.next();
                        break;
                    }
                    chars.next();
                }
            }
            plain.push_str(&link_text);
        } else if c == '*' || c == '_' || c == '`' || c == '#' {
            // skip markdown syntax
        } else {
            plain.push(c);
        }
    }
    plain
}

fn find_word_matches(chars: &[char], keyword_chars: &[char]) -> Vec<usize> {
    let mut matches = vec![];
    let n = chars.len();
    let m = keyword_chars.len();
    if m == 0 || n < m {
        return matches;
    }

    for i in 0..=(n - m) {
        let mut matched = true;
        for j in 0..m {
            if chars[i + j] != keyword_chars[j] {
                matched = false;
                break;
            }
        }

        if matched {
            let boundary_before = if i == 0 {
                true
            } else {
                !chars[i - 1].is_alphanumeric()
            };

            let boundary_after = if i + m >= n {
                true
            } else {
                !chars[i + m].is_alphanumeric()
            };

            if boundary_before && boundary_after {
                matches.push(i);
            }
        }
    }
    matches
}

fn get_context_snippet(chars: &[char], match_char_idx: usize, kw_char_len: usize) -> String {
    let start_char = if match_char_idx > 35 {
        match_char_idx - 35
    } else {
        0
    };
    let end_char = std::cmp::min(chars.len(), match_char_idx + kw_char_len + 35);

    let prefix: String = chars[start_char..match_char_idx].iter().collect();
    let matched: String = chars[match_char_idx..match_char_idx + kw_char_len]
        .iter()
        .collect();
    let suffix: String = chars[match_char_idx + kw_char_len..end_char]
        .iter()
        .collect();

    let clean_prefix = glib::markup_escape_text(&prefix);
    let clean_matched = glib::markup_escape_text(&matched);
    let clean_suffix = glib::markup_escape_text(&suffix);

    format!(
        "...{}<b>{}</b>{}...",
        clean_prefix, clean_matched, clean_suffix
    )
}

fn generate_page_ngrams(text: &str) -> std::collections::HashSet<String> {
    let words = clean_and_tokenize(text);
    let mut ngrams = std::collections::HashSet::new();
    let n = words.len();

    for len in 1..=4 {
        if n >= len {
            for i in 0..=(n - len) {
                let phrase = words[i..i + len].join(" ");
                ngrams.insert(phrase);
            }
        }
    }
    ngrams
}

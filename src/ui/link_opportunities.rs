use adw::prelude::*;
use glib::Object;
use glib::subclass::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use crate::state::CrawlState;

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

#[derive(Debug, Clone, Default)]
pub struct OpportunityInfo {
    pub source_url: String,
    pub target_url: String,
    pub keyword: String,
    pub context: String,
}

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

#[derive(Debug, Clone, Default)]
pub struct NgramInfo {
    pub phrase: String,
    pub length: u8,
    pub frequency: usize,
}

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

pub struct LinkOpportunitiesPanel {
    widget: gtk::Box,
    opp_store: gio::ListStore,
    ngram_store: gio::ListStore,
    status_label: gtk::Label,
    opp_sel: gtk::SingleSelection,
    _export_button: gtk::Button,
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

        let status_label = gtk::Label::new(Some("Ready (click Analyze to process crawled pages)"));
        status_label.add_css_class("dim-label");
        toolbar.append(&status_label);

        widget.append(&toolbar);

        // Split pane: Top = Opportunities table, Bottom = N-grams table
        let split_pane = gtk::Paned::new(gtk::Orientation::Vertical);
        split_pane.set_position(350);
        split_pane.set_wide_handle(true);
        split_pane.set_vexpand(true);
        split_pane.set_hexpand(true);
        widget.append(&split_pane);

        // --- Top Pane: Opportunities Table ---
        let opp_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        opp_box.set_vexpand(true);
        let opp_title = gtk::Label::builder()
            .label("Internal Link Opportunities (Missing Links)")
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading".to_string()])
            .build();
        opp_box.append(&opp_title);

        let opp_scrolled = gtk::ScrolledWindow::new();
        opp_scrolled.set_vexpand(true);
        opp_scrolled.set_hexpand(true);

        let opp_store = gio::ListStore::new::<OpportunityRowData>();
        let opp_sel = gtk::SingleSelection::new(Some(opp_store.clone()));
        let opp_view = gtk::ColumnView::new(Some(opp_sel.clone()));
        opp_view.add_css_class("data-table");
        opp_scrolled.set_child(Some(&opp_view));
        opp_box.append(&opp_scrolled);
        split_pane.set_start_child(Some(&opp_box));

        // Column setup for opportunities
        let col_source = Self::create_opp_column("Source Page", false, |info| info.source_url.clone());
        col_source.set_fixed_width(220);
        col_source.set_resizable(true);
        opp_view.append_column(&col_source);

        let col_target = Self::create_opp_column("Target Page (Suggested)", false, |info| info.target_url.clone());
        col_target.set_fixed_width(220);
        col_target.set_resizable(true);
        opp_view.append_column(&col_target);

        let col_keyword = Self::create_opp_column("Anchor Keyword", false, |info| info.keyword.clone());
        col_keyword.set_fixed_width(140);
        col_keyword.set_resizable(true);
        opp_view.append_column(&col_keyword);

        let col_context = Self::create_opp_column("Context Snippet", true, |info| info.context.clone());
        col_context.set_expand(true);
        col_context.set_resizable(true);
        opp_view.append_column(&col_context);

        // --- Bottom Pane: N-grams Table ---
        let ngram_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        ngram_box.set_vexpand(true);
        let ngram_title = gtk::Label::builder()
            .label("Top N-grams (Common Phrases Across Site Titles/Headings)")
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading".to_string()])
            .build();
        ngram_box.append(&ngram_title);

        let ngram_scrolled = gtk::ScrolledWindow::new();
        ngram_scrolled.set_vexpand(true);
        ngram_scrolled.set_hexpand(true);

        let ngram_store = gio::ListStore::new::<NgramRowData>();
        let ngram_sel = gtk::SingleSelection::new(Some(ngram_store.clone()));
        let ngram_view = gtk::ColumnView::new(Some(ngram_sel));
        ngram_view.add_css_class("data-table");
        ngram_scrolled.set_child(Some(&ngram_view));
        ngram_box.append(&ngram_scrolled);
        split_pane.set_end_child(Some(&ngram_box));

        // Column setup for n-grams
        let col_phrase = Self::create_ngram_column("Phrase", |info| info.phrase.clone());
        col_phrase.set_expand(true);
        ngram_view.append_column(&col_phrase);

        let col_length = Self::create_ngram_column("Word Count", |info| info.length.to_string());
        ngram_view.append_column(&col_length);

        let col_freq = Self::create_ngram_column("Occurrences", |info| info.frequency.to_string());
        ngram_view.append_column(&col_freq);

        // Wire up analyze action
        let state_clone = state.clone();
        let opp_store_clone = opp_store.clone();
        let ngram_store_clone = ngram_store.clone();
        let status_lbl_clone = status_label.clone();
        let button_clone = analyze_button.clone();
        let cancel_button_clone = cancel_button.clone();
        let export_button_clone = export_button.clone();

        let current_cancel_token = Rc::new(RefCell::new(None::<Arc<AtomicBool>>));
        let cancel_token_for_btn = current_cancel_token.clone();

        cancel_button.connect_clicked(move |_| {
            if let Some(token) = &*cancel_token_for_btn.borrow() {
                token.store(true, Ordering::Relaxed);
            }
        });

        // Wire up export action
        let opp_store_for_export = opp_store.clone();
        export_button.connect_clicked(move |btn| {
            let root = btn.root();
            let parent_window = root.as_ref().and_then(|r| r.downcast_ref::<gtk::Window>());
            
            let file_dialog = gtk::FileDialog::new();
            file_dialog.set_title("Export Link Opportunities to CSV");
            
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("CSV Files"));
            filter.add_pattern("*.csv");
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            file_dialog.set_filters(Some(&filters));
            
            let opp_store_inner = opp_store_for_export.clone();
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
                        
                        let _ = wtr.write_record(&["Source Page", "Target Page (Suggested)", "Anchor Keyword", "Context Snippet"]);
                        
                        for i in 0..opp_store_inner.n_items() {
                            if let Some(item) = opp_store_inner.item(i) {
                                if let Some(row_data) = item.downcast_ref::<OpportunityRowData>() {
                                    if let Some(info) = row_data.get_info() {
                                        let plain_context = info.context
                                            .replace("<b>", "")
                                            .replace("</b>", "");
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
        analyze_button.connect_clicked(move |_| {
            button_clone.set_sensitive(false);
            cancel_button_clone.set_sensitive(true);
            export_button_clone.set_sensitive(false);
            status_lbl_clone.set_text("Analyzing website structure in background...");
            
            let results = state_clone.get_all_results();
            if results.is_empty() {
                status_lbl_clone.set_text("No crawled pages to analyze. Please run a crawl first.");
                button_clone.set_sensitive(true);
                cancel_button_clone.set_sensitive(false);
                return;
            }

            let token = Arc::new(AtomicBool::new(false));
            *current_cancel_token.borrow_mut() = Some(token.clone());

            let status_lbl_inner = status_lbl_clone.clone();
            let opp_store_inner = opp_store_clone.clone();
            let ngram_store_inner = ngram_store_clone.clone();
            let button_inner = button_clone.clone();
            let cancel_inner = cancel_button_clone.clone();
            let export_inner = export_button_inner.clone();

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AnalysisUpdate>();

            // Receive streaming updates on the main UI thread
            glib::MainContext::default().spawn_local(async move {
                while let Some(update) = rx.recv().await {
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
                            cancel_inner.set_sensitive(false);
                            export_inner.set_sensitive(opp_count > 0);
                        }
                        AnalysisUpdate::Cancelled => {
                            status_lbl_inner.set_text("Analysis cancelled by user.");
                            button_inner.set_sensitive(true);
                            cancel_inner.set_sensitive(false);
                            export_inner.set_sensitive(opp_store_inner.n_items() > 0);
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
        }
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
        self.opp_store.remove_all();
        self.ngram_store.remove_all();
        self.status_label.set_text("Ready (click Analyze to process crawled pages)");
    }

    fn create_opp_column<F>(title: &str, use_markup: bool, select_fn: F) -> gtk::ColumnViewColumn
    where
        F: Fn(&OpportunityInfo) -> String + 'static,
    {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, obj| {
            let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_halign(gtk::Align::Start);
            label.set_valign(gtk::Align::Center);
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
            let row_data = list_item.item().and_downcast::<OpportunityRowData>().unwrap();
            if let Some(info) = row_data.get_info() {
                let text = select_fn(&info);
                if use_markup {
                    label.set_use_markup(true);
                    label.set_markup(&text);
                } else {
                    label.set_text(&text);
                }
            }
        });
        gtk::ColumnViewColumn::new(Some(title), Some(factory))
    }

    fn create_ngram_column<F>(title: &str, select_fn: F) -> gtk::ColumnViewColumn
    where
        F: Fn(&NgramInfo) -> String + 'static,
    {
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(|_, obj| {
            let list_item = obj.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::new(None);
            label.set_halign(gtk::Align::Start);
            label.set_valign(gtk::Align::Center);
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
                label.set_text(&select_fn(&info));
            }
        });
        gtk::ColumnViewColumn::new(Some(title), Some(factory))
    }
}

// ==========================================
// 4. Algorithm & Text Processing Helpers
// ==========================================

fn is_stop_word(word: &str) -> bool {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "if", "then", "else", "to", "in", "on", "at", "by", "for", 
        "with", "about", "of", "is", "are", "was", "were", "be", "been", "have", "has", "had", "this", 
        "that", "these", "those", "from", "our", "your", "their", "my", "its", "i", "you", "we", "they", 
        "he", "she", "it",
        "i", "w", "na", "z", "do", "o", "u", "dla", "ze", "za", "po", "oraz", "jest", "sa", "są", "wraz", 
        "pod", "nad", "przed", "miedzy", "między", "przez", "od", "takze", "także", "lub", "albo", "czy", 
        "bo", "poniewaz", "ponieważ", "gdyz", "gdyż", "ale", "lecz", "jednak", "a", "ze", "że", "to", "sie", "się",
        "go", "mu", "ich", "je", "ją", "jej", "jego", "nimi", "nim", "by", "być", "byc", "o"
    ].iter().cloned().collect();
    
    stop_words.contains(word)
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
                    let gram = &words[i..i+len];
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
    let start_char = if match_char_idx > 35 { match_char_idx - 35 } else { 0 };
    let end_char = std::cmp::min(chars.len(), match_char_idx + kw_char_len + 35);
    
    let prefix: String = chars[start_char..match_char_idx].iter().collect();
    let matched: String = chars[match_char_idx..match_char_idx + kw_char_len].iter().collect();
    let suffix: String = chars[match_char_idx + kw_char_len..end_char].iter().collect();
    
    let clean_prefix = glib::markup_escape_text(&prefix);
    let clean_matched = glib::markup_escape_text(&matched);
    let clean_suffix = glib::markup_escape_text(&suffix);
    
    format!("...{}<b>{}</b>{}...", clean_prefix, clean_matched, clean_suffix)
}

fn generate_page_ngrams(text: &str) -> std::collections::HashSet<String> {
    let words = clean_and_tokenize(text);
    let mut ngrams = std::collections::HashSet::new();
    let n = words.len();
    
    for len in 1..=4 {
        if n >= len {
            for i in 0..=(n - len) {
                let phrase = words[i..i+len].join(" ");
                ngrams.insert(phrase);
            }
        }
    }
    ngrams
}

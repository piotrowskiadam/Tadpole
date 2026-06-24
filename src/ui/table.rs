use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::state::CrawlResult;
use crate::ui::row_data::CrawlRowData;
use crate::ui::sidebar::CrawlFilter;

pub struct Table {
    scrolled_window: gtk::ScrolledWindow,
    list_store: gio::ListStore,
    filter_model: gtk::FilterListModel,
    selection_model: gtk::SingleSelection,
    url_to_index: Rc<RefCell<HashMap<String, u32>>>,
    active_filter: Rc<RefCell<CrawlFilter>>,
    search_query: Rc<RefCell<String>>,
}

fn map_ordering(ord: std::cmp::Ordering) -> gtk::Ordering {
    match ord {
        std::cmp::Ordering::Less => gtk::Ordering::Smaller,
        std::cmp::Ordering::Equal => gtk::Ordering::Equal,
        std::cmp::Ordering::Greater => gtk::Ordering::Larger,
    }
}

impl Table {
    pub fn new() -> Self {
        let scrolled_window = gtk::ScrolledWindow::new();
        scrolled_window.set_vexpand(true);
        scrolled_window.set_hexpand(true);

        let list_store = gio::ListStore::new::<CrawlRowData>();
        let url_to_index = Rc::new(RefCell::new(HashMap::new()));
        let active_filter = Rc::new(RefCell::new(CrawlFilter::All));
        let search_query = Rc::new(RefCell::new(String::new()));
 
        // Setup filter model
        let active_filter_clone = active_filter.clone();
        let search_query_clone = search_query.clone();
        let list_store_clone = list_store.clone();
        let filter = gtk::CustomFilter::new(move |item| {
            let row_data = item.downcast_ref::<CrawlRowData>().unwrap();
            if let Some(res) = row_data.get_result() {
                // To compute duplicate status, we need list of all results
                let mut all_results = vec![];
                let n = list_store_clone.n_items();
                for i in 0..n {
                    if let Some(obj) = list_store_clone.item(i) {
                        if let Some(data) = obj.downcast_ref::<CrawlRowData>() {
                            if let Some(r) = data.get_result() {
                                all_results.push(r);
                            }
                        }
                    }
                }
                
                // 1. Sidebar filter check
                if !active_filter_clone.borrow().matches(&res, &all_results) {
                    return false;
                }
                
                // 2. Search query check
                let query = search_query_clone.borrow().to_lowercase();
                if query.is_empty() {
                    return true;
                }
                
                let url_match = res.url.to_lowercase().contains(&query);
                let title_match = res.title.as_ref().map_or(false, |t| t.to_lowercase().contains(&query));
                let desc_match = res.meta_desc.as_ref().map_or(false, |d| d.to_lowercase().contains(&query));
                let status_match = res.status_code.map_or(false, |c| c.to_string().contains(&query));
                
                url_match || title_match || desc_match || status_match
            } else {
                false
            }
        });
 
        let filter_model = gtk::FilterListModel::new(Some(list_store.clone()), Some(filter));

        // Setup ColumnView first so we can use its built-in sorter
        let column_view = gtk::ColumnView::new(None::<gtk::SelectionModel>);
        let sorter = column_view.sorter();

        // Setup sort model
        let sort_model = gtk::SortListModel::new(Some(filter_model.clone()), sorter);

        // Setup selection
        let selection_model = gtk::SingleSelection::new(Some(sort_model));
        column_view.set_model(Some(&selection_model));
        column_view.add_css_class("data-table");
        scrolled_window.set_child(Some(&column_view));

        // Column 1: URL Address
        let col_url = Self::create_text_column("Address", |res| res.url.clone());
        col_url.set_expand(true);
        let sort_url = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.url).unwrap_or_default();
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.url).unwrap_or_default();
            map_ordering(a_val.cmp(&b_val))
        });
        col_url.set_sorter(Some(&sort_url));
        column_view.append_column(&col_url);

        // Column 2: Status Code
        let col_status = Self::create_text_column("Status", |res| {
            res.status_code.map_or("Err".to_string(), |c| c.to_string())
        });
        let sort_status = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().and_then(|r| r.status_code).unwrap_or(0);
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().and_then(|r| r.status_code).unwrap_or(0);
            map_ordering(a_val.cmp(&b_val))
        });
        col_status.set_sorter(Some(&sort_status));
        column_view.append_column(&col_status);

        // Column 3: Indexability
        let col_index = Self::create_text_column("Indexability", |res| res.indexability_status.clone());
        let sort_index = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.indexability_status).unwrap_or_default();
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.indexability_status).unwrap_or_default();
            map_ordering(a_val.cmp(&b_val))
        });
        col_index.set_sorter(Some(&sort_index));
        column_view.append_column(&col_index);

        // Column 4: Title
        let col_title = Self::create_text_column("Title", |res| res.title.clone().unwrap_or_default());
        let sort_title = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().and_then(|r| r.title).unwrap_or_default();
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().and_then(|r| r.title).unwrap_or_default();
            map_ordering(a_val.cmp(&b_val))
        });
        col_title.set_sorter(Some(&sort_title));
        column_view.append_column(&col_title);

        // Column 5: Word Count
        let col_words = Self::create_text_column("Word Count", |res| res.word_count.to_string());
        let sort_words = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.word_count).unwrap_or(0);
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.word_count).unwrap_or(0);
            map_ordering(a_val.cmp(&b_val))
        });
        col_words.set_sorter(Some(&sort_words));
        column_view.append_column(&col_words);

        // Column 6: Size (Bytes)
        let col_size = Self::create_text_column("Size (KB)", |res| format!("{:.2}", res.size_bytes as f64 / 1024.0));
        let sort_size = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.size_bytes).unwrap_or(0);
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.size_bytes).unwrap_or(0);
            map_ordering(a_val.cmp(&b_val))
        });
        col_size.set_sorter(Some(&sort_size));
        column_view.append_column(&col_size);
 
        // Column 7: Depth
        let col_depth = Self::create_text_column("Depth", |res| res.depth.to_string());
        let sort_depth = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.depth).unwrap_or(0);
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.depth).unwrap_or(0);
            map_ordering(a_val.cmp(&b_val))
        });
        col_depth.set_sorter(Some(&sort_depth));
        column_view.append_column(&col_depth);
 
        // Column 8: Response Time (ms)
        let col_time = Self::create_text_column("Response Time (ms)", |res| res.response_time_ms.to_string());
        let sort_time = gtk::CustomSorter::new(|a, b| {
            let a_val = a.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.response_time_ms).unwrap_or(0);
            let b_val = b.downcast_ref::<CrawlRowData>().unwrap().get_result().map(|r| r.response_time_ms).unwrap_or(0);
            map_ordering(a_val.cmp(&b_val))
        });
        col_time.set_sorter(Some(&sort_time));
        column_view.append_column(&col_time);
 
        Self {
            scrolled_window,
            list_store,
            filter_model,
            selection_model,
            url_to_index,
            active_filter,
            search_query,
        }
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.scrolled_window
    }

    pub fn add_or_update(&self, result: CrawlResult) {
        let mut index_map = self.url_to_index.borrow_mut();
        if let Some(&idx) = index_map.get(&result.url) {
            if let Some(item) = self.list_store.item(idx) {
                if let Some(row_data) = item.downcast_ref::<CrawlRowData>() {
                    row_data.set_result(result);
                    // Notify list model that item changed
                    self.list_store.items_changed(idx, 1, 1);
                }
            }
        } else {
            let idx = self.list_store.n_items();
            let row_data = CrawlRowData::new(result.clone());
            self.list_store.append(&row_data);
            index_map.insert(result.url, idx);
        }
    }

    pub fn clear(&self) {
        self.list_store.remove_all();
        self.url_to_index.borrow_mut().clear();
    }

    pub fn set_filter(&self, filter: CrawlFilter) {
        *self.active_filter.borrow_mut() = filter;
        self.apply_combined_filter();
    }
 
    pub fn set_search_query(&self, query: &str) {
        *self.search_query.borrow_mut() = query.trim().to_string();
        self.apply_combined_filter();
    }
 
    fn apply_combined_filter(&self) {
        let active_filter_clone = self.active_filter.clone();
        let search_query_clone = self.search_query.clone();
        let list_store_clone = self.list_store.clone();
        
        let custom_filter = gtk::CustomFilter::new(move |item| {
            let row_data = item.downcast_ref::<CrawlRowData>().unwrap();
            if let Some(res) = row_data.get_result() {
                let mut all_results = vec![];
                let n = list_store_clone.n_items();
                for i in 0..n {
                    if let Some(obj) = list_store_clone.item(i) {
                        if let Some(data) = obj.downcast_ref::<CrawlRowData>() {
                            if let Some(r) = data.get_result() {
                                all_results.push(r);
                            }
                        }
                    }
                }
                
                // 1. Sidebar filter check
                if !active_filter_clone.borrow().matches(&res, &all_results) {
                    return false;
                }
                
                // 2. Search query check
                let query = search_query_clone.borrow().to_lowercase();
                if query.is_empty() {
                    return true;
                }
                
                let url_match = res.url.to_lowercase().contains(&query);
                let title_match = res.title.as_ref().map_or(false, |t| t.to_lowercase().contains(&query));
                let desc_match = res.meta_desc.as_ref().map_or(false, |d| d.to_lowercase().contains(&query));
                let status_match = res.status_code.map_or(false, |c| c.to_string().contains(&query));
                
                url_match || title_match || desc_match || status_match
            } else {
                false
            }
        });
        self.filter_model.set_filter(Some(&custom_filter));
    }

    #[allow(dead_code)]
    pub fn get_selected_url(&self) -> Option<String> {
        let idx = self.selection_model.selected();
        if idx == gtk::INVALID_LIST_POSITION {
            return None;
        }
        if let Some(item) = self.selection_model.item(idx) {
            if let Some(row_data) = item.downcast_ref::<CrawlRowData>() {
                return row_data.get_result().map(|r| r.url);
            }
        }
        None
    }

    pub fn select_url(&self, url: &str) {
        let n = self.selection_model.n_items();
        for i in 0..n {
            if let Some(item) = self.selection_model.item(i) {
                if let Some(row_data) = item.downcast_ref::<CrawlRowData>() {
                    if let Some(res) = row_data.get_result() {
                        if res.url == url {
                            self.selection_model.set_selected(i);
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn export_to_csv(&self, filepath: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = std::fs::File::create(filepath)?;
        let mut wtr = csv::Writer::from_writer(file);
        
        // Headers matching details and containing Markdown details
        wtr.write_record(&[
            "Address", "Status Code", "Error Message", "Indexable", "Indexability",
            "Title", "Title Length", "Meta Description", "Meta Length",
            "H1", "H1 Count", "H2", "H2 Count", "Word Count", "Size (Bytes)", "Canonical",
            "Depth", "Response Time (ms)", "Inlinks", "Outlinks", "Images",
            "OG Title", "OG Description", "OG Image", "OG URL", "OG Type",
            "Twitter Title", "Twitter Description", "Twitter Image", "Twitter Card",
            "Schema Errors", "Headings", "Markdown", "Schema"
        ])?;
        
        let n = self.filter_model.n_items();
        for i in 0..n {
            if let Some(item) = self.filter_model.item(i) {
                if let Some(row_data) = item.downcast_ref::<CrawlRowData>() {
                    if let Some(res) = row_data.get_result() {
                        let title_str = res.title.clone().unwrap_or_default();
                        let title_len = title_str.chars().count().to_string();
                        let meta_str = res.meta_desc.clone().unwrap_or_default();
                        let meta_len = meta_str.chars().count().to_string();
                        
                        let headings_str = res.headings.iter()
                            .map(|h| format!("H{}: {}", h.level, h.text))
                            .collect::<Vec<String>>()
                            .join(" | ");
                            
                        let schema_str = res.schema_json_ld.join("\n");
                        let schema_errors_str = res.schema_errors.join("\n");
                        let markdown_str = res.markdown.clone().unwrap_or_default();
                        
                        let inlinks_str = res.inlinks.join("\n");
                        let outlinks_str = res.outlinks.join("\n");
                        
                        let images_str = res.images.iter()
                            .map(|img| {
                                if let Some(ref alt) = img.alt {
                                    format!("{} (alt: {})", img.src, alt)
                                } else {
                                    img.src.clone()
                                }
                            })
                            .collect::<Vec<String>>()
                            .join("\n");

                        wtr.write_record(&[
                            res.url,
                            res.status_code.map(|c| c.to_string()).unwrap_or_default(),
                            res.error_message.clone().unwrap_or_default(),
                            res.indexable.to_string(),
                            res.indexability_status,
                            title_str,
                            title_len,
                            meta_str,
                            meta_len,
                            res.h1.unwrap_or_default(),
                            res.h1_count.to_string(),
                            res.h2.unwrap_or_default(),
                            res.h2_count.to_string(),
                            res.word_count.to_string(),
                            res.size_bytes.to_string(),
                            res.canonical.unwrap_or_default(),
                            res.depth.to_string(),
                            res.response_time_ms.to_string(),
                            inlinks_str,
                            outlinks_str,
                            images_str,
                            res.og_title.unwrap_or_default(),
                            res.og_description.unwrap_or_default(),
                            res.og_image.unwrap_or_default(),
                            res.og_url.unwrap_or_default(),
                            res.og_type.unwrap_or_default(),
                            res.twitter_title.unwrap_or_default(),
                            res.twitter_description.unwrap_or_default(),
                            res.twitter_image.unwrap_or_default(),
                            res.twitter_card.unwrap_or_default(),
                            schema_errors_str,
                            headings_str,
                            markdown_str,
                            schema_str,
                        ])?;
                    }
                }
            }
        }
        wtr.flush()?;
        Ok(())
    }

    pub fn connect_selection_changed<F>(&self, callback: F)
    where
        F: Fn(Option<String>) + 'static,
    {
        let selection_model = self.selection_model.clone();
        selection_model.connect_selected_item_notify(move |model| {
            let item = model.selected_item();
            eprintln!("[Table Selection] Selected item changed: {:?}", item);
            if let Some(item) = item {
                if let Some(row_data) = item.downcast_ref::<CrawlRowData>() {
                    let url = row_data.get_result().map(|r| r.url);
                    eprintln!("[Table Selection] Found CrawlRowData, URL: {:?}", url);
                    callback(url);
                } else {
                    eprintln!("[Table Selection] Item failed to downcast to CrawlRowData");
                    callback(None);
                }
            } else {
                eprintln!("[Table Selection] No item selected, calling callback(None)");
                callback(None);
            }
        });
    }

    fn create_text_column<F>(title: &str, select_fn: F) -> gtk::ColumnViewColumn
    where
        F: Fn(&CrawlResult) -> String + 'static,
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
            let row_data = list_item.item().and_downcast::<CrawlRowData>().unwrap();
            if let Some(res) = row_data.get_result() {
                label.set_text(&select_fn(&res));
            }
        });

        gtk::ColumnViewColumn::new(Some(title), Some(factory))
    }
}

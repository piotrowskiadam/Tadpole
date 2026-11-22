use crate::state::CrawlState;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub use crate::filters::CrawlFilter;

#[derive(Clone)]
pub struct SidebarRow {
    pub filter: CrawlFilter,
    pub count_label: gtk::Label,
}

pub struct Sidebar {
    container: gtk::Box,
    rows: Rc<RefCell<Vec<SidebarRow>>>,
    list_boxes: Rc<RefCell<Vec<gtk::ListBox>>>,
    filter_positions: Rc<RefCell<Vec<(CrawlFilter, usize, i32)>>>,
}

impl Sidebar {
    pub fn new<F>(on_filter_changed: F) -> Self
    where
        F: Fn(CrawlFilter) + 'static,
    {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
        container.set_width_request(240);
        container.set_vexpand(true);

        // Sidebar title styled like a header
        let title_label = gtk::Label::new(Some("SEO Audit Filters"));
        title_label.add_css_class("title-4");
        title_label.set_halign(gtk::Align::Start);
        title_label.set_margin_top(15);
        title_label.set_margin_bottom(5);
        title_label.set_margin_start(15);
        container.append(&title_label);

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scrolled.set_propagate_natural_height(false);
        scrolled.set_min_content_height(120);
        scrolled.set_vexpand(true);
        container.append(&scrolled);

        let scroll_box = gtk::Box::new(gtk::Orientation::Vertical, 15);
        scroll_box.set_margin_bottom(15);
        scrolled.set_child(Some(&scroll_box));

        struct Category {
            name: &'static str,
            filters: Vec<CrawlFilter>,
        }

        let categories = vec![
            Category {
                name: "Overview",
                filters: vec![CrawlFilter::All],
            },
            Category {
                name: "Response Codes",
                filters: vec![
                    CrawlFilter::Status2xx,
                    CrawlFilter::Status3xx,
                    CrawlFilter::Status4xx,
                    CrawlFilter::Status5xx,
                ],
            },
            Category {
                name: "Page Titles",
                filters: vec![
                    CrawlFilter::TitleMissing,
                    CrawlFilter::TitleDuplicate,
                    CrawlFilter::TitleTooLong,
                    CrawlFilter::TitleTooShort,
                ],
            },
            Category {
                name: "Meta Descriptions",
                filters: vec![
                    CrawlFilter::DescMissing,
                    CrawlFilter::DescDuplicate,
                    CrawlFilter::DescTooLong,
                    CrawlFilter::DescTooShort,
                ],
            },
            Category {
                name: "Headings",
                filters: vec![
                    CrawlFilter::H1Missing,
                    CrawlFilter::H1Multiple,
                    CrawlFilter::H1Duplicate,
                    CrawlFilter::H2Missing,
                    CrawlFilter::H2Multiple,
                ],
            },
            Category {
                name: "Images",
                filters: vec![CrawlFilter::ImagesMissingAlt],
            },
            Category {
                name: "Canonicals",
                filters: vec![CrawlFilter::CanonicalMissing, CrawlFilter::CanonicalOther],
            },
            Category {
                name: "Social / OG",
                filters: vec![CrawlFilter::OgTitleMissing, CrawlFilter::OgDescMissing],
            },
            Category {
                name: "Structured Data",
                filters: vec![CrawlFilter::SchemaPresent, CrawlFilter::SchemaErrors],
            },
        ];

        let rows = Rc::new(RefCell::new(Vec::new()));
        let list_boxes = Rc::new(RefCell::new(Vec::new()));
        let filter_positions = Rc::new(RefCell::new(Vec::new()));
        let on_filter_changed = Rc::new(on_filter_changed);

        for (cat_idx, cat) in categories.into_iter().enumerate() {
            // Category header
            let cat_label = gtk::Label::builder()
                .label(cat.name)
                .halign(gtk::Align::Start)
                .margin_start(15)
                .margin_top(5)
                .margin_bottom(2)
                .build();
            cat_label.add_css_class("bold");
            cat_label.add_css_class("dim-label");
            scroll_box.append(&cat_label);

            let list_box = gtk::ListBox::new();
            list_box.add_css_class("navigation-sidebar");
            list_box.set_selection_mode(gtk::SelectionMode::Single);
            list_box.set_margin_start(10);
            list_box.set_margin_end(10);
            list_box.add_css_class("boxed-list");

            scroll_box.append(&list_box);
            list_boxes.borrow_mut().push(list_box.clone());

            let list_box_rows = Rc::new(RefCell::new(Vec::new()));

            for (row_idx, filter) in cat.filters.into_iter().enumerate() {
                let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                row_box.set_margin_start(10);
                row_box.set_margin_end(10);
                row_box.set_margin_top(6);
                row_box.set_margin_bottom(6);

                let label = gtk::Label::new(Some(filter.name()));
                label.set_halign(gtk::Align::Start);
                label.set_hexpand(true);
                row_box.append(&label);

                let count_label = gtk::Label::new(Some("0"));
                count_label.add_css_class("dim-label");
                count_label.add_css_class("numeric");
                row_box.append(&count_label);

                let list_row = gtk::ListBoxRow::new();
                list_row.set_child(Some(&row_box));
                list_box.append(&list_row);

                let sidebar_row = SidebarRow {
                    filter,
                    count_label,
                };
                list_box_rows.borrow_mut().push(sidebar_row.clone());
                rows.borrow_mut().push(sidebar_row);
                filter_positions
                    .borrow_mut()
                    .push((filter, cat_idx, row_idx as i32));
            }

            let list_boxes_clone = list_boxes.clone();
            let on_filter_changed_clone = on_filter_changed.clone();
            let list_box_rows_clone = list_box_rows.clone();
            list_box.connect_row_selected(move |lb, selected_row| {
                if let Some(row) = selected_row {
                    let index = row.index();
                    if index >= 0 && (index as usize) < list_box_rows_clone.borrow().len() {
                        // Unselect rows in all other list boxes
                        for other_lb in list_boxes_clone.borrow().iter() {
                            if other_lb != lb {
                                other_lb.select_row(None::<&gtk::ListBoxRow>);
                            }
                        }
                        let filter = list_box_rows_clone.borrow()[index as usize].filter;
                        on_filter_changed_clone(filter);
                    }
                }
            });
        }

        // Select the first row in the first listbox by default
        if let Some(first_lb) = list_boxes.borrow().first() {
            if let Some(first_row) = first_lb.row_at_index(0) {
                first_lb.select_row(Some(&first_row));
            }
        }

        Self {
            container,
            rows,
            list_boxes,
            filter_positions,
        }
    }

    pub fn select_filter(&self, target: CrawlFilter) -> bool {
        let found = self
            .filter_positions
            .borrow()
            .iter()
            .find(|(filter, _, _)| *filter == target)
            .copied();
        let Some((_, target_lb_idx, target_row_idx)) = found else {
            return false;
        };
        let list_boxes = self.list_boxes.borrow();
        for (i, lb) in list_boxes.iter().enumerate() {
            if i == target_lb_idx {
                if let Some(row) = lb.row_at_index(target_row_idx) {
                    lb.select_row(Some(&row));
                }
            } else {
                lb.select_row(None::<&gtk::ListBoxRow>);
            }
        }
        true
    }

    pub fn select_all(&self) {
        let _ = self.select_filter(CrawlFilter::All);
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn update_counts(&self, state: &CrawlState) {
        let results = state.get_all_results();
        let duplicates = crate::filters::DuplicateIndex::from_results(&results);
        for row in self.rows.borrow().iter() {
            let mut count = 0;
            for res in &results {
                if duplicates.matches(row.filter, res) {
                    count += 1;
                }
            }
            row.count_label.set_text(&count.to_string());
        }
    }
}

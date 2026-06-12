use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use crate::state::{CrawlResult, CrawlState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlFilter {
    All,
    Status2xx,
    Status3xx,
    Status4xx,
    Status5xx,
    TitleMissing,
    TitleDuplicate,
    TitleTooLong,
    TitleTooShort,
    DescMissing,
    DescDuplicate,
    DescTooLong,
    DescTooShort,
    H1Missing,
    H1Multiple,
    H1Duplicate,
    H2Missing,
    H2Multiple,
    ImagesMissingAlt,
    CanonicalMissing,
    CanonicalOther,
    OgTitleMissing,
    OgDescMissing,
    SchemaPresent,
    SchemaErrors,
}

impl CrawlFilter {
    pub fn name(&self) -> &'static str {
        match self {
            Self::All => "All URLs",
            Self::Status2xx => "Success (2xx)",
            Self::Status3xx => "Redirection (3xx)",
            Self::Status4xx => "Client Error (4xx)",
            Self::Status5xx => "Server Error (5xx)",
            Self::TitleMissing => "Missing Title",
            Self::TitleDuplicate => "Duplicate Title",
            Self::TitleTooLong => "Title > 60 chars",
            Self::TitleTooShort => "Title < 30 chars",
            Self::DescMissing => "Missing Description",
            Self::DescDuplicate => "Duplicate Description",
            Self::DescTooLong => "Description > 155 chars",
            Self::DescTooShort => "Description < 70 chars",
            Self::H1Missing => "Missing H1",
            Self::H1Multiple => "Multiple H1s",
            Self::H1Duplicate => "Duplicate H1",
            Self::H2Missing => "Missing H2",
            Self::H2Multiple => "Multiple H2s",
            Self::ImagesMissingAlt => "Images Missing Alt Text",
            Self::CanonicalMissing => "Missing Canonical",
            Self::CanonicalOther => "Canonicalised",
            Self::OgTitleMissing => "Missing og:title",
            Self::OgDescMissing => "Missing og:description",
            Self::SchemaPresent => "Structured Data Present",
            Self::SchemaErrors => "Structured Data Errors",
        }
    }

    pub fn matches(&self, res: &CrawlResult, all_results: &[CrawlResult]) -> bool {
        match self {
            Self::All => true,
            Self::Status2xx => res.status_code.map_or(false, |c| (200..300).contains(&c)),
            Self::Status3xx => res.status_code.map_or(false, |c| (300..400).contains(&c)),
            Self::Status4xx => res.status_code.map_or(false, |c| (400..500).contains(&c)),
            Self::Status5xx => res.status_code.map_or(false, |c| (500..600).contains(&c)),
            Self::TitleMissing => res.title.is_none() || res.title.as_ref().unwrap().is_empty(),
            Self::TitleDuplicate => {
                if let Some(ref t) = res.title {
                    !t.is_empty() && all_results.iter().filter(|r| r.title.as_ref() == Some(t)).count() > 1
                } else {
                    false
                }
            }
            Self::TitleTooLong => res.title.as_ref().map_or(false, |t| t.chars().count() > 60),
            Self::TitleTooShort => res.title.as_ref().map_or(false, |t| !t.is_empty() && t.chars().count() < 30),
            Self::DescMissing => res.meta_desc.is_none() || res.meta_desc.as_ref().unwrap().is_empty(),
            Self::DescDuplicate => {
                if let Some(ref d) = res.meta_desc {
                    !d.is_empty() && all_results.iter().filter(|r| r.meta_desc.as_ref() == Some(d)).count() > 1
                } else {
                    false
                }
            }
            Self::DescTooLong => res.meta_desc.as_ref().map_or(false, |d| d.chars().count() > 155),
            Self::DescTooShort => res.meta_desc.as_ref().map_or(false, |d| !d.is_empty() && d.chars().count() < 70),
            Self::H1Missing => res.h1.is_none() || res.h1.as_ref().unwrap().is_empty(),
            Self::H1Multiple => res.h1_count > 1,
            Self::H1Duplicate => {
                if let Some(ref h) = res.h1 {
                    !h.is_empty() && all_results.iter().filter(|r| r.h1.as_ref() == Some(h)).count() > 1
                } else {
                    false
                }
            }
            Self::H2Missing => res.h2.is_none() || res.h2.as_ref().unwrap().is_empty(),
            Self::H2Multiple => res.h2_count > 1,
            Self::ImagesMissingAlt => res.images.iter().any(|img| img.alt.is_none() || img.alt.as_ref().unwrap().is_empty()),
            Self::CanonicalMissing => res.canonical.is_none(),
            Self::CanonicalOther => res.canonical.is_some() && res.canonical.as_ref() != Some(&res.url),
            Self::OgTitleMissing => res.og_title.is_none() || res.og_title.as_ref().unwrap().trim().is_empty(),
            Self::OgDescMissing => res.og_description.is_none() || res.og_description.as_ref().unwrap().trim().is_empty(),
            Self::SchemaPresent => !res.schema_json_ld.is_empty(),
            Self::SchemaErrors => !res.schema_errors.is_empty(),
        }
    }
}

#[derive(Clone)]
pub struct SidebarRow {
    pub filter: CrawlFilter,
    pub count_label: gtk::Label,
}

pub struct Sidebar {
    container: gtk::Box,
    rows: Rc<RefCell<Vec<SidebarRow>>>,
}

impl Sidebar {
    pub fn new<F>(on_filter_changed: F) -> Self
    where
        F: Fn(CrawlFilter) + 'static,
    {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
        container.set_width_request(240);

        // Sidebar title styled like a header
        let title_label = gtk::Label::new(Some("SEO Audit Filters"));
        title_label.add_css_class("title-4");
        title_label.set_halign(gtk::Align::Start);
        title_label.set_margin_top(15);
        title_label.set_margin_bottom(5);
        title_label.set_margin_start(15);
        container.append(&title_label);

        let scrolled = gtk::ScrolledWindow::new();
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
        let on_filter_changed = Rc::new(on_filter_changed);

        for cat in categories {
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

            for filter in cat.filters {
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
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.container
    }

    pub fn update_counts(&self, state: &CrawlState) {
        let results = state.get_all_results();
        for row in self.rows.borrow().iter() {
            let mut count = 0;
            for res in &results {
                if row.filter.matches(res, &results) {
                    count += 1;
                }
            }
            row.count_label.set_text(&count.to_string());
        }
    }
}

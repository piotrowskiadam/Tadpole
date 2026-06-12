use glib::Object;
use glib::subclass::prelude::*;
use crate::state::CrawlResult;

glib::wrapper! {
    pub struct CrawlRowData(ObjectSubclass<imp::CrawlRowData>);
}

impl CrawlRowData {
    pub fn new(result: CrawlResult) -> Self {
        let obj: Self = Object::builder().build();
        *obj.imp().result.borrow_mut() = Some(result);
        obj
    }

    pub fn get_result(&self) -> Option<CrawlResult> {
        self.imp().result.borrow().clone()
    }

    pub fn set_result(&self, res: CrawlResult) {
        *self.imp().result.borrow_mut() = Some(res);
    }
}

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct CrawlRowData {
        pub result: RefCell<Option<CrawlResult>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CrawlRowData {
        const NAME: &'static str = "CrawlRowData";
        type Type = super::CrawlRowData;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for CrawlRowData {}
}

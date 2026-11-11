//! Small virtualized, wrapping tables for page inspection.
use gtk::prelude::*;
use std::rc::Rc;

pub type CellRenderer = Rc<dyn Fn(&glib::BoxedAnyObject) -> gtk::Widget>;

pub struct DetailTable {
    pub view: gtk::ColumnView,
    pub store: gio::ListStore,
    pub scroll: gtk::Overlay,
    empty: gtk::Label,
}

impl DetailTable {
    pub fn new(columns: Vec<(&str, Option<i32>, CellRenderer)>) -> Self {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let selection = gtk::SingleSelection::new(Some(store.clone()));
        let view = gtk::ColumnView::new(Some(selection));
        view.add_css_class("data-table");
        view.set_show_column_separators(true);
        view.set_show_row_separators(true);
        for (title, width, render) in columns {
            let factory = gtk::SignalListItemFactory::new();
            factory.connect_bind(move |_, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                let data = item.item().and_downcast::<glib::BoxedAnyObject>().unwrap();
                item.set_child(Some(&render(&data)));
            });
            // Release images and abort pending preview work when cells are recycled.
            factory.connect_unbind(|_, item| {
                item.downcast_ref::<gtk::ListItem>()
                    .unwrap()
                    .set_child(None::<&gtk::Widget>);
            });
            let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
            if let Some(width) = width {
                column.set_fixed_width(width);
            } else {
                column.set_expand(true);
            }
            view.append_column(&column);
        }
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .hexpand(true)
            .child(&view)
            .build();
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scroll));
        let empty = wrapped_label("No records for this page.");
        empty.set_halign(gtk::Align::Fill);
        empty.set_xalign(0.5);
        empty.set_valign(gtk::Align::Center);
        empty.add_css_class("dim-label");
        overlay.add_overlay(&empty);
        let weak = empty.downgrade();
        store.connect_items_changed(move |store, _, _, _| {
            if let Some(empty) = weak.upgrade() {
                empty.set_visible(store.n_items() == 0);
            }
        });
        Self {
            view,
            store,
            scroll: overlay,
            empty,
        }
    }

    pub fn set_empty_message(&self, text: &str) {
        self.empty.set_text(text);
    }

    pub fn replace<T: 'static>(&self, rows: Vec<T>) {
        self.store.remove_all();
        let rows: Vec<_> = rows.into_iter().map(glib::BoxedAnyObject::new).collect();
        self.store.extend_from_slice(&rows);
    }
}

pub fn wrapped_label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(0.)
        .yalign(0.)
        .wrap(true)
        .wrap_mode(gtk::pango::WrapMode::WordChar)
        .width_chars(1)
        .max_width_chars(1)
        .hexpand(true)
        .selectable(true)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(8)
        .margin_end(8)
        .build()
}

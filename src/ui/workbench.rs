//! A separate workspace for keyword research and editorial briefs; crawling is not required.
use crate::{
    integrations::{SeoIntegrations, SerpReport, brief_prompt, page_context},
    state::CrawlState,
};
use adw::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub fn present(parent: &adw::ApplicationWindow, state: CrawlState, selected_url: Option<String>) {
    let window = adw::Window::builder()
        .title("SEO Studio — Tadpole")
        .transient_for(parent)
        .default_width(900)
        .default_height(760)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&adw::HeaderBar::new());
    window.set_content(Some(&root));
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    root.append(&content);
    let title = gtk::Label::builder()
        .label("Research a topic. Build a better brief.")
        .xalign(0.0)
        .css_classes(["title-2"])
        .wrap(true)
        .build();
    content.append(&title);
    let intro = gtk::Label::builder().label("Fetch Google results with DataForSEO, then generate an editorial brief with your AI provider. Each button sends one paid API request. Configure keys in Preferences.")
        .wrap(true).xalign(0.0).css_classes(["dim-label"]).build();
    content.append(&intro);
    let keyword = gtk::Entry::builder()
        .placeholder_text("Topic or target keyword")
        .hexpand(true)
        .build();
    keyword.update_property(&[gtk::accessible::Property::Label("Topic or target keyword")]);
    content.append(&keyword);
    let settings = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let market = gtk::DropDown::from_strings(&[
        "Poland · Polish",
        "United States · English",
        "United Kingdom · English",
    ]);
    market.update_property(&[gtk::accessible::Property::Label(
        "Google market and language",
    )]);
    settings.append(&gtk::Label::new(Some("Google market")));
    settings.append(&market);
    content.append(&settings);
    let page = selected_url.and_then(|url| state.get_result(&url));
    let context = page.as_ref().map(page_context).unwrap_or_default();
    let include_page =
        gtk::CheckButton::with_label("Include selected page content in the AI brief");
    include_page.set_active(page.is_some());
    include_page.set_sensitive(page.is_some());
    content.append(&include_page);
    let source = gtk::Label::builder()
        .label(
            page.as_ref()
                .map(|p| p.url.as_str())
                .unwrap_or("No page selected. You can create a brief without running a crawl."),
        )
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .css_classes(["dim-label"])
        .build();
    content.append(&source);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let fetch = gtk::Button::with_label("Fetch Google SERP");
    let generate = gtk::Button::with_label("Generate content brief");
    generate.add_css_class("suggested-action");
    actions.append(&fetch);
    actions.append(&generate);
    content.append(&actions);
    let status = gtk::Label::builder()
        .label("Ready. No data is sent until you choose an action.")
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    content.append(&status);
    let reports = gtk::Notebook::new();
    reports.set_vexpand(true);
    let serp_view = report_view(
        "Your Google results will appear here. Fetch SERP to see competing pages and snippets.",
    );
    let brief_view = report_view(
        "Your content brief will appear here. You can generate one with or without SERP evidence.",
    );
    reports.append_page(
        &scroll(&serp_view),
        Some(&gtk::Label::new(Some("SERP evidence"))),
    );
    reports.append_page(
        &scroll(&brief_view),
        Some(&gtk::Label::new(Some("Content brief"))),
    );
    content.append(&reports);
    let export_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let copy = gtk::Button::with_label("Copy report");
    let save = gtk::Button::with_label("Save Markdown…");
    copy.set_sensitive(false);
    save.set_sensitive(false);
    export_actions.append(&copy);
    export_actions.append(&save);
    content.append(&export_actions);
    let serp: Rc<RefCell<Option<SerpReport>>> = Rc::default();
    let available = Rc::new(RefCell::new([false, false]));
    {
        let available = available.clone();
        let copy = copy.clone();
        let save = save.clone();
        reports.connect_switch_page(move |_, _, index| {
            let ready = available.borrow()[index as usize];
            copy.set_sensitive(ready);
            save.set_sensitive(ready);
        });
    }
    // A changed keyword/market invalidates evidence; an old report is never fed to a new topic.
    {
        let serp = serp.clone();
        let view = serp_view.clone();
        let available = available.clone();
        let reports = reports.clone();
        let copy = copy.clone();
        let save = save.clone();
        keyword.connect_changed(move |_| {
            *serp.borrow_mut() = None;
            available.borrow_mut()[0] = false;
            view.buffer()
                .set_text("Topic changed. Fetch SERP again to use current evidence.");
            if reports.current_page() == Some(0) {
                copy.set_sensitive(false);
                save.set_sensitive(false);
            }
        });
    }
    {
        let serp = serp.clone();
        let view = serp_view.clone();
        let available = available.clone();
        let reports = reports.clone();
        let copy = copy.clone();
        let save = save.clone();
        market.connect_selected_notify(move |_| {
            *serp.borrow_mut() = None;
            available.borrow_mut()[0] = false;
            view.buffer()
                .set_text("Market changed. Fetch SERP again to use current evidence.");
            if reports.current_page() == Some(0) {
                copy.set_sensitive(false);
                save.set_sensitive(false);
            }
        });
    }
    for (button, is_serp) in [(fetch.clone(), true), (generate.clone(), false)] {
        let state = state.clone();
        let keyword = keyword.clone();
        let market = market.clone();
        let fetch = fetch.clone();
        let generate = generate.clone();
        let status = status.clone();
        let serp = serp.clone();
        let reports = reports.clone();
        let serp_view = serp_view.clone();
        let brief_view = brief_view.clone();
        let context = context.clone();
        let include_page = include_page.clone();
        let available = available.clone();
        let copy = copy.clone();
        let save = save.clone();
        button.connect_clicked(move |_| {
            let topic = keyword.text().trim().to_string();
            if topic.is_empty() { status.set_text("Enter a topic or keyword first."); keyword.grab_focus(); return; }
            let config = state.get_config();
            let (location, language, output_language) = match market.selected() { 1 => (2840, "en", "English"), 2 => (2826, "en", "English"), _ => (2616, "pl", "Polish") };
            let prompt = brief_prompt(&topic, output_language, if include_page.is_active() { &context } else { "" }, serp.borrow().as_ref());
            let Ok(client) = SeoIntegrations::new() else { status.set_text("Unable to initialize API client."); return };
            fetch.set_sensitive(false); generate.set_sensitive(false); keyword.set_sensitive(false); market.set_sensitive(false);
            status.remove_css_class("error");
            status.set_text(if is_serp { "Fetching Google SERP… One DataForSEO task, depth 10." } else { "Generating brief… Sending topic, optional page content and SERP evidence to your AI provider." });
            let fetch = fetch.clone(); let generate = generate.clone(); let keyword = keyword.clone(); let market = market.clone();
            let status = status.clone(); let serp = serp.clone(); let reports = reports.clone(); let serp_view = serp_view.clone(); let brief_view = brief_view.clone(); let available = available.clone(); let copy = copy.clone(); let save = save.clone();
            glib::MainContext::default().spawn_local(async move {
                let result = if is_serp {
                    match client.serp(&config, &topic, location, language).await {
                        Ok(report) => { let text = report.markdown(); *serp.borrow_mut() = Some(report); Ok(text) },
                        Err(e) => Err(e),
                    }
                } else { client.complete(&config, &prompt).await };
                fetch.set_sensitive(true); generate.set_sensitive(true); keyword.set_sensitive(true); market.set_sensitive(true);
                match result {
                    Ok(text) => {
                        let index = if is_serp { 0 } else { 1 };
                        if is_serp { serp_view.buffer().set_text(&text); } else { brief_view.buffer().set_text(&text); }
                        available.borrow_mut()[index] = true;
                        reports.set_current_page(Some(index as u32)); copy.set_sensitive(true); save.set_sensitive(true);
                        status.set_text(if is_serp { "SERP ready. Review the evidence, then generate a brief when you are ready." } else { "Brief ready. Review suggestions and fact-check before publishing." });
                    }
                    Err(e) => { status.add_css_class("error"); status.set_text(&format!("Request failed: {e}")); }
                }
            });
        });
    }
    {
        let reports = reports.clone();
        let a = serp_view.clone();
        let b = brief_view.clone();
        let status = status.clone();
        copy.connect_clicked(move |_| {
            if let Some(display) = gdk::Display::default() {
                let view = if reports.current_page() == Some(0) {
                    &a
                } else {
                    &b
                };
                let buffer = view.buffer();
                display.clipboard().set_text(&buffer.text(
                    &buffer.start_iter(),
                    &buffer.end_iter(),
                    false,
                ));
                status.set_text("Report copied to clipboard.");
            }
        });
    }
    {
        let window = window.clone();
        let reports = reports.clone();
        let status = status.clone();
        save.connect_clicked(move |_| {
            let view = if reports.current_page() == Some(0) {
                &serp_view
            } else {
                &brief_view
            };
            let buffer = view.buffer();
            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();
            let dialog = gtk::FileDialog::builder()
                .title("Save report")
                .initial_name("seo-report.md")
                .build();
            let status = status.clone();
            dialog.save(Some(&window), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let status = status.clone();
                        glib::MainContext::default().spawn_local(async move {
                            match tokio::fs::write(path, text).await {
                                Ok(_) => status.set_text("Report saved."),
                                Err(e) => status.set_text(&format!("Could not save report: {e}")),
                            }
                        });
                    }
                }
            });
        });
    }
    window.present();
}

fn report_view(empty: &str) -> gtk::TextView {
    let view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(16)
        .right_margin(16)
        .top_margin(16)
        .bottom_margin(16)
        .build();
    view.buffer().set_text(empty);
    view
}
fn scroll(view: &gtk::TextView) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .child(view)
        .vexpand(true)
        .hexpand(true)
        .min_content_height(180)
        .build()
}

use gtk::prelude::*;
use crate::state::CrawlState;
use crate::ui::sidebar::CrawlFilter;

pub struct SummaryPanel {
    container: gtk::ScrolledWindow,

    // Crawl progress
    lbl_crawled: gtk::Label,
    progress_bar: gtk::ProgressBar,

    // Health score
    lbl_score: gtk::Label,
    lbl_score_status: gtk::Label,
    score_bar: gtk::ProgressBar,

    // Performance averages
    lbl_avg_response: gtk::Label,
    lbl_avg_size: gtk::Label,
    lbl_avg_words: gtk::Label,

    // Depth distribution (drawn via Cairo)
    depth_drawing: gtk::DrawingArea,

    // Top issues (as colored rows)
    top_issues_box: gtk::Box,
}

fn section_label(text: &str) -> gtk::Label {
    let lbl = gtk::Label::new(Some(text));
    lbl.add_css_class("heading");
    lbl.set_halign(gtk::Align::Start);
    lbl
}

fn stat_row(name: &str) -> (gtk::ListBoxRow, gtk::Label) {
    let row = gtk::ListBoxRow::new();
    row.set_selectable(false);
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    b.set_margin_start(12); b.set_margin_end(12);
    b.set_margin_top(7);    b.set_margin_bottom(7);
    let name_lbl = gtk::Label::new(Some(name));
    name_lbl.set_halign(gtk::Align::Start);
    name_lbl.set_hexpand(true);
    b.append(&name_lbl);
    let val_lbl = gtk::Label::new(Some("—"));
    val_lbl.add_css_class("dim-label");
    val_lbl.add_css_class("numeric");
    b.append(&val_lbl);
    row.set_child(Some(&b));
    (row, val_lbl)
}

impl SummaryPanel {
    pub fn new() -> Self {
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(false);
        scrolled.set_width_request(270);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 14);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);
        vbox.set_margin_top(14);
        vbox.set_margin_bottom(14);
        scrolled.set_child(Some(&vbox));

        // ── Title ─────────────────────────────────────────────────────────
        let title = gtk::Label::new(Some("SEO Dashboard"));
        title.add_css_class("title-4");
        title.set_halign(gtk::Align::Start);
        vbox.append(&title);

        // ── Progress ───────────────────────────────────────────────────────
        let progress_bar = gtk::ProgressBar::new();
        progress_bar.set_fraction(0.0);
        progress_bar.set_show_text(true);
        progress_bar.set_text(Some("No crawl in progress"));
        vbox.append(&progress_bar);

        let lbl_crawled = gtk::Label::new(Some("0 pages crawled"));
        lbl_crawled.add_css_class("dim-label");
        lbl_crawled.set_halign(gtk::Align::Center);
        vbox.append(&lbl_crawled);

        vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // ── Health Score ───────────────────────────────────────────────────
        vbox.append(&section_label("SEO Health Score"));

        let score_card = gtk::Box::new(gtk::Orientation::Vertical, 4);

        let score_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        score_row.set_halign(gtk::Align::Center);
        let lbl_score = gtk::Label::new(Some("—"));
        lbl_score.add_css_class("title-1");
        score_row.append(&lbl_score);
        let lbl_score_status = gtk::Label::new(Some(""));
        lbl_score_status.set_valign(gtk::Align::End);
        lbl_score_status.set_margin_bottom(6);
        score_row.append(&lbl_score_status);
        score_card.append(&score_row);

        let score_bar = gtk::ProgressBar::new();
        score_bar.set_fraction(0.0);
        score_card.append(&score_bar);
        vbox.append(&score_card);

        vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // ── Performance Averages ───────────────────────────────────────────
        vbox.append(&section_label("Performance Averages"));

        let perf_list = gtk::ListBox::new();
        perf_list.add_css_class("boxed-list");

        let (row_ar, lbl_avg_response) = stat_row("Avg. Response Time");
        perf_list.append(&row_ar);
        let (row_as, lbl_avg_size) = stat_row("Avg. Page Size");
        perf_list.append(&row_as);
        let (row_aw, lbl_avg_words) = stat_row("Avg. Word Count");
        perf_list.append(&row_aw);
        vbox.append(&perf_list);

        vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // ── Crawl Depth Distribution ───────────────────────────────────────
        vbox.append(&section_label("Crawl Depth Distribution"));

        let depth_drawing = gtk::DrawingArea::new();
        depth_drawing.set_height_request(120);
        depth_drawing.set_hexpand(true);
        vbox.append(&depth_drawing);

        vbox.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // ── Top Issues ─────────────────────────────────────────────────────
        vbox.append(&section_label("Top Issues"));
        let top_issues_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        vbox.append(&top_issues_box);

        Self {
            container: scrolled,
            lbl_crawled,
            progress_bar,
            lbl_score,
            lbl_score_status,
            score_bar,
            lbl_avg_response,
            lbl_avg_size,
            lbl_avg_words,
            depth_drawing,
            top_issues_box,
        }
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.container
    }

    pub fn update(&self, state: &CrawlState) {
        let stats = state.get_stats();
        let results = state.get_all_results();
        let n = results.len();

        // ── Progress ───────────────────────────────────────────────────────
        let limit = state.get_limit();
        let disc = stats.discovered;
        let craw = stats.crawled;
        let total_target = if limit > 0 { limit.min(disc).max(1) } else { disc.max(1) };
        let frac = ((craw as f64) / (total_target as f64)).min(1.0).max(0.0);
        self.progress_bar.set_fraction(frac);
        self.progress_bar.set_text(Some(&format!("{:.0}%", frac * 100.0)));
        self.lbl_crawled.set_text(&format!("{} pages crawled / {} discovered", craw, disc));

        if n == 0 {
            self.lbl_score.set_text("—");
            self.lbl_score_status.set_text("");
            self.score_bar.set_fraction(0.0);
            self.lbl_avg_response.set_text("—");
            self.lbl_avg_size.set_text("—");
            self.lbl_avg_words.set_text("—");
            // clear top issues
            while let Some(child) = self.top_issues_box.first_child() {
                self.top_issues_box.remove(&child);
            }
            return;
        }

        // ── Health Score ───────────────────────────────────────────────────
        let pct = |count: usize| (count as f64) / (n as f64);

        let missing_title   = results.iter().filter(|r| CrawlFilter::TitleMissing.matches(r, &results)).count();
        let dup_title       = results.iter().filter(|r| CrawlFilter::TitleDuplicate.matches(r, &results)).count();
        let missing_desc    = results.iter().filter(|r| CrawlFilter::DescMissing.matches(r, &results)).count();
        let missing_h1      = results.iter().filter(|r| CrawlFilter::H1Missing.matches(r, &results)).count();
        let images_no_alt   = results.iter().filter(|r| CrawlFilter::ImagesMissingAlt.matches(r, &results)).count();
        let canon_missing   = results.iter().filter(|r| CrawlFilter::CanonicalMissing.matches(r, &results)).count();
        let schema_errors   = results.iter().filter(|r| CrawlFilter::SchemaErrors.matches(r, &results)).count();
        let err_4xx         = stats.status_4xx;
        let err_5xx         = stats.status_5xx;

        let mut score: f64 = 100.0;
        score -= pct(missing_title)   * 20.0;
        score -= pct(dup_title)       * 5.0;
        score -= pct(missing_desc)    * 15.0;
        score -= pct(missing_h1)      * 10.0;
        score -= pct(images_no_alt)   * 5.0;
        score -= pct(canon_missing)   * 5.0;
        score -= pct(schema_errors)   * 3.0;
        score -= pct(err_4xx)         * 10.0;
        score -= pct(err_5xx)         * 10.0;
        let score = score.max(0.0).min(100.0);

        let score_int = score.round() as u32;
        self.lbl_score.set_text(&format!("{}", score_int));
        let (status_text, css_class) = match score_int {
            80..=100 => ("Good", "success"),
            50..=79  => ("Needs Work", "warning"),
            _        => ("Poor", "error"),
        };
        self.lbl_score_status.set_text(status_text);
        // clear old classes
        for cls in ["success", "warning", "error"] {
            self.lbl_score_status.remove_css_class(cls);
            self.lbl_score.remove_css_class(cls);
        }
        self.lbl_score_status.add_css_class(css_class);
        self.lbl_score.add_css_class(css_class);
        self.score_bar.set_fraction(score / 100.0);

        // Color the score bar using CSS
        for cls in ["score-good", "score-warn", "score-poor"] {
            self.score_bar.remove_css_class(cls);
        }
        match score_int {
            80..=100 => self.score_bar.add_css_class("score-good"),
            50..=79  => self.score_bar.add_css_class("score-warn"),
            _        => self.score_bar.add_css_class("score-poor"),
        }

        // ── Performance Averages ───────────────────────────────────────────
        let avg_resp = results.iter().map(|r| r.response_time_ms as u64).sum::<u64>() / n as u64;
        let avg_size = results.iter().map(|r| r.size_bytes as u64).sum::<u64>() / n as u64;
        let avg_words = results.iter().map(|r| r.word_count as u64).sum::<u64>() / n as u64;

        self.lbl_avg_response.set_text(&format!("{} ms", avg_resp));
        self.lbl_avg_size.set_text(&Self::format_bytes(avg_size));
        self.lbl_avg_words.set_text(&format!("{} words", avg_words));

        // ── Depth Distribution ─────────────────────────────────────────────
        // Pass depth counts to the drawing area via a shared Vec via draw function
        let mut depth_counts: Vec<usize> = vec![0usize; 6]; // 0..5+
        for r in &results {
            let d = r.depth.min(5);
            depth_counts[d] += 1;
        }
        let depth_counts_clone = depth_counts.clone();
        self.depth_drawing.set_draw_func(move |_widget, cr, width, height| {
            let max_count = *depth_counts_clone.iter().max().unwrap_or(&1).max(&1) as f64;
            let n_bars = depth_counts_clone.len();
            let bar_gap = 4.0_f64;
            let bar_w = ((width as f64) - bar_gap * (n_bars as f64 + 1.0)) / n_bars as f64;
            let label_h = 18.0_f64;
            let chart_h = height as f64 - label_h;

            // Background
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.paint().ok();

            for (i, &count) in depth_counts_clone.iter().enumerate() {
                let x = bar_gap + i as f64 * (bar_w + bar_gap);
                let bar_h = if count == 0 { 2.0 } else { (count as f64 / max_count) * (chart_h - 8.0) };
                let y = chart_h - bar_h;

                // Bar fill — use accent-ish blue
                cr.set_source_rgba(0.22, 0.55, 0.88, 0.85);
                cr.rectangle(x, y, bar_w, bar_h);
                cr.fill().ok();

                // Label: depth number
                cr.set_source_rgba(0.5, 0.5, 0.5, 1.0);
                cr.move_to(x + bar_w / 2.0 - 6.0, height as f64 - 2.0);
                cr.set_font_size(11.0);
                cr.show_text(&format!("D{}", i)).ok();

                // Count label above bar
                if count > 0 {
                    cr.set_source_rgba(0.3, 0.3, 0.3, 1.0);
                    cr.move_to(x + bar_w / 2.0 - 6.0, y - 3.0);
                    cr.set_font_size(10.0);
                    cr.show_text(&count.to_string()).ok();
                }
            }
        });
        self.depth_drawing.queue_draw();

        // ── Top Issues ─────────────────────────────────────────────────────
        while let Some(child) = self.top_issues_box.first_child() {
            self.top_issues_box.remove(&child);
        }

        let mut issues: Vec<(&str, usize, &str)> = vec![
            ("Missing Titles",       missing_title,   "error"),
            ("Missing Descriptions", missing_desc,    "warning"),
            ("Missing H1",           missing_h1,      "warning"),
            ("Images w/o Alt",       images_no_alt,   "warning"),
            ("4xx Errors",           err_4xx,         "error"),
            ("5xx Errors",           err_5xx,         "error"),
        ];
        issues.retain(|(_, count, _)| *count > 0);
        issues.sort_by(|a, b| b.1.cmp(&a.1));
        issues.truncate(5);

        if issues.is_empty() {
            let ok_lbl = gtk::Label::new(Some("✓  No major issues found"));
            ok_lbl.add_css_class("success");
            ok_lbl.set_halign(gtk::Align::Start);
            self.top_issues_box.append(&ok_lbl);
        } else {
            for (name, count, css) in issues {
                let chip = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                chip.set_margin_top(2);
                chip.set_margin_bottom(2);

                let dot = gtk::Label::new(Some("●"));
                dot.add_css_class(css);
                chip.append(&dot);

                let name_lbl = gtk::Label::new(Some(name));
                name_lbl.set_halign(gtk::Align::Start);
                name_lbl.set_hexpand(true);
                chip.append(&name_lbl);

                let count_lbl = gtk::Label::new(Some(&count.to_string()));
                count_lbl.add_css_class("dim-label");
                count_lbl.add_css_class("numeric");
                chip.append(&count_lbl);

                self.top_issues_box.append(&chip);
            }
        }
    }

    fn format_bytes(bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    }
}

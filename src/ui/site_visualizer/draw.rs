use crate::visualization::{NodeId, explorer::*};
use gtk::prelude::*;

fn rgb(c: &gdk::RGBA) -> (f64, f64, f64) {
    (c.red() as f64, c.green() as f64, c.blue() as f64)
}
fn color(cr: &cairo::Context, c: (f64, f64, f64)) {
    cr.set_source_rgb(c.0, c.1, c.2);
}
fn mix(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    (
        a.0 * t + b.0 * (1. - t),
        a.1 * t + b.1 * (1. - t),
        a.2 * t + b.2 * (1. - t),
    )
}
fn text(cr: &cairo::Context, x: f64, y: f64, s: &str, max: f64, size: f64) {
    if max < 8. {
        return;
    }
    cr.set_font_size(size);
    let chars: Vec<_> = s.chars().filter(|c| !c.is_control()).take(240).collect();
    let mut value: String = chars.iter().collect();
    if cr.text_extents(&value).is_ok_and(|e| e.x_advance() > max) || s.chars().count() > 240 {
        let (mut lo, mut hi) = (0, chars.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            let test = format!("{}…", chars[..mid].iter().collect::<String>());
            if cr.text_extents(&test).is_ok_and(|e| e.x_advance() <= max) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        value = format!("{}…", chars[..lo].iter().collect::<String>());
    }
    cr.move_to(x, y);
    let _ = cr.show_text(&value);
}
fn rounded(cr: &cairo::Context, b: Bounds, r: f64) {
    let r = r.min(b.w / 2.).min(b.h / 2.).max(0.);
    cr.new_sub_path();
    cr.arc(b.x + b.w - r, b.y + r, r, -std::f64::consts::FRAC_PI_2, 0.);
    cr.arc(
        b.x + b.w - r,
        b.y + b.h - r,
        r,
        0.,
        std::f64::consts::FRAC_PI_2,
    );
    cr.arc(
        b.x + r,
        b.y + b.h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        b.x + r,
        b.y + r,
        r,
        std::f64::consts::PI,
        1.5 * std::f64::consts::PI,
    );
    cr.close_path();
}
fn edge(cr: &cairo::Context, e: &Edge) {
    let (x1, y1) = e.from;
    let (x2, y2) = e.to;
    cr.move_to(x1, y1);
    if e.horizontal {
        let m = e.lane;
        let sign = if y2 > y1 { 1. } else { -1. };
        let r = ((y2 - y1).abs() / 2.).min(8.);
        cr.line_to(m - r, y1);
        cr.curve_to(m, y1, m, y1, m, y1 + sign * r);
        cr.line_to(m, y2 - sign * r);
        cr.curve_to(m, y2, m, y2, m + r, y2);
        cr.line_to(x2, y2);
    } else {
        let m = e.lane;
        let sign = if x2 > x1 { 1. } else { -1. };
        let r = ((x2 - x1).abs() / 2.).min(8.);
        cr.line_to(x1, m - 8.);
        cr.curve_to(x1, m, x1, m, x1 + sign * r, m);
        cr.line_to(x2 - sign * r, m);
        cr.curve_to(x2, m, x2, m, x2, m + 8.);
        cr.line_to(x2, y2);
    }
    let _ = cr.stroke();
    if e.horizontal {
        cr.move_to(x2, y2);
        cr.line_to(x2 - 6., y2 - 3.);
        cr.line_to(x2 - 6., y2 + 3.);
    } else {
        cr.move_to(x2, y2);
        cr.line_to(x2 - 3., y2 - 6.);
        cr.line_to(x2 + 3., y2 - 6.);
    }
    cr.close_path();
    let _ = cr.fill();
}
// GTK 4.12 compatibility: named semantic colors are exposed by StyleContext.
#[allow(deprecated)]
pub fn paint(
    area: &gtk::DrawingArea,
    cr: &cairo::Context,
    width: i32,
    height: i32,
    frame: Option<&PreparedView>,
    selected: Option<NodeId>,
    hover: Option<Target>,
) {
    let style = area.style_context();
    let dark = adw::StyleManager::default().is_dark();
    let lookup = |name: &str, fallback: (f64, f64, f64)| {
        style
            .lookup_color(name)
            .as_ref()
            .map(rgb)
            .unwrap_or(fallback)
    };
    let bg = lookup(
        "view_bg_color",
        if dark {
            (0.14, 0.14, 0.14)
        } else {
            (0.98, 0.98, 0.98)
        },
    );
    let fg = lookup(
        "view_fg_color",
        if dark {
            (0.96, 0.96, 0.96)
        } else {
            (0.18, 0.20, 0.21)
        },
    );
    let accent = lookup(
        "accent_color",
        if dark {
            (0.47, 0.68, 0.93)
        } else {
            (0.11, 0.44, 0.85)
        },
    );
    color(cr, bg);
    let _ = cr.paint();
    let family = area
        .pango_context()
        .font_description()
        .and_then(|d| d.family())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "sans-serif".into());
    cr.select_font_face(&family, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    let Some(f) = frame else {
        color(cr, fg);
        text(
            cr,
            24.,
            40.,
            "Preparing visualization…",
            width as f64 - 48.,
            18.,
        );
        text(
            cr,
            24.,
            72.,
            "The window is ready. Analysis runs in the background.",
            width as f64 - 48.,
            14.,
        );
        return;
    };
    let scale = (width as f64 / f.width).min(height as f64 / f.height);
    let _ = cr.save();
    cr.scale(scale, scale);
    color(cr, fg);
    text(cr, 24., 32., &f.title, f.width - 48., 20.);
    color(cr, mix(fg, bg, 0.78));
    text(cr, 24., 60., &f.subtitle, f.width - 48., 13.);
    if f.view == View::Rank && !f.marks.is_empty() {
        color(cr, mix(fg, bg, 0.3));
        cr.set_line_width(1.);
        let x = 80.;
        let y = 112.;
        let w = f.width - 136.;
        let h = f.height - 198.;
        for i in 0..=4 {
            let v = i as f64 / 4.;
            cr.move_to(x, y + v * h);
            cr.line_to(x + w, y + v * h);
            cr.move_to(x + v * w, y);
            cr.line_to(x + v * w, y + h);
        }
        let _ = cr.stroke();
        color(cr, fg);
        text(
            cr,
            x,
            f.height - 62.,
            "Shortest path from start (links)",
            w,
            12.,
        );
    }
    color(cr, mix(fg, bg, 0.6));
    cr.set_line_width(1.2);
    for e in &f.edges {
        edge(cr, e);
    }
    for m in &f.marks {
        let b = m.bounds;
        let active = m.focal
            || hover == Some(m.target)
            || matches!(m.target,Target::Page(id) if Some(id)==selected);
        let fill = if active {
            mix(accent, bg, 0.14)
        } else {
            mix(fg, bg, m.shade)
        };
        color(cr, fill);
        if m.kind == MarkKind::Dot {
            cr.arc(
                b.x + b.w / 2.,
                b.y + b.h / 2.,
                if active { 6. } else { 4. },
                0.,
                std::f64::consts::TAU,
            );
        } else {
            rounded(
                cr,
                Bounds {
                    x: b.x + 1.,
                    y: b.y + 1.,
                    w: (b.w - 2.).max(0.1),
                    h: (b.h - 2.).max(0.1),
                },
                if m.kind == MarkKind::Node { 6. } else { 0. },
            );
        }
        let _ = cr.fill_preserve();
        color(cr, if active { accent } else { mix(fg, bg, 0.65) });
        cr.set_line_width(if active { 2. } else { 1. });
        if m.error {
            cr.set_dash(&[4., 4.], 0.);
        }
        let _ = cr.stroke();
        cr.set_dash(&[], 0.);
        if b.w >= 38. && b.h >= 20. && m.kind != MarkKind::Dot {
            let _ = cr.save();
            cr.rectangle(b.x + 5., b.y + 2., (b.w - 10.).max(0.), (b.h - 4.).max(0.));
            cr.clip();
            color(cr, fg);
            text(
                cr,
                b.x + 8.,
                b.y + if b.h < 40. { b.h / 2. + 5. } else { 22. },
                &m.title,
                b.w - 16.,
                13.,
            );
            if b.h >= 44. {
                color(cr, mix(fg, bg, 0.8));
                text(cr, b.x + 8., b.y + 41., &m.detail, b.w - 16., 11.);
            }
            let _ = cr.restore();
        }
    }
    color(cr, fg);
    for label in &f.labels {
        let max = if f.view == View::Matrix && label.x < 200. {
            184.
        } else {
            f.width - label.x - 16.
        };
        text(cr, label.x, label.y, &label.text, max, 12.);
    }
    if f.marks.is_empty() {
        text(
            cr,
            32.,
            130.,
            if f.status.is_empty() {
                "No data to draw for these settings."
            } else {
                &f.status
            },
            f.width - 64.,
            15.,
        );
    }
    let _ = cr.restore();
}

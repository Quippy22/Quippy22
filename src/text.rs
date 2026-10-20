use xmltree::{Element, XMLNode};

use crate::stats::{Language, Stats, group_digits, uptime_since};
use crate::svg;
use crate::theme::{ACCENT, DIVIDER, MUTED, TEXT, TRACK, lang_color};

const FS_HEAD: f64 = 13.5;
const FS_BODY: f64 = 11.0;
const FS_LEGEND: f64 = 10.5;
const FS_SMALL: f64 = 9.5;
const ROW_H: f64 = 15.5;

/// Facts that don't come from the API; the uptime is counted, not stored.
const UPTIME_SINCE: (i32, u32, u32) = (2009, 1, 22);
const SPOKEN_LANGUAGES: &str = "English, Romanian";
const ACTIVE_HOURS: &str = "at night";

/// The right-hand info panel, laid out from (0, 0). Returns its elements and
/// the height they span, so the card can centre it.
pub fn panel(st: &Stats, w: f64) -> (Vec<Element>, f64) {
    let mut p = Panel {
        w,
        y: 0.0,
        out: Vec::new(),
    };

    p.header(st);
    p.rule();

    p.gap(14.0);
    p.title("Environment", None);
    p.chips(&["Arch Linux", "Hyprland", "Neovim"]);

    p.gap(15.0);
    p.title("Personal", None);
    p.kv(
        "Uptime",
        &uptime_since(UPTIME_SINCE.0, UPTIME_SINCE.1, UPTIME_SINCE.2),
    );
    p.kv_sized("Languages", SPOKEN_LANGUAGES, FS_SMALL);
    p.kv("Most active", ACTIVE_HOURS);

    p.gap(15.0);
    p.title("GitHub", Some("public"));
    p.kv("Repos", &group_digits(st.repos));
    p.kv("Commits", &group_digits(st.commits));
    p.kv("Lines of Code", &group_digits(st.loc));
    if st.stars > 0 {
        p.kv("Stars", &group_digits(st.stars));
    }

    let shares = st.language_shares();
    if !shares.is_empty() {
        p.gap(15.0);
        p.title("Languages", Some(&format!("{} total", shares.len())));
        p.bar(&shares);
        p.gap(8.0);
        p.legend(&shares);
    }

    // `y` has moved one row past the last baseline; trim the empty descender
    // space so the panel is measured by its ink.
    (p.out, p.y - 3.0)
}

struct Panel {
    w: f64,
    y: f64,
    out: Vec<Element>,
}

impl Panel {
    /// user@host, right-aligned uptime.
    fn header(&mut self, st: &Stats) {
        let base = FS_HEAD * 0.85;
        let mut t = svg::text(0.0, base, FS_HEAD, 600, TEXT, &st.user);
        t.children
            .push(XMLNode::Element(svg::span(ACCENT, "@github")));
        self.out.push(t);

        let meta = format!("{} on GitHub", st.uptime());
        self.out.push(svg::attr(
            svg::text(self.w, base, FS_SMALL, 400, MUTED, &meta),
            "text-anchor",
            "end",
        ));
        self.y = base + 12.5;
    }

    fn rule(&mut self) {
        self.out
            .push(svg::line(0.0, self.y, self.w, self.y, DIVIDER));
        self.y += 1.0;
    }

    fn title(&mut self, title: &str, meta: Option<&str>) {
        self.out
            .push(svg::rect(0.0, self.y - 7.0, 3.0, 8.0, 1.5, ACCENT));
        let t = svg::attr(
            svg::text(10.0, self.y, FS_SMALL, 600, MUTED, &title.to_uppercase()),
            "letter-spacing",
            "1.5",
        );
        self.out.push(t);
        if let Some(meta) = meta {
            self.out.push(svg::attr(
                svg::text(self.w, self.y, FS_SMALL, 400, MUTED, meta),
                "text-anchor",
                "end",
            ));
        }
        self.y += 13.0;
    }

    /// label on the left, value flush right.
    fn kv(&mut self, label: &str, value: &str) {
        self.kv_sized(label, value, FS_BODY);
    }

    /// Same row with the value in another size, for values whose usual size
    /// would crowd the label.
    fn kv_sized(&mut self, label: &str, value: &str, value_fs: f64) {
        self.out
            .push(svg::text(0.0, self.y, FS_BODY, 400, MUTED, label));
        self.out.push(svg::attr(
            svg::text(self.w, self.y, value_fs, 600, TEXT, value),
            "text-anchor",
            "end",
        ));
        self.y += ROW_H;
    }

    /// One line of facts, dot-separated. Items are placed by coordinate rather
    /// than by spaces, which renderers collapse differently.
    fn chips(&mut self, items: &[&str]) {
        let mut x = 0.0;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.out
                    .push(svg::text(x, self.y, FS_BODY, 400, MUTED, "·"));
                x += svg::width(" · ", FS_BODY);
            }
            self.out
                .push(svg::text(x, self.y, FS_BODY, 400, TEXT, item));
            x += svg::width(item, FS_BODY);
        }
        self.y += 18.0;
    }

    /// Proportional stacked bar of language bytes.
    fn bar(&mut self, shares: &[(&Language, f64)]) {
        let top: Vec<_> = shares.iter().filter(|(_, s)| *s >= 0.005).collect();
        let sum: f64 = top.iter().map(|(_, s)| *s).sum();
        if sum <= 0.0 {
            return;
        }

        self.out
            .push(svg::rect(0.0, self.y, self.w, 7.0, 3.5, TRACK));

        let mut clip = Element::new("clipPath");
        clip.attributes.insert("id".into(), "langbar".into());
        clip.children.push(XMLNode::Element(svg::rect(
            0.0, self.y, self.w, 7.0, 3.5, "#000",
        )));

        let mut segments = Element::new("g");
        segments
            .attributes
            .insert("clip-path".into(), "url(#langbar)".into());
        segments.children.push(XMLNode::Element(clip));

        let mut x = 0.0;
        for (lang, share) in top {
            let w = self.w * (share / sum);
            segments.children.push(XMLNode::Element(svg::rect(
                x,
                self.y,
                w,
                7.0,
                0.0,
                lang_color(&lang.name),
            )));
            x += w;
        }
        self.out.push(segments);
        self.y += 15.0;
    }

    /// Top languages with their share.
    fn legend(&mut self, shares: &[(&Language, f64)]) {
        let mut x = 0.0;
        for (lang, share) in shares.iter().take(4) {
            let name_w = svg::width(&lang.name, FS_LEGEND);
            let pct = format!("{}%", (share * 100.0).round() as u32);
            let item_w = 8.0 + name_w + svg::width(" ", FS_LEGEND) + svg::width(&pct, FS_LEGEND);
            if x + item_w > self.w {
                break;
            }
            self.out.push(svg::circle(
                x + 2.5,
                self.y - 3.5,
                2.5,
                lang_color(&lang.name),
            ));
            self.out
                .push(svg::text(x + 8.0, self.y, FS_LEGEND, 400, TEXT, &lang.name));
            self.out.push(svg::text(
                x + 8.0 + name_w + svg::width(" ", FS_LEGEND),
                self.y,
                FS_LEGEND,
                400,
                MUTED,
                &pct,
            ));
            x += item_w + 13.0;
        }
        self.y += 13.0;
    }

    fn gap(&mut self, h: f64) {
        self.y += h;
    }
}

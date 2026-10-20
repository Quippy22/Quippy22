use std::fs;

use xmltree::{Element, XMLNode};

mod art;
mod font;
mod refresh;
mod stats;
mod svg;
mod text;
mod theme;

use crate::theme::{BORDER, DIVIDER, FRAME_FILL, FRAME_STROKE};

// --- Card layout (px) ---
const PAD: f64 = 15.0;
const GUTTER: f64 = 18.0;
const PANEL_W: f64 = 312.0;
const FRAME_PAD: f64 = 9.0;
const RADIUS: f64 = 9.0;

// --- Art metrics ---
const ART_FS: f64 = 10.0;
const ART_CELL: f64 = 1.5; // cell height / cell width: the art's own aspect

fn build_svg(art_path: &str, stats_path: &str) -> Element {
    let st = stats::Stats::load(stats_path);
    let art = art::load(art_path, ART_FS, svg::ADVANCE, ART_CELL);
    let (panel, panel_h) = text::panel(&st, PANEL_W);

    let frame_w = art.w + 2.0 * FRAME_PAD;
    let frame_h = art.h + 2.0 * FRAME_PAD;
    let content_h = frame_h.max(panel_h);

    let w = PAD * 2.0 + frame_w + GUTTER * 2.0 + 1.0 + PANEL_W;
    let h = PAD * 2.0 + content_h;

    let art_x = PAD;
    let art_y = PAD + (content_h - frame_h) / 2.0;
    let panel_x = PAD + frame_w + GUTTER + 1.0 + GUTTER;
    let panel_y = PAD + (content_h - panel_h) / 2.0;
    let divider_x = PAD + frame_w + GUTTER + 0.5;

    let mut svg = Element::new("svg");
    svg.attributes
        .insert("xmlns".into(), "http://www.w3.org/2000/svg".into());
    svg.attributes.insert("width".into(), svg::num(w));
    svg.attributes.insert("height".into(), svg::num(h));
    svg.attributes.insert(
        "viewBox".into(),
        format!("0 0 {} {}", svg::num(w), svg::num(h)),
    );
    svg.attributes.insert("role".into(), "img".into());
    svg.attributes
        .insert("font-family".into(), font::STACK.into());

    let mut title = Element::new("title");
    title
        .children
        .push(XMLNode::Text(format!("{} — GitHub profile card", st.user)));
    svg.children.push(XMLNode::Element(title));

    let mut desc = Element::new("desc");
    desc.children.push(XMLNode::Text(
        "ASCII portrait with the account's live stats".into(),
    ));
    svg.children.push(XMLNode::Element(desc));

    svg.children.push(XMLNode::Element(font::style()));
    svg.children.push(XMLNode::Element(defs()));

    // Card background.
    svg.children.push(XMLNode::Element(svg::attr(
        svg::attr(
            svg::rect(0.5, 0.5, w - 1.0, h - 1.0, RADIUS, "url(#bg)"),
            "stroke",
            BORDER,
        ),
        "stroke-width",
        "1",
    )));

    // A soft accent glow behind the portrait.
    let mut glow = Element::new("ellipse");
    glow.attributes
        .insert("cx".into(), svg::num(art_x + frame_w / 2.0));
    glow.attributes.insert("cy".into(), svg::num(h / 2.0));
    glow.attributes
        .insert("rx".into(), svg::num(frame_w * 0.62));
    glow.attributes
        .insert("ry".into(), svg::num(frame_h * 0.62));
    glow.attributes.insert("fill".into(), "url(#glow)".into());
    svg.children.push(XMLNode::Element(glow));

    // Portrait frame.
    svg.children.push(XMLNode::Element(svg::attr(
        svg::attr(
            svg::rect(art_x, art_y, frame_w, frame_h, 8.0, FRAME_FILL),
            "stroke",
            FRAME_STROKE,
        ),
        "stroke-width",
        "1",
    )));

    // The art itself, centred in its frame.
    let mut art_group = Element::new("g");
    art_group.attributes.insert(
        "transform".into(),
        format!(
            "translate({},{})",
            svg::num(art_x + FRAME_PAD),
            svg::num(art_y + FRAME_PAD)
        ),
    );
    for e in art.elems {
        art_group.children.push(XMLNode::Element(e));
    }
    svg.children.push(XMLNode::Element(art_group));

    svg.children.push(XMLNode::Element(svg::line(
        divider_x,
        PAD,
        divider_x,
        h - PAD,
        DIVIDER,
    )));

    let mut panel_group = Element::new("g");
    panel_group.attributes.insert(
        "transform".into(),
        format!("translate({},{})", svg::num(panel_x), svg::num(panel_y)),
    );
    for e in panel {
        panel_group.children.push(XMLNode::Element(e));
    }
    svg.children.push(XMLNode::Element(panel_group));

    svg
}

fn defs() -> Element {
    let mut defs = Element::new("defs");

    let mut bg = Element::new("linearGradient");
    bg.attributes.insert("id".into(), "bg".into());
    bg.attributes.insert("x1".into(), "0".into());
    bg.attributes.insert("y1".into(), "0".into());
    bg.attributes.insert("x2".into(), "0".into());
    bg.attributes.insert("y2".into(), "1".into());
    for (offset, color) in [("0", theme::BG_TOP), ("1", theme::BG_BOTTOM)] {
        let mut stop = Element::new("stop");
        stop.attributes.insert("offset".into(), offset.into());
        stop.attributes.insert("stop-color".into(), color.into());
        bg.children.push(XMLNode::Element(stop));
    }
    defs.children.push(XMLNode::Element(bg));

    let mut glow = Element::new("radialGradient");
    glow.attributes.insert("id".into(), "glow".into());
    for (offset, opacity) in [("0", "0.10"), ("1", "0")] {
        let mut stop = Element::new("stop");
        stop.attributes.insert("offset".into(), offset.into());
        stop.attributes
            .insert("stop-color".into(), theme::ACCENT.into());
        stop.attributes
            .insert("stop-opacity".into(), opacity.into());
        glow.children.push(XMLNode::Element(stop));
    }
    defs.children.push(XMLNode::Element(glow));

    defs
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("refresh") {
        if let Err(err) = refresh::run(&args[1..]) {
            eprintln!("refresh failed: {err}");
            std::process::exit(1);
        }
        return;
    }

    let mut art_path = "assets/art.xml".to_string();
    let mut stats_path = "assets/stats.txt".to_string();
    let mut out_path = "card.svg".to_string();

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .unwrap_or_else(|| panic!("missing value for {arg}"))
        };
        match arg.as_str() {
            "--art" => art_path = value(),
            "--stats" => stats_path = value(),
            "--out" => out_path = value(),
            "-h" | "--help" => {
                println!(
                    "usage: cargo run -- [--art ART.xml] [--stats STATS.txt] [--out CARD.svg]"
                );
                println!("       cargo run -- refresh [--no-loc]");
                return;
            }
            other => panic!("unknown argument: {other}"),
        }
    }

    let card = svg::serialize(&build_svg(&art_path, &stats_path));
    fs::write(&out_path, card).unwrap_or_else(|_| panic!("write {out_path}"));
}

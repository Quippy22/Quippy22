use xmltree::{Element, XMLNode};

/// Format a coordinate without trailing zeros ("12.5", "240").
pub fn num(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A single-line <text> element.
pub fn text(x: f64, y: f64, size: f64, weight: u32, fill: &str, s: &str) -> Element {
    let mut t = Element::new("text");
    t.attributes.insert("x".into(), num(x));
    t.attributes.insert("y".into(), num(y));
    t.attributes.insert("font-size".into(), num(size));
    t.attributes
        .insert("font-weight".into(), weight.to_string());
    t.attributes.insert("fill".into(), fill.into());
    if !s.is_empty() {
        t.children.push(XMLNode::Text(s.into()));
    }
    t
}

pub fn circle(cx: f64, cy: f64, r: f64, fill: &str) -> Element {
    let mut e = Element::new("circle");
    e.attributes.insert("cx".into(), num(cx));
    e.attributes.insert("cy".into(), num(cy));
    e.attributes.insert("r".into(), num(r));
    e.attributes.insert("fill".into(), fill.into());
    e
}

/// A <tspan> carrying its own fill, for two-tone text runs.
pub fn span(fill: &str, s: &str) -> Element {
    let mut e = Element::new("tspan");
    e.attributes.insert("fill".into(), fill.into());
    e.children.push(XMLNode::Text(s.into()));
    e
}

pub fn attr(mut e: Element, key: &str, value: &str) -> Element {
    e.attributes.insert(key.into(), value.into());
    e
}

pub fn rect(x: f64, y: f64, w: f64, h: f64, rx: f64, fill: &str) -> Element {
    let mut e = Element::new("rect");
    e.attributes.insert("x".into(), num(x));
    e.attributes.insert("y".into(), num(y));
    e.attributes.insert("width".into(), num(w));
    e.attributes.insert("height".into(), num(h));
    e.attributes.insert("rx".into(), num(rx));
    e.attributes.insert("fill".into(), fill.into());
    e
}

pub fn line(x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str) -> Element {
    let mut e = Element::new("line");
    e.attributes.insert("x1".into(), num(x1));
    e.attributes.insert("y1".into(), num(y1));
    e.attributes.insert("x2".into(), num(x2));
    e.attributes.insert("y2".into(), num(y2));
    e.attributes.insert("stroke".into(), stroke.into());
    e.attributes.insert("stroke-width".into(), "1".into());
    e
}

/// Advance width of `s` in the monospace face. Every monospace font worth the
/// name uses a 0.6 em advance, so the layout holds even when the embedded face
/// is unavailable.
pub fn width(s: &str, size: f64) -> f64 {
    s.chars().count() as f64 * size * ADVANCE
}

/// Glyph advance of the embedded face, in em.
pub const ADVANCE: f64 = 0.6;

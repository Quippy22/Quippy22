use std::fs;
use xmltree::{Element, XMLNode};

use crate::svg::num;

pub struct Art {
    pub elems: Vec<Element>,
    pub w: f64,
    pub h: f64,
}

/// Load an art XML, crop it to its ink bounding box and lay the glyphs out on a
/// fixed grid, left-aligned at (0, 0).
///
/// `advance` is the glyph advance in em (JetBrains Mono: 0.6) and `cell` the
/// cell height as a multiple of the cell width, so the grid matches the aspect
/// the art was rendered at and the portrait isn't stretched.
pub fn load(path: &str, font_size: f64, advance: f64, cell: f64) -> Art {
    let raw = fs::read_to_string(path).unwrap_or_else(|_| panic!("cannot read {path}"));
    let doc = Element::parse(raw.as_bytes()).unwrap_or_else(|_| panic!("cannot parse {path}"));

    // One (char, fill) pair per cell, row by row.
    let grid: Vec<Vec<(String, String)>> = doc
        .children
        .iter()
        .filter_map(XMLNode::as_element)
        .map(|line| {
            line.children
                .iter()
                .filter_map(XMLNode::as_element)
                .map(|glyph| {
                    let fill = glyph
                        .attributes
                        .iter()
                        .find(|(k, _)| k.as_str() == "fg")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| "#ffffff".into());
                    let txt = glyph
                        .children
                        .iter()
                        .find_map(XMLNode::as_text)
                        .unwrap_or("")
                        .to_string();
                    (txt, fill)
                })
                .collect()
        })
        .collect();

    // Ink bounding box: trim blank cells, so the art sits flush in its frame.
    let (mut top, mut left, mut bottom, mut right) = (usize::MAX, usize::MAX, 0, 0);
    for (r, row) in grid.iter().enumerate() {
        for (c, (txt, _)) in row.iter().enumerate() {
            if inked(txt) {
                top = top.min(r);
                left = left.min(c);
                bottom = bottom.max(r);
                right = right.max(c);
            }
        }
    }
    assert!(top != usize::MAX, "art has no visible glyphs");
    let (rows, cols) = (bottom - top + 1, right - left + 1);

    let cw = font_size * advance;
    let ch = cw * cell;

    // One <text> per same-colour run of glyphs: the advance is exact, so a run
    // needs no per-glyph x coordinates.
    let mut elems = Vec::new();
    for (r, row) in grid.iter().enumerate().skip(top).take(rows) {
        let mut c = left;
        while c <= right {
            let Some((txt, fill)) = row.get(c) else { break };
            if !inked(txt) {
                c += 1;
                continue;
            }
            let start = c;
            let mut run = String::new();
            while let Some((txt, f)) = row.get(c) {
                if f != fill || !inked(txt) {
                    break;
                }
                run.push_str(txt);
                c += 1;
            }
            let mut t = Element::new("text");
            t.attributes
                .insert("x".into(), num((start - left) as f64 * cw));
            t.attributes
                .insert("y".into(), num((r - top) as f64 * ch + font_size * 0.8));
            t.attributes.insert("font-size".into(), num(font_size));
            t.attributes.insert("fill".into(), fill.clone());
            t.children.push(XMLNode::Text(run));
            elems.push(t);
        }
    }

    Art {
        elems,
        w: cols as f64 * cw,
        h: rows as f64 * ch,
    }
}

fn inked(txt: &str) -> bool {
    txt.chars().any(|c| !c.is_whitespace())
}

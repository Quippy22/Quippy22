use std::fs;

use xmltree::Element;

const REGULAR: &str = "assets/JetBrainsMono-Regular.woff";
const SEMIBOLD: &str = "assets/JetBrainsMono-SemiBold.woff";

/// Font stack used when the embedded face is unavailable.
pub const STACK: &str =
    "'JBMono','JetBrains Mono',ui-monospace,Menlo,Consolas,'DejaVu Sans Mono',monospace";

/// The <style> element: two subsetted JetBrains Mono faces (400/600) inlined as
/// data URIs and the font stack, so the card looks the same wherever it is
/// rendered. The subsets are ASCII-only and covered by assets/OFL.txt.
pub fn style() -> Element {
    let css = format!(
        "@font-face{{font-family:'JBMono';font-style:normal;font-weight:400;\
         src:url(data:font/woff;base64,{}) format('woff')}}\
         @font-face{{font-family:'JBMono';font-style:normal;font-weight:600;\
         src:url(data:font/woff;base64,{}) format('woff')}}\
         text{{font-family:{STACK}}}",
        base64(&fs::read(REGULAR).expect("read assets/JetBrainsMono-Regular.woff")),
        base64(&fs::read(SEMIBOLD).expect("read assets/JetBrainsMono-SemiBold.woff")),
    );

    let mut e = Element::new("style");
    e.children.push(xmltree::XMLNode::Text(css));
    e
}

/// Minimal base64 encoder (RFC 4648), keeps the crate dependency-free.
fn base64(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        let pad = 3 - chunk.len();
        for i in 0..4 - pad {
            out.push(TABLE[(n >> (18 - 6 * i) & 0x3f) as usize] as char);
        }
        out.extend(std::iter::repeat_n('=', pad));
    }
    out
}

//! The card's palette: one place for every colour in the drawing.

pub const BG_TOP: &str = "#161b22";
pub const BG_BOTTOM: &str = "#0d1117";
pub const BORDER: &str = "#262c36";
pub const FRAME_FILL: &str = "#0a0d12";
pub const FRAME_STROKE: &str = "#2a3038";
pub const DIVIDER: &str = "#232931";
pub const TEXT: &str = "#e6edf3";
pub const MUTED: &str = "#8b949e";
pub const ACCENT: &str = "#d478f0";
pub const TRACK: &str = "#1b2027";

/// GitHub's language colours, brightened for a dark background.
pub fn lang_color(name: &str) -> &'static str {
    match name {
        "Rust" => "#dea584",
        "Python" => "#4b8bbe",
        "HTML" => "#e34c26",
        "CSS" => "#8f7fe8",
        "JavaScript" => "#f1e05a",
        "TypeScript" => "#6ea8fe",
        "Lua" => "#7aa6d6",
        "Shell" | "Bash" => "#89e051",
        _ => "#6e7681",
    }
}

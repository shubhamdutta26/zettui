//! Writing-mode state and display-only helpers.
use crate::{outline, theme::Theme};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
};
use serde::Deserialize;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FocusMode {
    Line,
    #[default]
    Paragraph,
    Section,
}
impl FocusMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Line => "LINE",
            Self::Paragraph => "PARAGRAPH",
            Self::Section => "SECTION",
        }
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ZenSettings {
    pub enabled: bool,
    pub width: u16,
    pub show_line_numbers: bool,
    pub show_status_bar: bool,
    pub show_title: bool,
}
impl Default for ZenSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 80,
            show_line_numbers: false,
            show_status_bar: true,
            show_title: true,
        }
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FocusSettings {
    pub enabled: bool,
    pub mode: FocusMode,
    pub dim_strength: f32,
    pub typewriter: bool,
}
impl Default for FocusSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: FocusMode::Paragraph,
            dim_strength: 0.65,
            typewriter: false,
        }
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct EditorSettings {
    pub line_numbers: bool,
    pub word_wrap: bool,
}
impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            line_numbers: true,
            word_wrap: true,
        }
    }
}
#[derive(Default)]
pub struct Writing {
    pub zen: ZenSettings,
    pub focus: FocusSettings,
    pub editor: EditorSettings,
    pub leader: bool,
}
impl Writing {
    pub fn command(&mut self, cmd: &str) -> bool {
        match cmd {
            "zen" => self.zen.enabled = !self.zen.enabled,
            "focus" => self.focus.enabled = !self.focus.enabled,
            "focus off" => self.focus.enabled = false,
            "focus line" | "focus paragraph" | "focus section" => {
                self.focus.enabled = true;
                self.focus.mode = match cmd {
                    "focus line" => FocusMode::Line,
                    "focus section" => FocusMode::Section,
                    _ => FocusMode::Paragraph,
                };
            }
            "typewriter" => self.focus.typewriter = !self.focus.typewriter,
            "set number" => {
                if self.zen.enabled {
                    self.zen.show_line_numbers = true;
                } else {
                    self.editor.line_numbers = true;
                }
            }
            "set nonumber" => {
                if self.zen.enabled {
                    self.zen.show_line_numbers = false;
                } else {
                    self.editor.line_numbers = false;
                }
            }
            "set wrap" => self.editor.word_wrap = true,
            "set nowrap" => self.editor.word_wrap = false,
            "set status" => self.zen.show_status_bar = true,
            "set nostatus" => self.zen.show_status_bar = false,
            "toggle number" => {
                if self.zen.enabled {
                    self.zen.show_line_numbers = !self.zen.show_line_numbers;
                } else {
                    self.editor.line_numbers = !self.editor.line_numbers;
                }
            }
            "toggle status" => self.zen.show_status_bar = !self.zen.show_status_bar,
            _ => {
                if let Some(width) = cmd
                    .strip_prefix("set textwidth=")
                    .and_then(|s| s.parse::<u16>().ok())
                    .filter(|w| (20..=240).contains(w))
                {
                    self.zen.width = width;
                } else {
                    return false;
                }
            }
        }
        true
    }
    pub fn label(&self) -> String {
        let mut labels = Vec::new();
        if self.zen.enabled {
            labels.push("ZEN");
        }
        if self.focus.enabled {
            labels.push(self.focus.mode.label());
        }
        if self.focus.typewriter {
            labels.push("TYPEWRITER");
        }
        labels.join(" · ")
    }
}
pub fn centered(area: Rect, width: u16) -> Rect {
    let w = width.min(area.width);
    Rect::new(area.x + (area.width - w) / 2, area.y, w, area.height)
}
/// Logical-line range using the shared Markdown heading parser.
pub fn active_range(text: &str, row: usize, mode: FocusMode) -> Range<usize> {
    let lines: Vec<_> = text.split('\n').collect();
    let row = row.min(lines.len() - 1);
    if mode == FocusMode::Line {
        return row..row + 1;
    }
    let headings = outline::extract(text);
    if mode == FocusMode::Paragraph {
        let is_heading = |row| headings.binary_search_by_key(&row, |h| h.row).is_ok();
        if lines[row].trim().is_empty() || is_heading(row) {
            return row..row + 1;
        }
        let mut a = row;
        let mut b = row + 1;
        while a > 0 && !lines[a - 1].trim().is_empty() && !is_heading(a - 1) {
            a -= 1;
        }
        while b < lines.len() && !lines[b].trim().is_empty() && !is_heading(b) {
            b += 1;
        }
        return a..b;
    }
    if let Some(index) = outline::active_heading(&headings, row) {
        let heading = &headings[index];
        heading.row..heading.end_row
    } else {
        0..headings.first().map_or(lines.len(), |h| h.row)
    }
}
pub fn dim(style: Style, t: Theme, strength: f32) -> Style {
    let fg = style.fg.unwrap_or(t.foreground);
    let p = strength.clamp(0.0, 0.85);
    if let (Color::Rgb(r, g, b), Color::Rgb(br, bg, bb)) = (fg, t.background) {
        let mix = |a: u8, b: u8| (f32::from(a) * (1.0 - p) + f32::from(b) * p).round() as u8;
        style.fg(Color::Rgb(mix(r, br), mix(g, bg), mix(b, bb)))
    } else {
        style.fg(t.muted)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges() {
        let t = "# A\nfirst\nsecond\n\n## B\nlast\n# C\nend";
        assert_eq!(active_range(t, 2, FocusMode::Paragraph), 1..3);
        assert_eq!(active_range(t, 5, FocusMode::Section), 4..6);
        assert_eq!(active_range(t, 2, FocusMode::Line), 2..3);
    }
    #[test]
    fn fences() {
        assert_eq!(
            active_range("# A\n```r\n# fake\n```\ntext\n# B", 4, FocusMode::Section),
            0..5
        );
    }
    #[test]
    fn section_ignores_comments_math_and_indented_code() {
        let t = "# A\n<!--\n# Fake\n-->\n$$\n## Fake\n$$\n    # Code\ntext\n# B";
        assert_eq!(active_range(t, 8, FocusMode::Section), 0..9);
    }
    #[test]
    fn section_includes_children_and_stops_at_sibling() {
        let t = "intro\n# A\ntext\n## Child\nbody\n# B\n";
        assert_eq!(active_range(t, 0, FocusMode::Section), 0..1);
        assert_eq!(active_range(t, 2, FocusMode::Section), 1..5);
        assert_eq!(active_range(t, 4, FocusMode::Section), 3..5);
        assert_eq!(active_range(t, 6, FocusMode::Section), 5..7);
    }
    #[test]
    fn narrow() {
        assert_eq!(centered(Rect::new(0, 0, 20, 10), 80).width, 20);
    }
}

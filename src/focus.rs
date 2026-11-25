//! Writing-mode state and display-only helpers.
use crate::theme::Theme;
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
fn heading(line: &str) -> Option<usize> {
    let s = line.trim_start();
    let n = s.bytes().take_while(|&b| b == b'#').count();
    ((1..=6).contains(&n)
        && s.get(n..)
            .is_some_and(|s| s.is_empty() || s.starts_with(char::is_whitespace)))
    .then_some(n)
}
/// Logical-line range. Markdown headings inside fenced code and YAML are ignored.
pub fn active_range(text: &str, row: usize, mode: FocusMode) -> Range<usize> {
    let lines: Vec<_> = text.split('\n').collect();
    let row = row.min(lines.len() - 1);
    if mode == FocusMode::Line {
        return row..row + 1;
    }
    let mut headings = vec![None; lines.len()];
    let mut fence: Option<(char, usize)> = None;
    let mut yaml = false;
    for (i, line) in lines.iter().enumerate() {
        let s = line.trim();
        if i == 0 && s == "---" {
            yaml = true;
            continue;
        }
        if yaml {
            if matches!(s, "---" | "...") {
                yaml = false;
            }
            continue;
        }
        if let Some((m, n)) = fence {
            let k = s.chars().take_while(|&c| c == m).count();
            if k >= n && s.chars().skip(k).all(char::is_whitespace) {
                fence = None;
            }
            continue;
        }
        if let Some(m) = s.chars().next().filter(|c| matches!(*c, '`' | '~')) {
            let k = s.chars().take_while(|&c| c == m).count();
            if k >= 3 {
                fence = Some((m, k));
                continue;
            }
        }
        headings[i] = heading(line);
    }
    if mode == FocusMode::Paragraph {
        if lines[row].trim().is_empty() || headings[row].is_some() {
            return row..row + 1;
        }
        let mut a = row;
        let mut b = row + 1;
        while a > 0 && !lines[a - 1].trim().is_empty() && headings[a - 1].is_none() {
            a -= 1;
        }
        while b < lines.len() && !lines[b].trim().is_empty() && headings[b].is_none() {
            b += 1;
        }
        return a..b;
    }
    if let Some(a) = (0..=row).rev().find(|&i| headings[i].is_some()) {
        let level = headings[a].unwrap();
        let b = (row + 1..lines.len())
            .find(|&i| headings[i].is_some_and(|n| n <= level))
            .unwrap_or(lines.len());
        a..b
    } else {
        0..(row + 1..lines.len())
            .find(|&i| headings[i].is_some())
            .unwrap_or(lines.len())
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
    fn narrow() {
        assert_eq!(centered(Rect::new(0, 0, 20, 10), 80).width, 20);
    }
}

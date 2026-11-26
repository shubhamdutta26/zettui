//! Layout sizing and independent side-panel preferences.
//! A configured percentage is the desired *files pane* width.
//! On narrow terminals we reserve enough space for both panes when possible.

use ratatui::layout::Rect;

pub const DEFAULT_BROWSER_PERCENT: u16 = 35;
pub const MIN_BROWSER_PERCENT: u16 = 20;
pub const MAX_BROWSER_PERCENT: u16 = 70;
pub const RESIZE_STEP: u16 = 5;
pub const DEFAULT_OUTLINE_PERCENT: u16 = 25;
pub const MIN_OUTLINE_PERCENT: u16 = 15;
pub const MAX_OUTLINE_PERCENT: u16 = 50;
const MIN_OUTLINE_COLUMNS: u16 = 18;
const MIN_FILES_COLUMNS: u16 = 16;
const MIN_NOTE_COLUMNS: u16 = 24;

pub fn valid_browser_percent(percent: u16) -> bool {
    (MIN_BROWSER_PERCENT..=MAX_BROWSER_PERCENT).contains(&percent)
}

pub fn resize_browser_percent(current: u16, grow: bool) -> u16 {
    let current = current.clamp(MIN_BROWSER_PERCENT, MAX_BROWSER_PERCENT);
    if grow {
        current.saturating_add(RESIZE_STEP).min(MAX_BROWSER_PERCENT)
    } else {
        current.saturating_sub(RESIZE_STEP).max(MIN_BROWSER_PERCENT)
    }
}

/// Session preferences, independent of temporary Zen/narrow-terminal layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panels {
    pub show_files: bool,
    pub show_preview: bool,
    pub show_outline: bool,
    pub outline_width_percent: u16,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            show_files: true,
            show_preview: true,
            show_outline: false,
            outline_width_percent: DEFAULT_OUTLINE_PERCENT,
        }
    }
}

pub fn valid_outline_percent(percent: u16) -> bool {
    (MIN_OUTLINE_PERCENT..=MAX_OUTLINE_PERCENT).contains(&percent)
}

impl Panels {
    pub fn resize_outline(&mut self, grow: bool) {
        let current = self
            .outline_width_percent
            .clamp(MIN_OUTLINE_PERCENT, MAX_OUTLINE_PERCENT);
        self.outline_width_percent = if grow {
            current.saturating_add(RESIZE_STEP).min(MAX_OUTLINE_PERCENT)
        } else {
            current.saturating_sub(RESIZE_STEP).max(MIN_OUTLINE_PERCENT)
        };
    }

    /// [Files, Editor/Preview, Outline]. Browser mode always retains Files;
    /// editing always retains the editor. Zen temporarily hides sidebars.
    pub fn areas(self, area: Rect, browser_percent: u16, editing: bool, zen: bool) -> [Rect; 3] {
        let empty = Rect::default();
        if editing && zen {
            return [empty, area, empty];
        }
        if !editing && (!self.show_preview || area.width < 2) {
            return [area, empty, empty];
        }
        let files = !editing || (self.show_files && area.width >= 2);
        let min_files = if files { MIN_FILES_COLUMNS } else { 0 };
        // Hide the outline temporarily before sacrificing the editable area.
        let outline = editing
            && self.show_outline
            && area.width >= min_files + MIN_NOTE_COLUMNS + MIN_OUTLINE_COLUMNS;
        if !outline {
            return if files {
                let [left, note] = split_panes(area, browser_percent);
                [left, note, empty]
            } else {
                [empty, area, empty]
            };
        }
        let desired =
            |percent: u16| ((u32::from(area.width) * u32::from(percent) + 50) / 100) as u16;
        let right_width = desired(
            self.outline_width_percent
                .clamp(MIN_OUTLINE_PERCENT, MAX_OUTLINE_PERCENT),
        )
        .clamp(
            MIN_OUTLINE_COLUMNS,
            area.width - min_files - MIN_NOTE_COLUMNS,
        );
        let left_width = if files {
            desired(browser_percent.clamp(MIN_BROWSER_PERCENT, MAX_BROWSER_PERCENT)).clamp(
                MIN_FILES_COLUMNS,
                area.width - right_width - MIN_NOTE_COLUMNS,
            )
        } else {
            0
        };
        let note_width = area.width - left_width - right_width;
        let left = if files {
            Rect::new(area.x, area.y, left_width, area.height)
        } else {
            empty
        };
        let note = Rect::new(
            area.x.saturating_add(left_width),
            area.y,
            note_width,
            area.height,
        );
        let right = Rect::new(
            note.x.saturating_add(note_width),
            area.y,
            right_width,
            area.height,
        );
        [left, note, right]
    }
}

/// Returns [files pane, note pane] without ever allocating more than the terminal width.
pub fn split_panes(area: Rect, browser_percent: u16) -> [Rect; 2] {
    let total = area.width;
    let desired = ((u32::from(total)
        * u32::from(browser_percent.clamp(MIN_BROWSER_PERCENT, MAX_BROWSER_PERCENT)))
        + 50)
        / 100;
    let mut files = desired as u16;
    if total >= MIN_FILES_COLUMNS + MIN_NOTE_COLUMNS {
        files = files.clamp(MIN_FILES_COLUMNS, total - MIN_NOTE_COLUMNS);
    } else if total >= 2 {
        files = files.clamp(1, total - 1);
    } else {
        files = total;
    }
    let notes = total.saturating_sub(files);
    [
        Rect::new(area.x, area.y, files, area.height),
        Rect::new(area.x.saturating_add(files), area.y, notes, area.height),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_range_is_limited() {
        assert!(valid_browser_percent(35));
        assert!(valid_browser_percent(20));
        assert!(valid_browser_percent(70));
        assert!(!valid_browser_percent(19));
        assert!(!valid_browser_percent(71));
    }

    #[test]
    fn resize_uses_five_point_steps_and_stays_in_range() {
        assert_eq!(resize_browser_percent(35, true), 40);
        assert_eq!(resize_browser_percent(35, false), 30);
        assert_eq!(resize_browser_percent(70, true), 70);
        assert_eq!(resize_browser_percent(20, false), 20);
    }

    #[test]
    fn pane_widths_fill_the_screen() {
        let area = Rect::new(3, 4, 100, 20);
        let [left, right] = split_panes(area, 35);
        assert_eq!(left.width, 35);
        assert_eq!(right.width, 65);
        assert_eq!(right.x, left.x + left.width);
        assert_eq!(left.width + right.width, area.width);
        assert_eq!(right.y, area.y);
    }

    #[test]
    fn minimums_are_preserved_on_resizing_and_narrow_terminals() {
        let [left, right] = split_panes(Rect::new(0, 0, 40, 10), 70);
        assert!(left.width >= MIN_FILES_COLUMNS);
        assert!(right.width >= MIN_NOTE_COLUMNS);
        for width in 0..40 {
            let [left, right] = split_panes(Rect::new(0, 0, width, 8), 35);
            assert_eq!(left.width + right.width, width);
        }
    }

    #[test]
    fn defaults_preserve_two_pane_layout() {
        let area = Rect::new(3, 4, 100, 20);
        let panels = Panels::default();
        let [files, note, outline] = panels.areas(area, 35, true, false);
        assert_eq!([files, note], split_panes(area, 35));
        assert_eq!(outline.width, 0);
        assert_eq!(panels.areas(area, 35, false, false), [files, note, outline]);
    }

    #[test]
    fn independent_visibility_and_zen_preserve_preferences() {
        let area = Rect::new(3, 4, 100, 20);
        let mut panels = Panels {
            show_outline: true,
            ..Panels::default()
        };
        assert_eq!(
            panels.areas(area, 35, true, false).map(|r| r.width),
            [35, 40, 25]
        );
        panels.show_files = false;
        assert_eq!(
            panels.areas(area, 35, true, false).map(|r| r.width),
            [0, 75, 25]
        );
        assert_eq!(
            panels.areas(area, 35, true, true).map(|r| r.width),
            [0, 100, 0]
        );
        assert_eq!(
            panels.areas(area, 35, true, false).map(|r| r.width),
            [0, 75, 25]
        );
        panels.show_preview = false;
        assert_eq!(
            panels.areas(area, 35, false, true).map(|r| r.width),
            [100, 0, 0]
        );
        panels.show_outline = false;
        assert_eq!(
            panels.areas(area, 35, true, false).map(|r| r.width),
            [0, 100, 0]
        );
        panels.show_preview = true;
        assert_eq!(
            panels.areas(area, 35, false, false).map(|r| r.width),
            [35, 65, 0]
        );
    }

    #[test]
    fn geometry_is_contiguous_and_never_loses_the_active_pane() {
        for width in 0..=150 {
            let area = Rect::new(3, 4, width, 20);
            for browser_width in [20, 35, 70] {
                for outline_width in [15, 25, 50] {
                    for show_files in [false, true] {
                        for show_preview in [false, true] {
                            for show_outline in [false, true] {
                                let panels = Panels {
                                    show_files,
                                    show_preview,
                                    show_outline,
                                    outline_width_percent: outline_width,
                                };
                                for editing in [false, true] {
                                    let rects = panels.areas(area, browser_width, editing, false);
                                    let mut x = area.x;
                                    for rect in rects.iter().filter(|r| r.width > 0) {
                                        assert_eq!(rect.x, x);
                                        assert_eq!(rect.y, area.y);
                                        assert_eq!(rect.height, area.height);
                                        x += rect.width;
                                    }
                                    assert_eq!(x, area.x + area.width);
                                    if width > 0 {
                                        assert!(rects[if editing { 1 } else { 0 }].width > 0);
                                    }
                                    if rects[2].width > 0 {
                                        assert!(rects[1].width >= MIN_NOTE_COLUMNS);
                                        assert!(rects[2].width >= MIN_OUTLINE_COLUMNS);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn outline_resize_is_bounded_and_survives_hiding() {
        let mut panels = Panels::default();
        panels.resize_outline(true);
        assert_eq!(panels.outline_width_percent, 30);
        panels.show_outline = true;
        assert_eq!(
            panels.areas(Rect::new(0, 0, 100, 10), 35, true, false)[2].width,
            30
        );
        for _ in 0..20 {
            panels.resize_outline(false);
        }
        assert_eq!(panels.outline_width_percent, 15);
        for _ in 0..20 {
            panels.resize_outline(true);
        }
        assert_eq!(panels.outline_width_percent, 50);
    }
}

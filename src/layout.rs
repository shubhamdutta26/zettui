//! Layout sizing for the two-pane browser and editor.
//! A configured percentage is the desired *files pane* width.
//! On narrow terminals we reserve enough space for both panes when possible.

use ratatui::layout::Rect;

pub const DEFAULT_BROWSER_PERCENT: u16 = 35;
pub const MIN_BROWSER_PERCENT: u16 = 20;
pub const MAX_BROWSER_PERCENT: u16 = 70;
pub const RESIZE_STEP: u16 = 5;
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
}

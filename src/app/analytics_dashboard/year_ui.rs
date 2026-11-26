//! Theme-aware, horizontally windowed contribution heatmap.

use super::{Dashboard, History, calendar_mark, month_ui::activity_background, year_cells};
use crate::{system_theme::readable, theme::Theme};
use chrono::{Datelike, NaiveDate};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

const LABEL_WIDTH: u16 = 4;
const MAX_WIDTH: u16 = 168;

#[derive(Debug, PartialEq, Eq)]
struct Window {
    start: usize,
    end: usize,
    cell_width: u16,
}

impl Window {
    fn new(weeks: usize, selected: usize, columns: u16) -> Self {
        // Wide tiles occupy three terminal cells, with a half-cell of
        // padding on each edge. The colored portion is two cells wide.
        // Narrow tiles occupy one cell with a full-cell gap.
        let cell_width = if usize::from(columns) >= weeks * 3 {
            2
        } else {
            1
        };
        let visible = if cell_width == 2 {
            weeks
        } else {
            usize::from(columns).div_ceil(2).max(1).min(weeks)
        };
        let start = selected.saturating_sub(visible / 2).min(weeks - visible);
        Self {
            start,
            end: start + visible,
            cell_width,
        }
    }

    fn pitch(&self) -> u16 {
        self.cell_width + 1
    }

    fn tile_columns(&self) -> u16 {
        if self.cell_width == 2 { 3 } else { 1 }
    }

    fn width(&self) -> u16 {
        (self.end - self.start - 1) as u16 * self.pitch() + self.tile_columns()
    }
}

fn accent(theme: Theme) -> Style {
    Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD)
}

fn muted(theme: Theme) -> Style {
    Style::default().fg(theme.muted)
}

fn panel(title: String, theme: Theme) -> Block<'static> {
    Block::bordered()
        .title(format!(" {title} "))
        .title_style(accent(theme))
        .border_style(Style::default().fg(super::panel_border(theme)))
        .style(theme.base_style())
}

fn month_labels(weeks: &[[Option<NaiveDate>; 7]], window: &Window) -> Vec<(u16, String)> {
    let mut candidates = Vec::new();
    for (column, week) in weeks[window.start..window.end].iter().enumerate() {
        let date = week
            .iter()
            .flatten()
            .find(|date| date.day() == 1)
            .or_else(|| {
                if column == 0 {
                    week.iter().flatten().next()
                } else {
                    None
                }
            });
        if let Some(date) = date {
            candidates.push((
                column as u16 * window.pitch(),
                date.format("%b").to_string(),
            ));
        }
    }
    candidates
        .iter()
        .enumerate()
        .filter_map(|(index, (x, label))| {
            let limit = candidates
                .get(index + 1)
                .map_or(window.width(), |next| next.0);
            (x + 3 <= limit).then(|| (*x, label.clone()))
        })
        .collect()
}

fn tile(mark: char, selected: bool, today: bool, width: u16, theme: Theme) -> Span<'static> {
    let bg = activity_background(mark, theme);
    let fg = readable(theme.foreground, bg, theme.foreground, true);
    let style = Style::default().fg(fg).bg(bg);
    let symbol = if selected {
        '*'
    } else if today {
        'o'
    } else if matches!(mark, '+' | '-') {
        mark
    } else {
        ' '
    };
    let text = format!(
        "{symbol}{}",
        " ".repeat(usize::from(width.saturating_sub(1)))
    );
    Span::styled(text, style)
}

fn heatmap_tile(
    mark: char,
    selected: bool,
    today: bool,
    wide: bool,
    theme: Theme,
) -> Line<'static> {
    let center = tile(mark, selected, today, 1, theme);
    if !wide {
        return Line::from(center);
    }
    // A two-column solid rectangle has no middle character cell. Instead
    // use the inner halves of the outer columns and a full center column.
    // This preserves the original visual width and centers ASCII markers.
    let edge = theme.base_style().fg(activity_background(mark, theme));
    Line::from(vec![
        Span::styled("▐", edge),
        center,
        Span::styled("▌", edge),
    ])
}

fn legend(theme: Theme, narrow: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut spans = vec![Span::raw("Words: ")];
    for (index, (mark, label)) in [
        ('·', "0"),
        ('░', "1–249"),
        ('▒', "250–499"),
        ('▓', "500–999"),
        ('█', "1000+"),
    ]
    .into_iter()
    .enumerate()
    {
        if narrow && matches!(index, 2 | 4) {
            lines.push(Line::from(spans));
            spans = vec![Span::raw("  ")];
        }
        spans.push(tile(mark, false, false, 2, theme));
        spans.push(Span::raw(format!(" {label}  ")));
    }
    lines.push(Line::from(spans));
    if narrow {
        lines.push(Line::styled("* Selected   o Today", muted(theme)));
        lines.push(Line::styled("+ Chars only   - Deletion only", muted(theme)));
    } else {
        lines.push(Line::styled(
            "* Selected   o Today   + Chars only   - Deletion only",
            muted(theme),
        ));
    }
    lines.push(Line::styled("Blank: future / outside year", muted(theme)));
    lines
}

/// Returns false only when there is not enough space to render the full height
/// of the grid and selected-day details. Width is handled by a week window.
pub(super) fn draw(
    frame: &mut Frame,
    theme: Theme,
    body: Rect,
    dashboard: &Dashboard,
    history: &History,
    warning: Option<&str>,
) -> bool {
    let width = body.width.min(MAX_WIDTH);
    let narrow = width < 58;
    let legend_lines = legend(theme, narrow);
    let legend_height = legend_lines.len() as u16;
    let detail_height = if narrow { 6 } else { 4 };
    let warning_height = u16::from(warning.is_some());
    let fixed = warning_height + 1 + legend_height + detail_height;
    // One top border, month labels, seven weekday rows, bottom border.
    if width < 30 || body.height < fixed + 10 {
        return false;
    }
    let stride = if body.height >= fixed + 16 { 2 } else { 1 };
    let grid_height = 4 + 6 * stride;
    let x = body.x + (body.width - width) / 2;
    let mut y = body.y;
    if let Some(message) = warning {
        frame.render_widget(
            Paragraph::new(format!("Stats unsaved: {message}"))
                .style(Style::default().fg(theme.warning)),
            Rect::new(x, y, width, 1),
        );
        y += 1;
    }
    let weeks = year_cells(dashboard.selected);
    let selected = weeks
        .iter()
        .position(|week| week.contains(&Some(dashboard.selected)))
        .unwrap_or(0);
    let window = Window::new(weeks.len(), selected, width - 2 - LABEL_WIDTH);
    let week_label = if narrow {
        format!(
            "Weeks {}–{} of {} (Mon)",
            window.start + 1,
            window.end,
            weeks.len()
        )
    } else {
        format!(
            "Weeks {}–{} of {} · Monday start",
            window.start + 1,
            window.end,
            weeks.len()
        )
    };
    frame.render_widget(
        Paragraph::new(week_label).style(muted(theme)).centered(),
        Rect::new(x, y, width, 1),
    );
    y += 1;
    let area = Rect::new(x, y, width, grid_height);
    let block = panel(
        format!("{} · Writing heatmap", dashboard.selected.year()),
        theme,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let used = LABEL_WIDTH + window.width();
    let left = inner.x + (inner.width - used) / 2;
    let grid_x = left + LABEL_WIDTH;
    for (offset, label) in month_labels(&weeks, &window) {
        frame.render_widget(
            Paragraph::new(label).style(muted(theme)),
            Rect::new(grid_x + offset, inner.y, 3, 1),
        );
    }
    for (weekday, label) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .into_iter()
        .enumerate()
    {
        let row = inner.y + 1 + weekday as u16 * stride;
        frame.render_widget(
            Paragraph::new(label).style(muted(theme)),
            Rect::new(left, row, LABEL_WIDTH, 1),
        );
        for (column, week) in weeks[window.start..window.end].iter().enumerate() {
            let Some(date) = week[weekday] else { continue };
            if date > dashboard.today {
                continue;
            }
            let mark = calendar_mark(date, dashboard.today, history);

            let tile_area = Rect::new(
                grid_x + column as u16 * window.pitch(),
                row,
                window.tile_columns(),
                1,
            );
            frame.render_widget(
                Paragraph::new(heatmap_tile(
                    mark,
                    date == dashboard.selected,
                    date == dashboard.today,
                    window.cell_width == 2,
                    theme,
                )),
                tile_area,
            );
        }
    }
    y += grid_height;
    frame.render_widget(
        Paragraph::new(legend_lines),
        Rect::new(x, y, width, legend_height),
    );
    y += legend_height;
    let a = history
        .get(&dashboard.selected)
        .copied()
        .unwrap_or_default();
    let details = panel(
        format!("Selected: {}", dashboard.selected.format("%a %Y-%m-%d")),
        theme,
    );
    let area = Rect::new(x, y, width, detail_height);
    let inner = details.inner(area);
    frame.render_widget(details, area);
    let lines = if narrow {
        vec![
            Line::from(format!("Words +{}", a.words_added)),
            Line::from(format!("Words -{}", a.words_deleted)),
            Line::from(format!("Chars +{}", a.characters_added)),
            Line::from(format!("Chars -{}", a.characters_deleted)),
        ]
    } else {
        vec![
            Line::from(format!("Words +{} / -{}", a.words_added, a.words_deleted)),
            Line::from(format!(
                "Chars +{} / -{}",
                a.characters_added, a.characters_deleted
            )),
        ]
    };
    frame.render_widget(Paragraph::new(lines), inner);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use zettui::{
        analytics::{Goals, report},
        statistics::Activity,
    };

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn window_keeps_selected_week_visible_at_every_width() {
        for weeks in [53, 54] {
            for columns in 24..=162 {
                for selected in 0..weeks {
                    let window = Window::new(weeks, selected, columns);
                    assert!(window.start <= selected && selected < window.end);
                    assert!(window.end <= weeks);
                    assert!(window.width() <= columns);
                    if usize::from(columns) >= weeks * 2 - 1 {
                        assert_eq!((window.start, window.end), (0, weeks));
                    }
                }
            }
        }
    }

    #[test]
    fn labels_do_not_overlap_or_extend_past_window() {
        let weeks = year_cells(date(2012, 6, 1));
        for columns in [24, 54, 108, 162] {
            for selected in 0..weeks.len() {
                let window = Window::new(weeks.len(), selected, columns);
                let labels = month_labels(&weeks, &window);
                assert!(labels.iter().all(|(x, _)| x + 3 <= window.width()));
                assert!(labels.windows(2).all(|pair| pair[0].0 + 3 <= pair[1].0));
            }
        }
    }

    #[test]
    fn selection_preserves_activity_shading() {
        for name in ["dark", "light", "tokyo-night"] {
            let theme = Theme::named(name).unwrap();
            for mark in ['·', '░', '▒', '▓', '█', '+', '-'] {
                let bg = activity_background(mark, theme);
                for (selected, today, expected) in [
                    (true, false, "*"),
                    (true, true, "*"),
                    (false, true, "o"),
                    (
                        false,
                        false,
                        if mark == '+' {
                            "+"
                        } else if mark == '-' {
                            "-"
                        } else {
                            " "
                        },
                    ),
                ] {
                    let wide = heatmap_tile(mark, selected, today, true, theme);
                    assert_eq!(wide.spans.len(), 3);
                    assert_eq!(wide.spans[0].content.as_ref(), "▐");
                    assert_eq!(wide.spans[1].content.as_ref(), expected);
                    assert_eq!(wide.spans[2].content.as_ref(), "▌");
                    assert_eq!(wide.spans[1].style.bg, Some(bg));
                    for edge in [&wide.spans[0], &wide.spans[2]] {
                        assert_eq!(edge.style.fg, Some(bg));
                        assert_eq!(edge.style.bg, theme.base_style().bg);
                    }
                    let narrow = heatmap_tile(mark, selected, today, false, theme);
                    assert_eq!(narrow.spans.len(), 1);
                    assert_eq!(narrow.spans[0].content.as_ref(), expected);
                    assert_eq!(narrow.spans[0].style.bg, Some(bg));
                }
            }
        }
    }

    #[test]
    fn render_year_grid_and_selected_details_at_different_sizes() {
        let today = date(2026, 11, 26);
        let history = History::from([(
            today,
            Activity {
                words_added: 750,
                characters_added: 4500,
                ..Activity::default()
            },
        )]);
        let report = report(&history, today, Goals::default()).unwrap();
        let mut dashboard = Dashboard::new(today, None);
        dashboard.set_view(super::super::View::Year);
        dashboard.snapshot = Ok((history, report));
        for name in ["dark", "light", "tokyo-night"] {
            let theme = Theme::named(name).unwrap();
            for (width, height) in [(168, 40), (120, 30), (60, 30), (35, 28), (30, 8)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|frame| super::super::draw(frame, theme, &mut dashboard, None))
                    .unwrap();
                let buffer = terminal.backend().buffer();
                let text = (0..height)
                    .map(|y| {
                        (0..width)
                            .map(|x| buffer[(x, y)].symbol())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(text.contains("Selected:"));
                assert!(text.contains("750"));
                assert!(text.contains("Esc"));
                if height >= 28 {
                    assert!(text.contains("Writing heatmap"));
                    assert!(text.contains("1000+"));
                    assert!((0..height).any(|y| (0..width).any(|x| {
                        let cell = &buffer[(x, y)];
                        cell.symbol() == "*" && cell.bg == activity_background('▓', theme)
                    })));
                }
            }
        }
    }
}

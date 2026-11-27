//! Responsive, theme-aware monthly writing calendar.

use super::{
    Dashboard, History, activity_mark, month_cells,
    overview_ui::{compact, number},
};
use crate::{
    system_theme::{blend, readable},
    theme::Theme,
};
use chrono::{Datelike, NaiveDate};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
};
use zettui::statistics::Activity;

const MIN_WIDTH: u16 = 56;
const MAX_WIDTH: u16 = 140;
// Month heading, weekday labels, selected-day panel (4 rows), legend (2 rows).
const FIXED_ROWS: u16 = 8;

fn accent(theme: Theme) -> Style {
    Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD)
}

fn active_weeks(cells: &[Option<NaiveDate>]) -> u16 {
    cells
        .iter()
        .rposition(Option::is_some)
        .map_or(0, |i| (i / 7 + 1) as u16)
}

fn cell_rect(grid: Rect, index: usize, height: u16) -> Rect {
    let column = (index % 7) as u16;
    let start = u32::from(grid.width) * u32::from(column) / 7;
    let end = u32::from(grid.width) * u32::from(column + 1) / 7;
    Rect::new(
        grid.x + start as u16,
        grid.y + (index / 7) as u16 * height,
        (end - start) as u16,
        height,
    )
}

// Fixed bands match the existing calendar. The tint uses the active theme's
// accent color; text is contrast-adjusted for light and dark palettes.
pub(super) fn activity_background(mark: char, theme: Theme) -> Color {
    match mark {
        '░' => blend(theme.surface, theme.accent, 0.18),
        '▒' => blend(theme.surface, theme.accent, 0.35),
        '▓' => blend(theme.surface, theme.accent, 0.55),
        '█' => blend(theme.surface, theme.accent, 0.78),
        '+' => blend(theme.surface, theme.accent, 0.22),
        '-' => blend(theme.surface, theme.warning, 0.22),
        ' ' => theme.background,
        _ => theme.surface,
    }
}

fn cell_count(activity: Activity, mark: char, width: usize) -> String {
    match mark {
        ' ' => "—".to_owned(),
        '+' | '-' => format!("0{mark}"),
        _ => {
            let full = number(activity.words_added);
            if full.len() <= width {
                full
            } else {
                compact(activity.words_added, width)
            }
        }
    }
}

fn draw_cell(
    frame: &mut Frame,
    area: Rect,
    date: NaiveDate,
    dashboard: &Dashboard,
    history: &History,
    theme: Theme,
) {
    let future = date > dashboard.today;
    let activity = if future {
        Activity::default()
    } else {
        history.get(&date).copied().unwrap_or_default()
    };
    let mark = if future { ' ' } else { activity_mark(activity) };
    let selected = date == dashboard.selected;
    let today = date == dashboard.today;
    let bg = activity_background(mark, theme);
    let fg = readable(
        if future || mark == '·' {
            theme.muted
        } else {
            theme.foreground
        },
        bg,
        theme.foreground,
        true,
    );
    let border = if selected {
        readable(theme.accent, bg, theme.foreground, true)
    } else {
        super::panel_border(theme)
    };
    let base = Style::default().fg(fg).bg(bg);
    let block = Block::bordered()
        .border_type(if selected {
            BorderType::Double
        } else {
            BorderType::Plain
        })
        .style(theme.base_style())
        .border_style(Style::default().fg(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // Shade only the interior; border cells retain the dashboard background.
    frame.render_widget(Block::default().style(base), inner);
    let date_style = if today {
        base.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else if selected {
        base.add_modifier(Modifier::BOLD)
    } else {
        base
    };
    let count_style = if selected {
        base.add_modifier(Modifier::BOLD)
    } else {
        base
    };
    let width = usize::from(inner.width);
    if inner.height >= 2 {
        let date_label = if today {
            format!("{}*", date.day())
        } else {
            date.day().to_string()
        };
        let count = cell_count(activity, mark, width);
        let label = if width >= count.len() + 2 && !matches!(mark, ' ' | '+' | '-') {
            format!("{count} w")
        } else {
            count
        };
        let text_area = Rect::new(inner.x, inner.y + (inner.height - 2) / 2, inner.width, 2);
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(date_label, date_style),
                Line::styled(label, count_style),
            ])
            .style(base)
            .centered(),
            text_area,
        );
    } else {
        let count = cell_count(activity, mark, width.saturating_sub(3));
        let label = Line::from(vec![
            Span::styled(format!("{:>2} ", date.day()), date_style),
            Span::styled(count, count_style),
        ]);
        frame.render_widget(Paragraph::new(label).style(base).centered(), inner);
    }
}

fn legend(theme: Theme) -> Vec<Line<'static>> {
    let swatch = |mark, label: &'static str| {
        let bg = activity_background(mark, theme);
        [
            Span::styled("  ", Style::default().bg(bg)),
            Span::raw(label),
        ]
    };
    let mut first = vec![Span::raw("Words: ")];
    for (mark, label) in [
        ('·', " 0  "),
        ('░', " 1–249  "),
        ('▒', " 250–499  "),
        ('▓', " 500–999  "),
        ('█', " 1000+"),
    ] {
        first.extend(swatch(mark, label));
    }
    vec![
        Line::from(first),
        Line::styled(
            "Today: underlined   0+ Chars   0- Deletions   — Future",
            Style::default().fg(theme.muted),
        ),
    ]
}

// Match the Overview's header without changing the approved Overview renderer.
pub(super) fn header(
    frame: &mut Frame,
    area: Rect,
    today: NaiveDate,
    theme: Theme,
    view: super::View,
) {
    frame.render_widget(Clear, area);
    let title = "Writing Analytics";
    let date = today.format("%b %d, %Y").to_string();
    let gap = usize::from(area.width).saturating_sub(title.len() + date.len());
    let muted = Style::default().fg(theme.muted);
    let mut first = vec![Span::styled(title, accent(theme))];
    if gap > 0 {
        first.extend([Span::raw(" ".repeat(gap)), Span::styled(date, muted)]);
    }
    let active = Style::default()
        .fg(readable(
            theme.background,
            theme.accent,
            theme.foreground,
            true,
        ))
        .bg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let labels = if area.width >= 37 {
        [" 1 Overview ", " 2 Month ", " 3 Year "]
    } else {
        ["1 Overview", "2 Month", "3 Year"]
    };
    let tabs = Line::from(vec![
        Span::styled(labels[0], muted),
        Span::raw(" "),
        Span::styled(
            labels[1],
            if view == super::View::Month {
                active
            } else {
                muted
            },
        ),
        Span::raw(" "),
        Span::styled(
            labels[2],
            if view == super::View::Year {
                active
            } else {
                muted
            },
        ),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::from(first), tabs]).style(theme.base_style()),
        area,
    );
}

/// Return false when the legacy compact view is more appropriate. Never clip
/// a calendar week or the selected-day panel to fit a short viewport.
pub(super) fn draw(
    frame: &mut Frame,
    theme: Theme,
    body: Rect,
    dashboard: &Dashboard,
    history: &History,
    warning: Option<&str>,
) -> bool {
    let cells = month_cells(dashboard.selected);
    let weeks = active_weeks(&cells);
    let warning_rows = if warning.is_some() { 1 } else { 0 };
    let available = body.height.saturating_sub(FIXED_ROWS + warning_rows);
    if body.width < MIN_WIDTH + 2 || weeks == 0 || available / weeks < 3 {
        return false;
    }
    let height = (available / weeks).min(6);
    let width = (body.width - 2).min(MAX_WIDTH);
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
    frame.render_widget(
        Paragraph::new(dashboard.selected.format("%B %Y").to_string())
            .style(accent(theme))
            .centered(),
        Rect::new(x, y, width, 1),
    );
    let grid = Rect::new(x, y + 2, width, weeks * height);
    for (i, name) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .into_iter()
        .enumerate()
    {
        let column = cell_rect(grid, i, height);
        frame.render_widget(
            Paragraph::new(name)
                .style(Style::default().fg(theme.muted))
                .centered(),
            Rect::new(column.x, y + 1, column.width, 1),
        );
    }
    for (index, date) in cells.into_iter().enumerate() {
        if let Some(date) = date {
            draw_cell(
                frame,
                cell_rect(grid, index, height),
                date,
                dashboard,
                history,
                theme,
            );
        }
    }
    let a = history
        .get(&dashboard.selected)
        .copied()
        .unwrap_or_default();
    let details = Block::bordered()
        .title(format!(
            " Selected: {} ",
            dashboard.selected.format("%a %b %d, %Y")
        ))
        .title_style(accent(theme))
        .border_style(Style::default().fg(super::panel_border(theme)))
        .style(theme.base_style());
    let area = Rect::new(x, grid.y + grid.height, width, 4);
    let inner = details.inner(area);
    frame.render_widget(details, area);
    // Use the full selected-day counts, not the abbreviated grid values.
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Words +{} / -{}", a.words_added, a.words_deleted)),
            Line::from(format!(
                "Chars +{} / -{}",
                a.characters_added, a.characters_deleted
            )),
        ]),
        inner,
    );
    frame.render_widget(
        Paragraph::new(legend(theme)),
        Rect::new(x, area.y + area.height, width, 2),
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use zettui::analytics::{Goals, report};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn grid_uses_only_needed_weeks_and_all_columns() {
        for (selected, weeks) in [
            (date(2021, 2, 1), 4),
            (date(2026, 11, 26), 6),
            (date(2024, 2, 29), 5),
        ] {
            assert_eq!(active_weeks(&month_cells(selected)), weeks);
        }
        for width in [56, 57, 89, 140] {
            let grid = Rect::new(2, 4, width, 24);
            let columns: Vec<_> = (0..7).map(|i| cell_rect(grid, i, 4)).collect();
            assert_eq!(columns[0].x, grid.x);
            assert_eq!(columns[6].x + columns[6].width, grid.x + width);
            assert!(columns.windows(2).all(|p| p[0].x + p[0].width == p[1].x));
            assert_eq!(cell_rect(grid, 41, 4).y + 4, grid.y + grid.height);
        }
    }

    #[test]
    fn bands_and_special_counts_are_distinct() {
        let theme = Theme::named("dark").unwrap();
        let backgrounds: Vec<_> = ['·', '░', '▒', '▓', '█']
            .into_iter()
            .map(|mark| activity_background(mark, theme))
            .collect();
        assert!(backgrounds.windows(2).all(|p| p[0] != p[1]));
        assert_eq!(cell_count(Activity::default(), ' ', 4), "—");
        assert_eq!(cell_count(Activity::default(), '+', 4), "0+");
        assert_eq!(cell_count(Activity::default(), '-', 4), "0-");
        assert_eq!(
            cell_count(
                Activity {
                    words_added: 1200,
                    ..Activity::default()
                },
                '█',
                6
            ),
            "1,200"
        );
    }

    #[test]
    fn day_shading_stays_inside_border() {
        let today = date(2026, 11, 26);
        let yesterday = today.pred_opt().unwrap();
        let dashboard = Dashboard::new(today, None);
        let history = History::from([(
            today,
            Activity {
                words_added: 750,
                ..Activity::default()
            },
        )]);
        for name in ["dark", "light", "tokyo-night"] {
            let theme = Theme::named(name).unwrap();
            let mut terminal = Terminal::new(TestBackend::new(24, 6)).unwrap();
            let selected = Rect::new(0, 0, 12, 6);
            let inactive = Rect::new(12, 0, 12, 6);
            terminal
                .draw(|frame| {
                    draw_cell(frame, selected, today, &dashboard, &history, theme);
                    draw_cell(frame, inactive, yesterday, &dashboard, &history, theme);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            for (area, mark) in [(selected, '▓'), (inactive, '·')] {
                for y in area.y..area.y + area.height {
                    for x in area.x..area.x + area.width {
                        let on_border = x == area.x
                            || x == area.x + area.width - 1
                            || y == area.y
                            || y == area.y + area.height - 1;
                        let expected = if on_border {
                            theme.base_style().bg.unwrap_or(Color::Reset)
                        } else {
                            activity_background(mark, theme)
                        };
                        assert_eq!(buffer[(x, y)].bg, expected, "cell ({x}, {y})");
                    }
                }
            }
        }
    }

    #[test]
    fn month_renders_grid_details_and_compact_fallback() {
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
        dashboard.set_view(super::super::View::Month);
        dashboard.snapshot = Ok((history, report));
        for name in ["dark", "light"] {
            let theme = Theme::named(name).unwrap();
            for (width, height) in [(100, 40), (60, 30), (35, 20), (30, 8)] {
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
                if width >= 60 {
                    assert!(text.contains("November 2026"));
                    assert!(text.contains('┌'));
                    assert!(text.contains("1000+"));
                    assert!(text.contains('╔'));
                    assert!(text.contains("4500"));
                }
            }
        }
    }

    #[test]
    fn activity_bands_follow_theme_accent() {
        for name in ["dark", "light", "tokyo-night"] {
            let theme = Theme::named(name).unwrap();

            for (mark, intensity) in [('░', 0.18), ('▒', 0.35), ('▓', 0.55), ('█', 0.78)] {
                assert_eq!(
                    activity_background(mark, theme),
                    blend(theme.surface, theme.accent, intensity)
                );
            }
        }
    }
}

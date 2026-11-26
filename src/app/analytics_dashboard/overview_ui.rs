//! Theme-aware overview panels rendered to a scrollable terminal-cell canvas.

use super::{Dashboard, History};
use crate::theme::Theme;
use chrono::{Days, NaiveDate};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget},
};
use zettui::analytics::{GoalProgress, PeriodSummary, Report};

const PLOT_ROWS: u16 = 6;
const GOAL_HEIGHT: u16 = 10;

pub(super) fn number(value: u64) -> String {
    let digits = value.to_string();
    let mut result = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i != 0 && (digits.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

fn muted(theme: Theme) -> Style {
    Style::default().fg(theme.muted)
}

fn emphasis(theme: Theme) -> Style {
    Style::default()
        .fg(theme.foreground)
        .add_modifier(Modifier::BOLD)
}

fn accent(theme: Theme) -> Style {
    Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD)
}

fn panel(buffer: &mut Buffer, area: Rect, title: &str, theme: Theme) -> Rect {
    let block = Block::bordered()
        .title(format!(" {title} "))
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(super::panel_border(theme)))
        .title_style(accent(theme))
        .padding(Padding::horizontal(1))
        .style(theme.base_style());
    let inner = block.inner(area);
    block.render(area, buffer);
    inner
}

fn metric(buffer: &mut Buffer, area: Rect, label: &str, value: u64, unit: &str, theme: Theme) {
    let inner = panel(buffer, area, label, theme);
    let formatted = number(value);
    let value = if formatted.len() <= usize::from(inner.width) {
        formatted
    } else {
        compact(value, usize::from(inner.width))
    };
    Paragraph::new(vec![
        Line::styled(value, emphasis(theme)),
        Line::styled(unit.to_owned(), muted(theme)),
    ])
    .centered()
    .render(inner, buffer);
}

fn goal_bar_units(goal: GoalProgress, width: usize) -> usize {
    if goal.target == 0 || goal.written == 0 || width == 0 {
        return 0;
    }
    let capacity = (width as u128) * 8;
    let units =
        (capacity * u128::from(goal.written.min(goal.target)) / u128::from(goal.target)).max(1);
    units as usize
}

fn goal_lines(
    label: &str,
    goal: Option<GoalProgress>,
    width: u16,
    theme: Theme,
) -> Vec<Line<'static>> {
    let Some(goal) = goal else {
        return vec![
            Line::styled(format!("{label} goal"), emphasis(theme)),
            Line::styled("Disabled (config.toml)", muted(theme)),
            Line::default(),
            Line::default(),
        ];
    };
    let width = usize::from(width);
    let units = goal_bar_units(goal, width);
    let filled = units / 8;
    let fraction = units % 8;
    let color = if goal.reached() {
        theme.success
    } else {
        theme.accent
    };
    let count = format!("{} / {} words", number(goal.written), number(goal.target));
    let count = if count.len() <= width {
        count
    } else {
        // Keep the bar full width on narrow terminals; exact activity is in the totals.
        let half = width.saturating_sub(9) / 2;
        format!(
            "{} / {} words",
            compact(goal.written, half),
            compact(goal.target, half)
        )
    };
    vec![
        Line::styled(
            format!("{label} goal · {}", super::goal_percentage(goal)),
            emphasis(theme),
        ),
        Line::from(count),
        {
            let mut bar = vec![Span::styled(" ".repeat(filled), Style::default().bg(color))];
            if fraction > 0 {
                let glyph = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉"][fraction];
                bar.push(Span::styled(
                    glyph,
                    Style::default().fg(color).bg(theme.border),
                ));
            }
            let rest = width - filled - usize::from(fraction > 0);
            bar.push(Span::styled(
                " ".repeat(rest),
                Style::default().bg(theme.border),
            ));
            Line::from(bar)
        },
        if goal.reached() {
            Line::styled("Goal reached", Style::default().fg(theme.success))
        } else {
            Line::styled(
                format!(
                    "{} remaining",
                    compact(goal.remaining(), width.saturating_sub(10))
                ),
                muted(theme),
            )
        },
    ]
}

pub(super) fn compact(value: u64, width: usize) -> String {
    let raw = value.to_string();
    if raw.len() <= width {
        return raw;
    }
    for (divisor, suffix) in [
        (1_000_000_000_000_000_000, "E"),
        (1_000_000_000_000_000, "P"),
        (1_000_000_000_000, "T"),
        (1_000_000_000, "B"),
        (1_000_000, "M"),
        (1_000, "k"),
    ] {
        if value >= divisor {
            let whole = value / divisor;
            let decimal = value % divisor / (divisor / 10);
            let precise = format!("{whole}.{decimal}{suffix}");
            if decimal != 0 && precise.len() <= width {
                return precise;
            }
            let short = format!("{whole}{suffix}");
            if short.len() <= width {
                return short;
            }
        }
    }
    "…".to_owned()
}

fn week_days(history: &History, today: NaiveDate) -> [(Option<NaiveDate>, u64); 7] {
    std::array::from_fn(|index| {
        let date = today.checked_sub_days(Days::new(6 - index as u64));
        let count = date
            .and_then(|d| history.get(&d))
            .map_or(0, |a| a.words_added);
        (date, count)
    })
}

fn plot_units(value: u64, peak: u64) -> u16 {
    if value == 0 || peak == 0 {
        return 0;
    }
    let units = u128::from(value) * u128::from(PLOT_ROWS * 8) / u128::from(peak);
    units.clamp(1, u128::from(PLOT_ROWS * 8)) as u16
}

fn chart_lines(
    history: &History,
    today: NaiveDate,
    width: u16,
    theme: Theme,
) -> Vec<Line<'static>> {
    let days = week_days(history, today);
    let peak = days.iter().map(|(_, count)| *count).max().unwrap_or(0);
    let axis = if width >= 52 { 7 } else { 0 };
    let available = usize::from(width).saturating_sub(axis);
    let column = (available / 7).max(1);
    let leading = axis + available.saturating_sub(column * 7) / 2;
    let bar_width = column.saturating_sub(1).clamp(1, 4);
    let glyphs = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let mut lines = Vec::new();
    for row in (0..PLOT_ROWS).rev() {
        let tick = if axis != 0 && row == PLOT_ROWS - 1 {
            compact(peak, axis - 1)
        } else if axis != 0 && row == PLOT_ROWS / 2 - 1 {
            compact(peak / 2, axis - 1)
        } else {
            String::new()
        };
        let mut spans = vec![Span::styled(format!("{tick:>leading$}"), muted(theme))];
        for (index, (_, count)) in days.iter().enumerate() {
            let units = plot_units(*count, peak).saturating_sub(row * 8).min(8);
            let bar = glyphs[usize::from(units)].to_string().repeat(bar_width);
            let color = if index == 6 {
                theme.foreground
            } else {
                theme.accent
            };
            spans.push(Span::styled(
                format!("{bar:^column$}"),
                Style::default().fg(color),
            ));
        }
        lines.push(Line::from(spans));
    }
    let baseline = if axis == 0 {
        " ".repeat(leading)
    } else {
        format!("{:>leading$}", "0")
    };
    lines.push(Line::styled(
        format!("{baseline}{}", "─".repeat(column * 7)),
        muted(theme),
    ));
    let mut labels = vec![Span::raw(" ".repeat(leading))];
    let mut values = vec![Span::raw(" ".repeat(leading))];
    for (index, (date, count)) in days.iter().enumerate() {
        let label = date.map_or_else(|| "–".to_owned(), |d| d.format("%a").to_string());
        let label = if column < 3 {
            label.chars().take(column).collect::<String>()
        } else {
            label
        };
        let style = if index == 6 {
            emphasis(theme)
        } else {
            muted(theme)
        };
        labels.push(Span::styled(format!("{label:^column$}"), style));
        let value = compact(*count, column);
        values.push(Span::styled(format!("{value:^column$}"), style));
    }
    lines.extend([Line::from(labels), Line::from(values)]);
    // If compact axis labels were necessary, preserve all exact daily values below the plot.
    if days
        .iter()
        .any(|(_, count)| count.to_string().len() > column)
    {
        for (date, count) in days {
            if let Some(date) = date {
                let grouped = number(count);
                let value = if grouped.len() + 8 <= usize::from(width) {
                    grouped
                } else {
                    count.to_string()
                };
                lines.push(Line::from(format!("{}  {value}", date.format("%a %d"))));
            }
        }
    }
    lines
}

fn pair(lines: &mut Vec<Line<'static>>, label: &str, added: u64, deleted: u64, width: u16) {
    let combined = format!("{label} +{} / -{}", number(added), number(deleted));
    if combined.len() <= usize::from(width) {
        lines.push(Line::from(combined));
    } else {
        // Un-grouped decimal integers fit even in the minimum-width panel.
        lines.push(Line::from(format!("{label} +{added}")));
        lines.push(Line::from(format!("{label} -{deleted}")));
    }
}

fn summary_lines(summary: PeriodSummary, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let a = summary.totals;
    pair(&mut lines, "Words", a.words_added, a.words_deleted, width);
    pair(
        &mut lines,
        "Chars",
        a.characters_added,
        a.characters_deleted,
        width,
    );
    let average = format!("{:.1} words/day", summary.average_words());
    let active = format!(
        "{} {} active",
        summary.writing_days,
        super::day_unit(summary.writing_days)
    );
    if average.len() + active.len() + 3 <= usize::from(width) {
        lines.push(Line::from(format!("{average} · {active}")));
    } else {
        lines.extend([Line::from(average), Line::from(active)]);
    }
    lines
}

fn lifetime_lines(report: &Report, width: u16, theme: Theme) -> Vec<Line<'static>> {
    let summary = report.all_time;
    let a = summary.totals;
    let groups = [
        [
            "Words".to_owned(),
            format!("+{} added", number(a.words_added)),
            format!("-{} deleted", number(a.words_deleted)),
        ],
        [
            "Characters".to_owned(),
            format!("+{} added", number(a.characters_added)),
            format!("-{} deleted", number(a.characters_deleted)),
        ],
        [
            "Writing activity".to_owned(),
            format!("{:.1} words/day", summary.average_words()),
            format!(
                "{} {} active",
                number(summary.writing_days),
                super::day_unit(summary.writing_days)
            ),
        ],
        [
            "Longest streak".to_owned(),
            format!(
                "{} {}",
                number(report.streaks.longest),
                super::day_unit(report.streaks.longest)
            ),
            String::new(),
        ],
    ];
    let required = groups.iter().flatten().map(String::len).max().unwrap_or(0);
    let width = usize::from(width);
    let columns = [4, 2]
        .into_iter()
        .find(|&n| width >= n * required.max(18) + (n - 1) * 2);
    let Some(columns) = columns else {
        let mut lines = summary_lines(summary, width as u16);
        lines.push(Line::from(format!(
            "Longest streak: {} {}",
            number(report.streaks.longest),
            super::day_unit(report.streaks.longest)
        )));
        return lines;
    };
    let cell_width = (width - (columns - 1) * 2) / columns;
    let mut lines = Vec::new();
    for (index, group_row) in groups.chunks(columns).enumerate() {
        if index != 0 {
            lines.push(Line::default());
        }
        for (row, _) in group_row[0].iter().enumerate() {
            let spans: Vec<_> = group_row
                .iter()
                .enumerate()
                .map(|(i, group)| {
                    let size = if i + 1 == group_row.len() {
                        cell_width
                    } else {
                        cell_width + 2
                    };
                    Span::styled(
                        format!("{:<size$}", group[row]),
                        if row == 0 {
                            muted(theme)
                        } else {
                            emphasis(theme)
                        },
                    )
                })
                .collect();
            lines.push(Line::from(spans));
        }
    }
    lines
}

fn activity_lines(report: &Report, width: u16, theme: Theme) -> Vec<Line<'static>> {
    let periods = [
        ("Today", report.today),
        ("Week", report.week),
        ("Month", report.month),
        ("Year", report.year),
    ];
    let headers = [
        "Period", "Words +", "Words -", "Chars +", "Chars -", "Avg/day", "Active",
    ]
    .map(str::to_owned);
    let rows: Vec<[String; 7]> = periods
        .iter()
        .map(|(label, s)| {
            let a = s.totals;
            [
                (*label).to_owned(),
                number(a.words_added),
                number(a.words_deleted),
                number(a.characters_added),
                number(a.characters_deleted),
                format!("{:.1}", s.average_words()),
                number(s.writing_days),
            ]
        })
        .collect();
    let mut widths = std::array::from_fn::<_, 7, _>(|i| headers[i].len());
    for row in &rows {
        for (i, value) in row.iter().enumerate() {
            widths[i] = widths[i].max(value.len());
        }
    }
    let minimum = widths.iter().sum::<usize>() + 12;
    let available = usize::from(width);
    if available >= minimum {
        // Distribute spare columns between fields, with exact right alignment.
        let extra = available - minimum;
        let format_row = |row: &[String; 7]| {
            let mut text = String::new();
            for (i, value) in row.iter().enumerate() {
                let size = widths[i];
                if i == 0 {
                    text.push_str(&format!("{value:<size$}"));
                } else {
                    text.push_str(&format!("{value:>size$}"));
                }
                if i < 6 {
                    let gap = 2 + extra / 6 + usize::from(i < extra % 6);
                    text.push_str(&" ".repeat(gap));
                }
            }
            text
        };
        let mut lines = vec![Line::styled(format_row(&headers), muted(theme))];
        lines.extend(rows.iter().map(|row| Line::from(format_row(row))));
        return lines;
    }
    let mut lines = Vec::new();
    for (index, (label, summary)) in periods.into_iter().enumerate() {
        if index != 0 {
            lines.push(Line::default());
        }
        lines.push(Line::styled(label, accent(theme)));
        lines.extend(summary_lines(summary, width));
    }
    lines
}

fn overview_canvas(
    width: u16,
    report: &Report,
    history: &History,
    today: NaiveDate,
    theme: Theme,
) -> Buffer {
    let columns: u16 = if width >= 96 {
        4
    } else if width >= 34 {
        2
    } else {
        1
    };
    let metrics_height = (4 / columns) * 4 + (4 / columns - 1);
    let side_by_side = width >= 86;
    let left_width = if side_by_side { (width - 1) / 2 } else { width };
    let right_width = if side_by_side {
        width - left_width - 1
    } else {
        width
    };
    let chart = chart_lines(history, today, right_width - 4, theme);
    let chart_height = chart.len() as u16 + 2;
    let middle_height = if side_by_side {
        GOAL_HEIGHT.max(chart_height)
    } else {
        GOAL_HEIGHT + 1 + chart_height
    };
    let lifetime = lifetime_lines(report, width - 4, theme);
    let lifetime_height = lifetime.len() as u16 + 2;
    let activity = activity_lines(report, width - 4, theme);
    let activity_height = activity.len() as u16 + 2;
    let height = metrics_height + 1 + middle_height + 1 + lifetime_height + 1 + activity_height;
    let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
    Block::default()
        .style(theme.base_style())
        .render(buffer.area, &mut buffer);
    let metrics = [
        (
            "Words today",
            report.today.totals.words_added,
            "words added",
        ),
        ("This week", report.week.totals.words_added, "words added"),
        ("This month", report.month.totals.words_added, "words added"),
        (
            if columns == 2 && width < 42 {
                "Streak"
            } else {
                "Current streak"
            },
            report.streaks.current,
            super::day_unit(report.streaks.current),
        ),
    ];
    let usable = width - (columns - 1);
    for (index, (label, value, unit)) in metrics.into_iter().enumerate() {
        let col = index as u16 % columns;
        let start = usable * col / columns + col;
        let end = usable * (col + 1) / columns + col;
        let area = Rect::new(start, index as u16 / columns * 5, end - start, 4);
        metric(&mut buffer, area, label, value, unit, theme);
    }
    let y = metrics_height + 1;
    let goals = panel(
        &mut buffer,
        Rect::new(
            0,
            y,
            left_width,
            if side_by_side {
                middle_height
            } else {
                GOAL_HEIGHT
            },
        ),
        "Writing goals",
        theme,
    );
    let mut goal_text = goal_lines("Daily", report.daily_goal, goals.width, theme);
    goal_text.extend(goal_lines("Weekly", report.weekly_goal, goals.width, theme));
    Paragraph::new(goal_text).render(goals, &mut buffer);
    let (chart_x, chart_y) = if side_by_side {
        (left_width + 1, y)
    } else {
        (0, y + GOAL_HEIGHT + 1)
    };
    let chart_area = panel(
        &mut buffer,
        Rect::new(
            chart_x,
            chart_y,
            right_width,
            if side_by_side {
                middle_height
            } else {
                chart_height
            },
        ),
        "Last 7 days · words added",
        theme,
    );
    Paragraph::new(chart).render(chart_area, &mut buffer);
    let y = y + middle_height + 1;
    let inner = panel(
        &mut buffer,
        Rect::new(0, y, width, lifetime_height),
        "Lifetime statistics",
        theme,
    );
    Paragraph::new(lifetime).render(inner, &mut buffer);
    let inner = panel(
        &mut buffer,
        Rect::new(0, y + lifetime_height + 1, width, activity_height),
        "Activity totals",
        theme,
    );
    Paragraph::new(activity).render(inner, &mut buffer);
    buffer
}

fn header(frame: &mut Frame, area: Rect, today: NaiveDate, theme: Theme) {
    frame.render_widget(Clear, area);
    let date = today.format("%b %d, %Y").to_string();
    let title = "Writing Analytics";
    let gap = usize::from(area.width).saturating_sub(title.len() + date.len());
    let mut first = vec![Span::styled(title, accent(theme))];
    if gap >= 1 {
        first.extend([Span::raw(" ".repeat(gap)), Span::styled(date, muted(theme))]);
    }
    let active = Style::default()
        .fg(crate::system_theme::readable(
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
        Span::styled(labels[0], active),
        Span::raw(" "),
        Span::styled(labels[1], muted(theme)),
        Span::raw(" "),
        Span::styled(labels[2], muted(theme)),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::from(first), tabs]).style(theme.base_style()),
        area,
    );
}

pub(super) fn draw(
    frame: &mut Frame,
    theme: Theme,
    areas: &[Rect],
    dashboard: &mut Dashboard,
    warning: Option<&str>,
) {
    let Ok((history, report)) = &dashboard.snapshot else {
        return;
    };
    header(frame, areas[0], dashboard.today, theme);
    let body = areas[1];
    let width = (body.width - 2).min(180);
    let canvas = overview_canvas(width, report, history, dashboard.today, theme);
    let warning_rows = if warning.is_some() && body.height > 0 {
        1
    } else {
        0
    };
    let viewport = Rect::new(
        body.x + (body.width - width - 2) / 2,
        body.y + warning_rows,
        width + 2,
        body.height - warning_rows,
    );
    let content_height = canvas.area.height;
    let max_scroll = content_height.saturating_sub(viewport.height);
    dashboard.scroll = dashboard.scroll.min(max_scroll);
    if let Some(message) = warning {
        frame.render_widget(
            Paragraph::new(format!("Stats unsaved: {message}"))
                .style(Style::default().fg(theme.warning)),
            Rect::new(body.x, body.y, body.width, warning_rows),
        );
    }
    let visible = viewport.height.min(content_height - dashboard.scroll);
    let target = frame.buffer_mut();
    for y in 0..visible {
        for x in 0..width {
            target[(viewport.x + 1 + x, viewport.y + y)] =
                canvas[(x, dashboard.scroll + y)].clone();
        }
    }
    if max_scroll > 0 && viewport.height > 0 {
        let thumb = ((u32::from(viewport.height) * u32::from(viewport.height)
            / u32::from(content_height)) as u16)
            .clamp(1, viewport.height);
        let offset = (u32::from(dashboard.scroll) * u32::from(viewport.height - thumb)
            / u32::from(max_scroll)) as u16;
        let x = viewport.x + viewport.width - 1;
        for y in 0..viewport.height {
            let active = y >= offset && y < offset + thumb;
            target[(x, viewport.y + y)]
                .set_symbol(if active { "┃" } else { "│" })
                .set_style(Style::default().fg(if active { theme.accent } else { theme.border }));
        }
    }
    let scroll = if max_scroll == 0 {
        "".to_owned()
    } else {
        format!(
            " · {}%",
            u32::from(dashboard.scroll) * 100 / u32::from(max_scroll)
        )
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("j/k Scroll · PgUp/PgDn{scroll}")),
            Line::from(if areas[2].width < 36 {
                "Tab View t Today r Refresh Esc"
            } else {
                "Tab View · t Today · r Refresh · Esc"
            }),
        ])
        .style(muted(theme)),
        areas[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use zettui::{
        analytics::{Goals, report},
        statistics::Activity,
    };

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    #[test]
    fn small_goal_progress_is_visible_and_never_rounds_up_to_completion() {
        use super::super::goal_percentage;
        for (written, target, expected) in [
            (0, 1000, "0%"),
            (5, 1000, "0.5%"),
            (1, 5000, "<0.1%"),
            (9999, 10000, "99.9%"),
            (1000, 1000, "100%"),
            (2000, 1000, "100%"),
            (u64::MAX, u64::MAX, "100%"),
        ] {
            let goal = GoalProgress::new(written, target).unwrap();
            assert_eq!(goal_percentage(goal), expected);
        }
        assert_eq!(goal_bar_units(GoalProgress::new(0, 1000).unwrap(), 20), 0);
        assert_eq!(goal_bar_units(GoalProgress::new(1, 5000).unwrap(), 20), 1);
        assert_eq!(goal_bar_units(GoalProgress::new(5, 1000).unwrap(), 100), 4);
        assert_eq!(
            goal_bar_units(GoalProgress::new(1001, 1000).unwrap(), 20),
            160
        );
    }

    #[test]
    fn wide_lifetime_and_table_use_available_columns() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
        let history = History::from([(
            today,
            Activity {
                words_added: 1234,
                words_deleted: 1,
                characters_added: 5678,
                characters_deleted: 2,
            },
        )]);
        let report = report(&history, today, Goals::default()).unwrap();
        let theme = Theme::named("dark").unwrap();
        for width in [100, 140, 176] {
            let lifetime = lifetime_lines(&report, width, theme);
            assert_eq!(lifetime.len(), 3);
            let labels = &lifetime[0].spans;
            assert_eq!(labels.len(), 4);
            assert!(labels[3].content.contains("Longest streak"));
            assert!(lifetime[1].spans[3].content.contains("1 day"));
            let table = activity_lines(&report, width, theme);
            assert_eq!(table.len(), 5);
            assert!(table.iter().all(|line| line.width() == usize::from(width)));
            assert!(table[1].spans[0].content.contains("1,234"));
        }
        assert!(lifetime_lines(&report, 26, theme).len() >= 4);
        assert!(activity_lines(&report, 26, theme).len() > 5);
    }

    #[test]
    fn compact_overview_keeps_escape_visible() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
        let mut dashboard = Dashboard::new(today, None);
        let history = History::new();
        let report = report(&history, today, Goals::default()).unwrap();
        dashboard.snapshot = Ok((history, report));
        let theme = Theme::named("dark").unwrap();
        for width in [32, 33, 35, 36, 80, 160] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 16)).unwrap();
            terminal
                .draw(|frame| super::super::draw(frame, theme, &mut dashboard, None))
                .unwrap();
            assert!(row(terminal.backend().buffer(), 15).contains("Esc"));
        }
    }

    #[test]
    fn plot_scaling_handles_zero_small_and_maximum_counts() {
        assert_eq!(plot_units(0, 0), 0);
        assert_eq!(plot_units(1, u64::MAX), 1);
        assert_eq!(plot_units(u64::MAX, u64::MAX), PLOT_ROWS * 8);
        assert_eq!(plot_units(50, 100), PLOT_ROWS * 4);
    }

    #[test]
    fn overview_has_panels_and_vertical_bars_at_multiple_widths() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
        let history = History::from([(
            today,
            Activity {
                words_added: 750,
                ..Activity::default()
            },
        )]);
        let report = report(
            &history,
            today,
            Goals {
                daily_words: 1000,
                weekly_words: 5000,
            },
        )
        .unwrap();
        for name in ["dark", "light"] {
            let theme = Theme::named(name).unwrap();
            for width in [30, 38, 80, 100, 140] {
                let buffer = overview_canvas(width, &report, &history, today, theme);
                let text = (0..buffer.area.height)
                    .map(|y| row(&buffer, y))
                    .collect::<Vec<_>>()
                    .join("\n");
                for expected in [
                    "Words today",
                    "750",
                    "Writing goals",
                    "75%",
                    "Last 7 days",
                    "Lifetime statistics",
                    "Activity totals",
                ] {
                    assert!(
                        text.contains(expected),
                        "missing {expected} at {width} cells"
                    );
                }
                assert!(text.contains('█'));
                assert!(text.contains('┌'));
            }
        }
    }

    #[test]
    fn narrow_chart_preserves_exact_large_daily_counts() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
        let history = History::from([(
            today,
            Activity {
                words_added: 123_456,
                ..Activity::default()
            },
        )]);
        let lines = chart_lines(&history, today, 26, Theme::named("dark").unwrap());
        assert!(lines.len() > usize::from(PLOT_ROWS + 3));
        assert!(
            lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.content.contains("123,456"))
        );
    }

    #[test]
    fn overview_scroll_clamps_and_keeps_header_and_footer_visible() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
        let history = History::new();
        let report = report(&history, today, Goals::default()).unwrap();
        let mut dashboard = Dashboard::new(today, None);
        dashboard.snapshot = Ok((history, report));
        dashboard.scroll = u16::MAX;
        let backend = ratatui::backend::TestBackend::new(42, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let theme = Theme::named("dark").unwrap();
        terminal
            .draw(|frame| super::super::draw(frame, theme, &mut dashboard, None))
            .unwrap();
        assert!(dashboard.scroll > 0 && dashboard.scroll < u16::MAX);
        let buffer = terminal.backend().buffer();
        assert!(row(buffer, 0).contains("Writing Analytics"));
        assert!(row(buffer, 18).contains("100%"));
        assert!(row(buffer, 19).contains("Esc"));
        dashboard.scroll = 0;
        terminal
            .draw(|frame| super::super::draw(frame, theme, &mut dashboard, None))
            .unwrap();
        assert!(row(terminal.backend().buffer(), 2).contains("Words today"));
    }
}

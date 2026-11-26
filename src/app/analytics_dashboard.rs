//! Full-screen, read-only writing analytics and calendar.

use super::App;
use crate::theme::Theme;
use chrono::{Datelike, Days, Local, Months, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};
use std::collections::BTreeMap;
use zettui::{
    analytics::{GoalProgress, PeriodSummary, Report},
    statistics::Activity,
    writing_activity::Tracker,
};

mod month_ui;
mod overview_ui;
mod year_ui;

type History = BTreeMap<NaiveDate, Activity>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Overview,
    Month,
    Year,
}

impl View {
    fn next(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::Overview, false) | (Self::Year, true) => Self::Month,
            (Self::Month, false) | (Self::Overview, true) => Self::Year,
            _ => Self::Overview,
        }
    }
}

pub(super) struct Dashboard {
    view: View,
    today: NaiveDate,
    selected: NaiveDate,
    scroll: u16,
    snapshot: Result<(History, Report), String>,
}

impl Dashboard {
    fn new(today: NaiveDate, tracker: Option<&Tracker>) -> Self {
        Self {
            view: View::Overview,
            today,
            selected: today,
            scroll: 0,
            snapshot: Self::read(today, tracker),
        }
    }

    fn read(today: NaiveDate, tracker: Option<&Tracker>) -> Result<(History, Report), String> {
        tracker
            .ok_or_else(|| "Writing statistics are not initialized.".to_owned())?
            .snapshot(today)
            .map_err(|error| error.to_string())
    }

    fn refresh(&mut self, today: NaiveDate, tracker: Option<&Tracker>) {
        if self.selected == self.today {
            self.selected = today;
        }
        self.today = today;
        self.selected = self.selected.min(today);
        self.snapshot = Self::read(today, tracker);
    }

    fn set_view(&mut self, view: View) {
        self.view = view;
        self.scroll = 0;
    }

    fn move_days(&mut self, forward: bool, days: u64) {
        let next = if forward {
            self.selected.checked_add_days(Days::new(days))
        } else {
            self.selected.checked_sub_days(Days::new(days))
        };
        if let Some(next) = next {
            self.selected = next.min(self.today);
        }
    }

    fn move_period(&mut self, forward: bool) {
        let months = Months::new(if self.view == View::Year { 12 } else { 1 });
        let next = if forward {
            self.selected.checked_add_months(months)
        } else {
            self.selected.checked_sub_months(months)
        };
        if let Some(next) = next {
            self.selected = next.min(self.today);
        }
    }

    fn key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Tab => self.set_view(self.view.next(false)),
            KeyCode::BackTab => self.set_view(self.view.next(true)),
            KeyCode::Char('1') => self.set_view(View::Overview),
            KeyCode::Char('2') => self.set_view(View::Month),
            KeyCode::Char('3') => self.set_view(View::Year),
            KeyCode::Home | KeyCode::Char('t') => {
                self.selected = self.today;
                self.scroll = 0;
            }
            _ if self.view == View::Overview => match code {
                KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(8),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(8),
                _ => {}
            },
            _ => match code {
                KeyCode::Left | KeyCode::Char('h') => self.move_days(false, 1),
                KeyCode::Right | KeyCode::Char('l') => self.move_days(true, 1),
                KeyCode::Up | KeyCode::Char('k') => self.move_days(false, 7),
                KeyCode::Down | KeyCode::Char('j') => self.move_days(true, 7),
                KeyCode::PageUp | KeyCode::Char('[') => self.move_period(false),
                KeyCode::PageDown | KeyCode::Char(']') => self.move_period(true),
                _ => {}
            },
        }
    }
}

impl App {
    pub(super) fn open_analytics_dashboard(&mut self) {
        if self.has_active_popup() || self.command.is_some() || self.note_search.input.is_some() {
            return;
        }
        self.analytics_dashboard = Some(Dashboard::new(
            Local::now().date_naive(),
            self.statistics.as_ref(),
        ));
    }

    pub(super) fn handle_analytics_key(&mut self, key: KeyEvent) {
        use crossterm::event::KeyModifiers;
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        match key.code {
            KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') => {
                self.analytics_dashboard = None;
            }
            KeyCode::Char('r') => {
                if let Some(dashboard) = &mut self.analytics_dashboard {
                    dashboard.refresh(Local::now().date_naive(), self.statistics.as_ref());
                }
            }
            code => {
                if let Some(dashboard) = &mut self.analytics_dashboard {
                    dashboard.key(code);
                }
            }
        }
    }

    pub(super) fn draw_analytics_dashboard(&mut self, frame: &mut Frame) {
        let Some(dashboard) = &mut self.analytics_dashboard else {
            return;
        };
        let today = Local::now().date_naive();
        if today != dashboard.today {
            dashboard.refresh(today, self.statistics.as_ref());
        }
        let warning = self.statistics.as_ref().and_then(Tracker::warning);
        draw(frame, self.theme, dashboard, warning);
    }
}

fn heading(text: impl Into<String>, theme: Theme) -> Line<'static> {
    Line::styled(
        text.into(),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )
}

fn panel_border(theme: Theme) -> ratatui::style::Color {
    // Move the existing border toward the theme's muted text, not a fixed RGB.
    crate::system_theme::blend(theme.border, theme.muted, 0.35)
}

fn day_unit(days: u64) -> &'static str {
    if days == 1 { "day" } else { "days" }
}

fn goal_percentage(goal: GoalProgress) -> String {
    if goal.target == 0 {
        return "0%".to_owned();
    }
    let tenths = (u128::from(goal.written) * 1000 / u128::from(goal.target)).min(1000);
    if goal.written > 0 && tenths == 0 {
        "<0.1%".to_owned()
    } else if tenths.is_multiple_of(10) {
        format!("{}%", tenths / 10)
    } else {
        format!("{}.{}%", tenths / 10, tenths % 10)
    }
}

fn goal(label: &str, progress: Option<GoalProgress>, theme: Theme) -> Vec<Line<'static>> {
    let Some(progress) = progress else {
        return vec![Line::from(format!("{label}: disabled"))];
    };
    let filled = usize::from(progress.percent()) / 10;
    vec![
        Line::from(vec![
            Span::raw(format!("{label}: ")),
            Span::styled("█".repeat(filled), Style::default().fg(theme.success)),
            Span::styled("░".repeat(10 - filled), Style::default().fg(theme.muted)),
            Span::raw(format!(" {}", goal_percentage(progress))),
        ]),
        Line::from(format!("  {}/{} words", progress.written, progress.target)),
        Line::from(if progress.reached() {
            "  Goal reached".to_owned()
        } else {
            format!("  {} words remaining", progress.remaining())
        }),
    ]
}

fn period(label: &str, summary: PeriodSummary) -> [Line<'static>; 3] {
    let a = summary.totals;
    [
        Line::from(format!(
            "{label:<9} Words +{} / -{}",
            a.words_added, a.words_deleted
        )),
        Line::from(format!(
            "          Chars +{} / -{}",
            a.characters_added, a.characters_deleted
        )),
        Line::from(format!(
            "          {:.1}/day · {} {} active",
            summary.average_words(),
            summary.writing_days,
            day_unit(summary.writing_days)
        )),
    ]
}

fn overview(
    report: &Report,
    history: &History,
    today: NaiveDate,
    theme: Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = vec![heading("Writing goals", theme)];
    lines.extend(goal("Daily ", report.daily_goal, theme));
    lines.extend(goal("Weekly", report.weekly_goal, theme));
    lines.extend([
        Line::from(format!(
            "Current streak: {} {}",
            report.streaks.current,
            day_unit(report.streaks.current)
        )),
        Line::from(format!(
            "Longest streak: {} {}",
            report.streaks.longest,
            day_unit(report.streaks.longest)
        )),
        Line::default(),
        heading("Activity totals", theme),
    ]);
    for (label, summary) in [
        ("Today", report.today),
        ("Week", report.week),
        ("Month", report.month),
        ("Year", report.year),
        ("All time", report.all_time),
    ] {
        lines.extend(period(label, summary));
    }
    lines.push(Line::default());
    lines.push(heading("Last 7 days · words added", theme));
    let days: Vec<_> = (0..7)
        .rev()
        .filter_map(|n| today.checked_sub_days(Days::new(n)))
        .map(|day| (day, history.get(&day).map_or(0, |a| a.words_added)))
        .collect();
    let peak = days
        .iter()
        .map(|(_, words)| *words)
        .max()
        .unwrap_or(0)
        .max(1);
    let bar_width = u128::from(width.saturating_sub(16).clamp(1, 20));
    for (day, words) in days {
        let width = if words == 0 {
            0
        } else {
            ((u128::from(words) * bar_width / u128::from(peak)) as usize).max(1)
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{} {:>8} ", day.format("%a %d"), words)),
            Span::styled("█".repeat(width), Style::default().fg(theme.accent)),
        ]));
    }
    lines
}

/// Fixed activity bands make different months and years comparable. Character
/// additions and deletion-only days remain distinguishable from no activity.
fn activity_mark(a: Activity) -> char {
    match a.words_added {
        0 if a.characters_added > 0 => '+',
        0 if a.words_deleted > 0 || a.characters_deleted > 0 => '-',
        0 => '·',
        1..=249 => '░',
        250..=499 => '▒',
        500..=999 => '▓',
        _ => '█',
    }
}

fn mark_style(mark: char, selected: bool, theme: Theme) -> Style {
    if selected {
        return Style::default()
            .fg(theme.selected_fg)
            .bg(theme.selected_bg)
            .add_modifier(Modifier::REVERSED | Modifier::BOLD);
    }
    Style::default().fg(match mark {
        '·' | ' ' => theme.muted,
        '-' => theme.warning,
        '+' | '░' | '▒' => theme.accent,
        _ => theme.success,
    })
}

fn month_cells(selected: NaiveDate) -> Vec<Option<NaiveDate>> {
    let first = selected.with_day(1).unwrap_or(selected);
    let offset = first.weekday().num_days_from_monday() as usize;
    (0..42)
        .map(|i: usize| {
            i.checked_sub(offset)
                .and_then(|n| first.checked_add_days(Days::new(n as u64)))
                .filter(|date| date.month() == selected.month() && date.year() == selected.year())
        })
        .collect()
}

fn year_cells(selected: NaiveDate) -> Vec<[Option<NaiveDate>; 7]> {
    let first = NaiveDate::from_ymd_opt(selected.year(), 1, 1).unwrap_or(selected);
    let offset = first.weekday().num_days_from_monday() as usize;
    let days = NaiveDate::from_ymd_opt(selected.year(), 12, 31)
        .map_or(365, |date| date.ordinal() as usize);
    (0..(offset + days).div_ceil(7))
        .map(|week| {
            std::array::from_fn(|weekday| {
                (week * 7 + weekday)
                    .checked_sub(offset)
                    .and_then(|n| first.checked_add_days(Days::new(n as u64)))
                    .filter(|date| date.year() == selected.year())
            })
        })
        .collect()
}

fn calendar_mark(date: NaiveDate, today: NaiveDate, history: &History) -> char {
    if date > today {
        ' '
    } else {
        activity_mark(history.get(&date).copied().unwrap_or_default())
    }
}

fn details(lines: &mut Vec<Line<'static>>, dashboard: &Dashboard, history: &History, theme: Theme) {
    let a = history
        .get(&dashboard.selected)
        .copied()
        .unwrap_or_default();
    lines.push(Line::default());
    lines.push(heading(
        format!("Selected: {}", dashboard.selected.format("%a %Y-%m-%d")),
        theme,
    ));
    lines.push(Line::from(format!(
        "Words +{} / -{}",
        a.words_added, a.words_deleted
    )));
    lines.push(Line::from(format!(
        "Characters +{} / -{}",
        a.characters_added, a.characters_deleted
    )));
    lines.push(Line::from("· none  + chars  - deletions"));
    lines.push(Line::from("░ 1–249   ▒ 250–499"));
    lines.push(Line::from("▓ 500–999 █ 1000+ words"));
}

fn month(dashboard: &Dashboard, history: &History, theme: Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        heading(dashboard.selected.format("%B %Y").to_string(), theme),
        Line::from("Mon Tue Wed Thu Fri Sat Sun"),
    ];
    for week in month_cells(dashboard.selected).chunks(7) {
        let spans: Vec<_> = week
            .iter()
            .map(|date| match date {
                Some(date) => {
                    let mark = calendar_mark(*date, dashboard.today, history);
                    Span::styled(
                        format!("{:>2}{mark} ", date.day()),
                        mark_style(mark, *date == dashboard.selected, theme),
                    )
                }
                None => Span::raw("    "),
            })
            .collect();
        lines.push(Line::from(spans));
    }
    details(&mut lines, dashboard, history, theme);
    lines
}

fn year_fallback(dashboard: &Dashboard, history: &History, theme: Theme) -> Vec<Line<'static>> {
    let a = history
        .get(&dashboard.selected)
        .copied()
        .unwrap_or_default();
    vec![
        heading(format!("Selected: {}", dashboard.selected), theme),
        Line::from(format!("Words +{} / -{}", a.words_added, a.words_deleted)),
        Line::from(format!(
            "Chars +{} / -{}",
            a.characters_added, a.characters_deleted
        )),
        Line::from("Enlarge for full heatmap."),
    ]
}

fn draw(frame: &mut Frame, theme: Theme, dashboard: &mut Dashboard, warning: Option<&str>) {
    let area = frame.area();
    frame.render_widget(Block::default().style(theme.base_style()), area);
    if area.width < 30 || area.height < 8 {
        frame.render_widget(
            Paragraph::new("Resize terminal (30x8). Esc to close.").wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let areas = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(area);
    let tabs = [
        (View::Overview, "1 Overview"),
        (View::Month, "2 Month"),
        (View::Year, "3 Year"),
    ];
    let mut spans = Vec::new();
    for (view, label) in tabs {
        spans.push(Span::styled(
            label,
            if view == dashboard.view {
                Style::default()
                    .fg(theme.selected_fg)
                    .bg(theme.selected_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            },
        ));
        spans.push(Span::raw("  "));
    }
    frame.render_widget(
        Paragraph::new(vec![heading("Writing analytics", theme), Line::from(spans)]),
        areas[0],
    );
    let mut lines = match &dashboard.snapshot {
        Ok(_) if dashboard.view == View::Overview && areas[1].width >= 32 => {
            overview_ui::draw(frame, theme, &areas, dashboard, warning);
            return;
        }
        Ok((history, report)) => match dashboard.view {
            View::Overview => overview(report, history, dashboard.today, theme, areas[1].width),
            View::Month => {
                month_ui::header(frame, areas[0], dashboard.today, theme, dashboard.view);
                if month_ui::draw(frame, theme, areas[1], dashboard, history, warning) {
                    footer(frame, theme, areas[2], dashboard.view);
                    return;
                }
                month(dashboard, history, theme)
            }
            View::Year => {
                month_ui::header(frame, areas[0], dashboard.today, theme, dashboard.view);
                if year_ui::draw(frame, theme, areas[1], dashboard, history, warning) {
                    footer(frame, theme, areas[2], dashboard.view);
                    return;
                }
                year_fallback(dashboard, history, theme)
            }
        },
        Err(error) => vec![
            Line::styled(
                format!("Statistics unavailable: {error}"),
                Style::default().fg(theme.warning),
            ),
            Line::from("Press r to retry. Existing data has not been reset."),
        ],
    };
    if let Some(warning) = warning {
        lines.insert(
            0,
            Line::styled(
                format!("Stats unsaved: {warning}"),
                Style::default().fg(theme.warning),
            ),
        );
    }
    let body = areas[1];
    // Calendar needs enough vertical room for grid and selected-day details.
    // On a short terminal, keep navigation responsive and show the selected date.
    if dashboard.view != View::Overview
        && dashboard.snapshot.is_ok()
        && usize::from(body.height) < lines.len()
    {
        let selected = dashboard.selected;
        let a = dashboard
            .snapshot
            .as_ref()
            .ok()
            .and_then(|(history, _)| history.get(&selected))
            .copied()
            .unwrap_or_default();
        lines = vec![
            heading(format!("Selected: {selected}"), theme),
            Line::from(format!("Words +{} / -{}", a.words_added, a.words_deleted)),
            Line::from(format!(
                "Chars +{} / -{}",
                a.characters_added, a.characters_deleted
            )),
            Line::from("Enlarge for full calendar."),
        ];
        if warning.is_some() {
            lines[3] = Line::styled(
                "Stats unsaved; enlarge view.",
                Style::default().fg(theme.warning),
            );
        }
    }
    let max_scroll = lines
        .len()
        .saturating_sub(usize::from(body.height))
        .min(usize::from(u16::MAX)) as u16;
    dashboard.scroll = if dashboard.view == View::Overview {
        dashboard.scroll.min(max_scroll)
    } else {
        0
    };
    frame.render_widget(Paragraph::new(lines).scroll((dashboard.scroll, 0)), body);
    footer(frame, theme, areas[2], dashboard.view);
}

fn footer(frame: &mut Frame, theme: Theme, area: Rect, view: View) {
    let navigation = if view == View::Overview {
        "j/k Scroll · PgUp/PgDn"
    } else {
        "h/l Day j/k Week [/] Period"
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(navigation),
            Line::from("Tab View t Today r Refresh Esc"),
        ])
        .style(Style::default().fg(theme.muted)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Appearance, AutoThemes, ColorOverrides, ThemeChoice};
    use crossterm::event::KeyModifiers;
    use std::fs;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn month_grid_has_monday_start_and_leap_day() {
        let cells = month_cells(date(2024, 2, 29));
        assert_eq!(cells.len(), 42);
        assert_eq!(cells[3], Some(date(2024, 2, 1)));
        assert_eq!(cells[31], Some(date(2024, 2, 29)));
        assert_eq!(cells.iter().flatten().count(), 29);
        assert_eq!(month_cells(date(2025, 2, 1)).iter().flatten().count(), 28);
    }

    #[test]
    fn year_grid_covers_every_date_once_including_54_week_year() {
        for (year, days) in [(2023, 365), (2024, 366), (2012, 366)] {
            let cells = year_cells(date(year, 6, 1));
            let actual: Vec<_> = cells.iter().flatten().flatten().copied().collect();
            assert_eq!(actual.len(), days);
            assert_eq!(actual.first().copied(), Some(date(year, 1, 1)));
            assert_eq!(actual.last().copied(), Some(date(year, 12, 31)));
            assert!(
                actual
                    .windows(2)
                    .all(|pair| pair[0].succ_opt() == Some(pair[1]))
            );
        }
        assert_eq!(year_cells(date(2012, 1, 1)).len(), 54);
    }

    #[test]
    fn calendar_navigation_clamps_month_ends_and_future() {
        let mut dashboard = Dashboard::new(date(2024, 3, 31), None);
        dashboard.key(KeyCode::Tab);
        assert_eq!(dashboard.view, View::Month);
        dashboard.key(KeyCode::PageUp);
        assert_eq!(dashboard.selected, date(2024, 2, 29));
        dashboard.key(KeyCode::Char('h'));
        assert_eq!(dashboard.selected, date(2024, 2, 28));
        dashboard.key(KeyCode::Tab);
        dashboard.key(KeyCode::PageUp);
        assert_eq!(dashboard.selected, date(2023, 2, 28));
        dashboard.key(KeyCode::Char('t'));
        dashboard.key(KeyCode::PageDown);
        assert_eq!(dashboard.selected, dashboard.today);
        dashboard.key(KeyCode::BackTab);
        assert_eq!(dashboard.view, View::Month);
    }

    #[test]
    fn heatmap_distinguishes_activity_and_future() {
        for (words_added, expected) in [
            (0, '·'),
            (1, '░'),
            (249, '░'),
            (250, '▒'),
            (500, '▓'),
            (1000, '█'),
        ] {
            assert_eq!(
                activity_mark(Activity {
                    words_added,
                    ..Activity::default()
                }),
                expected
            );
        }
        assert_eq!(
            activity_mark(Activity {
                characters_added: 1,
                ..Activity::default()
            }),
            '+'
        );
        assert_eq!(
            activity_mark(Activity {
                words_deleted: 1,
                ..Activity::default()
            }),
            '-'
        );
        let today = date(2026, 11, 26);
        let tomorrow = today.succ_opt().unwrap();
        assert_eq!(calendar_mark(tomorrow, today, &BTreeMap::new()), ' ');
    }

    #[test]
    fn refresh_includes_pending_and_reports_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let mut tracker = Tracker::new(Some(path.clone()));
        let today = date(2026, 11, 26);
        let mut dashboard = Dashboard::new(today, Some(&tracker));
        assert!(!path.exists());
        tracker.record_edit(today, "", "hello world").unwrap();
        dashboard.refresh(today, Some(&tracker));
        let (history, report) = dashboard.snapshot.as_ref().unwrap();
        assert_eq!(history[&today].words_added, 2);
        assert_eq!(report.today.totals.words_added, 2);
        assert!(!path.exists());
        tracker.flush().unwrap();
        dashboard.refresh(today, Some(&tracker));
        assert_eq!(
            dashboard
                .snapshot
                .as_ref()
                .unwrap()
                .1
                .today
                .totals
                .words_added,
            2
        );
        fs::write(&path, "invalid").unwrap();
        dashboard.refresh(today, Some(&tracker));
        assert!(dashboard.snapshot.is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "invalid");
    }

    #[test]
    fn dashboard_is_modal_preserves_dirty_note_and_renders_small_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let note = dir.path().join("Note.md");
        fs::write(&note, "").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap();
        app.statistics = Some(Tracker::new(Some(dir.path().join("state.json"))));
        app.editor.open(&note).unwrap();
        app.handle_editor_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        app.handle_editor_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        app.handle_editor_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let before = app.editor.as_ref().unwrap().state.lines.to_string();
        let cursor = app.editor.as_ref().unwrap().state.cursor;
        app.open_analytics_dashboard();
        assert!(app.has_active_popup());
        for size in [(90, 30), (35, 20), (30, 8), (12, 4)] {
            let backend = ratatui::backend::TestBackend::new(size.0, size.1);
            let mut terminal = ratatui::Terminal::new(backend).unwrap();
            for code in [KeyCode::Char('1'), KeyCode::Char('2'), KeyCode::Char('3')] {
                app.handle_analytics_key(KeyEvent::new(code, KeyModifiers::NONE));
                terminal.draw(|frame| app.draw(frame)).unwrap();
            }
        }
        app.handle_analytics_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        app.handle_analytics_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.has_active_popup());
        let active = app.editor.as_ref().unwrap();
        assert!(active.is_dirty());
        assert_eq!(active.state.lines.to_string(), before);
        assert_eq!(active.state.cursor, cursor);
        assert_eq!(fs::read_to_string(note).unwrap(), "");
    }
}

//! Calendar-based writing analytics, independent of terminal rendering.

use crate::statistics::Activity;
use chrono::{Datelike, Days, NaiveDate};
use serde::Deserialize;
use std::{collections::BTreeMap, io};

/// Optional gross word-addition targets. Zero disables the corresponding goal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Goals {
    pub daily_words: u64,
    pub weekly_words: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoalProgress {
    pub written: u64,
    pub target: u64,
}

impl GoalProgress {
    pub fn new(written: u64, target: u64) -> Option<Self> {
        (target > 0).then_some(Self { written, target })
    }

    pub fn remaining(self) -> u64 {
        self.target.saturating_sub(self.written)
    }

    pub fn reached(self) -> bool {
        self.written >= self.target
    }

    /// A display-friendly, capped percentage (floor, not rounded up).
    pub fn percent(self) -> u8 {
        if self.target == 0 {
            return 0;
        }
        ((u128::from(self.written) * 100 / u128::from(self.target)).min(100)) as u8
    }
}

/// Inclusive range. Averages include elapsed days with no activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodSummary {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub totals: Activity,
    pub writing_days: u64,
}

impl PeriodSummary {
    pub fn calendar_days(self) -> u64 {
        (self.end.signed_duration_since(self.start).num_days() + 1) as u64
    }

    pub fn average_words(self) -> f64 {
        self.totals.words_added as f64 / self.calendar_days() as f64
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Streaks {
    pub current: u64,
    pub longest: u64,
}

/// Snapshot for the dashboard. Periods are month/week/year to date, not rolling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub today: PeriodSummary,
    pub week: PeriodSummary,
    pub month: PeriodSummary,
    pub year: PeriodSummary,
    pub all_time: PeriodSummary,
    pub streaks: Streaks,
    pub daily_goal: Option<GoalProgress>,
    pub weekly_goal: Option<GoalProgress>,
}

/// A writing day requires additions, even if the edit did not add a whole word.
/// Deletion-only activity is retained in totals but does not extend a streak.
fn is_writing(activity: Activity) -> bool {
    activity.words_added > 0 || activity.characters_added > 0
}

pub fn summarize(
    daily: &BTreeMap<NaiveDate, Activity>,
    start: NaiveDate,
    end: NaiveDate,
) -> io::Result<PeriodSummary> {
    if start > end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Reversed analytics range",
        ));
    }
    let mut summary = PeriodSummary {
        start,
        end,
        totals: Activity::default(),
        writing_days: 0,
    };
    for (_, &activity) in daily.range(start..=end) {
        summary.totals = summary.totals.checked_add(activity)?;
        if is_writing(activity) {
            summary.writing_days += 1;
        }
    }
    Ok(summary)
}

/// Today need not have activity yet; a streak through yesterday is still current.
/// Future-dated records never affect the report or streaks.
pub fn streaks(daily: &BTreeMap<NaiveDate, Activity>, today: NaiveDate) -> Streaks {
    let mut result = Streaks::default();
    let mut previous: Option<NaiveDate> = None;
    let mut run = 0;
    for (&date, &activity) in daily.range(..=today) {
        if !is_writing(activity) {
            continue;
        }
        run = if previous.and_then(|day| day.succ_opt()) == Some(date) {
            run + 1
        } else {
            1
        };
        result.longest = result.longest.max(run);
        previous = Some(date);
    }
    if previous.is_some() && (previous == Some(today) || previous == today.pred_opt()) {
        result.current = run;
    }
    result
}

pub fn report(
    daily: &BTreeMap<NaiveDate, Activity>,
    today: NaiveDate,
    goals: Goals,
) -> io::Result<Report> {
    let week_start = today
        .checked_sub_days(Days::new(u64::from(today.weekday().num_days_from_monday())))
        .unwrap_or(NaiveDate::MIN);
    let month_start =
        NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(NaiveDate::MIN);
    let year_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(NaiveDate::MIN);
    let first = daily
        .range(..=today)
        .next()
        .map_or(today, |(&date, _)| date);
    let day = summarize(daily, today, today)?;
    let week = summarize(daily, week_start, today)?;
    Ok(Report {
        today: day,
        week,
        month: summarize(daily, month_start, today)?,
        year: summarize(daily, year_start, today)?,
        all_time: summarize(daily, first, today)?,
        streaks: streaks(daily, today),
        daily_goal: GoalProgress::new(day.totals.words_added, goals.daily_words),
        weekly_goal: GoalProgress::new(week.totals.words_added, goals.weekly_words),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn words(n: u64) -> Activity {
        Activity {
            words_added: n,
            ..Activity::default()
        }
    }

    #[test]
    fn periods_cross_year_boundary_and_ignore_future() {
        let daily = BTreeMap::from([
            (date(2025, 12, 28), words(10)),
            (date(2025, 12, 29), words(20)),
            (date(2026, 1, 1), words(30)),
            (date(2026, 1, 2), words(40)),
            (date(2026, 1, 3), words(999)),
        ]);
        let r = report(&daily, date(2026, 1, 2), Goals::default()).unwrap();
        assert_eq!(r.today.totals.words_added, 40);
        assert_eq!(r.week.start, date(2025, 12, 29));
        assert_eq!(r.week.totals.words_added, 90);
        assert_eq!(r.week.calendar_days(), 5);
        assert_eq!(r.week.average_words(), 18.0);
        assert_eq!(r.month.totals.words_added, 70);
        assert_eq!(r.year.totals.words_added, 70);
        assert_eq!(r.all_time.totals.words_added, 100);
    }

    #[test]
    fn leap_day_and_missing_dates_are_included_in_average() {
        let daily = BTreeMap::from([(date(2024, 2, 29), words(290))]);
        let r = report(&daily, date(2024, 2, 29), Goals::default()).unwrap();
        assert_eq!(r.month.calendar_days(), 29);
        assert_eq!(r.month.writing_days, 1);
        assert_eq!(r.month.average_words(), 10.0);
        assert_eq!(r.year.calendar_days(), 60);
    }

    #[test]
    fn streaks_survive_midnight_but_not_a_missed_day() {
        let daily = BTreeMap::from([
            (date(2026, 1, 1), words(1)),
            (date(2026, 1, 2), words(1)),
            (date(2026, 1, 3), words(1)),
            (date(2026, 1, 5), words(1)),
            (
                date(2026, 1, 6),
                Activity {
                    characters_added: 1,
                    ..Activity::default()
                },
            ),
            (
                date(2026, 1, 7),
                Activity {
                    words_deleted: 1,
                    ..Activity::default()
                },
            ),
            (date(2026, 1, 9), words(1)),
        ]);
        assert_eq!(
            streaks(&daily, date(2026, 1, 6)),
            Streaks {
                current: 2,
                longest: 3
            }
        );
        assert_eq!(
            streaks(&daily, date(2026, 1, 7)),
            Streaks {
                current: 2,
                longest: 3
            }
        );
        assert_eq!(
            streaks(&daily, date(2026, 1, 8)),
            Streaks {
                current: 0,
                longest: 3
            }
        );
    }

    #[test]
    fn goals_are_independent_and_deletions_do_not_reduce_progress() {
        let today = date(2026, 11, 26);
        let daily = BTreeMap::from([(
            today,
            Activity {
                words_added: 750,
                words_deleted: 300,
                ..Activity::default()
            },
        )]);
        let r = report(
            &daily,
            today,
            Goals {
                daily_words: 1000,
                weekly_words: 500,
            },
        )
        .unwrap();
        let day = r.daily_goal.unwrap();
        assert_eq!(day.percent(), 75);
        assert_eq!(day.remaining(), 250);
        assert!(!day.reached());
        let week = r.weekly_goal.unwrap();
        assert_eq!(week.percent(), 100);
        assert_eq!(week.remaining(), 0);
        assert!(week.reached());
        assert_eq!(GoalProgress::new(10, 0), None);
        assert_eq!(
            GoalProgress::new(u64::MAX, u64::MAX).unwrap().percent(),
            100
        );
    }

    #[test]
    fn empty_history_has_zero_totals_and_disabled_goals() {
        let r = report(&BTreeMap::new(), date(2026, 11, 26), Goals::default()).unwrap();
        assert_eq!(r.all_time.calendar_days(), 1);
        assert_eq!(r.all_time.average_words(), 0.0);
        assert_eq!(r.streaks, Streaks::default());
        assert_eq!(r.daily_goal, None);
        assert_eq!(r.weekly_goal, None);
    }

    #[test]
    fn reversed_ranges_and_aggregate_overflow_are_errors() {
        let start = date(2026, 1, 1);
        let end = date(2026, 1, 2);
        let daily = BTreeMap::from([(start, words(u64::MAX)), (end, words(1))]);
        assert!(summarize(&daily, end, start).is_err());
        assert!(summarize(&daily, start, end).is_err());
    }
}

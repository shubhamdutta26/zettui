//! Buffer edit accumulation and periodic persistence, independent of the UI.

use crate::analytics::{self, Goals, Report};
use crate::statistics::{Activity, Store};
use chrono::NaiveDate;
use std::{
    collections::BTreeMap,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

const FLUSH_INTERVAL: Duration = Duration::from_secs(2);

/// Keep unsaved deltas in memory until a complete transaction succeeds.
/// Dates are assigned at edit time, not flush time (including across midnight).
pub struct Tracker {
    store: Option<Store>,
    goals: Goals,
    pending: BTreeMap<NaiveDate, Activity>,
    last_attempt: Instant,
    warning: Option<String>,
}

impl Tracker {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self::with_goals(path, Goals::default())
    }

    pub fn with_goals(path: Option<PathBuf>, goals: Goals) -> Self {
        Self {
            store: path.map(Store::new),
            goals,
            pending: BTreeMap::new(),
            last_attempt: Instant::now(),
            warning: None,
        }
    }

    pub fn record_edit(&mut self, date: NaiveDate, before: &str, after: &str) -> io::Result<()> {
        let delta = Activity::between(before, after);
        if delta != Activity::default() {
            let total = self.pending.entry(date).or_default();
            *total = total.checked_add(delta)?;
        }
        Ok(())
    }

    /// Combine the latest persisted device totals with unflushed local edits.
    /// Read errors are propagated, not silently presented as empty history.
    pub fn daily_totals(&self) -> io::Result<BTreeMap<NaiveDate, Activity>> {
        let mut totals = match &self.store {
            Some(store) => store.daily_totals()?,
            None => BTreeMap::new(),
        };
        for (&date, &activity) in &self.pending {
            let total = totals.entry(date).or_default();
            *total = total.checked_add(activity)?;
        }
        Ok(totals)
    }

    pub fn report(&self, today: NaiveDate) -> io::Result<Report> {
        analytics::report(&self.daily_totals()?, today, self.goals)
    }

    /// Read history once so the calendar and report describe the same snapshot.
    pub fn snapshot(
        &self,
        today: NaiveDate,
    ) -> io::Result<(BTreeMap<NaiveDate, Activity>, Report)> {
        let daily = self.daily_totals()?;
        let report = analytics::report(&daily, today, self.goals)?;
        Ok((daily, report))
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    /// Called from the event loop even when idle. Failed attempts are throttled
    /// too, so an unavailable state directory does not cause a busy loop.
    pub fn flush_if_due(&mut self) {
        if !self.pending.is_empty() && self.last_attempt.elapsed() >= FLUSH_INTERVAL {
            let _ = self.flush();
        }
    }

    /// Force a save on exit. Do not create a file if there is no activity.
    pub fn flush(&mut self) -> io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        self.last_attempt = Instant::now();
        let result = self.store.as_ref().map_or_else(
            || Err(io::Error::other("No absolute XDG_STATE_HOME or HOME")),
            |store| store.record_batch(&self.pending).map(|_| ()),
        );
        match result {
            Ok(()) => {
                self.pending.clear();
                self.warning = None;
                Ok(())
            }
            Err(error) => {
                self.warning = Some(error.to_string());
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn day(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, n).unwrap()
    }

    #[test]
    fn reports_include_pending_edits_without_double_counting_or_creating_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let goals = Goals {
            daily_words: 4,
            weekly_words: 10,
        };
        let mut tracker = Tracker::with_goals(Some(path.clone()), goals);
        assert_eq!(tracker.report(day(26)).unwrap().today.totals.words_added, 0);
        assert!(!path.exists());
        tracker.record_edit(day(26), "", "hello world").unwrap();
        let before = tracker.report(day(26)).unwrap();
        assert_eq!(before.daily_goal.unwrap().percent(), 50);
        assert!(!path.exists());
        tracker.flush().unwrap();
        assert_eq!(tracker.report(day(26)).unwrap(), before);
        tracker
            .record_edit(day(26), "hello world", "hello world again")
            .unwrap();
        assert_eq!(tracker.report(day(26)).unwrap().today.totals.words_added, 3);
        fs::write(path, "invalid data").unwrap();
        assert!(tracker.report(day(26)).is_err());
    }

    #[test]
    fn no_changes_do_not_create_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let mut tracker = Tracker::new(Some(path.clone()));
        tracker.record_edit(day(25), "same", "same").unwrap();
        tracker.flush().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn edits_on_both_sides_of_midnight_are_flushed_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let mut tracker = Tracker::new(Some(path.clone()));
        tracker.record_edit(day(25), "", "hello").unwrap();
        tracker
            .record_edit(day(26), "hello", "hello world")
            .unwrap();
        tracker
            .record_edit(day(26), "hello world", "hello")
            .unwrap();
        tracker.flush().unwrap();
        tracker.flush().unwrap();
        let totals = Store::new(path).load().unwrap().daily_totals().unwrap();
        assert_eq!(totals[&day(25)].words_added, 1);
        assert_eq!(totals[&day(26)].words_added, 1);
        assert_eq!(totals[&day(26)].words_deleted, 1);
        assert_eq!(totals[&day(26)].characters_deleted, 6);
    }

    #[test]
    fn periodic_flush_and_retry_preserve_pending_activity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let mut tracker = Tracker::new(Some(path.clone()));
        tracker.record_edit(day(25), "", "hello").unwrap();
        fs::write(&path, "invalid data").unwrap();
        tracker.last_attempt = Instant::now() - FLUSH_INTERVAL;
        tracker.flush_if_due();
        assert!(tracker.warning().is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), "invalid data");
        assert_eq!(tracker.pending.len(), 1);
        fs::remove_file(&path).unwrap();
        tracker.last_attempt = Instant::now() - FLUSH_INTERVAL;
        tracker.flush_if_due();
        assert!(tracker.warning().is_none());
        assert!(tracker.pending.is_empty());
        let totals = Store::new(path).load().unwrap().daily_totals().unwrap();
        assert_eq!(totals[&day(25)].words_added, 1);
    }
}

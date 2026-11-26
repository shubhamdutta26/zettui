//! Buffer edit accumulation and periodic persistence, independent of the UI.

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
    pending: BTreeMap<NaiveDate, Activity>,
    last_attempt: Instant,
    warning: Option<String>,
}

impl Tracker {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self {
            store: path.map(Store::new),
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

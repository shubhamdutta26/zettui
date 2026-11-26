//! Local, versioned writing statistics. No note content or paths are stored.
//!
//! Each installation owns monotonically increasing daily counters. Keeping
//! installations separate allows future snapshot imports to be idempotent.

use chrono::NaiveDate;
use rustix::fs::{FlockOperation, flock};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    fs::{File, OpenOptions},
    io,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const VERSION: u32 = 1;
const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// Nonnegative edit counters, not the current length of a document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activity {
    pub words_added: u64,
    pub words_deleted: u64,
    pub characters_added: u64,
    pub characters_deleted: u64,
}

impl Activity {
    /// Measure one visible edit. Words follow the editor's whitespace-token
    /// count; characters are Unicode scalar values, not UTF-8 bytes.
    pub fn between(before: &str, after: &str) -> Self {
        if before == after {
            return Self::default();
        }
        let old_words = before.split_whitespace().count() as u64;
        let new_words = after.split_whitespace().count() as u64;
        // Remove the shared prefix and suffix without splitting Unicode.
        // One event may replace a span rather than merely change its length.
        let prefix: usize = before
            .chars()
            .zip(after.chars())
            .take_while(|(a, b)| a == b)
            .map(|(ch, _)| ch.len_utf8())
            .sum();
        let old = &before[prefix..];
        let new = &after[prefix..];
        let suffix = old
            .chars()
            .rev()
            .zip(new.chars().rev())
            .take_while(|(a, b)| a == b)
            .count();
        Self {
            words_added: new_words.saturating_sub(old_words),
            words_deleted: old_words.saturating_sub(new_words),
            characters_added: (new.chars().count() - suffix) as u64,
            characters_deleted: (old.chars().count() - suffix) as u64,
        }
    }

    pub(crate) fn checked_add(self, other: Self) -> io::Result<Self> {
        let add = |a: u64, b: u64| {
            a.checked_add(b)
                .ok_or_else(|| invalid("Writing statistics counter overflow"))
        };
        Ok(Self {
            words_added: add(self.words_added, other.words_added)?,
            words_deleted: add(self.words_deleted, other.words_deleted)?,
            characters_added: add(self.characters_added, other.characters_added)?,
            characters_deleted: add(self.characters_deleted, other.characters_deleted)?,
        })
    }
}

/// A snapshot of the installation's local statistics state.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statistics {
    version: u32,
    local_device_id: String,
    devices: BTreeMap<String, BTreeMap<String, Activity>>,
}

impl Statistics {
    fn new() -> io::Result<Self> {
        let mut bytes = [0u8; 16];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        let id = format!("{:032x}", u128::from_be_bytes(bytes));
        let devices = BTreeMap::from([(id.clone(), BTreeMap::new())]);
        Ok(Self {
            version: VERSION,
            local_device_id: id,
            devices,
        })
    }

    /// Random, stable installation identifier (not a hostname or hardware ID).
    pub fn local_device_id(&self) -> &str {
        &self.local_device_id
    }

    /// Daily totals across all known installations, sorted by date.
    pub fn daily_totals(&self) -> io::Result<BTreeMap<NaiveDate, Activity>> {
        let mut totals = BTreeMap::<NaiveDate, Activity>::new();
        for days in self.devices.values() {
            for (date, activity) in days {
                let total = totals.entry(parse_date(date)?).or_default();
                *total = total.checked_add(*activity)?;
            }
        }
        Ok(totals)
    }

    fn validate(&self) -> io::Result<()> {
        if self.version != VERSION {
            return Err(invalid("Unsupported writing statistics version"));
        }
        if !self.devices.contains_key(&self.local_device_id) {
            return Err(invalid("Missing local writing statistics device"));
        }
        for (id, days) in &self.devices {
            if id.len() != 32
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid("Invalid writing statistics device ID"));
            }
            for date in days.keys() {
                parse_date(date)?;
            }
        }
        self.daily_totals()?;
        Ok(())
    }

    fn record(&mut self, date: NaiveDate, activity: Activity) -> io::Result<()> {
        let key = date.format("%Y-%m-%d").to_string();
        parse_date(&key)?;
        if activity == Activity::default() {
            return Ok(());
        }
        let days = self
            .devices
            .get_mut(&self.local_device_id)
            .ok_or_else(|| invalid("Missing local writing statistics device"))?;
        let total = days.entry(key).or_default();
        *total = total.checked_add(activity)?;
        Ok(())
    }
}

/// Installation-wide state, separate from workspace sessions and configuration.
/// Only absolute XDG paths are accepted, matching the session engine.
pub fn state_path() -> Option<PathBuf> {
    state_path_from(env::var_os("XDG_STATE_HOME"), env::var_os("HOME"))
}

fn state_path_from(
    xdg: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    let base = xdg
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            home.map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(".local/state"))
        })?;
    Some(base.join("zettui/statistics.json"))
}

/// Transactional access to a local statistics file. Each operation reloads
/// under a separate advisory lock so simultaneous Zettui processes cannot
/// overwrite each other's counters. Never remove the lock file while in use.
#[derive(Debug)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Load existing data or initialize a new installation on first use.
    /// Invalid files are errors, never silently reset or overwritten.
    pub fn load(&self) -> io::Result<Statistics> {
        let _lock = self.lock()?;
        match self.read()? {
            Some(stats) => Ok(stats),
            None => {
                let stats = Statistics::new()?;
                self.save(&stats)?;
                Ok(stats)
            }
        }
    }

    /// Read the current atomic snapshot without creating state or a lock file.
    /// A concurrent first write may not yet be visible, which is safe for reports.
    pub fn daily_totals(&self) -> io::Result<BTreeMap<NaiveDate, Activity>> {
        match self.read()? {
            Some(stats) => stats.daily_totals(),
            None => Ok(BTreeMap::new()),
        }
    }

    /// Persist an edit delta for the supplied local calendar date. The caller
    /// decides what constitutes a writing edit (implemented in Phase 2).
    pub fn record(&self, date: NaiveDate, activity: Activity) -> io::Result<Statistics> {
        self.record_batch(&BTreeMap::from([(date, activity)]))
    }

    /// Commit all pending dates in one transaction. On failure no date is
    /// committed, so the caller can safely retry the entire batch.
    pub fn record_batch(&self, pending: &BTreeMap<NaiveDate, Activity>) -> io::Result<Statistics> {
        let _lock = self.lock()?;
        let mut stats = match self.read()? {
            Some(stats) => stats,
            None => Statistics::new()?,
        };
        for (&date, &activity) in pending {
            stats.record(date, activity)?;
        }
        self.save(&stats)?;
        Ok(stats)
    }

    fn parent(&self) -> &Path {
        self.path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
    }

    fn lock(&self) -> io::Result<File> {
        fs::create_dir_all(self.parent())?;
        let mut lock_path = self.path.as_os_str().to_os_string();
        lock_path.push(".lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(PathBuf::from(lock_path))?;
        flock(&file, FlockOperation::LockExclusive).map_err(io::Error::from)?;
        Ok(file)
    }

    fn read(&self) -> io::Result<Option<Statistics>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(invalid("Writing statistics file too large"));
        }
        let stats: Statistics = serde_json::from_slice(&bytes).map_err(io::Error::from)?;
        stats.validate()?;
        Ok(Some(stats))
    }

    fn save(&self, stats: &Statistics) -> io::Result<()> {
        stats.validate()?;
        let mut temp = tempfile::NamedTempFile::new_in(self.parent())?;
        serde_json::to_writer_pretty(&mut temp, stats)?;
        temp.write_all(b"\n")?;
        if temp.as_file().metadata()?.len() > MAX_BYTES {
            return Err(invalid("Writing statistics file too large"));
        }
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|error| error.error)?;
        Ok(())
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn parse_date(value: &str) -> io::Result<NaiveDate> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| invalid("Invalid writing statistics date"))?;
    if value.len() != 10 || date.format("%Y-%m-%d").to_string() != value {
        return Err(invalid("Noncanonical writing statistics date"));
    }
    Ok(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, n).unwrap()
    }

    fn activity(n: u64) -> Activity {
        Activity {
            words_added: n,
            characters_added: n * 5,
            ..Activity::default()
        }
    }

    #[test]
    fn edit_deltas_count_words_and_unicode_characters() {
        assert_eq!(Activity::between("same", "same"), Activity::default());
        assert_eq!(
            Activity::between("hello", "hello 世界"),
            Activity {
                words_added: 1,
                characters_added: 3,
                ..Activity::default()
            }
        );
        assert_eq!(
            Activity::between("hello 世界", "hello"),
            Activity {
                words_deleted: 1,
                characters_deleted: 3,
                ..Activity::default()
            }
        );
        assert_eq!(
            Activity::between("aéz", "a界z"),
            Activity {
                characters_added: 1,
                characters_deleted: 1,
                ..Activity::default()
            }
        );
        assert_eq!(Activity::between("a", "ab").words_added, 0);
        assert_eq!(Activity::between("ab", "a b").words_added, 1);
        assert_eq!(Activity::between("a b", "ab").words_deleted, 1);
    }

    #[test]
    fn batch_failure_does_not_commit_any_date() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let store = Store::new(path.clone());
        store
            .record(
                day(26),
                Activity {
                    words_added: u64::MAX,
                    ..Activity::default()
                },
            )
            .unwrap();
        let before = fs::read(&path).unwrap();
        let pending = BTreeMap::from([(day(25), activity(2)), (day(26), activity(1))]);
        assert!(store.record_batch(&pending).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn stable_device_identity_and_independent_installations() {
        let dir = tempfile::tempdir().unwrap();
        let first = Store::new(dir.path().join("first/statistics.json"));
        let second = Store::new(dir.path().join("second/statistics.json"));
        let initial = first.load().unwrap();
        assert_eq!(initial.local_device_id().len(), 32);
        assert!(initial.daily_totals().unwrap().is_empty());
        assert_eq!(initial, first.load().unwrap());
        assert_ne!(
            initial.local_device_id(),
            second.load().unwrap().local_device_id()
        );
    }

    #[test]
    fn cumulative_daily_activity_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let store = Store::new(path.clone());
        store.record(day(25), activity(10)).unwrap();
        store.record(day(25), activity(4)).unwrap();
        store.record(day(26), activity(2)).unwrap();
        store
            .record(
                day(25),
                Activity {
                    words_deleted: 3,
                    characters_deleted: 15,
                    ..Activity::default()
                },
            )
            .unwrap();
        let totals = Store::new(path).load().unwrap().daily_totals().unwrap();
        assert_eq!(totals.len(), 2);
        assert_eq!(totals[&day(25)].words_added, 14);
        assert_eq!(totals[&day(25)].characters_added, 70);
        assert_eq!(totals[&day(25)].words_deleted, 3);
        assert_eq!(totals[&day(25)].characters_deleted, 15);
        assert_eq!(totals[&day(26)], activity(2));
    }

    #[test]
    fn zero_delta_creates_no_daily_record() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("statistics.json"));
        let stats = store.record(day(25), Activity::default()).unwrap();
        assert!(stats.daily_totals().unwrap().is_empty());
    }

    #[test]
    fn aggregates_devices_without_combining_their_storage() {
        let mut stats = Statistics::new().unwrap();
        stats.record(day(25), activity(10)).unwrap();
        let other = Statistics::new().unwrap();
        stats.devices.insert(
            other.local_device_id,
            BTreeMap::from([("2026-11-25".to_owned(), activity(3))]),
        );
        stats.validate().unwrap();
        assert_eq!(stats.daily_totals().unwrap()[&day(25)], activity(13));
        assert_eq!(stats.devices.len(), 2);
    }

    #[test]
    fn invalid_files_are_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let store = Store::new(path.clone());
        let stats = store.load().unwrap();
        let mut unsupported = serde_json::to_value(&stats).unwrap();
        unsupported["version"] = 999.into();
        let mut invalid_date = serde_json::to_value(&stats).unwrap();
        invalid_date["devices"][stats.local_device_id()] =
            serde_json::json!({"2026-02-30": activity(1)});
        let mut missing_device = serde_json::to_value(&stats).unwrap();
        missing_device["devices"] = serde_json::json!({});
        for bytes in [
            b"not JSON".to_vec(),
            serde_json::to_vec(&unsupported).unwrap(),
            serde_json::to_vec(&invalid_date).unwrap(),
            serde_json::to_vec(&missing_device).unwrap(),
        ] {
            fs::write(&path, &bytes).unwrap();
            assert!(store.load().is_err());
            assert!(store.record(day(25), activity(1)).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }

    #[test]
    fn overflow_leaves_previous_snapshot_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        let store = Store::new(path.clone());
        store
            .record(
                day(25),
                Activity {
                    words_added: u64::MAX,
                    ..Activity::default()
                },
            )
            .unwrap();
        let before = fs::read(&path).unwrap();
        assert!(store.record(day(25), activity(1)).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn concurrent_writers_do_not_lose_updates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statistics.json");
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let store = Store::new(path.clone());
                scope.spawn(move || {
                    for _ in 0..10 {
                        store.record(day(25), activity(1)).unwrap();
                    }
                });
            }
        });
        let stats = Store::new(path).load().unwrap();
        assert_eq!(stats.daily_totals().unwrap()[&day(25)], activity(40));
        assert_eq!(stats.devices.len(), 1);
    }

    #[test]
    fn xdg_path_precedes_home_and_relative_paths_are_ignored() {
        assert_eq!(
            state_path_from(Some("/state".into()), Some("/home/user".into())),
            Some(PathBuf::from("/state/zettui/statistics.json"))
        );
        assert_eq!(
            state_path_from(Some("relative".into()), Some("/home/user".into())),
            Some(PathBuf::from(
                "/home/user/.local/state/zettui/statistics.json"
            ))
        );
        assert_eq!(state_path_from(None, None), None);
    }
}

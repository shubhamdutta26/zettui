//! Explicit, duplicate-safe exchange of installation snapshots.

use super::{Activity, MAX_BYTES, Statistics, Store, invalid};
use std::{collections::BTreeMap, io, io::Write, path::Path};

/// Number of new device identities and changed device/date records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MergeSummary {
    pub new_devices: usize,
    pub updated_days: usize,
}

impl Activity {
    /// A later snapshot of one device/day cannot decrease any counter.
    fn dominates(self, other: Self) -> bool {
        self.words_added >= other.words_added
            && self.words_deleted >= other.words_deleted
            && self.characters_added >= other.characters_added
            && self.characters_deleted >= other.characters_deleted
    }
}

impl Statistics {
    fn merge(
        &mut self,
        incoming: BTreeMap<String, BTreeMap<String, Activity>>,
    ) -> io::Result<MergeSummary> {
        let mut summary = MergeSummary::default();
        for (id, days) in incoming {
            if !self.devices.contains_key(&id) {
                summary.new_devices += 1;
            }
            let local = self.devices.entry(id.clone()).or_default();
            for (date, activity) in days {
                use std::collections::btree_map::Entry;
                match local.entry(date) {
                    Entry::Vacant(entry) => {
                        entry.insert(activity);
                        summary.updated_days += 1;
                    }
                    Entry::Occupied(mut entry) => {
                        let previous = *entry.get();
                        if previous.dominates(activity) {
                            continue;
                        }
                        if !activity.dominates(previous) {
                            return Err(invalid(&format!(
                                "Conflicting counters for device {id} on {}; possible shared device ID",
                                entry.key()
                            )));
                        }
                        entry.insert(activity);
                        summary.updated_days += 1;
                    }
                }
            }
        }
        // Includes validation of the sums across devices, before any disk write.
        self.validate()?;
        Ok(summary)
    }
}

impl Store {
    /// Export a validated, persisted snapshot including every known device.
    /// Never replace an existing file (including the live state or a symlink).
    /// An empty installation is initialized on its first explicit export.
    pub fn export_to(&self, destination: &Path) -> io::Result<()> {
        let stats = self.load()?;
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut temp, &stats)?;
        temp.write_all(b"\n")?;
        if temp.as_file().metadata()?.len() > MAX_BYTES {
            return Err(invalid("Writing statistics file too large"));
        }
        temp.as_file().sync_all()?;
        temp.persist_noclobber(destination)
            .map_err(|error| error.error)?;
        Ok(())
    }

    /// Merge a complete exported snapshot. Preserve the receiving installation's
    /// identity; the imported snapshot's local identity never becomes ours.
    /// Read the input before locking our own state; exports are atomic files.
    pub fn import_from(&self, source: &Path) -> io::Result<MergeSummary> {
        let incoming = Store::new(source.to_path_buf()).read()?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Statistics snapshot not found")
        })?;
        self.merge_devices(incoming.devices)
    }

    pub(super) fn merge_devices(
        &self,
        incoming: BTreeMap<String, BTreeMap<String, Activity>>,
    ) -> io::Result<MergeSummary> {
        let _lock = self.lock()?;
        let existing = self.read()?;
        let was_missing = existing.is_none();
        let mut local = match existing {
            Some(stats) => stats,
            None => Statistics::new()?,
        };
        let summary = local.merge(incoming)?;
        if was_missing || summary != MergeSummary::default() {
            self.save(&local)?;
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::fs;

    fn day(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, n).unwrap()
    }

    fn activity(n: u64) -> Activity {
        Activity {
            words_added: n,
            characters_added: n * 5,
            words_deleted: n / 2,
            characters_deleted: n,
        }
    }

    #[test]
    fn merges_same_day_across_devices_and_preserves_local_id() {
        let dir = tempfile::tempdir().unwrap();
        let a = Store::new(dir.path().join("a.json"));
        let b = Store::new(dir.path().join("b.json"));
        let a_id = a.record(day(25), activity(10)).unwrap().local_device_id;
        let b_id = b.record(day(25), activity(4)).unwrap().local_device_id;
        assert_ne!(a_id, b_id);
        let snapshot = dir.path().join("export.json");
        a.export_to(&snapshot).unwrap();
        assert_eq!(
            b.import_from(&snapshot).unwrap(),
            MergeSummary {
                new_devices: 1,
                updated_days: 1
            }
        );
        assert_eq!(b.load().unwrap().local_device_id(), b_id);
        assert_eq!(b.daily_totals().unwrap()[&day(25)], activity(14));
        let before = fs::read(&b.path).unwrap();
        assert_eq!(b.import_from(&snapshot).unwrap(), MergeSummary::default());
        assert_eq!(fs::read(&b.path).unwrap(), before);
    }

    #[test]
    fn newer_older_and_round_trip_snapshots_do_not_double_count() {
        let dir = tempfile::tempdir().unwrap();
        let a = Store::new(dir.path().join("a.json"));
        let b = Store::new(dir.path().join("b.json"));
        let c = Store::new(dir.path().join("c.json"));
        let old = dir.path().join("old.json");
        let new = dir.path().join("new.json");
        let merged = dir.path().join("merged.json");
        a.record(day(25), activity(10)).unwrap();
        a.export_to(&old).unwrap();
        b.import_from(&old).unwrap();
        a.record(day(25), activity(4)).unwrap();
        a.record(day(26), activity(2)).unwrap();
        a.export_to(&new).unwrap();
        b.record(day(25), activity(6)).unwrap();
        assert_eq!(b.import_from(&new).unwrap().updated_days, 2);
        assert_eq!(b.import_from(&old).unwrap(), MergeSummary::default());
        b.export_to(&merged).unwrap();
        a.import_from(&merged).unwrap();
        c.import_from(&merged).unwrap();
        c.import_from(&new).unwrap();
        let totals = a.daily_totals().unwrap();
        assert_eq!(totals, b.daily_totals().unwrap());
        assert_eq!(totals, c.daily_totals().unwrap());
        assert_eq!(totals[&day(25)], activity(20));
        assert_eq!(totals[&day(26)], activity(2));
        assert_eq!(a.load().unwrap().devices.len(), 2);
        assert_eq!(c.load().unwrap().devices.len(), 3);
    }

    #[test]
    fn bad_input_overflow_and_conflict_leave_receiver_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let a = Store::new(dir.path().join("a.json"));
        let b = Store::new(dir.path().join("b.json"));
        a.record(day(25), activity(4)).unwrap();
        let before = fs::read(&a.path).unwrap();
        let source = dir.path().join("input.json");
        assert!(a.import_from(&source).is_err());
        fs::write(&source, "invalid json").unwrap();
        assert!(a.import_from(&source).is_err());
        let mut snapshot = a.load().unwrap();
        let id = snapshot.local_device_id.clone();
        let counter = snapshot
            .devices
            .get_mut(&id)
            .unwrap()
            .get_mut("2026-11-25")
            .unwrap();
        counter.words_added += 1;
        counter.characters_added -= 1;
        fs::write(&source, serde_json::to_vec(&snapshot).unwrap()).unwrap();
        assert!(a.import_from(&source).is_err());
        b.record(
            day(25),
            Activity {
                words_added: u64::MAX,
                ..Activity::default()
            },
        )
        .unwrap();
        assert!(a.import_from(&b.path).is_err());
        assert_eq!(fs::read(&a.path).unwrap(), before);
    }

    #[test]
    fn export_refuses_to_overwrite_and_import_does_not_change_source() {
        let dir = tempfile::tempdir().unwrap();
        let a = Store::new(dir.path().join("a.json"));
        let b = Store::new(dir.path().join("b.json"));
        let path = dir.path().join("export.json");
        a.record(day(25), activity(2)).unwrap();
        a.export_to(&path).unwrap();
        let original = fs::read(&path).unwrap();
        a.record(day(25), activity(2)).unwrap();
        assert!(a.export_to(&path).is_err());
        assert!(a.export_to(&a.path).is_err());
        b.import_from(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(b.daily_totals().unwrap()[&day(25)], activity(2));
        assert_eq!(a.daily_totals().unwrap()[&day(25)], activity(4));
    }

    #[test]
    fn local_writes_and_imports_are_serialized() {
        let dir = tempfile::tempdir().unwrap();
        let a = Store::new(dir.path().join("a.json"));
        let b = Store::new(dir.path().join("b.json"));
        let export = dir.path().join("export.json");
        a.record(day(25), activity(10)).unwrap();
        a.export_to(&export).unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for _ in 0..10 {
                    b.record(day(25), activity(2)).unwrap();
                }
            });
            scope.spawn(|| {
                for _ in 0..10 {
                    b.import_from(&export).unwrap();
                }
            });
        });
        assert_eq!(b.daily_totals().unwrap()[&day(25)], activity(30));
    }
}

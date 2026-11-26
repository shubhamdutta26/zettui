//! Import legacy daily Zettlr counters without inventing edit history.

use super::{Activity, MAX_BYTES, MergeSummary, Store, invalid, parse_date};
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, io, io::Read, path::Path};

// Source labels are explicit, stable and portable across installations.
// The namespace cannot overlap a native 32-digit hexadecimal device ID.
pub(super) fn valid_source(source: &str) -> bool {
    !source.is_empty()
        && source.len() <= 48
        && source
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

#[derive(Deserialize)]
struct Legacy {
    #[serde(rename = "wordCount")]
    words: BTreeMap<String, u64>,
    #[serde(rename = "charCount")]
    characters: BTreeMap<String, u64>,
    // Pomodoros and other unrelated fields are intentionally ignored.
}

impl Legacy {
    fn read(path: &Path) -> io::Result<Self> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(invalid("Zettlr statistics file too large"));
        }
        serde_json::from_slice(&bytes).map_err(io::Error::from)
    }

    fn into_days(self) -> io::Result<BTreeMap<String, Activity>> {
        let mut days = BTreeMap::<String, Activity>::new();
        for (date, count) in self.words {
            parse_date(&date)?;
            days.entry(date).or_default().words_added = count;
        }
        for (date, count) in self.characters {
            parse_date(&date)?;
            days.entry(date).or_default().characters_added = count;
        }
        Ok(days)
    }
}

impl Store {
    /// Import cumulative Zettlr counts for one explicitly named history.
    /// Reuse the same source for all exports of that history, on every machine.
    /// Older or identical records are no-ops; conflicting records are rejected.
    pub fn import_zettlr(&self, path: &Path, source: &str) -> io::Result<MergeSummary> {
        if !valid_source(source) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Zettlr SOURCE must be 1–48 lowercase ASCII letters, digits, hyphens or underscores",
            ));
        }
        let days = Legacy::read(path)?.into_days()?;
        self.merge_devices(BTreeMap::from([(format!("zettlr:{source}"), days)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::fs;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn input(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    const OLD: &str = r#"{"wordCount":{"2026-11-24":10,"2026-11-25":0},"charCount":{"2026-11-24":50,"2026-11-26":3},"pomodoros":{}}"#;
    const NEW: &str = r#"{"wordCount":{"2026-11-24":15,"2026-11-25":0,"2026-11-27":2},"charCount":{"2026-11-24":80,"2026-11-26":3},"pomodoros":{"2026-11-24":2}}"#;

    #[test]
    fn union_of_dates_preserves_zero_and_character_only_days() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("state.json"));
        let path = input(dir.path(), "stats.json", OLD);
        let summary = store.import_zettlr(&path, "desktop").unwrap();
        assert_eq!(
            summary,
            MergeSummary {
                new_devices: 1,
                updated_days: 3
            }
        );
        let totals = store.daily_totals().unwrap();
        assert_eq!(totals[&date(24)].words_added, 10);
        assert_eq!(totals[&date(24)].characters_added, 50);
        assert_eq!(totals[&date(25)], Activity::default());
        assert_eq!(totals[&date(26)].words_added, 0);
        assert_eq!(totals[&date(26)].characters_added, 3);
        assert!(
            totals
                .values()
                .all(|a| a.words_deleted == 0 && a.characters_deleted == 0)
        );
        assert_eq!(fs::read_to_string(path).unwrap(), OLD);
    }

    #[test]
    fn repeated_renamed_and_old_imports_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("state.json"));
        let old = input(dir.path(), "stats.json", OLD);
        let new = input(dir.path(), "later.json", NEW);
        store.import_zettlr(&old, "desktop").unwrap();
        assert_eq!(
            store.import_zettlr(&new, "desktop").unwrap().updated_days,
            2
        );
        let before = fs::read(&store.path).unwrap();
        let renamed = input(dir.path(), "copy.json", NEW);
        for path in [&old, &new, &renamed] {
            assert_eq!(
                store.import_zettlr(path, "desktop").unwrap(),
                MergeSummary::default()
            );
        }
        assert_eq!(fs::read(&store.path).unwrap(), before);
        let totals = store.daily_totals().unwrap();
        assert_eq!(totals[&date(24)].words_added, 15);
        assert_eq!(totals[&date(27)].words_added, 2);
    }

    #[test]
    fn distinct_histories_and_native_activity_merge_across_computers() {
        let dir = tempfile::tempdir().unwrap();
        let a = Store::new(dir.path().join("a.json"));
        let b = Store::new(dir.path().join("b.json"));
        let old = input(dir.path(), "stats.json", OLD);
        let new = input(dir.path(), "new.json", NEW);
        let a_id = a
            .record(
                date(24),
                Activity {
                    words_added: 3,
                    ..Activity::default()
                },
            )
            .unwrap()
            .local_device_id;
        let b_id = b.load().unwrap().local_device_id;
        a.import_zettlr(&old, "desktop").unwrap();
        b.import_zettlr(&new, "desktop").unwrap();
        b.import_zettlr(&old, "laptop").unwrap();
        let snapshot = dir.path().join("snapshot.json");
        b.export_to(&snapshot).unwrap();
        a.import_from(&snapshot).unwrap();
        assert_eq!(a.daily_totals().unwrap()[&date(24)].words_added, 28);
        assert_eq!(a.daily_totals().unwrap()[&date(24)].characters_added, 130);
        assert_eq!(
            a.import_zettlr(&new, "desktop").unwrap(),
            MergeSummary::default()
        );
        let merged = dir.path().join("merged.json");
        a.export_to(&merged).unwrap();
        b.import_from(&merged).unwrap();
        assert_eq!(a.daily_totals().unwrap(), b.daily_totals().unwrap());
        assert_eq!(a.load().unwrap().local_device_id(), a_id);
        assert_eq!(b.load().unwrap().local_device_id(), b_id);
    }

    #[test]
    fn malformed_input_conflicts_and_overflow_are_transactional() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("state.json"));
        let old = input(dir.path(), "stats.json", OLD);
        store.import_zettlr(&old, "desktop").unwrap();
        let before = fs::read(&store.path).unwrap();
        for text in [
            "not json",
            r#"{"wordCount":{}}"#,
            r#"{"wordCount":{"2026-02-30":1},"charCount":{}}"#,
            r#"{"wordCount":{},"charCount":{"2026-11-24":-1}}"#,
            r#"{"wordCount":{"2026-11-24":1.5},"charCount":{}}"#,
            r#"{"wordCount":{"2026-11-24":11},"charCount":{"2026-11-24":49}}"#,
            r#"{"wordCount":{"2026-11-24":18446744073709551615},"charCount":{}}"#,
        ] {
            let bad = input(dir.path(), "bad.json", text);
            // A separate history exercises aggregate overflow; other malformed
            // and conflicting snapshots use the original source.
            let source = if text.contains("18446744073709551615") {
                "laptop"
            } else {
                "desktop"
            };
            assert!(
                store.import_zettlr(&bad, source).is_err(),
                "accepted {text}"
            );
            assert_eq!(fs::read(&store.path).unwrap(), before);
        }
        for label in ["", "Desktop", "two words", "desktop:1", "../desktop"] {
            assert!(store.import_zettlr(&old, label).is_err());
        }
        assert_eq!(fs::read(&store.path).unwrap(), before);
    }
}

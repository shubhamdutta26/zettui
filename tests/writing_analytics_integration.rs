//! Single-host integration test with isolated installation state directories.
//! This does not replace a physical multi-computer interoperability test.

use chrono::NaiveDate;
use std::{fs, path::Path, process::Command};
use zettui::{
    analytics::Goals,
    statistics::{Activity, Store},
    writing_activity::Tracker,
};

fn day(n: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, n).unwrap()
}

fn run_cli(state: &Path, flag: &str, file: &Path, source: Option<&str>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zettui"));
    command.env("XDG_STATE_HOME", state).arg(flag).arg(file);
    if let Some(source) = source {
        command.arg(source);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{flag}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn migration_native_edits_pending_goals_and_round_trip_converge() {
    let dir = tempfile::tempdir().unwrap();
    let state_a = dir.path().join("state-a");
    let state_b = dir.path().join("state-b");
    let path_a = state_a.join("zettui/statistics.json");
    let path_b = state_b.join("zettui/statistics.json");
    let legacy = dir.path().join("legacy.json");
    let original = r#"{"wordCount":{"2026-11-23":10,"2026-11-24":20},"charCount":{"2026-11-23":50,"2026-11-24":100},"pomodoros":{}}"#;
    fs::write(&legacy, original).unwrap();
    for state in [&state_a, &state_b] {
        run_cli(state, "--stats-import-zettlr", &legacy, Some("desktop"));
    }
    let store_a = Store::new(path_a.clone());
    let store_b = Store::new(path_b.clone());
    let id_a = store_a.load().unwrap().local_device_id().to_owned();
    let id_b = store_b.load().unwrap().local_device_id().to_owned();
    assert_ne!(id_a, id_b);

    let goals = Goals {
        daily_words: 4,
        weekly_words: 40,
    };
    let mut a = Tracker::with_goals(Some(path_a), goals);
    let mut b = Tracker::with_goals(Some(path_b), goals);
    a.record_edit(day(25), "", "hello world").unwrap();
    b.record_edit(day(25), "", "one two three").unwrap();
    b.record_edit(day(25), "one two three", "one two").unwrap();
    a.flush().unwrap();
    b.flush().unwrap();
    b.record_edit(day(26), "", "alpha beta").unwrap();

    let export_a = dir.path().join("a.json");
    run_cli(&state_a, "--stats-export", &export_a, None);
    run_cli(&state_b, "--stats-import", &export_a, None);
    run_cli(&state_b, "--stats-import", &export_a, None);
    let (history, before_flush) = b.snapshot(day(26)).unwrap();
    assert_eq!(history[&day(23)].words_added, 10);
    assert_eq!(history[&day(24)].words_added, 20);
    assert_eq!(
        history[&day(25)],
        Activity {
            words_added: 5,
            words_deleted: 1,
            characters_added: 24,
            characters_deleted: 6,
        }
    );
    assert_eq!(history[&day(26)].words_added, 2);
    assert_eq!(before_flush.all_time.totals.words_added, 37);
    assert_eq!(before_flush.all_time.totals.characters_added, 184);
    assert_eq!(before_flush.week.totals.words_added, 37);
    assert_eq!(before_flush.streaks.current, 4);
    assert_eq!(before_flush.streaks.longest, 4);
    assert_eq!(before_flush.daily_goal.unwrap().percent(), 50);
    assert_eq!(before_flush.weekly_goal.unwrap().remaining(), 3);
    assert_eq!(before_flush.weekly_goal.unwrap().percent(), 92);
    assert!(!store_b.daily_totals().unwrap().contains_key(&day(26)));

    b.flush().unwrap();
    b.flush().unwrap();
    assert_eq!(
        b.snapshot(day(26)).unwrap(),
        (history.clone(), before_flush)
    );
    let export_b = dir.path().join("b.json");
    run_cli(&state_b, "--stats-export", &export_b, None);
    run_cli(&state_a, "--stats-import", &export_b, None);
    run_cli(&state_b, "--stats-import", &export_a, None);
    run_cli(&state_a, "--stats-import-zettlr", &legacy, Some("desktop"));
    assert_eq!(
        a.snapshot(day(26)).unwrap(),
        (history.clone(), before_flush)
    );
    assert_eq!(b.snapshot(day(26)).unwrap(), (history, before_flush));
    assert_eq!(store_a.load().unwrap().local_device_id(), id_a);
    assert_eq!(store_b.load().unwrap().local_device_id(), id_b);
    assert_eq!(fs::read_to_string(legacy).unwrap(), original);
}

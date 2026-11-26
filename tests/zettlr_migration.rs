use chrono::NaiveDate;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use zettui::statistics::Store;

fn cli(state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zettui"))
        .env("XDG_STATE_HOME", state)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn migration_cli_requires_source_and_merges_without_opening_tui() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let file = dir.path().join("legacy stats.json");
    fs::write(
        &file,
        r#"{"wordCount":{"2026-11-26":123},"charCount":{"2026-11-26":456},"pomodoros":{}}"#,
    )
    .unwrap();
    let file = file.to_str().unwrap();
    let store = Store::new(state.join("zettui/statistics.json"));
    assert!(
        !cli(&state, &["--stats-import-zettlr", file])
            .status
            .success()
    );
    assert!(store.daily_totals().unwrap().is_empty());
    for label in ["desktop", "desktop", "laptop"] {
        let output = cli(&state, &["--stats-import-zettlr", file, label]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let date = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
    let activity = store.daily_totals().unwrap()[&date];
    assert_eq!(activity.words_added, 246);
    assert_eq!(activity.characters_added, 912);
    let before = fs::read(state.join("zettui/statistics.json")).unwrap();
    assert!(
        !cli(&state, &["--stats-import-zettlr", file, "desktop", "extra"])
            .status
            .success()
    );
    assert_eq!(
        fs::read(state.join("zettui/statistics.json")).unwrap(),
        before
    );
}

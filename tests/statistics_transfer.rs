use chrono::NaiveDate;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use zettui::statistics::{Activity, Store};

fn cli(state: &Path, flag: &str, path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_zettui"));
    command.env("XDG_STATE_HOME", state).arg(flag);
    if let Some(path) = path {
        command.arg(path);
    }
    command.output().unwrap()
}

#[test]
fn command_line_round_trip_uses_independent_state_directories() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    let date = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
    let sa = Store::new(a.join("zettui/statistics.json"));
    let sb = Store::new(b.join("zettui/statistics.json"));
    sa.record(
        date,
        Activity {
            words_added: 10,
            ..Activity::default()
        },
    )
    .unwrap();
    sb.record(
        date,
        Activity {
            words_added: 4,
            ..Activity::default()
        },
    )
    .unwrap();
    let snapshot = temp.path().join("export with spaces.json");
    let output = cli(&a, "--stats-export", Some(&snapshot));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(cli(&b, "--stats-import", Some(&snapshot)).status.success());
    assert_eq!(sb.daily_totals().unwrap()[&date].words_added, 14);
    let output = cli(&b, "--stats-import", Some(&snapshot));
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("0 updated"));
    assert_eq!(sb.daily_totals().unwrap()[&date].words_added, 14);
    assert!(!cli(&b, "--stats-export", Some(&snapshot)).status.success());
    let id = cli(&b, "--stats-device-id", None);
    assert!(id.status.success());
    assert_eq!(
        String::from_utf8_lossy(&id.stdout).trim(),
        sb.load().unwrap().local_device_id()
    );
    let before = fs::read(b.join("zettui/statistics.json")).unwrap();
    assert!(!cli(&b, "--stats-import", None).status.success());
    assert_eq!(fs::read(b.join("zettui/statistics.json")).unwrap(), before);
}

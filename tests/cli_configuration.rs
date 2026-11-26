use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_zettui"))
        .current_dir(home)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn write_config(home: &Path, contents: &str) {
    let dir = home.join("config/zettui");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("config.toml"), contents).unwrap();
}

#[test]
fn help_and_version_do_not_require_valid_configuration() {
    let temp = tempfile::tempdir().unwrap();
    write_config(temp.path(), "invalid toml [");
    for flag in ["--help", "-h"] {
        let output = cli(temp.path(), &[flag]);
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("--check-config"));
    }
    for flag in ["--version", "-V"] {
        let output = cli(temp.path(), &[flag]);
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            concat!("zettui ", env!("CARGO_PKG_VERSION"))
        );
    }
    assert!(!temp.path().join("state").exists());
}

#[test]
fn check_config_accepts_defaults_and_example_without_writing_state() {
    let temp = tempfile::tempdir().unwrap();
    assert!(cli(temp.path(), &["--check-config"]).status.success());
    assert!(!temp.path().join("config").exists());
    write_config(temp.path(), include_str!("../config.example.toml"));
    let output = cli(temp.path(), &["--check-config"]);
    assert!(output.status.success(), "{:?}", output);
    assert!(!temp.path().join("state").exists());
}

#[test]
fn check_config_rejects_invalid_syntax_and_values() {
    let temp = tempfile::tempdir().unwrap();
    for (contents, expected) in [
        ("invalid toml [", "config.toml"),
        (
            "[layout]\nbrowser_width_percent = 71",
            "browser_width_percent",
        ),
        (
            "[layout]\noutline_width_percent = 14",
            "outline_width_percent",
        ),
        ("[appearance]\ntheme = 'no-such-theme'", "Unknown theme"),
        ("[appearance]\nlight_theme = 'dark'", "light_theme"),
        ("[colors]\naccent = 'invalid'", "[colors].accent"),
    ] {
        write_config(temp.path(), contents);
        let output = cli(temp.path(), &["--check-config"]);
        assert!(!output.status.success(), "{contents}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{:?}",
            output
        );
    }
}

#[test]
fn missing_option_values_fail_before_config_or_statistics_are_loaded() {
    let temp = tempfile::tempdir().unwrap();
    write_config(temp.path(), "invalid toml [");
    for (args, expected) in [
        (&["--theme", "--version"][..], "--theme requires"),
        (&["--bibliography", "--help"][..], "--bibliography requires"),
        (&["--stats-export", "--help"][..], "--stats-export requires"),
        (
            &["--stats-import", "--version"][..],
            "--stats-import requires",
        ),
    ] {
        let output = cli(temp.path(), args);
        assert!(!output.status.success(), "{args:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
    assert!(!temp.path().join("--help").exists());
    assert!(!temp.path().join("state").exists());
}

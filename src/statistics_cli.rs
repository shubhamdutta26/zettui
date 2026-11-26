//! Standalone snapshot commands, before configuration or terminal initialization.

use std::{env, io, path::PathBuf};
use zettui::statistics::{Store, state_path};

pub(crate) fn run() -> io::Result<bool> {
    let mut args = env::args_os().skip(1);
    let Some(first) = args.next() else {
        return Ok(false);
    };
    let Some(
        flag
        @ ("--stats-export" | "--stats-import" | "--stats-device-id" | "--stats-import-zettlr"),
    ) = first.to_str()
    else {
        return Ok(false);
    };
    let path = if flag == "--stats-device-id" {
        None
    } else {
        Some(PathBuf::from(args.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{flag} requires a file path"),
            )
        })?))
    };
    let source = if flag == "--stats-import-zettlr" {
        Some(
            args.next()
                .and_then(|value| value.into_string().ok())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--stats-import-zettlr requires FILE SOURCE (e.g. desktop)",
                    )
                })?,
        )
    } else {
        None
    };
    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unexpected arguments after {flag}"),
        ));
    }
    let store = Store::new(
        state_path().ok_or_else(|| io::Error::other("No absolute XDG_STATE_HOME or HOME"))?,
    );
    match (flag, path, source) {
        ("--stats-export", Some(path), None) => {
            store.export_to(&path)?;
            println!("Exported writing statistics to {}", path.display());
        }
        ("--stats-import", Some(path), None) => {
            let summary = store.import_from(&path)?;
            println!(
                "Imported writing statistics: {} new device(s), {} updated device/day record(s)",
                summary.new_devices, summary.updated_days
            );
        }
        ("--stats-import-zettlr", Some(path), Some(source)) => {
            let summary = store.import_zettlr(&path, &source)?;
            println!(
                "Imported Zettlr history '{source}': {} new source(s), {} updated source/day record(s)",
                summary.new_devices, summary.updated_days
            );
        }
        ("--stats-device-id", None, None) => println!("{}", store.load()?.local_device_id()),
        _ => unreachable!(),
    }
    Ok(true)
}

//! Standalone snapshot commands, before configuration or terminal initialization.

use std::{env, io, path::PathBuf};
use zettui::statistics::{Store, state_path};

pub(crate) fn run() -> io::Result<bool> {
    let mut args = env::args_os().skip(1);
    let Some(first) = args.next() else {
        return Ok(false);
    };
    let Some(flag @ ("--stats-export" | "--stats-import" | "--stats-device-id")) = first.to_str()
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
    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unexpected arguments after {flag}"),
        ));
    }
    let store = Store::new(
        state_path().ok_or_else(|| io::Error::other("No absolute XDG_STATE_HOME or HOME"))?,
    );
    match (flag, path) {
        ("--stats-export", Some(path)) => {
            store.export_to(&path)?;
            println!("Exported writing statistics to {}", path.display());
        }
        ("--stats-import", Some(path)) => {
            let summary = store.import_from(&path)?;
            println!(
                "Imported writing statistics: {} new device(s), {} updated device/day record(s)",
                summary.new_devices, summary.updated_days
            );
        }
        ("--stats-device-id", None) => println!("{}", store.load()?.local_device_id()),
        _ => unreachable!(),
    }
    Ok(true)
}

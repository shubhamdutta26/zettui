//! Local configuration, stored in $XDG_CONFIG_HOME/zettui/config.toml
//! (or ~/.config/zettui/config.toml). Zettui never writes this file.

use crate::theme::ColorOverrides;
use serde::Deserialize;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub bibliography: Option<PathBuf>,
    pub notes_dir: Option<PathBuf>,
    pub theme: Option<String>,
    #[serde(default)]
    pub colors: ColorOverrides,
}

impl Config {
    /// A missing config file is fine; an invalid one is an actionable error.
    pub fn load() -> io::Result<Self> {
        let Some(config_file) = default_config_path() else {
            return Ok(Self::default());
        };
        if !config_file.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&config_file)?;
        let mut cfg: Self = toml::from_str(&raw).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid {}: {error}", config_file.display()),
            )
        })?;
        let config_dir = config_file.parent().unwrap_or(Path::new("."));
        cfg.bibliography = cfg.bibliography.map(|p| expand_path(p, config_dir));
        cfg.notes_dir = cfg.notes_dir.map(|p| expand_path(p, config_dir));
        Ok(cfg)
    }
}

fn default_config_path() -> Option<PathBuf> {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(xdg).join("zettui/config.toml"));
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/zettui/config.toml"))
}

fn expand_path(path: PathBuf, config_dir: &Path) -> PathBuf {
    if path == Path::new("~") {
        return env::var_os("HOME").map(PathBuf::from).unwrap_or(path);
    }
    if let Ok(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    if path.is_relative() {
        config_dir.join(path)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_and_optional_fields() {
        let cfg: Config = toml::from_str(
            r#"bibliography = "/tmp/refs.json"
notes_dir = "/tmp/notes"
"#,
        )
        .unwrap();
        assert_eq!(cfg.bibliography.unwrap(), PathBuf::from("/tmp/refs.json"));
    }

    #[test]
    fn parses_theme_and_color_overrides() {
        let cfg: Config = toml::from_str("theme = 'auto'\n[colors]\naccent = '#123456'\n").unwrap();
        assert_eq!(cfg.theme.as_deref(), Some("auto"));
        assert_eq!(cfg.colors.accent.as_deref(), Some("#123456"));
    }

    #[test]
    fn resolves_relative_path_against_config_dir() {
        assert_eq!(
            expand_path(
                PathBuf::from("refs.json"),
                Path::new("/home/test/.config/zettui")
            ),
            PathBuf::from("/home/test/.config/zettui/refs.json")
        );
    }
}

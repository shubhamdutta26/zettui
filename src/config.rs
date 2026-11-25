//! Local configuration, stored in $XDG_CONFIG_HOME/zettui/config.toml
//! (or ~/.config/zettui/config.toml). Zettui never writes this file.

use crate::theme::ColorOverrides;
use serde::Deserialize;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Deserialize)]
#[serde(from = "RawConfig")]
pub struct Config {
    pub bibliography: Option<PathBuf>,
    pub notes_dir: Option<PathBuf>,
    pub snippets_dir: Option<PathBuf>,
    pub theme: Option<String>,
    pub system_theme: crate::system_theme::Settings,
    pub light_theme: Option<String>,
    pub dark_theme: Option<String>,
    /// Desired percent of terminal width used by the left file pane.
    pub browser_width_percent: Option<u16>,
    pub colors: ColorOverrides,
    pub zen: crate::focus::ZenSettings,
    pub focus: crate::focus::FocusSettings,
    pub editor: crate::focus::EditorSettings,
    pub markdown: crate::markdown::MarkdownSettings,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Workspace {
    notes_dir: Option<PathBuf>,
    bibliography: Option<PathBuf>,
    snippets_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AppearanceSettings {
    #[serde(flatten)]
    system_theme: crate::system_theme::Settings,
    theme: Option<String>,
    light_theme: Option<String>,
    dark_theme: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct LayoutSettings {
    browser_width_percent: Option<u16>,
}

// Accept legacy top-level keys as well as the canonical sectioned format.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawConfig {
    workspace: Workspace,
    appearance: AppearanceSettings,
    layout: LayoutSettings,
    notes_dir: Option<PathBuf>,
    bibliography: Option<PathBuf>,
    snippets_dir: Option<PathBuf>,
    theme: Option<String>,
    light_theme: Option<String>,
    dark_theme: Option<String>,
    browser_width_percent: Option<u16>,
    colors: ColorOverrides,
    editor: crate::focus::EditorSettings,
    zen: crate::focus::ZenSettings,
    focus: crate::focus::FocusSettings,
    markdown: crate::markdown::MarkdownSettings,
}

impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        Self {
            notes_dir: raw.workspace.notes_dir.or(raw.notes_dir),
            bibliography: raw.workspace.bibliography.or(raw.bibliography),
            snippets_dir: raw.workspace.snippets_dir.or(raw.snippets_dir),
            theme: raw.appearance.theme.or(raw.theme),
            system_theme: raw.appearance.system_theme,
            light_theme: raw.appearance.light_theme.or(raw.light_theme),
            dark_theme: raw.appearance.dark_theme.or(raw.dark_theme),
            browser_width_percent: raw
                .layout
                .browser_width_percent
                .or(raw.browser_width_percent),
            colors: raw.colors,
            editor: raw.editor,
            zen: raw.zen,
            focus: raw.focus,
            markdown: raw.markdown,
        }
    }
}

impl Config {
    pub fn auto_themes(&self) -> io::Result<crate::theme::AutoThemes> {
        crate::theme::AutoThemes::named(
            self.light_theme.as_deref().unwrap_or("light"),
            self.dark_theme.as_deref().unwrap_or("dark"),
        )
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))
    }

    pub fn browser_width(&self) -> io::Result<u16> {
        use crate::layout::{DEFAULT_BROWSER_PERCENT, valid_browser_percent};
        let width = self
            .browser_width_percent
            .unwrap_or(DEFAULT_BROWSER_PERCENT);
        if !valid_browser_percent(width) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("browser_width_percent must be between 20 and 70 (got {width})"),
            ));
        }
        Ok(width)
    }

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
        cfg.snippets_dir = cfg.snippets_dir.map(|p| expand_path(p, config_dir));
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
    if let Ok(rest) = path.strip_prefix("~/")
        && let Some(home) = env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
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
    fn parses_auto_theme_presets() {
        let cfg: Config = toml::from_str(
            "theme = 'auto'\nlight_theme = 'catppuccin-latte'\ndark_theme = 'everforest'\n",
        )
        .unwrap();
        let presets = cfg.auto_themes().unwrap();
        use crate::theme::{Appearance, ThemeChoice};
        assert_eq!(
            presets.resolve(ThemeChoice::Auto, Appearance::Light),
            Appearance::CatppuccinLatte
        );
        assert_eq!(
            presets.resolve(ThemeChoice::Auto, Appearance::Dark),
            Appearance::Everforest
        );
        assert!(
            toml::from_str::<Config>("dark_theme = 'auto'")
                .unwrap()
                .auto_themes()
                .is_err()
        );
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

#[cfg(test)]
mod lesson73_config_tests {
    use super::*;

    #[test]
    fn uses_default_and_parses_custom_width() {
        let cfg: Config = toml::from_str("theme = 'auto'").unwrap();
        assert_eq!(cfg.browser_width().unwrap(), 35);
        let cfg: Config = toml::from_str("browser_width_percent = 50").unwrap();
        assert_eq!(cfg.browser_width().unwrap(), 50);
    }

    #[test]
    fn rejects_width_outside_allowed_range() {
        let cfg: Config = toml::from_str("browser_width_percent = 75").unwrap();
        assert!(cfg.browser_width().is_err());
    }
}

#[cfg(test)]
mod sectioned_config_tests {
    use super::*;
    #[test]
    fn sectioned_values_override_legacy_per_field() {
        let cfg: Config = toml::from_str(
            r#"
notes_dir = "/old"
bibliography = "/refs.json"
theme = "dark"
browser_width_percent = 35
[workspace]
notes_dir = "/new"
[appearance]
theme = "light"
[layout]
browser_width_percent = 20
"#,
        )
        .unwrap();
        assert_eq!(cfg.notes_dir.as_deref(), Some(Path::new("/new")));
        assert_eq!(cfg.bibliography.as_deref(), Some(Path::new("/refs.json")));
        assert_eq!(cfg.theme.as_deref(), Some("light"));
        assert_eq!(cfg.browser_width().unwrap(), 20);
    }
    #[test]
    fn example_and_empty_config_parse() {
        for source in ["", include_str!("../config.example.toml")] {
            let cfg: Config = toml::from_str(source).unwrap();
            assert_eq!(cfg.browser_width().unwrap(), 35);
            assert!(cfg.markdown.syntax_highlighting);
            assert!(cfg.markdown.highlight_code_blocks);
            assert!(cfg.markdown.highlight_frontmatter);
            assert!(cfg.markdown.dim_delimiters);
            assert!(cfg.editor.word_wrap);
            assert!(!cfg.zen.enabled);
            assert!(cfg.auto_themes().is_ok());
        }
    }
    #[test]
    fn invalid_heading_color_is_rejected() {
        assert!(toml::from_str::<Config>("[markdown.heading_colors]\nh1 = 'red'").is_err());
    }
}

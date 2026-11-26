//! Local configuration, stored in $XDG_CONFIG_HOME/zettui/config.toml
//! (or ~/.config/zettui/config.toml). Zettui never writes this file.

use crate::theme::ColorOverrides;
use serde::Deserialize;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// Canonical configuration mirrors the TOML sections directly.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub workspace: Workspace,
    pub appearance: AppearanceSettings,
    pub layout: LayoutSettings,
    pub colors: ColorOverrides,
    pub zen: crate::focus::ZenSettings,
    pub focus: crate::focus::FocusSettings,
    pub editor: crate::focus::EditorSettings,
    pub markdown: crate::markdown::MarkdownSettings,
    pub buffers: BufferSettings,
    pub browser: BrowserSettings,
    pub session: crate::session::Settings,
    pub writing_goals: zettui::analytics::Goals,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Workspace {
    pub notes_dir: Option<PathBuf>,
    pub bibliography: Option<PathBuf>,
    pub snippets_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    #[serde(flatten)]
    pub system_theme: crate::system_theme::Settings,
    pub theme: Option<String>,
    pub light_theme: Option<String>,
    pub dark_theme: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutSettings {
    /// Desired percent of terminal width used by the left file pane.
    pub browser_width_percent: Option<u16>,
    pub outline_width_percent: Option<u16>,
    pub show_files: Option<bool>,
    pub show_preview: Option<bool>,
    pub show_outline: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct BufferSettings {
    pub show_tab_bar: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct BrowserSettings {
    pub show_hidden_folders: bool,
}

impl Default for BufferSettings {
    fn default() -> Self {
        Self { show_tab_bar: true }
    }
}

impl Config {
    pub fn auto_themes(&self) -> io::Result<crate::theme::AutoThemes> {
        crate::theme::AutoThemes::named(
            self.appearance.light_theme.as_deref().unwrap_or("light"),
            self.appearance.dark_theme.as_deref().unwrap_or("dark"),
        )
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))
    }

    pub fn browser_width(&self) -> io::Result<u16> {
        use crate::layout::{DEFAULT_BROWSER_PERCENT, valid_browser_percent};
        let width = self
            .layout
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

    pub fn panels(&self) -> io::Result<crate::layout::Panels> {
        use crate::layout::{Panels, valid_outline_percent};
        let defaults = Panels::default();
        let width = self
            .layout
            .outline_width_percent
            .unwrap_or(defaults.outline_width_percent);
        if !valid_outline_percent(width) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("outline_width_percent must be between 15 and 50 (got {width})"),
            ));
        }
        Ok(Panels {
            show_files: self.layout.show_files.unwrap_or(defaults.show_files),
            show_preview: self.layout.show_preview.unwrap_or(defaults.show_preview),
            show_outline: self.layout.show_outline.unwrap_or(defaults.show_outline),
            outline_width_percent: width,
        })
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
        cfg.workspace.bibliography = cfg
            .workspace
            .bibliography
            .map(|p| expand_path(p, config_dir));
        cfg.workspace.notes_dir = cfg.workspace.notes_dir.map(|p| expand_path(p, config_dir));
        cfg.workspace.snippets_dir = cfg
            .workspace
            .snippets_dir
            .map(|p| expand_path(p, config_dir));
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
    fn writing_goals_default_to_disabled_and_accept_independent_targets() {
        assert_eq!(
            Config::default().writing_goals,
            zettui::analytics::Goals::default()
        );
        let config: Config = toml::from_str("[writing_goals]\ndaily_words = 500").unwrap();
        assert_eq!(config.writing_goals.daily_words, 500);
        assert_eq!(config.writing_goals.weekly_words, 0);
        let config: Config = toml::from_str("[writing_goals]\nweekly_words = 2500").unwrap();
        assert_eq!(config.writing_goals.weekly_words, 2500);
        assert!(toml::from_str::<Config>("[writing_goals]\ndaily_words = -1").is_err());
        assert!(toml::from_str::<Config>("[writing_goals]\nunknown = 1").is_err());
        let example: Config = toml::from_str(include_str!("../config.example.toml")).unwrap();
        assert_eq!(example.writing_goals, zettui::analytics::Goals::default());
    }

    #[test]
    fn hidden_folder_config_defaults_and_override() {
        assert!(!Config::default().browser.show_hidden_folders);
        let config: Config = toml::from_str("[browser]\nshow_hidden_folders = true").unwrap();
        assert!(config.browser.show_hidden_folders);
        let example: Config = toml::from_str(include_str!("../config.example.toml")).unwrap();
        assert!(!example.browser.show_hidden_folders);
    }

    #[test]
    fn session_and_tab_settings_are_backward_compatible() {
        let defaults = Config::default();
        assert!(defaults.session.enabled);
        assert!(defaults.session.restore_on_startup);
        assert!(defaults.session.restore_cursor);
        assert!(defaults.session.restore_layout);
        assert!(defaults.buffers.show_tab_bar);
        let cfg: Config = toml::from_str(
            "[buffers]\nshow_tab_bar = false\n[session]\nenabled = false\nrestore_cursor = false",
        )
        .unwrap();
        assert!(!cfg.buffers.show_tab_bar);
        assert!(!cfg.session.enabled);
        assert!(!cfg.session.restore_cursor);
        assert!(cfg.session.restore_layout);
    }

    #[test]
    fn parses_config_and_optional_fields() {
        let cfg: Config = toml::from_str(
            r#"[workspace]
bibliography = "/tmp/refs.json"
notes_dir = "/tmp/notes"
"#,
        )
        .unwrap();
        assert_eq!(
            cfg.workspace.bibliography.unwrap(),
            PathBuf::from("/tmp/refs.json")
        );
    }

    #[test]
    fn parses_auto_theme_presets() {
        let cfg: Config = toml::from_str(
            "[appearance]\ntheme = 'auto'\nlight_theme = 'catppuccin-latte'\ndark_theme = 'everforest'\n",
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
            toml::from_str::<Config>("[appearance]\ndark_theme = 'auto'")
                .unwrap()
                .auto_themes()
                .is_err()
        );
    }

    #[test]
    fn parses_theme_and_color_overrides() {
        let cfg: Config =
            toml::from_str("[appearance]\ntheme = 'auto'\n[colors]\naccent = '#123456'\n").unwrap();
        assert_eq!(cfg.appearance.theme.as_deref(), Some("auto"));
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
        let cfg: Config = toml::from_str("[appearance]\ntheme = 'auto'").unwrap();
        assert_eq!(cfg.browser_width().unwrap(), 35);
        let cfg: Config = toml::from_str("[layout]\nbrowser_width_percent = 50").unwrap();
        assert_eq!(cfg.browser_width().unwrap(), 50);
    }

    #[test]
    fn rejects_width_outside_allowed_range() {
        let cfg: Config = toml::from_str("[layout]\nbrowser_width_percent = 75").unwrap();
        assert!(cfg.browser_width().is_err());
    }
}

#[cfg(test)]
mod sectioned_config_tests {
    use super::*;
    #[test]
    fn rejects_legacy_root_keys() {
        assert!(toml::from_str::<Config>("notes_dir = '/old'").is_err());
        assert!(toml::from_str::<Config>("theme = 'dark'").is_err());
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
    fn parses_native_theme_options() {
        let cfg: Config = toml::from_str(
            "[appearance]\ntheme = 'system'\nsource = 'omarchy'\nlive_reload = false\nensure_contrast = false",
        ).unwrap();
        assert_eq!(
            cfg.appearance.system_theme.source,
            crate::system_theme::Source::Omarchy
        );
        assert!(!cfg.appearance.system_theme.live_reload);
        assert!(!cfg.appearance.system_theme.ensure_contrast);
    }
    #[test]
    fn parses_cosmic_source() {
        let cfg: Config =
            toml::from_str("[appearance]\ntheme = 'system'\nsource = 'cosmic'").unwrap();
        assert_eq!(
            cfg.appearance.system_theme.source,
            crate::system_theme::Source::Cosmic
        );
    }

    #[test]
    fn invalid_heading_color_is_rejected() {
        assert!(toml::from_str::<Config>("[markdown.heading_colors]\nh1 = 'red'").is_err());
    }
}

#[cfg(test)]
mod panel_config_tests {
    use super::*;

    #[test]
    fn old_and_new_layouts_parse() {
        let old: Config = toml::from_str("[layout]\nbrowser_width_percent = 50").unwrap();
        assert_eq!(old.panels().unwrap(), crate::layout::Panels::default());
        let cfg: Config = toml::from_str("[layout]\nshow_files = false\nshow_preview = false\nshow_outline = true\noutline_width_percent = 30").unwrap();
        let panels = cfg.panels().unwrap();
        assert!(!panels.show_files);
        assert!(!panels.show_preview);
        assert!(panels.show_outline);
        assert_eq!(panels.outline_width_percent, 30);
        let example: Config = toml::from_str(include_str!("../config.example.toml")).unwrap();
        assert_eq!(example.panels().unwrap(), crate::layout::Panels::default());
    }

    #[test]
    fn rejects_invalid_outline_width() {
        for width in [0, 14, 51, 100] {
            let cfg: Config =
                toml::from_str(&format!("[layout]\noutline_width_percent = {width}")).unwrap();
            assert!(cfg.panels().is_err());
        }
    }
}

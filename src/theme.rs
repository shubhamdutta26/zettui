//! Built-in and configurable terminal color palettes.
//! All custom colors use #RRGGBB values; an invalid value is a configuration error.
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use ratatui::style::{Color, Style};
use serde::Deserialize;

/// User-facing choices: Auto follows Omarchy, Dark and Light are fixed.
pub const THEME_CHOICES: [&str; 3] = ["auto", "dark", "light"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

impl Appearance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeChoice {
    Auto,
    Dark,
    Light,
}

impl ThemeChoice {
    pub fn named(value: &str) -> Result<Self, ThemeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "dark" | "catppuccin" | "gruvbox" | "tokyo-night" => Ok(Self::Dark),
            "light" | "paper" => Ok(Self::Light),
            _ => Err(ThemeError(format!(
                "Unknown theme '{value}'. Choose one of: {}",
                THEME_CHOICES.join(", ")
            ))),
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Dark,
            Self::Dark => Self::Light,
            Self::Light => Self::Auto,
        }
    }
    pub fn resolve(self, system: Appearance) -> Appearance {
        match self {
            Self::Auto => system,
            Self::Dark => Appearance::Dark,
            Self::Light => Appearance::Light,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

/// Reads the active Omarchy palette instead of guessing from the clock.
/// Omarchy Quattro stores its current theme under ~/.local/state/omarchy/current/theme.
/// Older versions used ~/.config/omarchy/current/theme.
pub fn omarchy_appearance() -> Option<Appearance> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    [
        home.join(".local/state/omarchy/current/theme"),
        home.join(".config/omarchy/current/theme"),
    ]
    .iter()
    .find_map(|path| appearance_from_theme_dir(path))
}

fn appearance_from_theme_dir(dir: &Path) -> Option<Appearance> {
    let colors = dir.join("colors.toml");
    if let Ok(content) = fs::read_to_string(&colors) {
        if let Ok(doc) = content.parse::<toml::Value>() {
            match doc.get("mode").and_then(toml::Value::as_str) {
                Some("light") => return Some(Appearance::Light),
                Some("dark") => return Some(Appearance::Dark),
                _ => {}
            }
        }
    }
    // Older Omarchy themes use this marker; otherwise an active theme is dark.
    if dir.join("light.mode").is_file() {
        return Some(Appearance::Light);
    }
    if colors.is_file() {
        return Some(Appearance::Dark);
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub surface: Color,
    pub foreground: Color,
    pub muted: Color,
    pub border: Color,
    pub accent: Color,
    pub directory: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub warning: Color,
    pub success: Color,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorOverrides {
    pub background: Option<String>,
    pub surface: Option<String>,
    pub foreground: Option<String>,
    pub muted: Option<String>,
    pub border: Option<String>,
    pub accent: Option<String>,
    pub directory: Option<String>,
    pub selected_bg: Option<String>,
    pub selected_fg: Option<String>,
    pub warning: Option<String>,
    pub success: Option<String>,
}

#[derive(Debug)]
pub struct ThemeError(pub String);

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ThemeError {}

fn rgb(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

impl Theme {
    pub fn named(name: &str) -> Result<Self, ThemeError> {
        // Theme names are intentionally fixed; custom themes are built by
        // overriding an existing palette via [colors] in config.toml.
        // Keep just two app palettes. Older Lesson 6 names remain accepted
        // as aliases, so existing config.toml files still load.
        let result = match ThemeChoice::named(name)? {
            ThemeChoice::Dark => Self {
                name: "dark",
                background: rgb(0x1e1e2e),
                surface: rgb(0x313244),
                foreground: rgb(0xcdd6f4),
                muted: rgb(0xa6adc8),
                border: rgb(0x585b70),
                accent: rgb(0xcba6f7),
                directory: rgb(0x89b4fa),
                selected_bg: rgb(0x45475a),
                selected_fg: rgb(0xf5e0dc),
                warning: rgb(0xf38ba8),
                success: rgb(0xa6e3a1),
            },
            ThemeChoice::Light => Self {
                name: "light",
                background: rgb(0xfaf8f2),
                surface: rgb(0xe9e6dd),
                foreground: rgb(0x2f3437),
                muted: rgb(0x5f666d),
                border: rgb(0xb8b6ad),
                accent: rgb(0x634b9e),
                directory: rgb(0x245b89),
                selected_bg: rgb(0xcbd8e7),
                selected_fg: rgb(0x17263b),
                warning: rgb(0xad3333),
                success: rgb(0x257047),
            },
            ThemeChoice::Auto => return Err(ThemeError("Use a resolved dark/light mode".into())),
        };
        Ok(result)
    }

    pub fn with_overrides(mut self, overrides: &ColorOverrides) -> Result<Self, ThemeError> {
        macro_rules! set_color {
            ($field:ident) => {
                if let Some(value) = &overrides.$field {
                    self.$field = parse_hex(value).map_err(|message| {
                        ThemeError(format!("[colors].{}: {message}", stringify!($field)))
                    })?;
                }
            };
        }
        set_color!(background);
        set_color!(surface);
        set_color!(foreground);
        set_color!(muted);
        set_color!(border);
        set_color!(accent);
        set_color!(directory);
        set_color!(selected_bg);
        set_color!(selected_fg);
        set_color!(warning);
        set_color!(success);
        Ok(self)
    }

    pub fn base_style(self) -> Style {
        // Paint with the emulator's default background. This covers Foot's
        // character cells while matching its outer margins exactly, even
        // when Omarchy updates the terminal's background on a theme switch.
        Style::default().fg(self.foreground).bg(Color::Reset)
    }
}

fn parse_hex(value: &str) -> Result<Color, String> {
    let Some(digits) = value.strip_prefix('#') else {
        return Err(format!("expected #RRGGBB, found '{value}'"));
    };
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("expected #RRGGBB, found '{value}'"));
    }
    let hex = u32::from_str_radix(digits, 16).map_err(|_| format!("invalid color '{value}'"))?;
    Ok(rgb(hex))
}

impl FromStr for Theme {
    type Err = ThemeError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::named(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_palettes_and_three_choices() {
        assert_eq!(Theme::named("dark").unwrap().name, "dark");
        assert_eq!(Theme::named("light").unwrap().name, "light");
        assert_eq!(ThemeChoice::Auto.next(), ThemeChoice::Dark);
        assert_eq!(ThemeChoice::Dark.next(), ThemeChoice::Light);
        assert_eq!(ThemeChoice::Light.next(), ThemeChoice::Auto);
        assert_eq!(
            ThemeChoice::Auto.resolve(Appearance::Light),
            Appearance::Light
        );
        assert_eq!(
            ThemeChoice::Dark.resolve(Appearance::Light),
            Appearance::Dark
        );
    }

    #[test]
    fn inherit_terminal_background() {
        assert_eq!(
            Theme::named("dark").unwrap().base_style().bg,
            Some(Color::Reset)
        );
    }

    #[test]
    fn omarchy_mode_detection() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path();
        fs::write(
            path.join("colors.toml"),
            "mode = 'light'\nbackground = '#eeeeee'\n",
        )
        .unwrap();
        assert_eq!(appearance_from_theme_dir(path), Some(Appearance::Light));
        fs::write(path.join("colors.toml"), "mode = 'dark'\n").unwrap();
        assert_eq!(appearance_from_theme_dir(path), Some(Appearance::Dark));
        fs::write(path.join("colors.toml"), "accent = '#123456'\n").unwrap();
        fs::write(path.join("light.mode"), "").unwrap();
        assert_eq!(appearance_from_theme_dir(path), Some(Appearance::Light));
    }

    #[test]
    fn hex_values_override_individual_color() {
        let overrides = ColorOverrides {
            accent: Some("#12ABef".into()),
            ..Default::default()
        };
        let base = Theme::named("dark").unwrap();
        let changed = base.with_overrides(&overrides).unwrap();
        assert_eq!(changed.accent, Color::Rgb(0x12, 0xab, 0xef));
        assert_eq!(changed.background, base.background);
    }

    #[test]
    fn rejects_malformed_color_and_unknown_palette() {
        let overrides = ColorOverrides {
            selected_fg: Some("red".to_owned()),
            ..Default::default()
        };
        assert!(
            Theme::named("paper")
                .unwrap()
                .with_overrides(&overrides)
                .unwrap_err()
                .to_string()
                .contains("selected_fg")
        );
        assert!(Theme::named("other").is_err());
        assert!(ThemeChoice::named("other").is_err());
    }

    #[test]
    fn parses_override_fields_from_toml() {
        #[derive(Deserialize)]
        struct TestCfg {
            colors: ColorOverrides,
        }
        let cfg: TestCfg = toml::from_str("[colors]\naccent = '#AABBCC'\n").unwrap();
        assert_eq!(cfg.colors.accent.as_deref(), Some("#AABBCC"));
    }
}

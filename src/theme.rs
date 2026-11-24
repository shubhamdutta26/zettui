//! Built-in and configurable terminal color palettes.
//! All custom colors use #RRGGBB values; an invalid value is a configuration error.
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use ratatui::style::{Color, Style};
use serde::Deserialize;

/// Auto follows Omarchy; all other choices are fixed palettes.
pub const THEME_CHOICES: &[&str] = &[
    "auto",
    "dark",
    "light",
    "nord",
    "dracula",
    "everforest",
    "rose-pine",
    "kanagawa",
    "solarized-dark",
    "catppuccin-latte",
    "rose-pine-dawn",
    "gruvbox",
    "tokyo-night",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
    Nord,
    Dracula,
    Everforest,
    RosePine,
    Kanagawa,
    SolarizedDark,
    CatppuccinLatte,
    RosePineDawn,
    Gruvbox,
    TokyoNight,
}

impl Appearance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Nord => "nord",
            Self::Dracula => "dracula",
            Self::Everforest => "everforest",
            Self::RosePine => "rose-pine",
            Self::Kanagawa => "kanagawa",
            Self::SolarizedDark => "solarized-dark",
            Self::CatppuccinLatte => "catppuccin-latte",
            Self::RosePineDawn => "rose-pine-dawn",
            Self::Gruvbox => "gruvbox",
            Self::TokyoNight => "tokyo-night",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeChoice {
    Auto,
    Dark,
    Light,
    Nord,
    Dracula,
    Everforest,
    RosePine,
    Kanagawa,
    SolarizedDark,
    CatppuccinLatte,
    RosePineDawn,
    Gruvbox,
    TokyoNight,
}

impl ThemeChoice {
    pub fn named(value: &str) -> Result<Self, ThemeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "dark" | "catppuccin" => Ok(Self::Dark),
            "light" | "paper" => Ok(Self::Light),
            "nord" => Ok(Self::Nord),
            "dracula" => Ok(Self::Dracula),
            "everforest" => Ok(Self::Everforest),
            "rose-pine" => Ok(Self::RosePine),
            "kanagawa" => Ok(Self::Kanagawa),
            "solarized-dark" => Ok(Self::SolarizedDark),
            "catppuccin-latte" => Ok(Self::CatppuccinLatte),
            "rose-pine-dawn" => Ok(Self::RosePineDawn),
            "gruvbox" => Ok(Self::Gruvbox),
            "tokyo-night" => Ok(Self::TokyoNight),
            _ => Err(ThemeError(format!(
                "Unknown theme '{value}'. Choose one of: {}",
                THEME_CHOICES.join(", ")
            ))),
        }
    }
    pub fn next(self) -> Self {
        let index = THEME_CHOICES
            .iter()
            .position(|&name| name == self.as_str())
            .expect("every theme choice has a registered name");
        Self::named(THEME_CHOICES[(index + 1) % THEME_CHOICES.len()])
            .expect("all registered theme names are valid")
    }
    pub fn resolve(self, system: Appearance) -> Appearance {
        match self {
            Self::Auto => system,
            Self::Dark => Appearance::Dark,
            Self::Light => Appearance::Light,
            Self::Nord => Appearance::Nord,
            Self::Dracula => Appearance::Dracula,
            Self::Everforest => Appearance::Everforest,
            Self::RosePine => Appearance::RosePine,
            Self::Kanagawa => Appearance::Kanagawa,
            Self::SolarizedDark => Appearance::SolarizedDark,
            Self::CatppuccinLatte => Appearance::CatppuccinLatte,
            Self::RosePineDawn => Appearance::RosePineDawn,
            Self::Gruvbox => Appearance::Gruvbox,
            Self::TokyoNight => Appearance::TokyoNight,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Nord => "nord",
            Self::Dracula => "dracula",
            Self::Everforest => "everforest",
            Self::RosePine => "rose-pine",
            Self::Kanagawa => "kanagawa",
            Self::SolarizedDark => "solarized-dark",
            Self::CatppuccinLatte => "catppuccin-latte",
            Self::RosePineDawn => "rose-pine-dawn",
            Self::Gruvbox => "gruvbox",
            Self::TokyoNight => "tokyo-night",
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
        // Dark and Light retain their original palettes and legacy aliases.
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
            ThemeChoice::Nord => Self {
                name: "nord",
                background: rgb(0x2e3440),
                surface: rgb(0x3b4252),
                foreground: rgb(0xeceff4),
                muted: rgb(0x9da8bd),
                border: rgb(0x4c566a),
                accent: rgb(0x88c0d0),
                directory: rgb(0x81a1c1),
                selected_bg: rgb(0x434c5e),
                selected_fg: rgb(0xeceff4),
                warning: rgb(0xbf616a),
                success: rgb(0xa3be8c),
            },
            ThemeChoice::Dracula => Self {
                name: "dracula",
                background: rgb(0x282a36),
                surface: rgb(0x343746),
                foreground: rgb(0xf8f8f2),
                muted: rgb(0x9399b2),
                border: rgb(0x6272a4),
                accent: rgb(0xbd93f9),
                directory: rgb(0x8be9fd),
                selected_bg: rgb(0x44475a),
                selected_fg: rgb(0xf8f8f2),
                warning: rgb(0xff5555),
                success: rgb(0x50fa7b),
            },
            ThemeChoice::Everforest => Self {
                name: "everforest",
                background: rgb(0x2d353b),
                surface: rgb(0x343f44),
                foreground: rgb(0xd3c6aa),
                muted: rgb(0x9da9a0),
                border: rgb(0x56635f),
                accent: rgb(0xa7c080),
                directory: rgb(0x7fbbb3),
                selected_bg: rgb(0x475258),
                selected_fg: rgb(0xd3c6aa),
                warning: rgb(0xe67e80),
                success: rgb(0xa7c080),
            },
            ThemeChoice::RosePine => Self {
                name: "rose-pine",
                background: rgb(0x191724),
                surface: rgb(0x1f1d2e),
                foreground: rgb(0xe0def4),
                muted: rgb(0x908caa),
                border: rgb(0x524f67),
                accent: rgb(0xc4a7e7),
                directory: rgb(0x9ccfd8),
                selected_bg: rgb(0x403d52),
                selected_fg: rgb(0xe0def4),
                warning: rgb(0xeb6f92),
                success: rgb(0x9ccfd8),
            },
            ThemeChoice::Kanagawa => Self {
                name: "kanagawa",
                background: rgb(0x1f1f28),
                surface: rgb(0x2a2a37),
                foreground: rgb(0xdcd7ba),
                muted: rgb(0xa6a69c),
                border: rgb(0x54546d),
                accent: rgb(0xdca561),
                directory: rgb(0x7e9cd8),
                selected_bg: rgb(0x363646),
                selected_fg: rgb(0xdcd7ba),
                warning: rgb(0xe82424),
                success: rgb(0x76946a),
            },
            ThemeChoice::SolarizedDark => Self {
                name: "solarized-dark",
                background: rgb(0x002b36),
                surface: rgb(0x073642),
                foreground: rgb(0x93a1a1),
                muted: rgb(0x839496),
                border: rgb(0x586e75),
                accent: rgb(0xb58900),
                directory: rgb(0x268bd2),
                selected_bg: rgb(0x174653),
                selected_fg: rgb(0xfdf6e3),
                warning: rgb(0xdc322f),
                success: rgb(0x859900),
            },
            ThemeChoice::CatppuccinLatte => Self {
                name: "catppuccin-latte",
                background: rgb(0xeff1f5),
                surface: rgb(0xe6e9ef),
                foreground: rgb(0x4c4f69),
                muted: rgb(0x6c6f85),
                border: rgb(0x9ca0b0),
                accent: rgb(0x8839ef),
                directory: rgb(0x1e66f5),
                selected_bg: rgb(0xccd0da),
                selected_fg: rgb(0x4c4f69),
                warning: rgb(0xd20f39),
                success: rgb(0x40a02b),
            },
            ThemeChoice::RosePineDawn => Self {
                name: "rose-pine-dawn",
                background: rgb(0xfaf4ed),
                surface: rgb(0xf2e9e1),
                foreground: rgb(0x575279),
                muted: rgb(0x797593),
                border: rgb(0xb5a9a3),
                accent: rgb(0x907aa9),
                directory: rgb(0x286983),
                selected_bg: rgb(0xdfdad9),
                selected_fg: rgb(0x575279),
                warning: rgb(0xb4637a),
                success: rgb(0x56949f),
            },
            ThemeChoice::Gruvbox => Self {
                name: "gruvbox",
                background: rgb(0x282828),
                surface: rgb(0x3c3836),
                foreground: rgb(0xebdbb2),
                muted: rgb(0xa89984),
                border: rgb(0x665c54),
                accent: rgb(0xfabd2f),
                directory: rgb(0x83a598),
                selected_bg: rgb(0x504945),
                selected_fg: rgb(0xfbf1c7),
                warning: rgb(0xfb4934),
                success: rgb(0xb8bb26),
            },
            ThemeChoice::TokyoNight => Self {
                name: "tokyo-night",
                background: rgb(0x1a1b26),
                surface: rgb(0x24283b),
                foreground: rgb(0xc0caf5),
                muted: rgb(0x9aa5ce),
                border: rgb(0x565f89),
                accent: rgb(0xbb9af7),
                directory: rgb(0x7aa2f7),
                selected_bg: rgb(0x33467c),
                selected_fg: rgb(0xc0caf5),
                warning: rgb(0xf7768e),
                success: rgb(0x9ece6a),
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
        // Preserve terminal inheritance for the original Auto/Dark/Light modes.
        // Explicit extra palettes need their own background for readable text.
        let background = match self.name {
            "dark" | "light" => Color::Reset,
            _ => self.background,
        };
        Style::default().fg(self.foreground).bg(background)
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
    fn original_modes_and_full_cycle() {
        assert_eq!(Theme::named("dark").unwrap().name, "dark");
        assert_eq!(Theme::named("light").unwrap().name, "light");
        assert_eq!(ThemeChoice::Auto.next(), ThemeChoice::Dark);
        assert_eq!(ThemeChoice::Dark.next(), ThemeChoice::Light);
        assert_eq!(ThemeChoice::Light.next(), ThemeChoice::Nord);
        let mut choice = ThemeChoice::Auto;
        for &name in THEME_CHOICES {
            assert_eq!(choice.as_str(), name);
            assert_eq!(ThemeChoice::named(name).unwrap(), choice);
            if choice != ThemeChoice::Auto {
                for system in [Appearance::Dark, Appearance::Light] {
                    let resolved = choice.resolve(system);
                    assert_eq!(resolved.as_str(), name);
                    assert_eq!(Theme::named(resolved.as_str()).unwrap().name, name);
                }
            }
            choice = choice.next();
        }
        assert_eq!(choice, ThemeChoice::Auto);
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
    fn explicit_palettes_use_their_own_background() {
        for &name in &THEME_CHOICES[3..] {
            let theme = Theme::named(name).unwrap();
            assert_eq!(theme.base_style().bg, Some(theme.background));
        }
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

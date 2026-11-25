//! Native theme providers. Terminal queries run only before the event loop.
use crate::theme::{Appearance, AutoThemes, Theme, ThemeChoice, ThemeError, parse_hex};
use ratatui::style::Color;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

mod cosmic;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Auto,
    Omarchy,
    Cosmic,
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub source: Source,
    pub live_reload: bool,
    pub ensure_contrast: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            source: Source::Auto,
            live_reload: true,
            ensure_contrast: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalPalette {
    pub background: Color,
    pub foreground: Color,
}
impl TerminalPalette {
    pub fn query() -> Option<Self> {
        let bg =
            terminal_colorsaurus::background_color(terminal_colorsaurus::QueryOptions::default())
                .ok()?;
        let fg =
            terminal_colorsaurus::foreground_color(terminal_colorsaurus::QueryOptions::default())
                .ok()?;
        let convert = |c: terminal_colorsaurus::Color| {
            Color::Rgb((c.r / 257) as u8, (c.g / 257) as u8, (c.b / 257) as u8)
        };
        Some(Self {
            background: convert(bg),
            foreground: convert(fg),
        })
    }
    pub fn appearance(self) -> Appearance {
        if luminance(self.background) > 0.35 {
            Appearance::Light
        } else {
            Appearance::Dark
        }
    }
}

fn channels(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}
fn blend(a: Color, b: Color, amount: f64) -> Color {
    let (r, g, bl) = channels(a);
    let (x, y, z) = channels(b);
    let mix = |a: u8, b: u8| (f64::from(a) * (1.0 - amount) + f64::from(b) * amount).round() as u8;
    Color::Rgb(mix(r, x), mix(g, y), mix(bl, z))
}
fn luminance(c: Color) -> f64 {
    let (r, g, b) = channels(c);
    let linear = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}
fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
fn readable(color: Color, bg: Color, fg: Color, enabled: bool) -> Color {
    if !enabled || contrast(color, bg) >= 4.5 {
        return color;
    }
    let target = if contrast(fg, bg) >= 4.5 {
        fg
    } else {
        let white = Color::Rgb(255, 255, 255);
        let black = Color::Rgb(0, 0, 0);
        if contrast(white, bg) >= contrast(black, bg) {
            white
        } else {
            black
        }
    };
    for i in 1..=20 {
        let c = blend(color, target, f64::from(i) / 20.0);
        if contrast(c, bg) >= 4.5 {
            return c;
        }
    }
    target
}
fn base(bg: Color, fg: Color, fallback: Theme) -> Theme {
    Theme {
        background: bg,
        foreground: fg,
        surface: blend(bg, fg, 0.08),
        border: blend(bg, fg, 0.25),
        muted: blend(bg, fg, 0.60),
        selected_bg: blend(bg, fg, 0.18),
        selected_fg: fg,
        ..fallback
    }
}
fn native_palette(doc: &toml::Value, fallback: Theme, settings: Settings) -> Option<Theme> {
    let get = |key: &str| {
        doc.get(key)
            .and_then(toml::Value::as_str)
            .and_then(|s| parse_hex(s).ok())
    };
    let bg = get("background")?;
    let fg = get("foreground")?;
    let mut t = base(bg, fg, fallback);
    t.name = "omarchy";
    t.surface = get("lighter_background").unwrap_or(t.surface);
    t.border = get("muted").unwrap_or(t.border);
    t.muted = get("light_foreground")
        .or_else(|| get("color8"))
        .unwrap_or(t.muted);
    t.accent = get("accent").or_else(|| get("blue")).unwrap_or(t.accent);
    t.directory = get("blue").or_else(|| get("color4")).unwrap_or(t.directory);
    t.success = get("green").or_else(|| get("color2")).unwrap_or(t.success);
    t.warning = get("yellow").or_else(|| get("color3")).unwrap_or(t.warning);
    t.selected_bg = get("selection").unwrap_or_else(|| blend(bg, t.accent, 0.24));
    let heads = [
        get("magenta").or_else(|| get("color5")),
        get("blue").or_else(|| get("color4")),
        get("cyan").or_else(|| get("color6")),
        get("green").or_else(|| get("color2")),
        get("yellow").or_else(|| get("color3")),
        get("orange")
            .or_else(|| get("red"))
            .or_else(|| get("color1")),
    ];
    t.heading_colors = heads.map(|c| c.map(|c| readable(c, bg, fg, settings.ensure_contrast)));
    for c in [
        &mut t.foreground,
        &mut t.muted,
        &mut t.accent,
        &mut t.directory,
        &mut t.success,
        &mut t.warning,
    ] {
        *c = readable(*c, bg, fg, settings.ensure_contrast);
    }
    t.selected_fg = readable(
        get("selection_foreground").unwrap_or(fg),
        t.selected_bg,
        fg,
        settings.ensure_contrast,
    );
    Some(t)
}
fn omarchy_roots() -> Option<[PathBuf; 2]> {
    let home = env::var_os("HOME").map(PathBuf::from)?;
    let state = env::var_os("XDG_STATE_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/state"));
    let config = env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    Some([state, config])
}

/// Compare actual file contents rather than timestamps: theme switches may
/// replace a symlink or reuse timestamps. Include the legacy appearance paths.
#[derive(Debug, PartialEq, Eq)]
pub struct ThemeSnapshot(Vec<ThemeFile>, Vec<Option<String>>);

#[derive(Debug, PartialEq, Eq)]
struct ThemeFile {
    directory: PathBuf,
    colors: Option<String>,
    light_marker: bool,
}

impl ThemeSnapshot {
    pub fn capture(settings: Settings) -> Self {
        let cosmic = if use_cosmic(settings) {
            cosmic::snapshot()
        } else {
            Vec::new()
        };
        let Some(roots) = omarchy_roots() else {
            return Self(Vec::new(), cosmic);
        };
        let mut directories: Vec<_> = roots
            .into_iter()
            .map(|root| root.join("omarchy/current/theme"))
            .collect();
        if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
            for relative in [
                ".local/state/omarchy/current/theme",
                ".config/omarchy/current/theme",
            ] {
                let directory = home.join(relative);
                if !directories.contains(&directory) {
                    directories.push(directory);
                }
            }
        }
        let mut snapshot = Self::from_directories(directories);
        snapshot.1 = cosmic;
        snapshot
    }

    fn from_directories(directories: Vec<PathBuf>) -> Self {
        Self(
            directories
                .into_iter()
                .map(|directory| ThemeFile {
                    colors: fs::read_to_string(directory.join("colors.toml")).ok(),
                    light_marker: directory.join("light.mode").is_file(),
                    directory,
                })
                .collect(),
            Vec::new(),
        )
    }
}

fn omarchy(fallback: Theme, settings: Settings) -> Option<Theme> {
    omarchy_roots()?.iter().find_map(|root| {
        let text = fs::read_to_string(root.join("omarchy/current/theme/colors.toml")).ok()?;
        native_palette(&text.parse().ok()?, fallback, settings)
    })
}
fn terminal(p: TerminalPalette, settings: Settings) -> Theme {
    // Actual RGB foreground/background; ANSI semantic colors use emulator slots.
    let fallback = Theme::named(p.appearance().as_str()).expect("built-in palette");
    let mut t = base(p.background, p.foreground, fallback);
    t.name = "terminal";
    t.foreground = readable(
        t.foreground,
        t.background,
        t.foreground,
        settings.ensure_contrast,
    );
    t.muted = readable(
        t.muted,
        t.background,
        t.foreground,
        settings.ensure_contrast,
    );
    t.selected_fg = readable(
        t.selected_fg,
        t.selected_bg,
        t.foreground,
        settings.ensure_contrast,
    );
    t.accent = Color::Cyan;
    t.directory = Color::Blue;
    t.success = Color::Green;
    t.warning = Color::Yellow;
    t.heading_colors = [
        Color::Magenta,
        Color::Blue,
        Color::Cyan,
        Color::Green,
        Color::Yellow,
        Color::Red,
    ]
    .map(Some);
    t
}

fn use_cosmic(settings: Settings) -> bool {
    settings.source == Source::Cosmic || (settings.source == Source::Auto && cosmic::is_session())
}

/// Resolve light/dark preferences for native themes and automatic presets.
pub fn appearance(settings: Settings, fallback: Appearance) -> Appearance {
    if use_cosmic(settings) {
        cosmic::appearance()
    } else if matches!(settings.source, Source::Auto | Source::Omarchy) {
        crate::theme::omarchy_appearance().unwrap_or(fallback)
    } else {
        fallback
    }
}

pub fn resolve(
    choice: ThemeChoice,
    auto: AutoThemes,
    appearance: Appearance,
    settings: Settings,
    palette: Option<TerminalPalette>,
) -> Result<Theme, ThemeError> {
    let fallback = Theme::named(auto.resolve(ThemeChoice::Auto, appearance).as_str())?;
    if choice == ThemeChoice::Terminal {
        return Ok(palette.map(|p| terminal(p, settings)).unwrap_or(fallback));
    }
    if choice == ThemeChoice::System {
        if use_cosmic(settings)
            && let Some(t) = cosmic::resolve(fallback, settings)
        {
            return Ok(t);
        }
        if matches!(settings.source, Source::Auto | Source::Omarchy)
            && !use_cosmic(settings)
            && let Some(t) = omarchy(fallback, settings)
        {
            return Ok(t);
        }
        if matches!(settings.source, Source::Auto | Source::Terminal)
            && let Some(p) = palette
        {
            return Ok(terminal(p, settings));
        }
        return Ok(fallback);
    }
    Theme::named(auto.resolve(choice, appearance).as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_detects_palette_and_marker_changes() {
        let dir = tempfile::tempdir().unwrap();
        let capture = || ThemeSnapshot::from_directories(vec![dir.path().to_path_buf()]);
        let initial = capture();
        assert_eq!(initial, capture());
        fs::write(dir.path().join("colors.toml"), "mode = 'dark'").unwrap();
        let dark = capture();
        assert_ne!(initial, dark);
        fs::write(dir.path().join("colors.toml"), "mode = 'light'").unwrap();
        let light = capture();
        assert_ne!(dark, light);
        fs::write(dir.path().join("light.mode"), "").unwrap();
        assert_ne!(light, capture());
    }

    #[test]
    fn reads_omarchy_named_colors() {
        let doc = "background='#2d353b'\nforeground='#d3c6aa'\naccent='#7fbbb3'\nselection='#3d484d'\nmuted='#475258'\nlighter_background='#343f44'\nmagenta='#d699b6'\nblue='#7fbbb3'\ncyan='#83c092'\ngreen='#a7c080'\nyellow='#dbbc7f'\norange='#e09d7f'".parse().unwrap();
        let t = native_palette(&doc, Theme::named("dark").unwrap(), Settings::default()).unwrap();
        assert_eq!(t.background, Color::Rgb(45, 53, 59));
        assert_eq!(t.accent, Color::Rgb(127, 187, 179));
        assert_eq!(t.selected_bg, Color::Rgb(61, 72, 77));
        for (i, c) in t.heading_colors.iter().enumerate() {
            assert!(c.is_some());
            assert!(!t.heading_colors[..i].contains(c));
        }
    }
    #[test]
    fn incomplete_palette_is_rejected() {
        let doc = "background='#ffffff'".parse().unwrap();
        assert!(
            native_palette(&doc, Theme::named("light").unwrap(), Settings::default()).is_none()
        );
    }
    #[test]
    fn contrast_correction_and_terminal_inheritance() {
        let p = TerminalPalette {
            background: Color::Rgb(250, 250, 250),
            foreground: Color::Rgb(40, 40, 40),
        };
        let t = terminal(p, Settings::default());
        assert_eq!(t.background, p.background);
        assert_eq!(t.foreground, p.foreground);
        assert_eq!(t.directory, Color::Blue);
        assert!(contrast(t.muted, t.background) >= 4.5);
    }
}

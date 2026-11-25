//! Read COSMIC's resolved v2 RON palette without a desktop-library dependency.
use super::{Settings, base, blend, readable};
use crate::theme::{Appearance, Theme, parse_hex};
use ratatui::style::Color;
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const MODE: &str = "com.system76.CosmicTheme.Mode/v1/is_dark";
const KEYS: &[&str] = &[
    "background",
    "primary",
    "accent",
    "accent_text",
    "success",
    "warning",
    "palette",
];

pub(super) fn is_session() -> bool {
    [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
    ]
    .iter()
    .filter_map(|key| env::var(key).ok())
    .any(|value| value.to_ascii_lowercase().contains("cosmic"))
}

fn roots() -> Vec<PathBuf> {
    let user = env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    let mut roots: Vec<_> = user.into_iter().map(|p| p.join("cosmic")).collect();
    roots.push(PathBuf::from("/usr/share/cosmic"));
    roots
}

fn read(roots: &[PathBuf], key: &Path) -> Option<String> {
    roots
        .iter()
        .find_map(|root| fs::read_to_string(root.join(key)).ok())
}

fn mode(roots: &[PathBuf]) -> Appearance {
    match read(roots, Path::new(MODE)).as_deref().map(str::trim) {
        Some("false") => Appearance::Light,
        _ => Appearance::Dark, // COSMIC's default when no preference is saved.
    }
}

pub(super) fn appearance() -> Appearance {
    mode(&roots())
}

fn directory(appearance: Appearance) -> PathBuf {
    PathBuf::from(match appearance {
        Appearance::Light => "com.system76.CosmicTheme.Light/v2",
        _ => "com.system76.CosmicTheme.Dark/v2",
    })
}

pub(super) fn snapshot() -> Vec<Option<String>> {
    snapshot_from(&roots())
}

fn snapshot_from(roots: &[PathBuf]) -> Vec<Option<String>> {
    let dir = directory(mode(roots));
    std::iter::once(read(roots, Path::new(MODE)))
        .chain(KEYS.iter().map(|key| read(roots, &dir.join(key))))
        .collect()
}

// COSMIC v2 serializes colors as #RRGGBBAA strings. Composite translucent
// colors onto the relevant opaque terminal background; terminal cells lack alpha.
fn color(value: &str, behind: Color) -> Option<Color> {
    let digits = value.strip_prefix('#')?;
    if !digits.is_ascii() || !matches!(digits.len(), 6 | 8) {
        return None;
    }
    let rgb = parse_hex(&value[..7]).ok()?;
    let alpha = if digits.len() == 8 {
        u8::from_str_radix(&digits[6..], 16).ok()?
    } else {
        255
    };
    Some(blend(behind, rgb, f64::from(alpha) / 255.0))
}

#[derive(Deserialize)]
struct Layer {
    base: String,
    on: String,
    divider: String,
    component: Selection,
}

#[derive(Deserialize)]
struct Selection {
    selected: String,
    selected_text: String,
}

#[derive(Deserialize)]
struct Component {
    base: String,
}

#[derive(Deserialize)]
struct Palette {
    accent_blue: String,
    accent_purple: String,
    accent_indigo: String,
    accent_green: String,
    accent_yellow: String,
    accent_orange: String,
}

pub(super) fn resolve(fallback: Theme, settings: Settings) -> Option<Theme> {
    resolve_from(&roots(), fallback, settings)
}

fn resolve_from(roots: &[PathBuf], fallback: Theme, settings: Settings) -> Option<Theme> {
    let dir = directory(mode(roots));
    let get = |key: &str| read(roots, &dir.join(key));
    let background: Layer = ron::from_str(&get("background")?).ok()?;
    let bg = color(&background.base, fallback.background)?;
    let fg = color(&background.on, bg)?;
    let mut t = base(bg, fg, fallback);
    t.name = "cosmic";
    let component = |key: &str| {
        let value: Component = ron::from_str(&get(key)?).ok()?;
        color(&value.base, bg)
    };
    if let Some(primary) = get("primary").and_then(|s| ron::from_str::<Component>(&s).ok()) {
        t.surface = color(&primary.base, bg).unwrap_or(t.surface);
    }
    t.border = color(&background.divider, bg).unwrap_or(t.border);
    let accent = component("accent").unwrap_or(t.accent);
    t.accent = get("accent_text")
        .and_then(|s| ron::from_str::<Option<String>>(&s).ok().flatten())
        .and_then(|s| color(&s, bg))
        .unwrap_or(accent);
    t.directory = t.accent;
    t.success = component("success").unwrap_or(t.success);
    t.warning = component("warning").unwrap_or(t.warning);
    t.selected_bg =
        color(&background.component.selected, bg).unwrap_or_else(|| blend(bg, accent, 0.24));
    t.selected_fg = color(&background.component.selected_text, t.selected_bg).unwrap_or(fg);
    t.heading_colors[0] = Some(t.accent);
    if let Some(p) = get("palette").and_then(|s| ron::from_str::<Palette>(&s).ok()) {
        t.directory = color(&p.accent_blue, bg).unwrap_or(t.directory);
        t.heading_colors = [
            Some(t.accent),
            color(&p.accent_blue, bg),
            color(&p.accent_indigo, bg),
            color(&p.accent_green, bg),
            color(&p.accent_yellow, bg),
            color(&p.accent_orange, bg),
        ];
        // Prefer a distinct third-level color if the active accent is indigo.
        if t.heading_colors[2] == t.heading_colors[0] {
            t.heading_colors[2] = color(&p.accent_purple, bg);
        }
    }
    t.heading_colors = t
        .heading_colors
        .map(|c| c.map(|c| readable(c, bg, fg, settings.ensure_contrast)));
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
    t.selected_fg = readable(t.selected_fg, t.selected_bg, fg, settings.ensure_contrast);
    Some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, key: &Path, value: &str) {
        let file = root.join(key);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, value).unwrap();
    }

    fn fixture(root: &Path, appearance: Appearance, bg: &str, fg: &str) {
        let dir = directory(appearance);
        write(
            root,
            &dir.join("background"),
            &format!(
                r##"(base: "{bg}", on: "{fg}", divider: "#808080FF", component: (base: "{bg}", selected: "#80808080", selected_text: "{fg}"), small_widget: "#00000000")"##
            ),
        );
        write(
            root,
            &dir.join("primary"),
            r##"(base: "#445566FF", on: "#FFFFFFFF")"##,
        );
        write(
            root,
            &dir.join("accent"),
            r##"(base: "#123456FF", on: "#FFFFFFFF")"##,
        );
    }

    #[test]
    fn reads_v2_and_detects_mode_and_accent_changes() {
        let temp = tempfile::tempdir().unwrap();
        let roots = [temp.path().to_path_buf()];
        let root = &roots[0];
        fixture(root, Appearance::Dark, "#202020FF", "#EEEEEEFF");
        fixture(root, Appearance::Light, "#EEEEEEFF", "#202020FF");
        let settings = Settings {
            ensure_contrast: false,
            ..Settings::default()
        };
        let resolve = || resolve_from(&roots, Theme::named("dark").unwrap(), settings).unwrap();
        write(root, Path::new(MODE), "true");
        let before = snapshot_from(&roots);
        let dark = resolve();
        assert_eq!(dark.name, "cosmic");
        assert_eq!(dark.background, Color::Rgb(32, 32, 32));
        assert_eq!(dark.foreground, Color::Rgb(238, 238, 238));
        assert_eq!(dark.surface, Color::Rgb(68, 85, 102));
        assert_eq!(dark.accent, Color::Rgb(18, 52, 86));
        write(root, Path::new(MODE), "false");
        let light = snapshot_from(&roots);
        assert_ne!(before, light);
        assert_eq!(resolve().background, Color::Rgb(238, 238, 238));
        write(
            root,
            &directory(Appearance::Light).join("accent"),
            r##"(base: "#AABBCCFF")"##,
        );
        assert_ne!(light, snapshot_from(&roots));
        assert_eq!(resolve().accent, Color::Rgb(170, 187, 204));
    }

    #[test]
    fn rejects_malformed_or_missing_background() {
        let temp = tempfile::tempdir().unwrap();
        let roots = [temp.path().to_path_buf()];
        let fallback = Theme::named("dark").unwrap();
        assert!(resolve_from(&roots, fallback, Settings::default()).is_none());
        write(
            &roots[0],
            &directory(Appearance::Dark).join("background"),
            "broken",
        );
        assert!(resolve_from(&roots, fallback, Settings::default()).is_none());
        assert_eq!(
            color("#FFFFFF80", Color::Rgb(0, 0, 0)),
            Some(Color::Rgb(128, 128, 128))
        );
        assert!(color("#ééé", fallback.background).is_none());
    }
}

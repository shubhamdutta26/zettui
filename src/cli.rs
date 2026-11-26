//! Resolve command-line arguments and configuration before terminal initialization.

use std::{env, io, path::PathBuf};

use crate::theme::{Appearance, AutoThemes, ColorOverrides, THEME_CHOICES, ThemeChoice};
use crate::{config::Config, focus, markdown, snippets, system_theme};

/// Resolved startup settings from the configuration and command line.
pub(crate) struct StartupOptions {
    pub(crate) root: PathBuf,
    pub(crate) bibliography: Option<PathBuf>,
    pub(crate) choice: ThemeChoice,
    pub(crate) system_theme: system_theme::Settings,
    pub(crate) terminal_palette: Option<system_theme::TerminalPalette>,
    pub(crate) overrides: ColorOverrides,
    pub(crate) fallback: Appearance,
    pub(crate) auto_themes: AutoThemes,
    pub(crate) browser_width: u16,
    pub(crate) panels: crate::layout::Panels,
    pub(crate) snippets_dir: PathBuf,
    pub(crate) writing: focus::Writing,
    pub(crate) writing_goals: zettui::analytics::Goals,
    pub(crate) markdown_settings: markdown::MarkdownSettings,
    pub(crate) session_settings: crate::session::Settings,
    pub(crate) show_tab_bar: bool,
    pub(crate) show_hidden_folders: bool,
}

/// Command line syntax:
///   `zettui [NOTES_DIR] [--bibliography PATH] [--theme NAME]`
/// Configuration acts as a default; explicit CLI arguments win.
pub(crate) fn arguments() -> io::Result<StartupOptions> {
    if env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("zettui {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    let config = Config::load()?;
    let browser_width = config.browser_width()?;
    let panels = config.panels()?;
    let auto_themes = config.auto_themes()?;
    let snippets_dir = config
        .workspace
        .snippets_dir
        .clone()
        .unwrap_or_else(snippets::default_snippets_dir);
    let mut notes_dir = None;
    let mut bibliography = None;
    let mut theme_override = None;
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let flag = arg.to_string_lossy().into_owned();
        match flag.as_str() {
            "--bibliography" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--bibliography requires a path",
                    )
                })?;
                bibliography = Some(PathBuf::from(value));
            }
            "--theme" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--theme requires a name")
                })?;
                theme_override = Some(value.to_string_lossy().into_owned());
            }
            "-h" | "--help" => {
                println!(
                    "zettui [NOTES_DIR] [--bibliography FILE] [--theme NAME]\n\nModes: {}\nConfig: ~/.config/zettui/config.toml",
                    THEME_CHOICES.join(", "),
                );
                std::process::exit(0);
            }
            other if other.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown option: {other}"),
                ));
            }
            _ if notes_dir.is_none() => notes_dir = Some(PathBuf::from(arg)),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Provide at most one NOTES_DIR",
                ));
            }
        }
    }
    let name = theme_override
        .or(config.appearance.theme)
        .unwrap_or_else(|| "system".to_owned());
    let choice = ThemeChoice::named(&name)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    let terminal_palette = system_theme::TerminalPalette::query();
    let system = terminal_palette
        .map(|p| p.appearance())
        .unwrap_or(Appearance::Dark);
    Ok(StartupOptions {
        root: notes_dir
            .or(config.workspace.notes_dir)
            .unwrap_or(env::current_dir()?),
        bibliography: bibliography.or(config.workspace.bibliography),
        choice,
        system_theme: config.appearance.system_theme,
        terminal_palette,
        overrides: config.colors,
        fallback: system,
        auto_themes,
        browser_width,
        panels,
        snippets_dir,
        writing: focus::Writing {
            zen: config.zen,
            focus: config.focus,
            editor: config.editor,
            leader: false,
        },
        writing_goals: config.writing_goals,
        markdown_settings: config.markdown,
        session_settings: config.session,
        show_tab_bar: config.buffers.show_tab_bar,
        show_hidden_folders: config.browser.show_hidden_folders,
    })
}

//! Resolve command-line arguments and configuration before terminal initialization.

use std::{env, ffi::OsString, io, path::PathBuf};

use crate::theme::{Appearance, AutoThemes, ColorOverrides, THEME_CHOICES, Theme, ThemeChoice};
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

#[derive(Debug, Default, PartialEq, Eq)]
struct Overrides {
    notes_dir: Option<PathBuf>,
    bibliography: Option<PathBuf>,
    theme: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Start(Overrides),
    Help,
    Version,
    CheckConfig,
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    let message: String = message.into();
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// Do not interpret another option as the missing value of a flag.
/// Prefix a dash-leading relative filename with `./`.
pub(crate) fn required_value(
    args: &mut impl Iterator<Item = OsString>,
    flag: &str,
    expected: &str,
) -> io::Result<OsString> {
    args.next()
        .filter(|value| !value.is_empty() && !value.to_string_lossy().starts_with('-'))
        .ok_or_else(|| invalid_input(format!("{flag} requires {expected}")))
}

fn parse(args: impl IntoIterator<Item = OsString>) -> io::Result<Action> {
    let mut args = args.into_iter().peekable();
    if args.peek().is_some_and(|arg| arg == "--check-config") {
        let _ = args.next();
        if args.next().is_some() {
            return Err(invalid_input("--check-config does not accept arguments"));
        }
        return Ok(Action::CheckConfig);
    }
    let mut options = Overrides::default();
    let mut positional_only = false;
    while let Some(arg) = args.next() {
        if !positional_only {
            match arg.to_str() {
                Some("--") => {
                    positional_only = true;
                    continue;
                }
                Some("--bibliography") => {
                    options.bibliography = Some(PathBuf::from(required_value(
                        &mut args,
                        "--bibliography",
                        "a path",
                    )?));
                    continue;
                }
                Some("--theme") => {
                    let value = required_value(&mut args, "--theme", "a name")?;
                    options.theme = Some(
                        value
                            .into_string()
                            .map_err(|_| invalid_input("--theme requires a valid UTF-8 name"))?,
                    );
                    continue;
                }
                Some("-h" | "--help") => return Ok(Action::Help),
                Some("-V" | "--version") => return Ok(Action::Version),
                _ if arg.to_string_lossy().starts_with('-') => {
                    return Err(invalid_input(format!(
                        "Unknown option: {}",
                        arg.to_string_lossy()
                    )));
                }
                _ => {}
            }
        }
        if options.notes_dir.is_some() {
            return Err(invalid_input("Provide at most one NOTES_DIR"));
        }
        options.notes_dir = Some(PathBuf::from(arg));
    }
    Ok(Action::Start(options))
}

fn print_help() {
    println!(
        "zettui [NOTES_DIR] [--bibliography FILE] [--theme NAME]\n\
         zettui --help | -h\n\
         zettui --version | -V\n\
         zettui --check-config\n\
         zettui --stats-export FILE\n\
         zettui --stats-import FILE\n\
         zettui --stats-import-zettlr FILE SOURCE\n\
         zettui --stats-device-id\n\n\
         Use -- before a dash-leading NOTES_DIR.\n\
         Themes: {} (aliases: catppuccin, paper)\n\
         Config: $XDG_CONFIG_HOME/zettui/config.toml or ~/.config/zettui/config.toml",
        THEME_CHOICES.join(", "),
    );
}

fn theme_choice(config: &Config, override_name: Option<&str>) -> io::Result<ThemeChoice> {
    let name = override_name
        .or(config.appearance.theme.as_deref())
        .unwrap_or("system");
    ThemeChoice::named(name).map_err(|error| invalid_input(error.to_string()))
}

fn check_config(config: &Config) -> io::Result<()> {
    config.browser_width()?;
    config.panels()?;
    config.auto_themes()?;
    theme_choice(config, None)?;
    // Global overrides are independent of the selected palette.
    Theme::named("dark")
        .and_then(|theme| theme.with_overrides(&config.colors))
        .map_err(|error| invalid_input(error.to_string()))?;
    Ok(())
}

/// Configuration supplies defaults; explicit CLI arguments win.
pub(crate) fn arguments() -> io::Result<StartupOptions> {
    let options = match parse(env::args_os().skip(1))? {
        Action::Help => {
            print_help();
            std::process::exit(0);
        }
        Action::Version => {
            println!("zettui {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        Action::CheckConfig => {
            check_config(&Config::load()?)?;
            println!("Configuration valid (defaults used if no file exists).");
            std::process::exit(0);
        }
        Action::Start(options) => options,
    };
    let config = Config::load()?;
    let browser_width = config.browser_width()?;
    let panels = config.panels()?;
    let auto_themes = config.auto_themes()?;
    let choice = theme_choice(&config, options.theme.as_deref())?;
    let snippets_dir = config
        .workspace
        .snippets_dir
        .clone()
        .unwrap_or_else(snippets::default_snippets_dir);
    let root = match options.notes_dir.or(config.workspace.notes_dir) {
        Some(path) => path,
        None => env::current_dir()?,
    };
    let terminal_palette = system_theme::TerminalPalette::query();
    let system = terminal_palette
        .map(|p| p.appearance())
        .unwrap_or(Appearance::Dark);
    Ok(StartupOptions {
        root,
        bibliography: options.bibliography.or(config.workspace.bibliography),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> io::Result<Action> {
        parse(args.iter().copied().map(OsString::from))
    }

    #[test]
    fn parses_startup_overrides_in_any_order() {
        assert_eq!(
            parse_str(&["--theme", "dark", "My Notes", "--bibliography", "refs.json"]).unwrap(),
            Action::Start(Overrides {
                notes_dir: Some(PathBuf::from("My Notes")),
                bibliography: Some(PathBuf::from("refs.json")),
                theme: Some("dark".to_owned()),
            })
        );
    }

    #[test]
    fn end_of_options_preserves_dash_leading_paths() {
        assert_eq!(
            parse_str(&["--", "--version"]).unwrap(),
            Action::Start(Overrides {
                notes_dir: Some(PathBuf::from("--version")),
                ..Overrides::default()
            })
        );
    }

    #[test]
    fn rejects_missing_values_and_extra_paths() {
        for args in [
            &["--theme"][..],
            &["--theme", "--version"],
            &["--bibliography", "--help"],
            &["--bibliography", ""],
            &["a", "b"],
            &["--check-config", "a"],
        ] {
            assert!(parse_str(args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn cli_theme_overrides_config_and_legacy_aliases_work() {
        let config: Config = toml::from_str("[appearance]\ntheme = 'nord'").unwrap();
        assert_eq!(theme_choice(&config, None).unwrap(), ThemeChoice::Nord);
        assert_eq!(
            theme_choice(&config, Some("paper")).unwrap(),
            ThemeChoice::Light
        );
    }
}

//! Resolve and refresh the active palette, and resize the browser.

use super::App;
use crate::commands::Command;
use crate::layout::resize_browser_percent;
use crate::theme::{Theme, ThemeChoice};
use crate::{system_theme, theme};
use std::time::Duration;

impl App {
    pub(super) fn resolve_theme(&self, choice: ThemeChoice) -> Result<Theme, theme::ThemeError> {
        system_theme::resolve(
            choice,
            self.auto_themes,
            system_theme::appearance(self.system_theme, self.fallback_appearance),
            self.system_theme,
            self.terminal_palette,
        )
        .and_then(|t| t.with_overrides(&self.theme_overrides))
    }

    pub(super) fn sync_system_theme(&mut self) {
        if !self.system_theme.live_reload
            || !matches!(self.theme_choice, ThemeChoice::Auto | ThemeChoice::System)
            || self.last_theme_check.elapsed() < Duration::from_millis(400)
        {
            return;
        }
        self.last_theme_check = std::time::Instant::now();
        let snapshot = system_theme::ThemeSnapshot::capture(self.system_theme);
        if self.theme_snapshot.as_ref() == Some(&snapshot) {
            return;
        }
        if let Ok(theme) = self.resolve_theme(self.theme_choice) {
            self.theme = theme;
            self.theme_snapshot = Some(snapshot);
        }
    }

    /// Cycle all palettes without changing the configuration file.
    pub(super) fn cycle_theme(&mut self) {
        let next = self.theme_choice.next();
        match self.resolve_theme(next) {
            Ok(theme) => {
                self.theme_snapshot = None;
                self.theme_choice = next;
                self.theme = theme;
                self.message = format!(
                    "Theme mode: {} ({}) — session only. Set appearance.theme = \"{}\" in config.toml.",
                    next.as_str(),
                    theme.name,
                    next.as_str()
                );
            }
            Err(error) => self.message = format!("Theme error: {error}"),
        }
    }

    /// Resize panes for this session without changing the configuration file.
    pub(super) fn resize_browser(&mut self, grow: bool) {
        self.browser_width_percent = resize_browser_percent(self.browser_width_percent, grow);
        self.message = format!(
            "Files preferred width: {}% (session only)",
            self.browser_width_percent,
        );
    }

    pub(super) fn resize_outline(&mut self, grow: bool) {
        self.panels.resize_outline(grow);
        self.message = format!(
            "Outline preferred width: {}% (session only)",
            self.panels.outline_width_percent
        );
    }

    /// Toggle preferences without changing input ownership or the open buffer.
    pub(super) fn toggle_panel(&mut self, command: Command) {
        self.writing.leader = false;
        self.pending_open_bracket = false;
        let (name, shown) = match command {
            Command::ToggleFiles => {
                self.panels.show_files = !self.panels.show_files;
                ("Files", self.panels.show_files)
            }
            Command::TogglePreview => {
                self.panels.show_preview = !self.panels.show_preview;
                ("Preview", self.panels.show_preview)
            }
            Command::ToggleOutline => {
                self.panels.show_outline = !self.panels.show_outline;
                ("Outline", self.panels.show_outline)
            }
            _ => return,
        };
        let state = if shown { "shown" } else { "hidden" };
        let context = match command {
            Command::ToggleFiles if self.editor.is_none() => {
                " when editing (Files remains available in browser)"
            }
            Command::TogglePreview if self.editor.is_some() => " in browser mode",
            Command::ToggleOutline if self.editor.is_none() => " when editing",
            _ if self.editor.is_some() && self.writing.zen.enabled => " after leaving Zen",
            _ => "",
        };
        self.message = format!("{name}: {state}{context} (session only)");
    }
}

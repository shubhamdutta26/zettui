//! Resolve and refresh the active palette, and resize the browser.

use super::App;
use crate::layout::resize_browser_percent;
use crate::theme::{Theme, ThemeChoice};
use crate::{system_theme, theme};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    pub(super) fn resize_columns(&mut self, key: KeyEvent) -> bool {
        if key.modifiers != KeyModifiers::ALT {
            return false;
        }
        let grow = match key.code {
            KeyCode::Char('h') | KeyCode::Left => false,
            KeyCode::Char('l') | KeyCode::Right => true,
            _ => return false,
        };
        self.browser_width_percent = resize_browser_percent(self.browser_width_percent, grow);
        self.message = format!(
            "Files: {}% | Notes: {}% (session only; set layout.browser_width_percent in config.toml)",
            self.browser_width_percent,
            100 - self.browser_width_percent,
        );
        true
    }
}

//! Keyboard ownership and navigation for the live document outline.

use super::App;
use crate::{
    commands::{Command, KeyScope},
    markdown, outline,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{EditorMode, Index2};

impl App {
    pub(super) fn toggle_outline_focus(&mut self) {
        if self.outline_view.focused {
            self.outline_view.focused = false;
            self.message.clear();
            return;
        }
        let Some(note) = &self.editor else {
            self.message = "Open a Markdown note before focusing the outline.".into();
            return;
        };
        if note.state.mode != EditorMode::Normal
            || self.command.is_some()
            || self.has_active_popup()
            || self.note_search.input.is_some()
        {
            return;
        }
        if self.writing.zen.enabled {
            self.message = "Leave Zen before focusing the outline.".into();
            return;
        }
        if !markdown::is_markdown(&note.path) {
            self.message = "Outline navigation is available for Markdown notes only.".into();
            return;
        }
        self.panels.show_outline = true;
        self.writing.leader = false;
        self.pending_open_bracket = false;
        let source = note.state.lines.to_string();
        let headings = outline::extract(&source);
        self.outline_view.sync(&headings, note.state.cursor.row);
        if headings.is_empty() {
            self.message = "No headings to navigate.".into();
            return;
        }
        self.outline_view.focused = true;
        if self.outline_view.selected.is_none() {
            self.outline_view.select_edge(&headings, false);
        }
        self.message.clear();
    }

    /// Called only while the outline owns keys. Never forwards text motions to edtui.
    pub(super) fn handle_outline_key(&mut self, key: KeyEvent) {
        if let Some(command @ (Command::Save | Command::CycleTheme)) =
            Command::from_key(KeyScope::Editor, key)
        {
            self.execute_editor_command(command);
            return;
        }
        if !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            if key.code == KeyCode::Esc {
                self.outline_view.focused = false;
                self.message.clear();
                return;
            }
            let Some(note) = &mut self.editor else {
                self.outline_view.reset();
                return;
            };
            let source = note.state.lines.to_string();
            let headings = outline::extract(&source);
            let view = &mut self.outline_view;
            view.sync(&headings, note.state.cursor.row);
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => view.move_selection(&headings, true),
                KeyCode::Char('k') | KeyCode::Up => view.move_selection(&headings, false),
                KeyCode::Home => view.select_edge(&headings, false),
                KeyCode::End => view.select_edge(&headings, true),
                KeyCode::Char('h') | KeyCode::Left => view.collapse_or_parent(&headings),
                KeyCode::Char('l') | KeyCode::Right => view.expand_or_child(&headings),
                KeyCode::Enter => {
                    if let Some(heading) = view.selected.and_then(|i| headings.get(i)) {
                        note.state.cursor = Index2::new(heading.row, 0);
                        note.state
                            .set_viewport_offset(0, heading.row.saturating_sub(1));
                        view.focused = false;
                        self.message = format!("Heading: line {}", heading.row + 1);
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Appearance, AutoThemes, ColorOverrides, ThemeChoice};
    use std::fs;

    fn app_with_note() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Outline.md");
        fs::write(&path, "# A\nbody\n## B\n# C\n").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap();
        app.select_path(&path);
        app.enter().unwrap();
        (dir, app)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn navigation_preserves_unsaved_text_and_jumps_only_on_enter() {
        let (_dir, mut app) = app_with_note();
        app.handle_editor_key(key(KeyCode::Char('i')));
        app.handle_editor_key(key(KeyCode::Char('X')));
        app.handle_editor_key(key(KeyCode::Esc));
        let note = app.editor.as_ref().unwrap();
        assert!(note.is_dirty());
        let original = note.state.lines.to_string();
        let cursor = note.state.cursor;
        app.toggle_outline_focus();
        assert!(app.panels.show_outline && app.outline_view.focused);
        app.handle_outline_key(key(KeyCode::End));
        app.handle_outline_key(key(KeyCode::Char('i')));
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, cursor);
        app.handle_outline_key(key(KeyCode::Esc));
        assert!(!app.outline_view.focused);
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, cursor);
        app.toggle_outline_focus();
        app.handle_outline_key(key(KeyCode::End));
        app.handle_outline_key(key(KeyCode::Enter));
        let note = app.editor.as_ref().unwrap();
        assert_eq!(note.state.cursor, Index2::new(3, 0));
        assert_eq!(note.state.lines.to_string(), original);
        assert!(note.is_dirty());
        assert!(!app.outline_view.focused);
        assert_eq!(
            fs::read_to_string(&note.path).unwrap(),
            "# A\nbody\n## B\n# C\n"
        );
    }

    #[test]
    fn narrow_render_releases_focus_without_forgetting_visibility() {
        let (_dir, mut app) = app_with_note();
        app.toggle_outline_focus();
        let backend = ratatui::backend::TestBackend::new(45, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        assert!(!app.outline_view.focused);
        assert!(app.panels.show_outline);
        assert!(app.editor.is_some());
    }

    #[test]
    fn focus_is_restricted_to_normal_mode_and_released_on_hide_or_close() {
        let (_dir, mut app) = app_with_note();
        app.handle_editor_key(key(KeyCode::Char('i')));
        app.toggle_outline_focus();
        assert!(!app.outline_view.focused);
        app.handle_editor_key(key(KeyCode::Esc));
        app.writing.zen.enabled = true;
        app.toggle_outline_focus();
        assert!(!app.outline_view.focused);
        app.writing.zen.enabled = false;
        app.toggle_outline_focus();
        assert!(app.outline_view.focused);
        app.toggle_panel(Command::ToggleOutline);
        assert!(!app.outline_view.focused);
        app.toggle_outline_focus();
        app.close_editor(false);
        assert!(app.editor.is_none());
        assert!(!app.outline_view.focused);
        assert_eq!(app.outline_view.selected, None);
    }
}

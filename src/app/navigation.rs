//! Keyboard ownership and navigation for the live document outline.

use super::App;
use crate::heading_search::HeadingSearch;
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
        let Some(note) = self.editor.as_ref() else {
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

    /// Both heading navigation surfaces share exactly the same jump behavior.
    fn jump_to_heading(&mut self, row: usize) {
        if let Some(note) = self.editor.as_mut() {
            note.state.cursor = Index2::new(row, 0);
            note.state.set_viewport_offset(0, row.saturating_sub(1));
            self.outline_view.focused = false;
            self.message = format!("Heading: line {}", row + 1);
        }
    }

    pub(super) fn open_heading_search(&mut self) {
        if self.has_active_popup() || self.command.is_some() || self.note_search.input.is_some() {
            return;
        }
        let Some(note) = self.editor.as_ref() else {
            self.message = "Open a Markdown note before searching headings.".into();
            return;
        };
        if note.state.mode != EditorMode::Normal {
            return;
        }
        if !markdown::is_markdown(&note.path) {
            self.message = "Heading search is available for Markdown notes only.".into();
            return;
        }
        // The popup owns input, so the immutable snapshot cannot become stale
        // through editing. It deliberately ignores outline visibility/collapse.
        self.heading_search = Some(HeadingSearch::new(&note.state.lines.to_string()));
        self.writing.leader = false;
        self.pending_open_bracket = false;
        self.message.clear();
    }

    pub(super) fn handle_heading_search_key(&mut self, key: KeyEvent) {
        let Some(mut search) = self.heading_search.take() else {
            return;
        };
        let ctrl = key.modifiers == KeyModifiers::CONTROL;
        match key.code {
            KeyCode::Esc | KeyCode::F(7) if key.modifiers.is_empty() => return,
            KeyCode::Enter if key.modifiers.is_empty() => {
                if let Some(row) = search.selected_row() {
                    self.jump_to_heading(row);
                    return;
                }
            }
            KeyCode::Up => search.move_selection(false),
            KeyCode::Down => search.move_selection(true),
            KeyCode::Char('p') if ctrl => search.move_selection(false),
            KeyCode::Char('n') if ctrl => search.move_selection(true),
            KeyCode::Home => search.selected = 0,
            KeyCode::End => search.selected = search.matches.len().saturating_sub(1),
            KeyCode::Backspace => {
                search.query.pop();
                search.filter();
            }
            KeyCode::Char('u') if ctrl => {
                search.query.clear();
                search.filter();
            }
            KeyCode::Char(ch)
                if !ch.is_control()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                search.query.push(ch);
                search.filter();
            }
            _ => {}
        }
        self.heading_search = Some(search);
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
            let Some(note) = self.editor.as_mut() else {
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
                        self.jump_to_heading(heading.row);
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
    fn search_uses_unsaved_headings_and_only_jumps_on_confirmation() {
        let (_dir, mut app) = app_with_note();
        app.panels.show_outline = false;
        app.editor.as_mut().unwrap().state.cursor = Index2::new(2, 3);
        app.handle_editor_key(key(KeyCode::Char('i')));
        app.handle_editor_key(key(KeyCode::Char('X')));
        app.handle_editor_key(key(KeyCode::Esc));
        let note = app.editor.as_ref().unwrap();
        assert!(note.is_dirty());
        let original = note.state.lines.to_string();
        assert!(original.contains("## XB"));
        app.editor.as_mut().unwrap().state.cursor = Index2::new(0, 0);
        app.open_heading_search();
        assert!(app.has_active_popup());
        for ch in ['x', 'b'] {
            app.handle_heading_search_key(key(KeyCode::Char(ch)));
        }
        assert_eq!(app.heading_search.as_ref().unwrap().selected_row(), Some(2));
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, Index2::new(0, 0));
        app.handle_heading_search_key(key(KeyCode::Esc));
        assert!(!app.has_active_popup());
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, Index2::new(0, 0));
        app.open_heading_search();
        for ch in ['x', 'b'] {
            app.handle_heading_search_key(key(KeyCode::Char(ch)));
        }
        app.handle_heading_search_key(key(KeyCode::Enter));
        let note = app.editor.as_ref().unwrap();
        assert_eq!(note.state.cursor, Index2::new(2, 0));
        assert_eq!(note.state.lines.to_string(), original);
        assert!(note.is_dirty());
        assert!(!app.panels.show_outline);
        assert!(app.heading_search.is_none());
        assert_eq!(
            fs::read_to_string(&note.path).unwrap(),
            "# A\nbody\n## B\n# C\n"
        );
    }

    #[test]
    fn search_includes_collapsed_headings_and_restores_outline_on_cancel() {
        let (_dir, mut app) = app_with_note();
        app.editor.as_mut().unwrap().state.cursor = Index2::new(0, 0);
        app.toggle_outline_focus();
        app.handle_outline_key(key(KeyCode::Char('h')));
        assert!(app.outline_view.is_collapsed(0));
        app.open_heading_search();
        app.handle_heading_search_key(key(KeyCode::Char('b')));
        assert_eq!(app.heading_search.as_ref().unwrap().selected_row(), Some(2));
        app.handle_heading_search_key(key(KeyCode::Esc));
        assert!(app.outline_view.focused);
        app.open_heading_search();
        app.handle_heading_search_key(key(KeyCode::Char('b')));
        app.handle_heading_search_key(key(KeyCode::Enter));
        assert!(!app.outline_view.focused);
        assert!(app.outline_view.is_collapsed(0));
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, Index2::new(2, 0));
    }

    #[test]
    fn search_works_in_zen_and_no_match_enter_does_not_close() {
        let (_dir, mut app) = app_with_note();
        app.handle_editor_key(key(KeyCode::Char('i')));
        app.open_heading_search();
        assert!(app.heading_search.is_none());
        app.handle_editor_key(key(KeyCode::Esc));
        app.writing.zen.enabled = true;
        app.open_heading_search();
        app.handle_heading_search_key(key(KeyCode::Char('z')));
        app.handle_heading_search_key(key(KeyCode::Enter));
        assert!(app.heading_search.is_some());
        app.handle_heading_search_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.heading_search.as_ref().unwrap().matches.len(), 3);
        app.handle_heading_search_key(key(KeyCode::End));
        app.handle_heading_search_key(key(KeyCode::Enter));
        assert!(app.writing.zen.enabled);
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, Index2::new(3, 0));
        assert!(!app.editor.as_ref().unwrap().is_dirty());
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

//! Terminal lifecycle, cursor shape and keyboard event routing.

use super::App;
use crate::commands::{Command, KeyScope};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use edtui::EditorMode;
use ratatui::DefaultTerminal;
use std::{io, time::Duration};

impl App {
    pub(super) fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let result = self.run_loop(terminal);
        // Restore the cursor configured by the user in Foot/other terminals.
        let reset = crossterm::execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        match result {
            Err(error) => Err(error),
            Ok(()) => reset,
        }
    }

    fn run_loop(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut previous_insert_mode = None;
        loop {
            let in_insert = self
                .editor
                .as_ref()
                .is_some_and(|note| note.state.mode == EditorMode::Insert);
            if previous_insert_mode != Some(in_insert) {
                let shape = if in_insert {
                    SetCursorStyle::BlinkingBar
                } else {
                    SetCursorStyle::BlinkingBlock
                };
                crossterm::execute!(io::stdout(), shape)?;
                previous_insert_mode = Some(in_insert);
            }
            self.sync_system_theme();
            terminal.draw(|frame| self.draw(frame))?;
            // Poll instead of waiting forever: The desktop may change its theme
            // while the user isn't pressing any keys.
            if !event::poll(Duration::from_millis(400))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if self.route_key(key)? {
                return Ok(());
            }
        }
    }

    /// Route one terminal key; true requests application exit.
    fn route_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if self.snippet_form.is_some() {
            self.handle_snippet_form_key(key);
            return Ok(false);
        }
        if self.snippet_picker.is_some() {
            self.handle_snippet_picker_key(key);
            return Ok(false);
        }
        if self.links.is_some() {
            self.handle_link_key(key);
            return Ok(false);
        }
        if self.citation_picker.is_some() {
            self.handle_citation_key(key);
            return Ok(false);
        }
        if self.wiki_completion.is_some() {
            self.handle_wiki_completion_key(key);
            return Ok(false);
        }
        if self.prompt.is_some() {
            self.handle_prompt_key(key);
            return Ok(false);
        }
        if self.find_dialog.is_some() {
            self.handle_find_key(key);
            return Ok(false);
        }
        if self.vault_search.is_some() {
            self.handle_vault_search_key(key);
            return Ok(false);
        }
        if self.heading_search.is_some() {
            self.handle_heading_search_key(key);
            return Ok(false);
        }
        if self.command_palette.is_some() {
            return self.handle_command_palette_key(key);
        }
        if self.note_search.input.is_some() {
            self.handle_note_search_key(key);
            return Ok(false);
        }
        // Ex input owns its keys just like the other modal inputs.
        if self.command.is_some() {
            self.handle_editor_key(key);
            return Ok(false);
        }
        if let Some(command) = Command::from_key(KeyScope::Global, key) {
            self.writing.leader = false;
            self.pending_open_bracket = false;
            return self.execute_command(command);
        }
        if self.outline_view.focused {
            self.handle_outline_key(key);
            return Ok(false);
        }
        if self.editor.is_some() {
            self.handle_editor_key(key);
            return Ok(false);
        }
        if let Some(command) = Command::from_key(KeyScope::Browser, key) {
            return self.execute_command(command);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Appearance, AutoThemes, ColorOverrides, ThemeChoice};
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;

    fn fixture() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Navigation.md");
        fs::write(&path, "# First\nbody\n## Second\n").unwrap();
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
        (dir, app)
    }

    fn press(app: &mut App, code: KeyCode) {
        assert!(
            !app.route_key(KeyEvent::new(code, KeyModifiers::NONE))
                .unwrap()
        );
    }

    fn type_text(app: &mut App, text: &str) {
        for ch in text.chars() {
            press(app, KeyCode::Char(ch));
        }
    }

    #[test]
    fn hidden_dirty_buffers_block_quit_and_file_operations() {
        let (dir, mut app) = fixture();
        let first = dir.path().join("Navigation.md");
        let second = dir.path().join("Second.md");
        fs::write(&second, "Second note").unwrap();
        app.refresh().unwrap();
        app.select_path(&first);
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, "iX");
        press(&mut app, KeyCode::Esc);
        let before = app.editor.as_ref().unwrap().state.lines.to_string();
        let cursor = app.editor.as_ref().unwrap().state.cursor;
        type_text(&mut app, ":hide");
        press(&mut app, KeyCode::Enter);
        assert!(app.editor.is_none());
        assert_eq!(app.editor.len(), 1);
        press(&mut app, KeyCode::Char('q'));
        assert!(app.message.contains("Unsaved buffers"));
        for code in ['r', 'm', 'd'] {
            press(&mut app, KeyCode::Char(code));
            assert!(app.prompt.is_none());
            assert!(first.exists());
        }
        app.select_path(&second);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.len(), 2);
        type_text(&mut app, ":q");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.len(), 1);
        app.select_path(&first);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.as_ref().unwrap().state.lines.to_string(), before);
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, cursor);
        assert!(app.editor.as_ref().unwrap().is_dirty());
        type_text(&mut app, ":wq");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.len(), 0);
        assert_eq!(fs::read_to_string(&first).unwrap(), before);
        assert!(
            app.route_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
                .unwrap()
        );
    }

    #[test]
    fn ex_bar_owns_global_shortcuts_until_confirmed() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, ":headings");
        let panels = app.panels;
        for key in [KeyCode::F(2), KeyCode::F(4), KeyCode::F(8)] {
            press(&mut app, key);
        }
        assert!(
            !app.route_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
                .unwrap()
        );
        assert_eq!(app.command.as_deref(), Some("headings"));
        assert_eq!(app.panels, panels);
        assert!(!app.has_active_popup());
        press(&mut app, KeyCode::Enter);
        assert!(app.heading_search.is_some());
        assert!(app.command.is_none());
        press(&mut app, KeyCode::Esc);
        type_text(&mut app, ":commands");
        press(&mut app, KeyCode::Enter);
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn global_shortcuts_cancel_pending_leader() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char(' '));
        assert!(app.writing.leader);
        assert!(
            !app.route_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT))
                .unwrap()
        );
        assert!(!app.writing.leader);
        press(&mut app, KeyCode::Char('i'));
        assert_eq!(app.editor.as_ref().unwrap().state.mode, EditorMode::Insert);
    }

    #[test]
    fn palette_routes_to_heading_picker_without_leaking_keys() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::F(8));
        type_text(&mut app, "search document headings");
        press(&mut app, KeyCode::Enter);
        assert!(app.command_palette.is_none());
        assert!(app.heading_search.is_some());
        let panels = app.panels;
        press(&mut app, KeyCode::F(4));
        press(&mut app, KeyCode::F(8));
        assert_eq!(app.panels, panels);
        assert!(app.command_palette.is_none());
        type_text(&mut app, "scnd");
        press(&mut app, KeyCode::Enter);
        let note = app.editor.as_ref().unwrap();
        assert_eq!(note.state.cursor.row, 2);
        assert!(!note.is_dirty());
        assert!(!app.has_active_popup());
    }

    #[test]
    fn palette_save_and_close_preserves_external_changes_and_dirty_buffer() {
        let (dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, "iX");
        press(&mut app, KeyCode::Esc);
        let before = app.editor.as_ref().unwrap().state.lines.to_string();
        let path = dir.path().join("Navigation.md");
        fs::write(&path, "# External change\n").unwrap();
        press(&mut app, KeyCode::F(8));
        type_text(&mut app, "save and close note");
        press(&mut app, KeyCode::Enter);
        let note = app.editor.as_ref().unwrap();
        assert!(note.is_dirty());
        assert_eq!(note.state.lines.to_string(), before);
        assert_eq!(fs::read_to_string(path).unwrap(), "# External change\n");
        assert!(app.message.contains("SAVE FAILED"));
    }

    #[test]
    fn browser_palette_quit_signal_reaches_event_loop() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::F(8));
        type_text(&mut app, "quit zettui");
        assert!(
            app.route_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .unwrap()
        );
        assert!(!app.has_active_popup());
    }
}

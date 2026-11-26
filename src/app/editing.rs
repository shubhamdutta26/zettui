//! Editor commands, saving and keyboard interception.

use super::App;
use crate::commands::{Command, KeyScope};
use crate::editor_search::Direction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::EditorMode;
use ratatui::style::Style;

impl App {
    pub(super) fn save_editor(&mut self) -> bool {
        let Some(note) = &mut self.editor else {
            return false;
        };
        match note.save() {
            Ok(true) => {
                self.message = "Saved successfully.".to_owned();
                true
            }
            Ok(false) => {
                self.message = "No changes to save.".to_owned();
                true
            }
            Err(e) => {
                self.message = format!("SAVE FAILED: {e}");
                false
            }
        }
    }

    pub(super) fn close_editor(&mut self, discard_changes: bool) {
        if self.editor.as_ref().is_some_and(|note| note.is_dirty()) && !discard_changes {
            self.message = "Unsaved changes! Use :w to save or :q! to discard.".to_owned();
            return;
        }
        self.note_search.reset_matches();
        self.outline_view.reset();
        self.editor = None;
        self.writing.leader = false;
        self.wiki_completion = None;
        self.pending_open_bracket = false;
        self.command = None;
        self.update_preview();
        self.message = "Returned to file browser.".to_owned();
    }

    /// Minimal Vim-style application commands (separate from edtui's text motions).
    fn run_command(&mut self, command: String) {
        if let Some(action) = Command::from_ex(&command) {
            self.execute_editor_command(action);
        } else if self.writing.command(command.trim()) {
            self.message.clear();
        } else {
            self.message = format!("Unknown command :{}", command.trim());
        }
    }

    pub(super) fn handle_note_search_key(&mut self, key: KeyEvent) {
        let Some(note) = &mut self.editor else {
            self.note_search.input = None;
            return;
        };
        let style = Style::default()
            .fg(self.theme.selected_fg)
            .bg(self.theme.selected_bg);
        let current = Style::default()
            .fg(self.theme.background)
            .bg(self.theme.accent);
        match key.code {
            KeyCode::Esc => {
                self.note_search.cancel(&mut note.state);
                self.message = "Search canceled.".into();
                return;
            }
            KeyCode::Enter => {
                if let Some(input) = self.note_search.input.take() {
                    let query = if input.query.is_empty() {
                        self.note_search
                            .last
                            .as_ref()
                            .map(|(q, _)| q.clone())
                            .unwrap_or_default()
                    } else {
                        input.query
                    };
                    if !query.is_empty() {
                        self.note_search.jump(
                            &mut note.state,
                            &query,
                            input.origin,
                            input.direction,
                            style,
                            current,
                        );
                        self.note_search.last = Some((query, input.direction));
                    }
                }
                self.message = if self.note_search.count == 0 {
                    "No match.".into()
                } else {
                    format!(
                        "Match {}/{} | n next | N previous",
                        self.note_search.active, self.note_search.count
                    )
                };
                return;
            }
            KeyCode::Backspace => {
                if let Some(input) = &mut self.note_search.input {
                    input.query.pop();
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(input) = &mut self.note_search.input {
                    input.query.clear();
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let Some(input) = &mut self.note_search.input {
                    input.query.push(c);
                }
            }
            _ => return,
        }
        self.note_search.preview(&mut note.state, style, current);
    }

    fn intercept_note_search(&mut self, key: KeyEvent) -> bool {
        if self.note_search.input.is_some() {
            self.handle_note_search_key(key);
            return true;
        }
        let Some(note) = &mut self.editor else {
            return false;
        };
        if note.state.mode != EditorMode::Normal
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        match key.code {
            KeyCode::Char('/') | KeyCode::Char('?') => {
                let direction = if key.code == KeyCode::Char('/') {
                    Direction::Forward
                } else {
                    Direction::Backward
                };
                note.state.clear_highlights();
                self.note_search.begin(&note.state, direction);
                self.message.clear();
                true
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some((query, direction)) = self.note_search.last.clone() {
                    let direction = if key.code == KeyCode::Char('N') {
                        direction.reverse()
                    } else {
                        direction
                    };
                    let origin = note.state.cursor;
                    self.note_search.jump(
                        &mut note.state,
                        &query,
                        origin,
                        direction,
                        Style::default()
                            .fg(self.theme.selected_fg)
                            .bg(self.theme.selected_bg),
                        Style::default()
                            .fg(self.theme.background)
                            .bg(self.theme.accent),
                    );
                    self.message = if self.note_search.count == 0 {
                        "No match.".into()
                    } else {
                        format!(
                            "Match {}/{} | n next | N previous",
                            self.note_search.active, self.note_search.count
                        )
                    };
                } else {
                    self.message = "No previous search. Press / or ?.".into();
                }
                true
            }
            _ => false,
        }
    }

    pub(super) fn handle_editor_key(&mut self, key: KeyEvent) {
        // Only two consecutive `[` keystrokes in Insert mode trigger a picker.
        // A navigation key, modifier shortcut or editor command resets it.
        if key.code != KeyCode::Char('[')
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            self.pending_open_bracket = false;
        }
        // Once a colon command is started, keys go into the command bar.
        if let Some(command) = &mut self.command {
            match key.code {
                KeyCode::Esc => {
                    self.command = None;
                }
                KeyCode::Enter => {
                    let cmd = self.command.take().unwrap_or_default();
                    self.run_command(cmd);
                }
                KeyCode::Backspace => {
                    command.pop();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    command.push(c)
                }
                _ => {}
            }
            return;
        }

        if self.writing.leader {
            self.writing.leader = false;
            if let Some(command) = Command::from_key(KeyScope::Leader, key) {
                self.execute_editor_command(command);
                return;
            }
            if key.code == KeyCode::Esc {
                return;
            }
        } else if key.code == KeyCode::Char(' ')
            && key.modifiers.is_empty()
            && self
                .editor
                .as_ref()
                .is_some_and(|n| n.state.mode == EditorMode::Normal)
        {
            self.writing.leader = true;
            return;
        }
        if self.intercept_note_search(key) {
            return;
        }
        // Ordinary editing can invalidate the previous match coordinates.
        self.note_search.painted.clear();
        if let Some(note) = &mut self.editor {
            note.state.clear_highlights();
        }

        if let Some(command) = Command::from_key(KeyScope::Editor, key) {
            self.execute_editor_command(command);
            return;
        }

        // Only intercept ':' in Normal mode; it is literal text in Insert mode.
        if self
            .editor
            .as_ref()
            .is_some_and(|note| note.state.mode == EditorMode::Normal)
            && key.code == KeyCode::Char(':')
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            self.command = Some(String::new());
            return;
        }

        let opening_bracket = self.editor.as_ref().is_some_and(|note| {
            note.state.mode == EditorMode::Insert
                && note
                    .path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown")
                    })
                && key.code == KeyCode::Char('[')
                && !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        });
        let open_picker = opening_bracket && self.pending_open_bracket;
        if let Some(note) = &mut self.editor {
            self.editor_keys.on_key_event(key, &mut note.state);
        }
        self.pending_open_bracket = opening_bracket && !open_picker;
        if open_picker {
            self.open_wiki_completion();
        }
    }
}

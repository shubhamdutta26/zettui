//! Terminal lifecycle, cursor shape and keyboard event routing.

use super::App;
use crate::commands::{Command, KeyScope};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyEventKind};
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
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if self.snippet_form.is_some() {
                self.handle_snippet_form_key(key);
                continue;
            }
            if self.snippet_picker.is_some() {
                self.handle_snippet_picker_key(key);
                continue;
            }
            if self.links.is_some() {
                self.handle_link_key(key);
                continue;
            }
            if self.citation_picker.is_some() {
                self.handle_citation_key(key);
                continue;
            }
            if self.wiki_completion.is_some() {
                self.handle_wiki_completion_key(key);
                continue;
            }
            if self.prompt.is_some() {
                self.handle_prompt_key(key);
                continue;
            }
            if self.find_dialog.is_some() {
                self.handle_find_key(key);
                continue;
            }
            if self.vault_search.is_some() {
                self.handle_vault_search_key(key);
                continue;
            }
            if self.heading_search.is_some() {
                self.handle_heading_search_key(key);
                continue;
            }
            if self.note_search.input.is_some() {
                self.handle_note_search_key(key);
                continue;
            }
            if let Some(command) = Command::from_key(KeyScope::Global, key) {
                self.execute_command(command)?;
                continue;
            }
            if self.outline_view.focused {
                self.handle_outline_key(key);
                continue;
            }
            if self.editor.is_some() {
                self.handle_editor_key(key);
                continue;
            }
            if let Some(command) = Command::from_key(KeyScope::Browser, key)
                && self.execute_command(command)?
            {
                return Ok(());
            }
        }
    }
}

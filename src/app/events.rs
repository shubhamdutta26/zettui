//! Terminal lifecycle, cursor shape and keyboard event routing.

use super::{App, FindDialog, PromptKind};
use crate::search::SearchMode;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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
            // Poll instead of waiting forever: Omarchy may change its theme
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
            if self.note_search.input.is_some() {
                self.handle_note_search_key(key);
                continue;
            }
            if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.writing.leader = false;
                self.open_vault_search(SearchMode::Text);
                continue;
            }
            if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::ALT) {
                self.writing.leader = false;
                self.open_vault_search(SearchMode::Tags);
                continue;
            }
            if self.resize_columns(key) {
                continue;
            }
            if self.editor.is_some() {
                self.handle_editor_key(key);
                continue;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
                self.cycle_theme();
                continue;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('o') => self.show_links(false),
                KeyCode::Char('b') => self.show_links(true),
                KeyCode::Char('/') => {
                    self.find_dialog = Some(FindDialog::new(&self.entries));
                }
                KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.previous_note()
                }
                KeyCode::Char('n') => self.begin_prompt(PromptKind::NewNote),
                KeyCode::Char('N') => self.begin_prompt(PromptKind::NewFolder),
                KeyCode::Char('r') => {
                    if let Some(source) = self.selected_path() {
                        self.begin_prompt(PromptKind::Rename { source });
                    }
                }
                KeyCode::Char('m') => {
                    if let Some(source) = self.selected_path() {
                        self.begin_prompt(PromptKind::Move { source });
                    }
                }
                KeyCode::Char('d') => {
                    if let Some(source) = self.selected_path() {
                        self.begin_prompt(PromptKind::Trash { source });
                    }
                }
                KeyCode::Char('R') => {
                    if let Err(e) = self.refresh() {
                        self.message = format!("Refresh failed: {e}");
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => self.move_down(),
                KeyCode::Char('k') | KeyCode::Up => self.move_up(),
                KeyCode::Enter => self.enter()?,

                KeyCode::Char('l') | KeyCode::Right => {
                    if self
                        .entries
                        .get(self.selected)
                        .is_some_and(|entry| entry.is_dir)
                    {
                        self.enter()?;
                    }
                }
                KeyCode::Char('h') | KeyCode::Backspace | KeyCode::Left => self.go_back()?,
                _ => {}
            }
        }
    }
}

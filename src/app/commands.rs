//! Shared execution of application commands from keyboard and future pickers.

use super::{App, FindDialog, PromptKind};
use crate::{commands::Command, search::SearchMode};
use std::io;

impl App {
    /// Returns true only when the application should quit.
    pub(super) fn execute_command(&mut self, command: Command) -> io::Result<bool> {
        match command {
            Command::SearchText => {
                self.writing.leader = false;
                self.open_vault_search(SearchMode::Text);
            }
            Command::SearchTags => {
                self.writing.leader = false;
                self.open_vault_search(SearchMode::Tags);
            }
            Command::NarrowBrowser => self.resize_browser(false),
            Command::WidenBrowser => self.resize_browser(true),
            Command::NarrowOutline => self.resize_outline(false),
            Command::WidenOutline => self.resize_outline(true),
            Command::ToggleFiles | Command::TogglePreview | Command::ToggleOutline => {
                self.toggle_panel(command);
            }
            Command::FocusOutline => self.toggle_outline_focus(),
            Command::CycleTheme => self.cycle_theme(),
            Command::Quit => {
                // Browser-only binding. Do not permit a future caller to bypass dirty notes.
                if self.editor.is_some() {
                    self.message = "Close the note before quitting.".to_owned();
                } else {
                    return Ok(true);
                }
            }
            Command::OutgoingLinks => self.show_links(false),
            Command::Backlinks => self.show_links(true),
            Command::FindFilename => {
                self.find_dialog = Some(FindDialog::new(&self.entries));
            }
            Command::PreviousNote => self.previous_note(),
            Command::NewNote => self.begin_prompt(PromptKind::NewNote),
            Command::NewFolder => self.begin_prompt(PromptKind::NewFolder),
            Command::Rename => {
                if let Some(source) = self.selected_path() {
                    self.begin_prompt(PromptKind::Rename { source });
                }
            }
            Command::Move => {
                if let Some(source) = self.selected_path() {
                    self.begin_prompt(PromptKind::Move { source });
                }
            }
            Command::Trash => {
                if let Some(source) = self.selected_path() {
                    self.begin_prompt(PromptKind::Trash { source });
                }
            }
            Command::Refresh => {
                if let Err(e) = self.refresh() {
                    self.message = format!("Refresh failed: {e}");
                }
            }
            Command::MoveDown => self.move_down(),
            Command::MoveUp => self.move_up(),
            Command::OpenSelection => self.enter()?,
            Command::EnterFolder => {
                if self
                    .entries
                    .get(self.selected)
                    .is_some_and(|entry| entry.is_dir)
                {
                    self.enter()?;
                }
            }
            Command::ParentFolder => self.go_back()?,
            Command::CitationPicker => self.open_citation_picker(),
            Command::SnippetPicker => self.open_snippet_picker(),
            Command::Save => {
                self.save_editor();
            }
            Command::Close => self.close_editor(false),
            Command::DiscardAndClose => self.close_editor(true),
            Command::SaveAndClose => {
                if self.save_editor() {
                    self.close_editor(false);
                }
            }
            Command::Writing(input) => {
                if self.writing.command(input) {
                    self.message.clear();
                }
            }
        }
        Ok(false)
    }

    /// Editor actions cannot request application exit; report any I/O error.
    pub(super) fn execute_editor_command(&mut self, command: Command) {
        if let Err(error) = self.execute_command(command) {
            self.message = format!("Command failed: {error}");
        }
    }
}

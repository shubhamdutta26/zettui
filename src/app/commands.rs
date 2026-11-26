//! Shared execution of application commands from keyboard and command palette.

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
            Command::SearchHeadings => self.open_heading_search(),
            Command::OpenPalette => self.open_command_palette(),
            Command::WritingAnalytics => self.open_analytics_dashboard(),
            Command::CycleTheme => self.cycle_theme(),
            Command::Quit => {
                // Browser-only binding. Do not permit a future caller to bypass dirty notes.
                if self.editor.is_some() {
                    self.message = "Close the note before quitting.".to_owned();
                } else if self.editor.any_dirty() {
                    self.message =
                        "Unsaved buffers remain. Reopen them to save or discard before quitting."
                            .to_owned();
                } else {
                    return Ok(true);
                }
            }
            Command::QuitAll => {
                if self.editor.any_dirty() {
                    self.message = "Unsaved buffers remain. Use :wa or :wqa to save all.".into();
                } else {
                    return Ok(true);
                }
            }
            Command::SaveQuitAll => {
                self.save_all_buffers();
                if !self.editor.any_dirty() {
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
            Command::Hide => self.hide_editor(),
            Command::BufferPicker => self.open_buffer_picker(),
            Command::NextBuffer => self.cycle_buffer(true),
            Command::PreviousBuffer => self.cycle_buffer(false),
            Command::AlternateBuffer => self.alternate_buffer(),
            Command::CloseBuffer => self.close_buffer(),
            Command::CloseOtherBuffers => self.close_other_buffers(),
            Command::ToggleTabs => {
                self.show_tab_bar = !self.show_tab_bar;
                self.message = format!(
                    "Buffer tabs {}.",
                    if self.show_tab_bar { "shown" } else { "hidden" }
                );
            }
            Command::ToggleHiddenFolders => self.toggle_hidden_folders(),
            Command::SaveAll => self.save_all_buffers(),
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

    /// Forward an application-wide quit signal from editor input to event routing.
    pub(super) fn execute_editor_command(&mut self, command: Command) {
        match self.execute_command(command) {
            Ok(true) => self.quit_requested = true,
            Ok(false) => {}
            Err(error) => self.message = format!("Command failed: {error}"),
        }
    }
}

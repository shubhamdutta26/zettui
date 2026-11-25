//! Directory navigation and file previews.

use super::{App, Entry};
use crate::editor::OpenNote;
use edtui::EditorEventHandler;
use std::{fs, io};

impl App {
    pub(super) fn refresh(&mut self) -> io::Result<()> {
        let previously_selected = self
            .entries
            .get(self.selected)
            .map(|entry| entry.path.clone());
        let previous_index = self.selected;
        self.entries = fs::read_dir(&self.current_dir)?
            .filter_map(|item| {
                let item = item.ok()?;
                let kind = item.file_type().ok()?;
                if kind.is_symlink() || item.file_name() == ".git" {
                    return None;
                }
                Some(Entry {
                    name: item.file_name().to_string_lossy().into_owned(),
                    path: item.path(),
                    is_dir: kind.is_dir(),
                })
            })
            .collect();
        self.entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.selected = previously_selected
            .and_then(|path| self.entries.iter().position(|item| item.path == path))
            .unwrap_or_else(|| previous_index.min(self.entries.len().saturating_sub(1)));
        self.update_preview();
        Ok(())
    }

    pub(super) fn select_path(&mut self, path: &std::path::Path) {
        if let Some(index) = self.entries.iter().position(|item| item.path == path) {
            self.selected = index;
            self.update_preview();
        }
    }

    pub(super) fn update_preview(&mut self) {
        self.preview = match self.entries.get(self.selected) {
            None => "This directory is empty.".to_owned(),
            Some(entry) if entry.is_dir => {
                format!("Directory: {}\n\nPress Enter to open it.", entry.name)
            }
            Some(entry) => match fs::metadata(&entry.path) {
                Ok(info) if info.len() > 256_000 => {
                    "File too large to preview (256 KB limit).".to_owned()
                }
                Ok(_) => fs::read_to_string(&entry.path)
                    .unwrap_or_else(|e| format!("Cannot read UTF-8 text: {e}")),
                Err(e) => format!("Cannot inspect file: {e}"),
            },
        };
    }

    pub(super) fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            self.update_preview();
        }
    }

    pub(super) fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.update_preview();
        }
    }

    pub(super) fn enter(&mut self) -> io::Result<()> {
        let Some(entry) = self.entries.get(self.selected) else {
            return Ok(());
        };
        let path = entry.path.clone();
        if entry.is_dir {
            self.current_dir = path;
            self.refresh()?;
        } else {
            match OpenNote::load(path) {
                Ok(note) => {
                    self.note_search.reset_matches();
                    self.editor = Some(note);
                    self.editor_keys = EditorEventHandler::default();
                    self.pending_open_bracket = false;
                    self.wiki_completion = None;
                    self.command = None;
                    self.message = "Editor opened. Press i to insert text.".to_owned();
                }
                Err(e) => self.message = format!("Cannot edit: {e}"),
            }
        }
        Ok(())
    }

    pub(super) fn go_back(&mut self) -> io::Result<()> {
        if self.current_dir != self.root
            && let Some(parent) = self.current_dir.parent()
        {
            let child = self.current_dir.clone();
            self.current_dir = parent.to_path_buf();
            self.refresh()?;
            self.select_path(&child);
        }
        Ok(())
    }
}

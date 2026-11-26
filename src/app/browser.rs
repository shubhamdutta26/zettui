//! Directory navigation and file previews.

use super::{App, Entry};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

impl App {
    /// Keep the browser outside hidden subtrees when their visibility is disabled.
    /// The workspace root itself is always browsable, even if its name starts with a dot.
    fn visible_browser_directory(&self) -> PathBuf {
        let Ok(relative) = self.current_dir.strip_prefix(&self.root) else {
            return self.root.clone();
        };
        let mut directory = self.root.clone();
        for component in relative.components() {
            let name = component.as_os_str();
            if name == ".git"
                || (!self.show_hidden_folders && name.to_string_lossy().starts_with('.'))
            {
                break;
            }
            directory.push(name);
        }
        directory
    }

    pub(super) fn toggle_hidden_folders(&mut self) {
        let previous = self.show_hidden_folders;
        self.show_hidden_folders = !previous;
        match self.refresh() {
            Ok(()) => {
                let state = if self.show_hidden_folders {
                    "shown"
                } else {
                    "hidden"
                };
                self.message = format!("Hidden folders {state}.");
            }
            Err(error) => {
                self.show_hidden_folders = previous;
                self.message = format!("Cannot change hidden-folder visibility: {error}");
            }
        }
    }

    pub(super) fn refresh(&mut self) -> io::Result<()> {
        let previously_selected = self
            .entries
            .get(self.selected)
            .map(|entry| entry.path.clone());
        let previous_index = self.selected;
        let directory = self.visible_browser_directory();
        let changed_directory = directory != self.current_dir;
        let entries: Vec<Entry> = fs::read_dir(&directory)?
            .filter_map(|item| {
                let item = item.ok()?;
                let kind = item.file_type().ok()?;
                if kind.is_symlink() || item.file_name() == ".git" {
                    return None;
                }
                let name = item.file_name().to_string_lossy().into_owned();
                if kind.is_dir() && !self.show_hidden_folders && name.starts_with('.') {
                    return None;
                }
                Some(Entry {
                    name,
                    path: item.path(),
                    is_dir: kind.is_dir(),
                })
            })
            .collect();
        self.current_dir = directory;
        self.entries = entries;
        self.entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.selected = previously_selected
            .and_then(|path| self.entries.iter().position(|item| item.path == path))
            .unwrap_or_else(|| {
                if changed_directory {
                    0
                } else {
                    previous_index.min(self.entries.len().saturating_sub(1))
                }
            });
        self.update_preview();
        Ok(())
    }

    pub(super) fn select_path(&mut self, path: &Path) {
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
            match self.editor.open(&path) {
                Ok(changed) => {
                    if changed {
                        self.reset_buffer_context();
                    }
                    self.message = format!("Editor opened. {} open buffer(s).", self.editor.len());
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

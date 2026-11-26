//! In-memory note buffers. Each open note retains its own editor state.

use crate::editor::OpenNote;
use edtui::EditorMode;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Stable opening order, with an optional active editor. Browser mode does not
/// imply that all notes have been closed.
#[derive(Default)]
pub(crate) struct Buffers {
    notes: Vec<OpenNote>,
    active: Option<usize>,
    last: Option<PathBuf>,
    previous: Option<PathBuf>,
}

impl Buffers {
    pub fn as_ref(&self) -> Option<&OpenNote> {
        self.active.map(|index| &self.notes[index])
    }

    pub fn as_mut(&mut self) -> Option<&mut OpenNote> {
        self.active.map(|index| &mut self.notes[index])
    }

    pub fn is_some(&self) -> bool {
        self.active.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.active.is_none()
    }

    /// Open a regular file or activate its existing buffer. On failure the
    /// current buffer and the opening order remain unchanged.
    pub fn open(&mut self, path: &Path) -> io::Result<bool> {
        // An already open note remains accessible even if removed externally.
        let index = if let Some(index) = self.notes.iter().position(|note| note.path == path) {
            index
        } else {
            // Do not resolve a final symlink into an editable file.
            if fs::symlink_metadata(path)?.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Only regular files can be edited",
                ));
            }
            let canonical = path.canonicalize()?;
            if let Some(index) = self.notes.iter().position(|note| note.path == canonical) {
                index
            } else {
                let note = OpenNote::load(canonical)?;
                self.notes.push(note);
                self.notes.len() - 1
            }
        };
        Ok(self.activate(index))
    }

    pub fn notes(&self) -> &[OpenNote] {
        &self.notes
    }

    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    pub fn activate(&mut self, index: usize) -> bool {
        if index >= self.notes.len() || self.active == Some(index) {
            return false;
        }
        let path = &self.notes[index].path;
        if self.last.as_ref() != Some(path) {
            self.previous = self.last.take();
            self.last = Some(path.clone());
        }
        self.hide();
        self.active = Some(index);
        true
    }

    pub fn cycle(&mut self, forward: bool) -> bool {
        let len = self.notes.len();
        if len == 0 {
            return false;
        }
        let index = match self.active {
            Some(i) if forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
            None if forward => 0,
            None => len - 1,
        };
        self.activate(index)
    }

    pub fn activate_previous(&mut self) -> bool {
        let target = if self.active.is_none() {
            &self.last
        } else {
            &self.previous
        };
        let index = target
            .as_ref()
            .and_then(|path| self.notes.iter().position(|note| &note.path == path));
        index.is_some_and(|i| self.activate(i))
    }

    /// Refuse the entire operation if any other buffer has unsaved edits.
    pub fn close_others(&mut self) -> bool {
        let Some(active) = self.active else {
            return false;
        };
        if self
            .notes
            .iter()
            .enumerate()
            .any(|(i, note)| i != active && note.is_dirty())
        {
            return false;
        }
        let note = self.notes.remove(active);
        self.notes.clear();
        self.last = Some(note.path.clone());
        self.previous = None;
        self.notes.push(note);
        self.active = Some(0);
        true
    }

    /// Attempt each save independently; failed buffers remain open and dirty.
    pub fn save_all(&mut self) -> (usize, Vec<String>) {
        let mut saved = 0;
        let mut failures = Vec::new();
        for note in &mut self.notes {
            match note.save() {
                Ok(true) => saved += 1,
                Ok(false) => {}
                Err(error) => failures.push(format!("{}: {error}", note.path.display())),
            }
        }
        (saved, failures)
    }

    /// Park the active note without touching its content, undo or viewport.
    pub fn hide(&mut self) {
        if let Some(note) = self.as_mut() {
            note.state.mode = EditorMode::Normal;
            note.state.clear_highlights();
        }
        self.active = None;
    }

    /// Close only the active note. The caller must explicitly authorize discard.
    pub fn close(&mut self, discard_changes: bool) -> bool {
        let Some(index) = self.active else {
            return true;
        };
        if !discard_changes && self.notes[index].is_dirty() {
            return false;
        }
        let removed = self.notes.remove(index);
        if self.last.as_ref() == Some(&removed.path) {
            self.last = self.previous.take();
        }
        if self.previous.as_ref() == Some(&removed.path) {
            self.previous = None;
        }
        self.active = None;
        true
    }

    pub fn any_dirty(&self) -> bool {
        self.notes.iter().any(OpenNote::is_dirty)
    }

    /// Reject file operations on an open note or a directory containing one.
    pub fn affects_open_note(&self, path: &Path) -> bool {
        self.notes.iter().any(|note| note.path.starts_with(path))
    }

    pub fn len(&self) -> usize {
        self.notes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edtui::{Index2, Lines};

    #[test]
    fn switching_preserves_unsaved_text_cursor_and_opening_order() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        fs::write(&a, "Alpha").unwrap();
        fs::write(&b, "Beta").unwrap();
        let mut buffers = Buffers::default();
        assert!(buffers.open(&a).unwrap());
        let note = buffers.as_mut().unwrap();
        note.state.lines = Lines::new(vec!["Edited Alpha".chars().collect()]);
        note.state.cursor = Index2::new(0, 3);
        note.state.mode = EditorMode::Insert;
        assert!(buffers.open(&b).unwrap());
        assert_eq!(buffers.as_ref().unwrap().state.lines.to_string(), "Beta");
        assert!(buffers.any_dirty());
        assert!(buffers.open(&a).unwrap());
        assert_eq!(
            buffers.as_ref().unwrap().state.lines.to_string(),
            "Edited Alpha"
        );
        assert_eq!(buffers.as_ref().unwrap().state.cursor, Index2::new(0, 3));
        assert_eq!(buffers.as_ref().unwrap().state.mode, EditorMode::Normal);
        assert!(!buffers.open(&a).unwrap());
        assert_eq!(buffers.len(), 2);
        assert_eq!(buffers.notes[0].path, a);
        assert_eq!(fs::read_to_string(&a).unwrap(), "Alpha");
        assert!(!buffers.close(false));
        assert!(buffers.close(true));
        assert_eq!(buffers.len(), 1);
        assert!(buffers.is_none());
        assert!(!buffers.any_dirty());
    }

    #[test]
    fn save_all_keeps_conflicting_buffer_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        fs::write(&a, "Alpha").unwrap();
        fs::write(&b, "Beta").unwrap();
        let mut buffers = Buffers::default();
        buffers.open(&a).unwrap();
        buffers.as_mut().unwrap().state.lines = Lines::new(vec![vec!['A']]);
        buffers.open(&b).unwrap();
        buffers.as_mut().unwrap().state.lines = Lines::new(vec![vec!['B']]);
        fs::write(&a, "External").unwrap();
        assert!(!buffers.close_others());
        let (saved, errors) = buffers.save_all();
        assert_eq!(saved, 1);
        assert_eq!(errors.len(), 1);
        assert!(buffers.any_dirty());
        assert_eq!(fs::read_to_string(a).unwrap(), "External");
        assert_eq!(fs::read_to_string(b).unwrap(), "B");
    }

    #[test]
    fn hidden_buffers_are_protected_and_failed_open_keeps_active_note() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.md");
        fs::write(&a, "Alpha").unwrap();
        let mut buffers = Buffers::default();
        buffers.open(&a).unwrap();
        assert!(buffers.open(&dir.path().join("missing.md")).is_err());
        assert_eq!(buffers.as_ref().unwrap().path, a);
        buffers.as_mut().unwrap().state.lines = Lines::new(vec![vec!['X']]);
        buffers.hide();
        assert!(buffers.is_none());
        assert!(buffers.any_dirty());
        assert!(buffers.affects_open_note(dir.path()));
        assert!(buffers.affects_open_note(&a));
        assert!(!buffers.affects_open_note(&dir.path().join("other.md")));
        buffers.open(&a).unwrap();
        buffers.as_mut().unwrap().save().unwrap();
        assert!(!buffers.any_dirty());
        assert_eq!(fs::read_to_string(a).unwrap(), "X");
    }
}

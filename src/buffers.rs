//! In-memory note buffers. Each open note retains its own editor state.

use crate::editor::OpenNote;
use edtui::EditorMode;
use std::{fs, io, path::Path};

/// Stable opening order, with an optional active editor. Browser mode does not
/// imply that all notes have been closed.
#[derive(Default)]
pub(crate) struct Buffers {
    notes: Vec<OpenNote>,
    active: Option<usize>,
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
        if self.active == Some(index) {
            return Ok(false);
        }
        self.hide();
        self.active = Some(index);
        Ok(true)
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
        if self.notes[index].is_dirty() && !discard_changes {
            return false;
        }
        self.notes.remove(index);
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

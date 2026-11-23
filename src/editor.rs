//! The document model: loading UTF-8 notes, detecting changes, and saving safely.
//! This module does not draw a TUI. It can be tested independently.

use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use edtui::{EditorState, Lines};

const MAX_EDIT_BYTES: u64 = 256_000;

/// An open document together with the version we originally read from disk.
pub struct OpenNote {
    pub path: PathBuf,
    pub state: EditorState,
    original_disk: String,
    saved_text: String,
    crlf: bool,
    bom: bool,
}

impl OpenNote {
    /// Load a regular UTF-8 file up to MAX_EDIT_BYTES into the Vim editor.
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Only regular files can be edited",
            ));
        }
        if metadata.len() > MAX_EDIT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "File is too large to edit in Lesson 2 (256 KB limit)",
            ));
        }
        let original_disk = fs::read_to_string(&path)?;
        let (saved_text, crlf, bom) = normalize(&original_disk);
        let state = EditorState::new(lines_from_text(&saved_text));
        Ok(Self {
            path,
            state,
            original_disk,
            saved_text,
            crlf,
            bom,
        })
    }

    pub fn is_dirty(&self) -> bool {
        self.state.lines.to_string() != self.saved_text
    }

    /// Save only if the content has changed AND no other program changed the file.
    pub fn save(&mut self) -> io::Result<bool> {
        if !self.is_dirty() {
            return Ok(false);
        }

        // Avoid clobbering changes Zettlr or another editor made meanwhile.
        let current_disk = fs::read_to_string(&self.path)?;
        if current_disk != self.original_disk {
            return Err(io::Error::other(
                "File changed on disk since you opened it. Changes NOT saved. Copy your edits elsewhere before retrying.",
            ));
        }
        if fs::symlink_metadata(&self.path)?.file_type().is_symlink() {
            return Err(io::Error::other(
                "File became a symlink; refusing to overwrite",
            ));
        }

        let text = self.state.lines.to_string();
        let mut disk_text = if self.crlf {
            text.replace('\n', "\r\n")
        } else {
            text.clone()
        };
        if self.bom {
            disk_text.insert(0, '\u{feff}');
        }
        save_atomically(&self.path, &disk_text)?;
        self.original_disk = disk_text;
        self.saved_text = text;
        Ok(true)
    }
}

/// Preserve UTF-8 BOM and common Windows CRLF line endings on save.
fn lines_from_text(text: &str) -> Lines {
    // `split` (unlike `str::lines`) includes a final empty line, so an
    // existing final newline is preserved when we save.
    let rows: Vec<Vec<char>> = text
        .split('\n')
        .map(|line| line.chars().collect())
        .collect();
    Lines::new(rows)
}

fn normalize(disk: &str) -> (String, bool, bool) {
    let (bom, text) = match disk.strip_prefix('\u{feff}') {
        Some(text) => (true, text),
        None => (false, disk),
    };
    let crlf = text.contains("\r\n");
    (text.replace("\r\n", "\n").to_owned(), crlf, bom)
}

/// Write to a temporary sibling first, then rename it over the original.
/// Linux rename in the same directory is atomic.
fn save_atomically(path: &Path, text: &str) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing parent directory"))?;
    let permissions = fs::metadata(path)?.permissions();
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.as_file().set_permissions(permissions)?;
    temp.write_all(text.as_bytes())?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_yaml_id_tags_and_final_newline() {
        let raw = "---\nid: 202611221000\ntags: [rust, notes]\n---\n\n# Hello\n";
        let (editable, crlf, bom) = normalize(raw);
        assert_eq!(editable, raw);
        assert_eq!(lines_from_text(&editable).to_string(), raw);
        assert!(!crlf);
        assert!(!bom);
    }

    #[test]
    fn preserves_windows_line_endings_and_bom() {
        let raw = "\u{feff}---\r\nid: abc\r\n---\r\n";
        let (editable, crlf, bom) = normalize(raw);
        assert_eq!(editable, "---\nid: abc\n---\n");
        assert!(crlf);
        assert!(bom);
    }

    #[test]
    fn detects_external_file_change_and_refuses_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "Hello\n").unwrap();
        let mut note = OpenNote::load(path.clone()).unwrap();
        note.state.lines = lines_from_text("My edit\n");
        fs::write(&path, "External change\n").unwrap();
        assert!(note.save().is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "External change\n");
    }

    #[test]
    fn saves_note_without_losing_yaml_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "---\nid: 123\ntags: [test]\n---\nOriginal\n").unwrap();
        let mut note = OpenNote::load(path.clone()).unwrap();
        note.state.lines = lines_from_text("---\nid: 123\ntags: [test]\n---\nEdited\n");
        assert!(note.save().unwrap());
        assert!(!note.is_dirty());
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "---\nid: 123\ntags: [test]\n---\nEdited\n"
        );
    }
}

//! Small, independently testable filesystem actions for Lesson 3.
//! We never permanently delete notes, overwrite rename targets, or follow
//! directory symlinks when choosing a move destination.

use std::{
    fs::{self, OpenOptions},
    io,
    path::{Component, Path, PathBuf},
};

use rustix::fs::{CWD, RenameFlags, renameat_with};

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// Restrict names to ONE file/folder name, not a path.
fn check_name(name: &str) -> io::Result<&str> {
    let name = name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name == ".git"
        || name.contains('/')
        || name.contains('\\')
        || name.chars().any(char::is_control)
    {
        return Err(invalid(
            "Enter a single filename (not a path); avoid . and ..",
        ));
    }
    Ok(name)
}

/// Create an empty UTF-8 note.  If there is no extension, use .md.
/// `create_new(true)` fails rather than overwrite an existing entry.
pub fn create_note(folder: &Path, name: &str) -> io::Result<PathBuf> {
    let name = check_name(name)?;
    let filename = if Path::new(name).extension().is_some() {
        name.to_owned()
    } else {
        format!("{name}.md")
    };
    let target = folder.join(filename);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)?;
    Ok(target)
}

pub fn create_folder(folder: &Path, name: &str) -> io::Result<PathBuf> {
    let name = check_name(name)?;
    let target = folder.join(name);
    fs::create_dir(&target)?;
    Ok(target)
}

fn ensure_entry(path: &Path) -> io::Result<()> {
    let md = fs::symlink_metadata(path)?;
    if md.file_type().is_symlink() {
        return Err(invalid("Symlinks are not managed in this version"));
    }
    if !md.is_file() && !md.is_dir() {
        return Err(invalid("Only regular files and folders are supported"));
    }
    Ok(())
}

/// Linux renameat2 with NOREPLACE means the destination is NEVER clobbered,
/// even if another program creates the name just before our rename.
fn rename_no_replace(source: &Path, dest: &Path) -> io::Result<()> {
    if source == dest {
        return Err(invalid("Source and destination are the same"));
    }
    renameat_with(CWD, source, CWD, dest, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

/// Rename an entry, keeping it in its current folder.
pub fn rename_entry(source: &Path, new_name: &str) -> io::Result<PathBuf> {
    ensure_entry(source)?;
    let new_name = check_name(new_name)?;
    let parent = source
        .parent()
        .ok_or_else(|| invalid("Missing parent folder"))?;
    let dest = parent.join(new_name);
    rename_no_replace(source, &dest)?;
    Ok(dest)
}

/// Resolve a *root-relative* destination folder, rejecting `..`, absolute
/// paths, and symlinks at ANY directory component.
fn destination_folder(root: &Path, input: &str) -> io::Result<PathBuf> {
    let input = input.trim();
    if input.is_empty() {
        return Err(invalid(
            "Type a folder relative to the notes root (or . for root)",
        ));
    }
    let mut dest = root.to_path_buf();
    for part in Path::new(input).components() {
        match part {
            Component::CurDir => {}
            Component::Normal(piece) => {
                dest.push(piece);
                let meta = fs::symlink_metadata(&dest)?;
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err(invalid(
                        "Destination must be a real existing folder, not a symlink",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    "Use a relative folder; absolute paths and .. are forbidden",
                ));
            }
        }
    }
    Ok(dest)
}

/// Move a file or directory to an existing folder underneath the notes root.
pub fn move_entry(root: &Path, source: &Path, destination: &str) -> io::Result<PathBuf> {
    ensure_entry(source)?;
    let target_dir = destination_folder(root, destination)?;
    if !source.starts_with(root) {
        return Err(invalid("Source is outside the notes root"));
    }
    if fs::symlink_metadata(source)?.is_dir() && target_dir.starts_with(source) {
        return Err(invalid(
            "Cannot move a directory into itself or a child directory",
        ));
    }
    let name = source
        .file_name()
        .ok_or_else(|| invalid("Source has no filename"))?;
    let target = target_dir.join(name);
    rename_no_replace(source, &target)?;
    Ok(target)
}

/// Send to the operating system's Trash. There is deliberately NO permanent
/// delete fallback if Trash is unavailable.
pub fn send_to_trash(source: &Path) -> io::Result<()> {
    ensure_entry(source)?;
    trash::delete(source).map_err(|e| io::Error::other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_note_defaults_to_md_and_wont_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let path = create_note(tmp.path(), "VHH affinity").unwrap();
        assert_eq!(path.file_name().unwrap(), "VHH affinity.md");
        assert!(create_note(tmp.path(), "VHH affinity").is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "");
    }

    #[test]
    fn existing_extensions_are_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let path = create_note(tmp.path(), "metadata.json").unwrap();
        assert!(path.ends_with("metadata.json"));
    }

    #[test]
    fn rejects_paths_as_names() {
        let tmp = tempfile::tempdir().unwrap();
        for bad in ["", ".", "..", "../oops", "foo/bar", "/tmp/oops", ".git"] {
            assert!(
                create_note(tmp.path(), bad).is_err(),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn successful_rename_keeps_content() {
        let tmp = tempfile::tempdir().unwrap();
        let first = create_note(tmp.path(), "first.md").unwrap();
        fs::write(&first, "---\nid: 20261122\ntags: [test]\n---\n").unwrap();
        let second = rename_entry(&first, "second.md").unwrap();
        assert!(!first.exists());
        assert_eq!(
            fs::read_to_string(second).unwrap(),
            "---\nid: 20261122\ntags: [test]\n---\n"
        );
    }

    #[test]
    fn moving_does_not_overwrite_existing_destination() {
        let tmp = tempfile::tempdir().unwrap();
        create_folder(tmp.path(), "Target").unwrap();
        let source = create_note(tmp.path(), "same.md").unwrap();
        let destination = create_note(&tmp.path().join("Target"), "same.md").unwrap();
        fs::write(&source, "new").unwrap();
        fs::write(&destination, "original").unwrap();
        assert!(move_entry(tmp.path(), &source, "Target").is_err());
        assert_eq!(fs::read_to_string(&destination).unwrap(), "original");
        assert_eq!(fs::read_to_string(&source).unwrap(), "new");
    }

    #[test]
    fn rename_refuses_to_replace_another_note() {
        let tmp = tempfile::tempdir().unwrap();
        let one = create_note(tmp.path(), "one.md").unwrap();
        let two = create_note(tmp.path(), "two.md").unwrap();
        fs::write(&two, "precious data").unwrap();
        assert!(rename_entry(&one, "two.md").is_err());
        assert!(one.exists());
        assert_eq!(fs::read_to_string(two).unwrap(), "precious data");
    }

    #[test]
    fn move_into_folder_and_block_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let source = create_note(tmp.path(), "antibody.md").unwrap();
        create_folder(tmp.path(), "Readings").unwrap();
        let moved = move_entry(tmp.path(), &source, "Readings").unwrap();
        assert!(moved.exists());
        assert!(!source.exists());
        assert!(move_entry(tmp.path(), &moved, "../").is_err());
        assert!(move_entry(tmp.path(), &moved, "/tmp").is_err());
    }

    #[test]
    fn cannot_move_folder_into_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let top = create_folder(tmp.path(), "Projects").unwrap();
        create_folder(&top, "Child").unwrap();
        assert!(move_entry(tmp.path(), &top, "Projects/Child").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn cannot_move_via_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let note = create_note(tmp.path(), "note.md").unwrap();
        symlink(outside.path(), tmp.path().join("shortcut")).unwrap();
        assert!(move_entry(tmp.path(), &note, "shortcut").is_err());
        assert!(note.exists());
    }
}

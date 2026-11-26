//! Versioned, workspace-local session metadata stored outside the notes directory.
use serde::{Deserialize, Serialize};
use std::{
    env, fs, io,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const VERSION: u32 = 1;
const MAX_SESSION_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Settings {
    pub enabled: bool,
    pub restore_on_startup: bool,
    pub restore_cursor: bool,
    pub restore_layout: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            restore_on_startup: true,
            restore_cursor: true,
            restore_layout: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BufferState {
    pub path: PathBuf,
    pub row: usize,
    pub col: usize,
    pub scroll_x: usize,
    pub scroll_y: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LayoutState {
    pub browser_width_percent: u16,
    pub outline_width_percent: u16,
    pub show_files: bool,
    pub show_preview: bool,
    pub show_outline: bool,
    pub show_tab_bar: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Session {
    version: u32,
    workspace: PathBuf,
    pub buffers: Vec<BufferState>,
    pub active: Option<PathBuf>,
    pub directory: PathBuf,
    pub selection: Option<PathBuf>,
    pub layout: LayoutState,
    #[serde(default)]
    pub show_hidden_folders: Option<bool>,
}

impl Session {
    pub fn new(workspace: PathBuf, layout: LayoutState) -> Self {
        Self {
            version: VERSION,
            workspace,
            buffers: Vec::new(),
            active: None,
            directory: PathBuf::new(),
            selection: None,
            layout,
            show_hidden_folders: None,
        }
    }

    pub fn load(path: &Path, workspace: &Path) -> io::Result<Option<Self>> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_SESSION_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_SESSION_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Session too large",
            ));
        }
        let session: Self = serde_json::from_slice(&bytes)?;
        if session.version != VERSION || session.workspace != workspace {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported session version or different workspace",
            ));
        }
        Ok(Some(session))
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("Missing session directory"))?;
        fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut temp, self)?;
        temp.write_all(b"\n")?;
        if temp.as_file().metadata()?.len() > MAX_SESSION_BYTES {
            return Err(io::Error::other("Session too large"));
        }
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
}

/// FNV-1a provides a deterministic filename across process restarts. The full
/// canonical workspace path in the JSON is also checked when reading.
pub(crate) fn workspace_id(root: &Path) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in root.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn state_path(root: &Path) -> Option<PathBuf> {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(".local/state"))
        })?;
    Some(
        base.join("zettui/sessions")
            .join(format!("{}.json", workspace_id(root))),
    )
}

/// Only restore paths within the original workspace; never follow symlinks or
/// accept absolute paths or traversal components from session metadata.
pub(crate) fn resolve(root: &Path, relative: &Path) -> Option<PathBuf> {
    if !relative
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let path = root.join(relative);
    let canonical = path.canonicalize().ok()?;
    (canonical == path && canonical.starts_with(root)).then_some(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> LayoutState {
        LayoutState {
            browser_width_percent: 35,
            outline_width_percent: 25,
            show_files: true,
            show_preview: true,
            show_outline: false,
            show_tab_bar: true,
        }
    }

    #[test]
    fn hidden_folder_preference_round_trips_and_old_sessions_load() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("state.json");
        let mut session = Session::new(root.clone(), layout());
        session.show_hidden_folders = Some(true);
        session.save(&path).unwrap();
        let restored = Session::load(&path, &root).unwrap().unwrap();
        assert_eq!(restored.show_hidden_folders, Some(true));
        let mut old: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        old.as_object_mut().unwrap().remove("show_hidden_folders");
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let restored = Session::load(&path, &root).unwrap().unwrap();
        assert_eq!(restored.show_hidden_folders, None);
    }

    #[test]
    fn round_trip_and_workspace_validation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("state.json");
        let mut session = Session::new(root.clone(), layout());
        session.active = Some("Note.md".into());
        session.buffers.push(BufferState {
            path: "Note.md".into(),
            row: 8,
            col: 4,
            scroll_x: 2,
            scroll_y: 3,
        });
        assert!(Session::load(&path, &root).unwrap().is_none());
        session.save(&path).unwrap();
        let restored = Session::load(&path, &root).unwrap().unwrap();
        assert_eq!(restored.active, session.active);
        assert_eq!(restored.buffers[0].row, 8);
        assert_eq!(restored.buffers[0].scroll_x, 2);
        assert!(Session::load(&path, &root.join("other")).is_err());
        assert_ne!(workspace_id(&root), workspace_id(&root.join("other")));
        fs::write(&path, "invalid json").unwrap();
        assert!(Session::load(&path, &root).is_err());
    }

    #[test]
    fn rejects_traversal_and_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::write(root.join("Note.md"), "test").unwrap();
        assert_eq!(resolve(&root, Path::new("")), Some(root.clone()));
        assert_eq!(
            resolve(&root, Path::new("Note.md")),
            Some(root.join("Note.md"))
        );
        assert!(resolve(&root, Path::new("../outside")).is_none());
        assert!(resolve(&root, &root.join("Note.md")).is_none());
        assert!(resolve(&root, Path::new("missing")).is_none());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("Note.md"), root.join("link.md")).unwrap();
            assert!(resolve(&root, Path::new("link.md")).is_none());
        }
    }
}

//! Capture and restore workspace state, never unsaved document content.
use super::App;
use crate::{
    layout,
    session::{self, BufferState, LayoutState, Session},
};
use edtui::Index2;
use std::{io, path::Path};

impl App {
    fn capture_session(&self) -> Session {
        let relative = |path: &Path| path.strip_prefix(&self.root).ok().map(Path::to_path_buf);
        let mut session = Session::new(
            self.root.clone(),
            LayoutState {
                browser_width_percent: self.browser_width_percent,
                outline_width_percent: self.panels.outline_width_percent,
                show_files: self.panels.show_files,
                show_preview: self.panels.show_preview,
                show_outline: self.panels.show_outline,
                show_tab_bar: self.show_tab_bar,
            },
        );
        session.show_hidden_folders = Some(self.show_hidden_folders);
        session.directory = relative(&self.current_dir).unwrap_or_default();
        session.selection = self
            .entries
            .get(self.selected)
            .and_then(|entry| relative(&entry.path));
        session.active = self.editor.as_ref().and_then(|note| relative(&note.path));
        session.buffers = self
            .editor
            .notes()
            .iter()
            .filter_map(|note| {
                let (scroll_x, scroll_y) = note.state.viewport_offset();
                Some(BufferState {
                    path: relative(&note.path)?,
                    row: note.state.cursor.row,
                    col: note.state.cursor.col,
                    scroll_x,
                    scroll_y,
                })
            })
            .collect();
        session
    }

    pub(super) fn restore_session(&mut self) {
        if !self.session_settings.enabled || !self.session_settings.restore_on_startup {
            return;
        }
        let Some(path) = session::state_path(&self.root) else {
            return;
        };
        match Session::load(&path, &self.root) {
            Ok(Some(session)) => self.apply_session(session),
            Ok(None) => {}
            Err(error) => self.message = format!("Session ignored: {error}"),
        }
    }

    fn apply_session(&mut self, session: Session) {
        if self.session_settings.restore_layout {
            let state = session.layout;
            self.browser_width_percent = state
                .browser_width_percent
                .clamp(layout::MIN_BROWSER_PERCENT, layout::MAX_BROWSER_PERCENT);
            self.panels.outline_width_percent = state
                .outline_width_percent
                .clamp(layout::MIN_OUTLINE_PERCENT, layout::MAX_OUTLINE_PERCENT);
            self.panels.show_files = state.show_files;
            self.panels.show_preview = state.show_preview;
            self.panels.show_outline = state.show_outline;
            self.show_tab_bar = state.show_tab_bar;
            if let Some(show) = session.show_hidden_folders {
                self.show_hidden_folders = show;
            }
        }
        if let Some(directory) =
            session::resolve(&self.root, &session.directory).filter(|p| p.is_dir())
        {
            let old = std::mem::replace(&mut self.current_dir, directory);
            if self.refresh().is_err() {
                self.current_dir = old;
                // Reapply the restored filter to the previous directory if possible.
                let _ = self.refresh();
            }
        } else {
            let _ = self.refresh();
        }
        if let Some(selection) = session
            .selection
            .as_deref()
            .and_then(|p| session::resolve(&self.root, p))
        {
            self.select_path(&selection);
        }
        let mut skipped = 0;
        for buffer in session.buffers {
            let Some(path) = session::resolve(&self.root, &buffer.path) else {
                skipped += 1;
                continue;
            };
            match self.editor.open(&path) {
                Ok(true) => {
                    if self.session_settings.restore_cursor
                        && let Some(note) = self.editor.as_mut()
                    {
                        // Files may have changed since the session was written.
                        let text = note.state.lines.to_string();
                        let row = buffer.row.min(text.split('\n').count().saturating_sub(1));
                        let col = buffer.col.min(
                            text.split('\n')
                                .nth(row)
                                .unwrap_or("")
                                .chars()
                                .count()
                                .saturating_sub(1),
                        );
                        note.state.cursor = Index2::new(row, col);
                        note.state.set_viewport_offset(
                            buffer.scroll_x.min(col),
                            buffer.scroll_y.min(row),
                        );
                    }
                }
                Ok(false) => {}
                Err(_) => skipped += 1,
            }
        }
        if let Some(active) = session.active {
            let index = session::resolve(&self.root, &active).and_then(|path| {
                self.editor
                    .notes()
                    .iter()
                    .position(|note| note.path == path)
            });
            if let Some(index) = index {
                self.editor.activate(index);
            } else if self.editor.len() > 0 {
                self.editor.activate(0);
            }
        } else {
            self.editor.hide();
        }
        self.reset_buffer_context();
        self.message = format!(
            "Session restored: {} open buffer(s), {skipped} skipped.",
            self.editor.len()
        );
    }

    pub(super) fn persist_session(&self) -> io::Result<()> {
        if !self.session_settings.enabled {
            return Ok(());
        }
        if self.editor.any_dirty() {
            return Err(io::Error::other(
                "Refusing to persist a session with unsaved buffers",
            ));
        }
        let path = session::state_path(&self.root)
            .ok_or_else(|| io::Error::other("No XDG state directory or HOME"))?;
        self.capture_session().save(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Appearance, AutoThemes, ColorOverrides, ThemeChoice};
    use std::{fs, path::PathBuf};

    #[test]
    fn buffer_save_quit_and_hidden_folder_session_round_trip() {
        use crate::commands::Command;
        use crate::search::{SearchMode, VaultIndex};
        use edtui::Lines;

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let a = root.join("Visible.md");
        let hidden = root.join(".archive");
        let b = hidden.join("Archived.md");
        fs::create_dir(&hidden).unwrap();
        fs::write(&a, "first\nsecond\nthird").unwrap();
        fs::write(&b, "archive-only-token").unwrap();

        let mut first = app(root.clone());
        assert!(!first.show_hidden_folders);
        first.select_path(&a);
        first.enter().unwrap();
        let note = first.editor.as_mut().unwrap();
        note.state.lines = Lines::new(vec![
            "first".chars().collect(),
            "edited-visible-token".chars().collect(),
            "third".chars().collect(),
        ]);
        note.state.cursor = Index2::new(1, 3);
        note.state.set_viewport_offset(1, 1);
        assert!(note.is_dirty());

        assert!(!first.execute_command(Command::Hide).unwrap());
        assert!(!first.execute_command(Command::ToggleHiddenFolders).unwrap());
        first.select_path(&hidden);
        first.enter().unwrap();
        first.select_path(&b);
        first.enter().unwrap();
        assert_eq!(first.editor.len(), 2);
        assert!(!first.execute_command(Command::ToggleHiddenFolders).unwrap());
        assert_eq!(first.current_dir, root);
        assert_eq!(first.editor.as_ref().unwrap().path, b);

        // A dirty inactive note must block application exit.
        assert!(!first.execute_command(Command::QuitAll).unwrap());
        assert!(!first.execute_command(Command::CloseOtherBuffers).unwrap());
        assert_eq!(first.editor.len(), 2);
        assert!(first.editor.any_dirty());
        assert_eq!(fs::read_to_string(&a).unwrap(), "first\nsecond\nthird");

        // Save-all-and-quit leaves the open set intact for session capture.
        assert!(first.execute_command(Command::SaveQuitAll).unwrap());
        assert!(!first.editor.any_dirty());
        assert_eq!(first.editor.len(), 2);
        let state_dir = tempfile::tempdir().unwrap();
        let state_file = state_dir.path().join("session.json");
        first.capture_session().save(&state_file).unwrap();
        let json = fs::read_to_string(&state_file).unwrap();
        assert!(!json.contains("edited-visible-token"));
        assert!(!json.contains("archive-only-token"));

        let mut second = app(root.clone());
        second.apply_session(Session::load(&state_file, &root).unwrap().unwrap());
        assert_eq!(second.editor.len(), 2);
        assert_eq!(second.editor.notes()[0].path, a);
        assert_eq!(second.editor.as_ref().unwrap().path, b);
        assert!(!second.show_hidden_folders);
        assert!(!second.entries.iter().any(|entry| entry.path == hidden));
        assert!(!second.execute_command(Command::PreviousBuffer).unwrap());
        let restored = second.editor.as_ref().unwrap();
        assert_eq!(
            restored.state.lines.to_string(),
            "first\nedited-visible-token\nthird"
        );
        assert_eq!(restored.state.cursor, Index2::new(1, 3));
        assert_eq!(restored.state.viewport_offset(), (1, 1));
        assert!(!restored.is_dirty());

        assert!(
            !second
                .execute_command(Command::ToggleHiddenFolders)
                .unwrap()
        );
        let index = VaultIndex::load(&root).unwrap();
        assert_eq!(
            index
                .search(SearchMode::Text, "edited-visible-token", 10)
                .len(),
            1
        );
        assert!(
            index
                .search(SearchMode::Text, "archive-only-token", 10)
                .is_empty()
        );
    }

    fn app(root: PathBuf) -> App {
        App::new(
            root,
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap()
    }

    #[test]
    fn restores_order_active_cursor_selection_and_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let folder = root.join("Research");
        fs::create_dir(&folder).unwrap();
        let a = folder.join("a.md");
        let b = folder.join("b.md");
        fs::write(&a, "first\nsecond\nthird").unwrap();
        fs::write(&b, "other").unwrap();
        let mut first = app(root.clone());
        first.current_dir = folder.clone();
        first.refresh().unwrap();
        first.select_path(&b);
        first.editor.open(&a).unwrap();
        first.editor.as_mut().unwrap().state.cursor = Index2::new(2, 3);
        first
            .editor
            .as_mut()
            .unwrap()
            .state
            .set_viewport_offset(1, 1);
        first.editor.open(&b).unwrap();
        first.editor.activate(0);
        first.browser_width_percent = 50;
        first.panels.show_outline = true;
        first.show_tab_bar = false;
        let file = root.join("state.json");
        first.capture_session().save(&file).unwrap();
        let mut second = app(root.clone());
        second.apply_session(Session::load(&file, &root).unwrap().unwrap());
        assert_eq!(second.editor.len(), 2);
        assert_eq!(second.editor.notes()[0].path, a);
        assert_eq!(second.editor.notes()[1].path, b);
        assert_eq!(second.editor.active_index(), Some(0));
        assert_eq!(
            second.editor.as_ref().unwrap().state.cursor,
            Index2::new(2, 3)
        );
        assert_eq!(
            second.editor.as_ref().unwrap().state.viewport_offset(),
            (1, 1)
        );
        assert_eq!(second.current_dir, folder);
        assert_eq!(second.entries[second.selected].path, b);
        assert_eq!(second.browser_width_percent, 50);
        assert!(second.panels.show_outline);
        assert!(!second.show_tab_bar);
        // An explicit browser-mode exit must not activate a hidden buffer.
        second.editor.hide();
        let mut third = app(root.clone());
        third.apply_session(second.capture_session());
        assert!(third.editor.is_none());
        assert_eq!(third.editor.len(), 2);
        // Shortened and deleted files should not invalidate the whole session.
        second.editor.activate(0);
        let session = second.capture_session();
        fs::write(&a, "x").unwrap();
        fs::remove_file(&b).unwrap();
        let mut fourth = app(root);
        fourth.apply_session(session);
        assert_eq!(fourth.editor.len(), 1);
        assert_eq!(
            fourth.editor.as_ref().unwrap().state.cursor,
            Index2::new(0, 0)
        );
        assert_eq!(
            fourth.editor.as_ref().unwrap().state.viewport_offset(),
            (0, 0)
        );
    }

    #[test]
    fn restores_hidden_folder_preference_before_browser_selection() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let hidden = root.join(".archive");
        fs::create_dir(&hidden).unwrap();
        let note = hidden.join("Note.md");
        fs::write(&note, "archive").unwrap();
        let mut first = app(root.clone());
        first.toggle_hidden_folders();
        first.select_path(&hidden);
        first.enter().unwrap();
        first.select_path(&note);
        let mut second = app(root.clone());
        second.apply_session(first.capture_session());
        assert!(second.show_hidden_folders);
        assert_eq!(second.current_dir, hidden);
        assert_eq!(second.entries[second.selected].path, note);

        let mut third = app(root.clone());
        third.session_settings.restore_layout = false;
        third.apply_session(first.capture_session());
        assert!(!third.show_hidden_folders);
        assert_eq!(third.current_dir, root);
        assert!(!third.entries.iter().any(|entry| entry.path == hidden));

        // Older sessions without this field respect the configured default.
        let mut older = first.capture_session();
        older.show_hidden_folders = None;
        third.session_settings.restore_layout = true;
        third.apply_session(older);
        assert!(!third.show_hidden_folders);
    }

    #[test]
    fn restore_preferences_can_be_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("Note.md");
        fs::write(&path, "abc\ndef").unwrap();
        let mut first = app(root.clone());
        first.editor.open(&path).unwrap();
        first.editor.as_mut().unwrap().state.cursor = Index2::new(1, 1);
        first.browser_width_percent = 50;
        first.show_tab_bar = false;
        let mut second = app(root);
        second.session_settings.restore_cursor = false;
        second.session_settings.restore_layout = false;
        second.apply_session(first.capture_session());
        assert_eq!(
            second.editor.as_ref().unwrap().state.cursor,
            Index2::new(0, 0)
        );
        assert_eq!(
            second.browser_width_percent,
            layout::DEFAULT_BROWSER_PERCENT
        );
        assert!(second.show_tab_bar);
    }
}

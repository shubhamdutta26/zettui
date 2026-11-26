//! Terminal lifecycle, cursor shape and keyboard event routing.

use super::App;
use crate::commands::{Command, KeyScope};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use edtui::EditorMode;
use ratatui::DefaultTerminal;
use std::{io, time::Duration};

impl App {
    pub(super) fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let result = self.run_loop(terminal);
        // Restore the cursor configured by the user in Foot/other terminals.
        let reset = crossterm::execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        match result {
            Err(error) => Err(error),
            Ok(()) => reset,
        }
    }

    fn run_loop(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut previous_insert_mode = None;
        loop {
            if let Some(tracker) = &mut self.statistics {
                tracker.flush_if_due();
            }
            let in_insert = self
                .editor
                .as_ref()
                .is_some_and(|note| note.state.mode == EditorMode::Insert);
            if previous_insert_mode != Some(in_insert) {
                let shape = if in_insert {
                    SetCursorStyle::BlinkingBar
                } else {
                    SetCursorStyle::BlinkingBlock
                };
                crossterm::execute!(io::stdout(), shape)?;
                previous_insert_mode = Some(in_insert);
            }
            self.sync_system_theme();
            terminal.draw(|frame| self.draw(frame))?;
            // Poll instead of waiting forever: The desktop may change its theme
            // while the user isn't pressing any keys.
            if !event::poll(Duration::from_millis(400))? {
                continue;
            }
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if self.route_key(key)? {
                return Ok(());
            }
        }
    }

    /// Observe the result of a routed key, including modal insertions and Vim
    /// commands. Never compare the contents of two different buffers.
    fn route_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        let before = if key.kind == KeyEventKind::Press
            && self.statistics.is_some()
            && self.analytics_dashboard.is_none()
        {
            self.editor
                .as_ref()
                .map(|note| (note.path.clone(), note.state.lines.to_string()))
        } else {
            None
        };
        let result = self.dispatch_key(key);
        if let Some((path, before)) = before
            && let Some(note) = self.editor.as_ref().filter(|note| note.path == path)
            && let Some(tracker) = &mut self.statistics
            && let Err(error) = tracker.record_edit(
                chrono::Local::now().date_naive(),
                &before,
                &note.state.lines.to_string(),
            )
        {
            self.message = format!("Writing statistics edit not counted: {error}");
        }
        result
    }

    /// Route one terminal key; true requests application exit.
    fn dispatch_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if self.analytics_dashboard.is_some() {
            self.handle_analytics_key(key);
            return Ok(false);
        }
        if self.snippet_form.is_some() {
            self.handle_snippet_form_key(key);
            return Ok(false);
        }
        if self.snippet_picker.is_some() {
            self.handle_snippet_picker_key(key);
            return Ok(false);
        }
        if self.links.is_some() {
            self.handle_link_key(key);
            return Ok(false);
        }
        if self.citation_picker.is_some() {
            self.handle_citation_key(key);
            return Ok(false);
        }
        if self.wiki_completion.is_some() {
            self.handle_wiki_completion_key(key);
            return Ok(false);
        }
        if self.prompt.is_some() {
            self.handle_prompt_key(key);
            return Ok(false);
        }
        if self.find_dialog.is_some() {
            self.handle_find_key(key);
            return Ok(false);
        }
        if self.vault_search.is_some() {
            self.handle_vault_search_key(key);
            return Ok(false);
        }
        if self.heading_search.is_some() {
            self.handle_heading_search_key(key);
            return Ok(false);
        }
        if self.buffer_picker.is_some() {
            self.handle_buffer_picker_key(key);
            return Ok(false);
        }
        if self.command_palette.is_some() {
            return self.handle_command_palette_key(key);
        }
        if self.note_search.input.is_some() {
            self.handle_note_search_key(key);
            return Ok(false);
        }
        // Ex input owns its keys just like the other modal inputs.
        if self.command.is_some() {
            self.handle_editor_key(key);
            return Ok(std::mem::take(&mut self.quit_requested));
        }
        if let Some(command) = Command::from_key(KeyScope::Global, key) {
            self.writing.leader = false;
            self.buffer_prefix = None;
            self.pending_open_bracket = false;
            return self.execute_command(command);
        }
        if self.outline_view.focused {
            self.handle_outline_key(key);
            return Ok(false);
        }
        if self.editor.is_some() {
            self.handle_editor_key(key);
            return Ok(std::mem::take(&mut self.quit_requested));
        }
        if let Some(command) = Command::from_key(KeyScope::Browser, key) {
            return self.execute_command(command);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Appearance, AutoThemes, ColorOverrides, ThemeChoice};
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::fs;

    fn fixture() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Navigation.md");
        fs::write(&path, "# First\nbody\n## Second\n").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap();
        app.select_path(&path);
        (dir, app)
    }

    fn press(app: &mut App, code: KeyCode) {
        assert!(
            !app.route_key(KeyEvent::new(code, KeyModifiers::NONE))
                .unwrap()
        );
    }

    fn type_text(app: &mut App, text: &str) {
        for ch in text.chars() {
            press(app, KeyCode::Char(ch));
        }
    }

    #[test]
    fn modified_browser_keys_do_not_open_prompts_or_quit() {
        let (dir, mut app) = fixture();
        for (code, modifiers) in [
            ('d', KeyModifiers::CONTROL),
            ('m', KeyModifiers::ALT),
            ('r', KeyModifiers::CONTROL),
            ('n', KeyModifiers::CONTROL),
            ('q', KeyModifiers::ALT),
        ] {
            assert!(
                !app.route_key(KeyEvent::new(KeyCode::Char(code), modifiers))
                    .unwrap()
            );
            assert!(!app.has_active_popup());
        }
        assert!(app.editor.is_none());
        assert!(dir.path().join("Navigation.md").exists());
        press(&mut app, KeyCode::Char('d'));
        assert!(app.prompt.is_some());
        press(&mut app, KeyCode::Esc);
        assert!(!app.has_active_popup());
        assert!(dir.path().join("Navigation.md").exists());
    }

    #[test]
    fn insert_text_does_not_trigger_normal_mode_commands() {
        let (dir, mut app) = fixture();
        let path = dir.path().join("Empty.md");
        fs::write(&path, "").unwrap();
        app.editor.open(&path).unwrap();
        press(&mut app, KeyCode::Char('i'));
        let literal = ":w /?nN ]b [b bc";
        type_text(&mut app, literal);
        for code in [KeyCode::F(5), KeyCode::F(6), KeyCode::F(7), KeyCode::F(8)] {
            press(&mut app, code);
        }
        assert!(!app.has_active_popup());
        assert!(app.command.is_none());
        assert!(app.note_search.input.is_none());
        assert!(!app.outline_view.focused);
        let note = app.editor.as_ref().unwrap();
        assert_eq!(note.state.mode, EditorMode::Insert);
        assert_eq!(note.state.lines.to_string(), literal);
        assert!(note.is_dirty());
        press(&mut app, KeyCode::Esc);
        type_text(&mut app, ":w");
        press(&mut app, KeyCode::Enter);
        assert_eq!(fs::read_to_string(path).unwrap(), literal);
    }

    #[test]
    fn note_search_owns_keys_until_cancelled() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        let panels = app.panels;
        let original = app.editor.as_ref().unwrap().state.lines.to_string();
        type_text(&mut app, "/Second");
        assert!(app.note_search.input.is_some());
        for code in [KeyCode::F(2), KeyCode::F(5), KeyCode::F(8), KeyCode::F(10)] {
            press(&mut app, code);
        }
        assert_eq!(app.panels, panels);
        assert!(!app.has_active_popup());
        assert!(app.note_search.input.is_some());
        press(&mut app, KeyCode::Esc);
        assert!(app.note_search.input.is_none());
        assert_eq!(
            app.editor.as_ref().unwrap().state.lines.to_string(),
            original
        );
        press(&mut app, KeyCode::F(8));
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn statistics_track_edits_not_navigation_search_or_save() {
        let (dir, mut app) = fixture();
        let state = dir.path().join("statistics.json");
        let note = dir.path().join("Empty.md");
        fs::write(&note, "").unwrap();
        app.statistics = Some(zettui::writing_activity::Tracker::new(Some(state.clone())));
        press(&mut app, KeyCode::Enter);
        app.editor.open(&note).unwrap();
        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "hello world");
        press(&mut app, KeyCode::Backspace);
        press(&mut app, KeyCode::Esc);
        type_text(&mut app, ":w");
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, "/hello");
        press(&mut app, KeyCode::Esc);
        type_text(&mut app, "[b");
        type_text(&mut app, "]b");
        assert_eq!(app.editor.as_ref().unwrap().path, note);
        assert_eq!(
            app.editor.as_ref().unwrap().state.lines.to_string(),
            "hello worl"
        );
        app.statistics.as_mut().unwrap().flush().unwrap();
        let totals = zettui::statistics::Store::new(state)
            .load()
            .unwrap()
            .daily_totals()
            .unwrap();
        // Sum dates in case the test happens to cross local midnight.
        assert_eq!(totals.values().map(|a| a.words_added).sum::<u64>(), 2);
        assert_eq!(totals.values().map(|a| a.characters_added).sum::<u64>(), 11);
        assert_eq!(
            totals.values().map(|a| a.characters_deleted).sum::<u64>(),
            1
        );
        assert_eq!(totals.values().map(|a| a.words_deleted).sum::<u64>(), 0);
    }

    #[test]
    fn analytics_keys_are_modal_and_do_not_count_as_edits() {
        let (dir, mut app) = fixture();
        let state = dir.path().join("statistics.json");
        app.statistics = Some(zettui::writing_activity::Tracker::new(Some(state.clone())));
        press(&mut app, KeyCode::F(10));
        assert!(app.analytics_dashboard.is_some());
        press(&mut app, KeyCode::Char('q'));
        assert!(app.analytics_dashboard.is_none());
        press(&mut app, KeyCode::Enter);
        let before = app.editor.as_ref().unwrap().state.lines.to_string();
        type_text(&mut app, ":analytics");
        press(&mut app, KeyCode::Enter);
        assert!(app.analytics_dashboard.is_some());
        type_text(&mut app, "2hjkl3r");
        press(&mut app, KeyCode::Esc);
        assert!(app.analytics_dashboard.is_none());
        assert_eq!(app.editor.as_ref().unwrap().state.lines.to_string(), before);
        app.statistics.as_mut().unwrap().flush().unwrap();
        assert!(!state.exists());
    }

    #[test]
    fn hidden_folder_toggle_preserves_selection_and_dirty_buffers() {
        let (dir, mut app) = fixture();
        let root = dir.path().canonicalize().unwrap();
        let visible = root.join("Navigation.md");
        let folder = root.join(".archive");
        let nested = folder.join("nested");
        let hidden_note = nested.join("Note.md");
        fs::create_dir_all(&nested).unwrap();
        fs::write(&hidden_note, "unique secret phrase").unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join(".settings"), "hidden file").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&folder, root.join("shortcut")).unwrap();
        app.refresh().unwrap();
        app.select_path(&visible);
        assert!(!app.show_hidden_folders);
        assert!(!app.entries.iter().any(|entry| entry.path == folder));
        assert!(app.entries.iter().any(|entry| entry.name == ".settings"));
        press(&mut app, KeyCode::F(9));
        assert!(app.show_hidden_folders);
        assert!(app.entries.iter().any(|entry| entry.path == folder));
        assert!(
            !app.entries
                .iter()
                .any(|entry| entry.name == ".git" || entry.name == "shortcut")
        );
        assert_eq!(app.entries[app.selected].path, visible);
        let index = crate::search::VaultIndex::load(&root).unwrap();
        assert!(
            index
                .search(crate::search::SearchMode::Text, "unique secret phrase", 10)
                .is_empty()
        );
        app.select_path(&folder);
        press(&mut app, KeyCode::Enter);
        app.select_path(&nested);
        press(&mut app, KeyCode::Enter);
        app.select_path(&hidden_note);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "X");
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::F(9));
        assert!(!app.show_hidden_folders);
        assert_eq!(app.current_dir, root);
        assert!(!app.entries.iter().any(|entry| entry.path == folder));
        assert_eq!(app.editor.as_ref().unwrap().path, hidden_note);
        assert!(app.editor.as_ref().unwrap().is_dirty());
        assert_eq!(app.editor.len(), 1);
    }

    #[test]
    fn quit_all_refuses_dirty_inactive_buffers_and_save_quit_preserves_open_set() {
        let (dir, mut app) = fixture();
        let a = dir.path().join("Alpha.md");
        let b = dir.path().join("Beta.md");
        fs::write(&a, "Alpha").unwrap();
        fs::write(&b, "Beta").unwrap();
        app.editor.open(&a).unwrap();
        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "X");
        press(&mut app, KeyCode::Esc);
        app.editor.open(&b).unwrap();
        type_text(&mut app, ":qa");
        press(&mut app, KeyCode::Enter);
        assert!(app.editor.any_dirty());
        assert_eq!(app.editor.len(), 2);
        // A conflicting external edit must block save-and-quit as well.
        fs::write(&a, "External").unwrap();
        type_text(&mut app, ":wqa");
        press(&mut app, KeyCode::Enter);
        assert_eq!(fs::read_to_string(&a).unwrap(), "External");
        assert!(app.editor.any_dirty());
        // Once the conflict is resolved, the same command may exit safely.
        fs::write(&a, "Alpha").unwrap();
        type_text(&mut app, ":wqa");
        assert!(
            app.route_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .unwrap()
        );
        assert!(!app.editor.any_dirty());
        assert_eq!(app.editor.len(), 2);
        assert_eq!(app.editor.as_ref().unwrap().path, b);
        assert!(fs::read_to_string(&a).unwrap().contains('X'));
    }

    #[test]
    fn buffer_shortcuts_and_picker_preserve_dirty_notes() {
        let (dir, mut app) = fixture();
        let a = dir.path().join("Alpha.md");
        let b = dir.path().join("Beta.md");
        fs::write(&a, "Alpha").unwrap();
        fs::write(&b, "Beta").unwrap();
        app.editor.open(&a).unwrap();
        app.editor.open(&b).unwrap();
        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "]b");
        press(&mut app, KeyCode::Esc);
        assert!(app.editor.as_ref().unwrap().is_dirty());
        assert_eq!(app.editor.as_ref().unwrap().path, b);
        type_text(&mut app, "[b");
        assert_eq!(app.editor.as_ref().unwrap().path, a);
        type_text(&mut app, " bo");
        assert_eq!(app.editor.len(), 2);
        press(&mut app, KeyCode::F(5));
        type_text(&mut app, "beta");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.as_ref().unwrap().path, b);
        assert!(app.editor.as_ref().unwrap().is_dirty());
        type_text(&mut app, " bp");
        assert_eq!(app.editor.as_ref().unwrap().path, a);
        type_text(&mut app, " bs");
        assert!(!app.editor.any_dirty());
        type_text(&mut app, " bd");
        assert_eq!(app.editor.len(), 1);
        assert_eq!(app.editor.as_ref().unwrap().path, b);
    }

    #[test]
    fn hidden_dirty_buffers_block_quit_and_file_operations() {
        let (dir, mut app) = fixture();
        let first = dir.path().join("Navigation.md");
        let second = dir.path().join("Second.md");
        fs::write(&second, "Second note").unwrap();
        app.refresh().unwrap();
        app.select_path(&first);
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, "iX");
        press(&mut app, KeyCode::Esc);
        let before = app.editor.as_ref().unwrap().state.lines.to_string();
        let cursor = app.editor.as_ref().unwrap().state.cursor;
        type_text(&mut app, ":hide");
        press(&mut app, KeyCode::Enter);
        assert!(app.editor.is_none());
        assert_eq!(app.editor.len(), 1);
        press(&mut app, KeyCode::Char('q'));
        assert!(app.message.contains("Unsaved buffers"));
        for code in ['r', 'm', 'd'] {
            press(&mut app, KeyCode::Char(code));
            assert!(app.prompt.is_none());
            assert!(first.exists());
        }
        app.select_path(&second);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.len(), 2);
        type_text(&mut app, ":q");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.len(), 1);
        app.select_path(&first);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.as_ref().unwrap().state.lines.to_string(), before);
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, cursor);
        assert!(app.editor.as_ref().unwrap().is_dirty());
        type_text(&mut app, ":wq");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.len(), 0);
        assert_eq!(fs::read_to_string(&first).unwrap(), before);
        assert!(
            app.route_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
                .unwrap()
        );
    }

    #[test]
    fn ex_bar_owns_global_shortcuts_until_confirmed() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, ":headings");
        let panels = app.panels;
        for key in [KeyCode::F(2), KeyCode::F(4), KeyCode::F(8)] {
            press(&mut app, key);
        }
        assert!(
            !app.route_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL))
                .unwrap()
        );
        assert_eq!(app.command.as_deref(), Some("headings"));
        assert_eq!(app.panels, panels);
        assert!(!app.has_active_popup());
        press(&mut app, KeyCode::Enter);
        assert!(app.heading_search.is_some());
        assert!(app.command.is_none());
        press(&mut app, KeyCode::Esc);
        type_text(&mut app, ":commands");
        press(&mut app, KeyCode::Enter);
        assert!(app.command_palette.is_some());
    }

    #[test]
    fn global_shortcuts_cancel_pending_leader() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char(' '));
        assert!(app.writing.leader);
        assert!(
            !app.route_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT))
                .unwrap()
        );
        assert!(!app.writing.leader);
        press(&mut app, KeyCode::Char('i'));
        assert_eq!(app.editor.as_ref().unwrap().state.mode, EditorMode::Insert);
    }

    #[test]
    fn palette_routes_to_heading_picker_without_leaking_keys() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::F(8));
        type_text(&mut app, "search document headings");
        press(&mut app, KeyCode::Enter);
        assert!(app.command_palette.is_none());
        assert!(app.heading_search.is_some());
        let panels = app.panels;
        press(&mut app, KeyCode::F(4));
        press(&mut app, KeyCode::F(8));
        assert_eq!(app.panels, panels);
        assert!(app.command_palette.is_none());
        type_text(&mut app, "scnd");
        press(&mut app, KeyCode::Enter);
        let note = app.editor.as_ref().unwrap();
        assert_eq!(note.state.cursor.row, 2);
        assert!(!note.is_dirty());
        assert!(!app.has_active_popup());
    }

    #[test]
    fn palette_save_and_close_preserves_external_changes_and_dirty_buffer() {
        let (dir, mut app) = fixture();
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, "iX");
        press(&mut app, KeyCode::Esc);
        let before = app.editor.as_ref().unwrap().state.lines.to_string();
        let path = dir.path().join("Navigation.md");
        fs::write(&path, "# External change\n").unwrap();
        press(&mut app, KeyCode::F(8));
        type_text(&mut app, "save and close note");
        press(&mut app, KeyCode::Enter);
        let note = app.editor.as_ref().unwrap();
        assert!(note.is_dirty());
        assert_eq!(note.state.lines.to_string(), before);
        assert_eq!(fs::read_to_string(path).unwrap(), "# External change\n");
        assert!(app.message.contains("SAVE FAILED"));
    }

    #[test]
    fn browser_palette_quit_signal_reaches_event_loop() {
        let (_dir, mut app) = fixture();
        press(&mut app, KeyCode::F(8));
        type_text(&mut app, "quit zettui");
        assert!(
            app.route_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .unwrap()
        );
        assert!(!app.has_active_popup());
    }
}

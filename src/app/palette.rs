//! Global command palette input and context checks.

use super::App;
use crate::{
    command_palette::CommandPalette,
    commands::{Command, KeyScope},
    markdown,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::EditorMode;
use std::io;

impl App {
    fn palette_available(&self, command: Command) -> bool {
        use Command::*;
        let note = self.editor.as_ref();
        let editing = note.is_some();
        let selected = self.entries.get(self.selected);
        match command {
            QuitAll | SaveQuitAll | WritingAnalytics => true,
            SearchText | SearchTags | ToggleFiles | TogglePreview | ToggleOutline
            | NarrowBrowser | WidenBrowser | NarrowOutline | WidenOutline | CycleTheme => true,
            SearchHeadings => note.is_some_and(|note| markdown::is_markdown(&note.path)),
            ExportDocument => note.map_or_else(
                || selected.is_none_or(|entry| entry.is_dir || markdown::is_markdown(&entry.path)),
                |note| markdown::is_markdown(&note.path),
            ),
            ToggleMetadata => note.is_some_and(|note| markdown::is_markdown(&note.path)),
            FocusMetadata => false, // legacy alias; avoid a duplicate palette entry
            FocusOutline => {
                note.is_some_and(|note| markdown::is_markdown(&note.path))
                    && !self.writing.zen.enabled
            }
            Save | Hide | Close | SaveAndClose | Writing(_) | CloseBuffer | CloseOtherBuffers => {
                editing
            }
            BufferPicker | NextBuffer | PreviousBuffer | AlternateBuffer | SaveAll => {
                self.editor.len() > 0
            }
            ToggleTabs | ToggleHiddenFolders => true,
            CitationPicker | CitationPreview | CitationCheck => {
                editing && self.bibliography_path.is_some()
            }
            SnippetPicker => note.is_some_and(|note| note.state.lines.to_string().is_empty()),
            OutgoingLinks | Backlinks => editing || selected.is_some_and(|entry| !entry.is_dir),
            PreviousNote => !self.history.is_empty(),
            FindFilename | NewNote | NewFolder | Refresh | Quit => !editing,
            OpenSelection | Rename | Move | Trash => !editing && selected.is_some(),
            ParentFolder => !editing && self.current_dir != self.root,
            // Motion commands and explicit forced discard remain keyboard/Ex only.
            MoveDown | MoveUp | EnterFolder | DiscardAndClose | OpenPalette => false,
        }
    }

    pub(super) fn open_command_palette(&mut self) {
        if self.has_active_popup() || self.command.is_some() || self.note_search.input.is_some() {
            return;
        }
        if self
            .editor
            .as_ref()
            .is_some_and(|note| note.state.mode != EditorMode::Normal)
        {
            self.message = "Switch to Normal mode to open the command palette.".into();
            return;
        }
        self.command_palette = Some(CommandPalette::new(|command| {
            self.palette_available(command)
        }));
        self.writing.leader = false;
        self.pending_open_bracket = false;
        self.message.clear();
    }

    /// Returns the same quit signal as the shared command dispatcher.
    pub(super) fn handle_command_palette_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        let Some(mut palette) = self.command_palette.take() else {
            return Ok(false);
        };
        if Command::from_key(KeyScope::Global, key) == Some(Command::OpenPalette) {
            return Ok(false);
        }
        let ctrl = key.modifiers == KeyModifiers::CONTROL;
        match key.code {
            KeyCode::Esc if key.modifiers.is_empty() => return Ok(false),
            KeyCode::Enter if key.modifiers.is_empty() => {
                if let Some(command) = palette.selected_command() {
                    if !self.palette_available(command) {
                        self.message = "Command is no longer available in this context.".into();
                        return Ok(false);
                    }
                    // A canceled palette preserves Outline focus. Executing a
                    // command uses editor context, except for the focus toggle itself.
                    if !matches!(command, Command::FocusOutline | Command::WritingAnalytics) {
                        self.outline_view.focused = false;
                    }
                    return self.execute_command(command);
                }
            }
            KeyCode::Up => palette.move_selection(false),
            KeyCode::Down => palette.move_selection(true),
            KeyCode::Char('p') if ctrl => palette.move_selection(false),
            KeyCode::Char('n') if ctrl => palette.move_selection(true),
            KeyCode::Home => palette.selected = 0,
            KeyCode::End => palette.selected = palette.matches.len().saturating_sub(1),
            KeyCode::Backspace => {
                palette.query.pop();
                palette.filter();
            }
            KeyCode::Char('u') if ctrl => {
                palette.query.clear();
                palette.filter();
            }
            KeyCode::Char(ch)
                if !ch.is_control()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                palette.query.push(ch);
                palette.filter();
            }
            _ => {}
        }
        self.command_palette = Some(palette);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PromptKind;
    use crate::theme::{Appearance, AutoThemes, ColorOverrides, ThemeChoice};
    use std::fs;

    fn fixture() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Note.md");
        fs::write(&path, "# Heading\nbody\n").unwrap();
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

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn select(app: &mut App, command: Command) {
        let palette = app.command_palette.as_mut().unwrap();
        palette.selected = palette
            .matches
            .iter()
            .position(|&i| palette.items[i].command == command)
            .unwrap();
    }

    #[test]
    fn browser_and_editor_expose_only_safe_context_actions() {
        let (_dir, mut app) = fixture();
        app.open_command_palette();
        let palette = app.command_palette.as_ref().unwrap();
        assert!(
            palette
                .items
                .iter()
                .any(|item| item.command == Command::Trash)
        );
        assert!(
            !palette
                .items
                .iter()
                .any(|item| item.command == Command::Save)
        );
        select(&mut app, Command::OpenSelection);
        assert!(!app.handle_command_palette_key(key(KeyCode::Enter)).unwrap());
        assert!(app.editor.is_some());
        app.open_command_palette();
        let palette = app.command_palette.as_ref().unwrap();
        for command in [
            Command::Trash,
            Command::Quit,
            Command::NewNote,
            Command::CitationPicker,
            Command::SnippetPicker,
        ] {
            assert!(!palette.items.iter().any(|item| item.command == command));
        }
        assert!(
            palette
                .items
                .iter()
                .any(|item| item.command == Command::SearchHeadings)
        );
        assert!(
            palette
                .items
                .iter()
                .any(|item| item.command == Command::Save)
        );
    }

    #[test]
    fn trash_requires_existing_confirmation_and_blocks_nested_palette() {
        let (dir, mut app) = fixture();
        app.open_command_palette();
        select(&mut app, Command::Trash);
        app.handle_command_palette_key(key(KeyCode::Enter)).unwrap();
        assert!(matches!(
            app.prompt.as_ref().map(|p| &p.kind),
            Some(PromptKind::Trash { .. })
        ));
        app.open_command_palette();
        assert!(app.command_palette.is_none());
        app.handle_prompt_key(key(KeyCode::Enter));
        assert!(dir.path().join("Note.md").exists());
        assert!(!app.has_active_popup());
    }

    #[test]
    fn close_protects_dirty_notes_and_save_and_close_uses_existing_save() {
        let (dir, mut app) = fixture();
        app.enter().unwrap();
        for code in [KeyCode::Char('i'), KeyCode::Char('X'), KeyCode::Esc] {
            app.handle_editor_key(key(code));
        }
        let original = app.editor.as_ref().unwrap().state.lines.to_string();
        assert!(app.editor.as_ref().unwrap().is_dirty());
        app.open_command_palette();
        select(&mut app, Command::Close);
        app.handle_command_palette_key(key(KeyCode::Enter)).unwrap();
        assert!(app.editor.as_ref().unwrap().is_dirty());
        assert!(app.message.contains("Unsaved changes"));
        assert_eq!(
            fs::read_to_string(dir.path().join("Note.md")).unwrap(),
            "# Heading\nbody\n"
        );
        app.open_command_palette();
        select(&mut app, Command::SaveAndClose);
        app.handle_command_palette_key(key(KeyCode::Enter)).unwrap();
        assert!(app.editor.is_none());
        assert_eq!(
            fs::read_to_string(dir.path().join("Note.md")).unwrap(),
            original
        );
    }

    #[test]
    fn cancellation_preserves_outline_focus_and_no_results_do_not_execute() {
        let (_dir, mut app) = fixture();
        app.enter().unwrap();
        app.toggle_outline_focus();
        assert!(app.outline_view.focused);
        let cursor = app.editor.as_ref().unwrap().state.cursor;
        app.open_command_palette();
        for ch in "zzzzzzzzzz".chars() {
            app.handle_command_palette_key(key(KeyCode::Char(ch)))
                .unwrap();
        }
        app.handle_command_palette_key(key(KeyCode::Enter)).unwrap();
        assert!(app.command_palette.is_some());
        app.handle_command_palette_key(key(KeyCode::Esc)).unwrap();
        assert!(app.outline_view.focused);
        assert_eq!(app.editor.as_ref().unwrap().state.cursor, cursor);
        assert!(!app.has_active_popup());
    }

    #[test]
    fn insert_mode_does_not_open_palette_and_quit_signal_propagates() {
        let (_dir, mut app) = fixture();
        app.enter().unwrap();
        app.handle_editor_key(key(KeyCode::Char('i')));
        app.open_command_palette();
        assert!(app.command_palette.is_none());
        assert_eq!(app.editor.as_ref().unwrap().state.mode, EditorMode::Insert);
        app.handle_editor_key(key(KeyCode::Esc));
        app.close_editor(false);
        app.open_command_palette();
        select(&mut app, Command::Quit);
        assert!(app.handle_command_palette_key(key(KeyCode::Enter)).unwrap());
    }

    #[test]
    fn palette_can_open_heading_picker_without_nesting() {
        let (_dir, mut app) = fixture();
        app.enter().unwrap();
        app.open_command_palette();
        select(&mut app, Command::SearchHeadings);
        app.handle_command_palette_key(key(KeyCode::Enter)).unwrap();
        assert!(app.command_palette.is_none());
        assert!(app.heading_search.is_some());
    }
}

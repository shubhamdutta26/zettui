//! Modal, source-preserving frontmatter editing.

use super::App;
use crate::{markdown, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{
    EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Index2, LineNumbers,
    Lines,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, BorderType, Clear, Padding, Paragraph},
};
use std::path::PathBuf;

pub(super) struct MetadataEditor {
    path: PathBuf,
    original: String,
    applied: String,
    pub(super) state: EditorState,
    keys: EditorEventHandler,
    command: Option<String>,
    message: String,
    cache: markdown::HighlightCache,
    width: u16,
    height: u16,
}

// Return the exact frontmatter prefix, excluding the newline after the closing
// delimiter. Distinguish absent frontmatter from a possibly malformed block.
fn prefix(source: &str) -> Result<Option<&str>, &'static str> {
    let mut rows = source.split('\n');
    let first = rows.next().unwrap_or_default();
    if first.trim_end() != "---" {
        return Ok(None);
    }
    let mut end = first.len();
    for row in rows {
        end += 1 + row.len();
        if matches!(row.trim_end(), "---" | "...") {
            return Ok(Some(&source[..end]));
        }
    }
    Err("Unclosed YAML frontmatter. Repair it in the note first.")
}

fn lines(text: &str) -> Lines {
    Lines::new(
        text.split('\n')
            .map(|line| line.chars().collect::<Vec<char>>())
            .collect::<Vec<Vec<char>>>(),
    )
}

fn merge(source: &str, draft: &str) -> Result<String, &'static str> {
    if prefix(draft)? != Some(draft) {
        return Err(
            "Keep exactly one YAML block, starting with --- and ending with --- or ... (no trailing lines).",
        );
    }
    match prefix(source)? {
        Some(old) => Ok(format!("{draft}{}", &source[old.len()..])),
        None if source.is_empty() => Ok(draft.to_owned()),
        None => Ok(format!("{draft}\n{source}")),
    }
}

fn send(keys: &mut EditorEventHandler, state: &mut EditorState, code: KeyCode) {
    keys.on_key_event(KeyEvent::new(code, KeyModifiers::NONE), state);
}

fn insert(keys: &mut EditorEventHandler, state: &mut EditorState, text: &str) {
    for ch in text.chars() {
        send(
            keys,
            state,
            if ch == '\n' {
                KeyCode::Enter
            } else {
                KeyCode::Char(ch)
            },
        );
    }
    send(keys, state, KeyCode::Esc);
}

// Use EdTUI actions on a cloned state, preserving the original buffer and its
// undo stack until the exact expected text has been verified. Prefer a visual
// change of the prefix; fall back to explicit line deletion and insertion.
fn apply_to_state(
    state: &EditorState,
    source: &str,
    draft: &str,
    expected: &str,
) -> Result<EditorState, &'static str> {
    let old = prefix(source)?;
    let mut candidate = state.clone();
    let mut keys = EditorEventHandler::default();
    candidate.cursor = Index2::new(0, 0);
    if let Some(old) = old {
        send(&mut keys, &mut candidate, KeyCode::Char('v'));
        for _ in 0..old.bytes().filter(|b| *b == b'\n').count() {
            send(&mut keys, &mut candidate, KeyCode::Char('j'));
        }
        send(&mut keys, &mut candidate, KeyCode::Char('$'));
        send(&mut keys, &mut candidate, KeyCode::Char('c'));
        if candidate.mode == EditorMode::Insert {
            insert(&mut keys, &mut candidate, draft);
        }
    } else {
        send(&mut keys, &mut candidate, KeyCode::Char('i'));
        let new_prefix = if source.is_empty() {
            draft.to_owned()
        } else {
            format!("{draft}\n")
        };
        insert(&mut keys, &mut candidate, &new_prefix);
    }
    if candidate.lines.to_string() != expected {
        // A version-specific visual motion can behave differently at an empty
        // final row. Try deleting the old frontmatter rows, never the body.
        candidate = state.clone();
        keys = EditorEventHandler::default();
        candidate.cursor = Index2::new(0, 0);
        if let Some(old) = old {
            for _ in old.split('\n') {
                send(&mut keys, &mut candidate, KeyCode::Char('d'));
                send(&mut keys, &mut candidate, KeyCode::Char('d'));
            }
        }
        candidate.cursor = Index2::new(0, 0);
        send(&mut keys, &mut candidate, KeyCode::Char('i'));
        let new_prefix = if old.is_some_and(|p| p.len() == source.len()) || source.is_empty() {
            draft.to_owned()
        } else {
            format!("{draft}\n")
        };
        insert(&mut keys, &mut candidate, &new_prefix);
    }
    if candidate.lines.to_string() != expected || candidate.mode != EditorMode::Normal {
        return Err("Could not safely apply metadata with this editor. The note was not changed.");
    }
    // Preserve the cursor's relative location in the note body when possible.
    let old_rows = old.map_or(0, |p| p.split('\n').count());
    let new_rows = draft.split('\n').count();
    let row = if state.cursor.row >= old_rows {
        state.cursor.row - old_rows + new_rows
    } else {
        state.cursor.row.min(new_rows - 1)
    };
    let row = row.min(expected.split('\n').count() - 1);
    let col = expected
        .split('\n')
        .nth(row)
        .unwrap_or_default()
        .chars()
        .count()
        .saturating_sub(1);
    candidate.cursor = Index2::new(row, state.cursor.col.min(col));
    candidate.set_viewport_offset(0, row.saturating_sub(2));
    candidate.clear_highlights();
    Ok(candidate)
}

impl MetadataEditor {
    fn dirty(&self) -> bool {
        self.state.lines.to_string() != self.applied
    }

    fn new(path: PathBuf, original: String) -> Result<Self, &'static str> {
        let existing = prefix(&original)?;
        let applied = existing.unwrap_or("---\n\n---").to_owned();
        let mut state = EditorState::new(lines(&applied));
        if existing.is_none() {
            state.cursor = Index2::new(1, 0);
        }
        Ok(Self {
            path,
            original,
            applied,
            state,
            keys: EditorEventHandler::default(),
            command: None,
            message: String::new(),
            cache: markdown::HighlightCache::default(),
            width: 80,
            height: 76,
        })
    }
}

impl App {
    pub(super) fn open_metadata(&mut self) {
        if self.has_active_popup() || self.command.is_some() || self.note_search.input.is_some() {
            return;
        }
        let Some(note) = self.editor.as_ref().filter(|note| {
            markdown::is_markdown(&note.path) && note.state.mode == EditorMode::Normal
        }) else {
            self.message = "Open a Markdown note in Normal mode to edit metadata.".into();
            return;
        };
        match MetadataEditor::new(note.path.clone(), note.state.lines.to_string()) {
            Ok(editor) => {
                self.metadata = Some(editor);
                self.outline_view.focused = false;
                self.writing.leader = false;
                self.buffer_prefix = None;
                self.pending_open_bracket = false;
                self.message.clear();
            }
            Err(error) => self.message = error.into(),
        }
    }

    fn apply_metadata(&mut self, editor: &mut MetadataEditor) -> Result<(), &'static str> {
        let draft = editor.state.lines.to_string();
        let Some(note) = self.editor.as_mut().filter(|note| note.path == editor.path) else {
            return Err("Active note changed; metadata was not applied.");
        };
        let source = note.state.lines.to_string();
        if source != editor.original {
            return Err("Note changed while metadata was open; discard and reopen.");
        }
        let expected = merge(&source, &draft)?;
        if expected != source {
            note.state = apply_to_state(&note.state, &source, &draft, &expected)?;
            self.editor_keys = EditorEventHandler::default();
            self.note_search.reset_matches();
            self.highlight_cache = markdown::HighlightCache::default();
        }
        editor.original = expected;
        editor.applied = draft;
        Ok(())
    }

    pub(super) fn handle_metadata_key(&mut self, key: KeyEvent) {
        let Some(mut editor) = self.metadata.take() else {
            return;
        };
        if let Some(input) = &mut editor.command {
            match key.code {
                KeyCode::Esc => editor.command = None,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Enter => {
                    let command = editor.command.take().unwrap_or_default();
                    match command.trim() {
                        "w" | "wq" | "x" => match self.apply_metadata(&mut editor) {
                            Ok(()) => {
                                if command.trim() != "w" {
                                    self.message =
                                        "Metadata applied to note. Use :w to save to disk.".into();
                                    return;
                                }
                                editor.message = "Applied to note (not saved to disk).".into();
                            }
                            Err(error) => editor.message = error.into(),
                        },
                        "q!" => return,
                        "q" if !editor.dirty() => return,
                        "q" => editor.message = "Unapplied edits! :wq apply, :q! discard.".into(),
                        _ => editor.message = "Metadata commands: :w :wq :q :q!".into(),
                    }
                }
                KeyCode::Char(ch)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    input.push(ch)
                }
                _ => {}
            }
        } else if editor.state.mode == EditorMode::Normal
            && key.code == KeyCode::Char(':')
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            editor.command = Some(String::new());
            editor.keys = EditorEventHandler::default();
        } else if editor.state.mode == EditorMode::Normal && key.code == KeyCode::F(11) {
            if !editor.dirty() {
                return;
            }
            editor.message = "Unapplied edits! :wq apply, :q! discard.".into();
        } else if editor.state.mode == EditorMode::Normal
            && key.modifiers == KeyModifiers::CONTROL
            && matches!(
                key.code,
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            )
        {
            match key.code {
                KeyCode::Left => editor.width = editor.width.saturating_sub(5).max(40),
                KeyCode::Right => editor.width = (editor.width + 5).min(100),
                KeyCode::Down => editor.height = editor.height.saturating_sub(5).max(40),
                KeyCode::Up => editor.height = (editor.height + 5).min(100),
                _ => {}
            }
        } else {
            editor.keys.on_key_event(key, &mut editor.state);
            editor.message.clear();
        }
        self.metadata = Some(editor);
    }

    pub(super) fn draw_metadata_editor(&mut self, frame: &mut Frame) {
        let Some(editor) = self.metadata.as_mut() else {
            return;
        };
        let theme: Theme = self.theme;
        let screen = frame.area();
        let width = ((u32::from(screen.width) * u32::from(editor.width) / 100) as u16)
            .max(32)
            .min(screen.width);
        let height = ((u32::from(screen.height) * u32::from(editor.height) / 100) as u16)
            .max(8)
            .min(screen.height);
        let area = Rect::new(
            screen.x + (screen.width - width) / 2,
            screen.y + (screen.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, area);
        if width < 16 || height < 6 {
            frame.render_widget(
                Paragraph::new("Resize terminal to edit metadata.").style(theme.base_style()),
                area,
            );
            return;
        }
        let dirty = if editor.dirty() { " [+]" } else { "" };
        let block = Block::bordered()
            .title(format!(" Metadata Editor{dirty} "))
            .border_type(BorderType::Thick)
            .padding(Padding::horizontal(1))
            .style(theme.base_style())
            .border_style(Style::default().fg(theme.accent));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let rows = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);
        let text = editor.state.lines.to_string();
        editor
            .cache
            .prepare(&text, theme, true, &self.markdown_settings);
        editor
            .state
            .set_highlights(editor.cache.highlights(&[], 0, None));
        let cursor_style = if editor.state.mode == EditorMode::Insert || editor.command.is_some() {
            theme.base_style()
        } else {
            Style::default().fg(theme.background).bg(theme.accent)
        };
        let editor_theme = EditorTheme::default()
            .base(theme.base_style())
            .selection_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg))
            .cursor_style(cursor_style)
            .hide_status_line();
        frame.render_widget(
            EditorView::new(&mut editor.state)
                .wrap(true)
                .line_numbers(LineNumbers::None)
                .theme(editor_theme),
            rows[0],
        );
        let status = match &editor.command {
            Some(command) => format!(":{command}▏"),
            None => format!(
                "{} | :w Apply  :wq Apply & close  :q Close  :q! Discard",
                editor.state.mode.name().to_uppercase()
            ),
        };
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(theme.accent).bg(theme.background)),
            rows[1],
        );
        let help = if editor.message.is_empty() {
            "Ctrl+arrows resize | Note saved separately"
        } else {
            &editor.message
        };
        frame.render_widget(
            Paragraph::new(help).style(Style::default().fg(theme.foreground).bg(theme.background)),
            rows[2],
        );
        if editor.command.is_none()
            && let Some(cursor) = editor.state.cursor_screen_position()
        {
            frame.set_cursor_position((cursor.x, cursor.y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_body_and_yaml_source() {
        let source = "---\ntitle: Test\ncustom: |\n  ---\n...\n\nBody β\n";
        let draft = "---\ntitle: New\n# comment\ncustom: |\n  ---\n...";
        assert_eq!(
            merge(source, draft).unwrap(),
            format!("{draft}\n\nBody β\n")
        );
    }

    #[test]
    fn missing_and_unclosed_blocks_are_distinct() {
        assert_eq!(prefix("# Body\n---\n").unwrap(), None);
        assert!(prefix("---\ntitle: A").is_err());
        assert!(merge("Body", "---\n---\nExtra").is_err());
        assert!(merge("Body", "title: A").is_err());
    }

    #[test]
    fn apply_exact_text_across_document_shapes() {
        for source in [
            "# Body\nText\n",
            "",
            "---\ntitle: Old\n---\nBody\n",
            "---\n---",
            "---\n---\n",
            "---\n---\n\nBody",
            "---\ntags: [a]\n...\nβ",
        ] {
            let draft = "---\ntitle: New\ntags: [a, b]\n---";
            let expected = merge(source, draft).unwrap();
            let state = EditorState::new(lines(source));
            let candidate = apply_to_state(&state, source, draft, &expected).unwrap();
            assert_eq!(candidate.lines.to_string(), expected);
            assert_eq!(state.lines.to_string(), source);
        }
    }
}

//! Modal insertion of ordinary Markdown links and image references.

use super::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode};

pub(super) struct InsertDialog {
    pub image: bool,
    pub label: String,
    pub target: String,
    pub target_active: bool,
    pub error: &'static str,
}

/// Escape Markdown delimiters in the label and encode characters that could
/// terminate a link destination. Existing percent escapes are left intact.
fn markdown_reference(image: bool, label: &str, target: &str) -> Option<String> {
    let target = target.trim();
    if target.is_empty() || target.chars().any(char::is_control) {
        return None;
    }
    let mut escaped = String::new();
    for ch in label.chars() {
        if matches!(ch, '\\' | '[' | ']' | '*' | '_' | '`' | '<' | '>') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    let mut destination = String::new();
    for ch in target.chars() {
        if ch.is_whitespace() || matches!(ch, '(' | ')' | '<' | '>' | '\\') {
            let mut bytes = [0; 4];
            for byte in ch.encode_utf8(&mut bytes).bytes() {
                use std::fmt::Write;
                let _ = write!(destination, "%{byte:02X}");
            }
        } else {
            destination.push(ch);
        }
    }
    let prefix = if image { "!" } else { "" };
    Some(format!("{prefix}[{escaped}]({destination})"))
}

impl App {
    pub(super) fn open_markdown_insert(&mut self, image: bool) {
        let Some(note) = self.editor.as_ref() else {
            return;
        };
        if !crate::markdown::is_markdown(&note.path) {
            self.message = "Link and image insertion requires a Markdown note.".into();
            return;
        }
        if !matches!(note.state.mode, EditorMode::Normal | EditorMode::Insert) {
            self.message = "Switch to Normal or Insert mode first.".into();
            return;
        }
        self.markdown_insert = Some(InsertDialog {
            image,
            label: String::new(),
            target: String::new(),
            target_active: false,
            error: "",
        });
        self.pending_open_bracket = false;
        self.message.clear();
    }

    pub(super) fn handle_markdown_insert_key(&mut self, key: KeyEvent) {
        let Some(mut dialog) = self.markdown_insert.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => return,
            KeyCode::Tab | KeyCode::BackTab => {
                dialog.target_active = !dialog.target_active;
            }
            KeyCode::Enter if key.modifiers.is_empty() => {
                if !dialog.target_active {
                    dialog.target_active = true;
                } else if let Some(text) =
                    markdown_reference(dialog.image, &dialog.label, &dialog.target)
                {
                    self.insert_markdown_reference(&text);
                    return;
                } else {
                    dialog.error = "Enter a URL or path.";
                }
            }
            _ => {
                let field = if dialog.target_active {
                    &mut dialog.target
                } else {
                    &mut dialog.label
                };
                match key.code {
                    KeyCode::Backspace => {
                        field.pop();
                    }
                    KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => field.clear(),
                    KeyCode::Char(ch)
                        if !ch.is_control()
                            && !key
                                .modifiers
                                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        field.push(ch);
                    }
                    _ => {}
                }
                dialog.error = "";
            }
        }
        self.markdown_insert = Some(dialog);
    }

    fn insert_markdown_reference(&mut self, text: &str) {
        let Some(note) = self.editor.as_mut() else {
            return;
        };
        let normal = note.state.mode == EditorMode::Normal;
        if !normal && note.state.mode != EditorMode::Insert {
            return;
        }
        // Use the same EdTUI event path as citation insertion, preserving undo,
        // dirty tracking, and the original editor mode.
        if normal {
            self.editor_keys = EditorEventHandler::default();
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
                &mut note.state,
            );
        }
        for ch in text.chars() {
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
                &mut note.state,
            );
        }
        if normal {
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                &mut note.state,
            );
        }
        self.note_search.painted.clear();
        note.state.clear_highlights();
        self.message = "Markdown reference inserted (unsaved).".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_escape_delimiters_and_preserve_existing_url_encoding() {
        assert_eq!(
            markdown_reference(false, "Paper [1]", "https://example.org/a%20b?q=1&x=2").as_deref(),
            Some(r"[Paper \[1\]](https://example.org/a%20b?q=1&x=2)")
        );
        assert_eq!(
            markdown_reference(true, "Figure", "../assets/Figure (1).png").as_deref(),
            Some("![Figure](../assets/Figure%20%281%29.png)")
        );
        assert_eq!(
            markdown_reference(true, "", "a.png").as_deref(),
            Some("![](a.png)")
        );
        assert!(markdown_reference(false, "", "  ").is_none());
        assert!(markdown_reference(false, "", "a\nb").is_none());
    }
}

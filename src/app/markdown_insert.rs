//! Markdown link/image dialogs and parameterized text insertion commands.

use super::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode, Index2};

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
                    self.insert_generated_text(&text, "Markdown reference inserted (unsaved).");
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

    /// Reuse the compact Ex input as a language prompt in either editing mode.
    pub(super) fn open_code_block_input(&mut self) {
        if !self.code_block_available() {
            return;
        }
        self.command = Some("code ".into());
        self.pending_open_bracket = false;
        self.message = "Type a language (e.g. r, python, rust); Enter for plain code.".into();
    }

    fn code_block_available(&mut self) -> bool {
        if self.editor.as_ref().is_some_and(|note| {
            crate::markdown::is_markdown(&note.path)
                && matches!(note.state.mode, EditorMode::Normal | EditorMode::Insert)
        }) {
            true
        } else {
            self.message = "Code blocks require a Markdown note in Normal or Insert mode.".into();
            false
        }
    }

    fn insert_code_block(&mut self, language: &str) {
        if !self.code_block_available() {
            return;
        }
        let Some(note) = self.editor.as_mut() else {
            return;
        };
        let source = note.state.lines.to_string();
        let Some(plan) = code_block_plan(&source, note.state.cursor.row, language) else {
            self.message = "Use :code [language] with a single language identifier.".into();
            return;
        };
        // Apply on a clone, using EdTUI's undo history. Verify the complete
        // document before replacing the live state (including surrounding text).
        let mut candidate = note.state.clone();
        let mut keys = EditorEventHandler::default();
        let send = |keys: &mut EditorEventHandler, state: &mut edtui::EditorState, code| {
            keys.on_key_event(KeyEvent::new(code, KeyModifiers::NONE), state);
        };
        if candidate.mode == EditorMode::Insert {
            send(&mut keys, &mut candidate, KeyCode::Esc);
        }
        candidate.cursor = Index2::new(plan.row, 0);
        send(&mut keys, &mut candidate, KeyCode::Char('i'));
        candidate.cursor = Index2::new(plan.row, plan.col);
        for ch in plan.text.chars() {
            send(
                &mut keys,
                &mut candidate,
                if ch == '\n' {
                    KeyCode::Enter
                } else {
                    KeyCode::Char(ch)
                },
            );
        }
        send(&mut keys, &mut candidate, KeyCode::Esc);
        if candidate.lines.to_string() != plan.expected || candidate.mode != EditorMode::Normal {
            self.message = "Could not safely insert the code block; note unchanged.".into();
            return;
        }
        candidate.cursor = Index2::new(plan.body_row, 0);
        send(&mut keys, &mut candidate, KeyCode::Char('i'));
        if candidate.mode != EditorMode::Insert
            || candidate.cursor != Index2::new(plan.body_row, 0)
            || candidate.lines.to_string() != plan.expected
        {
            self.message = "Could not enter the code block; note unchanged.".into();
            return;
        }
        candidate.clear_highlights();
        note.state = candidate;
        self.editor_keys = keys;
        self.note_search.painted.clear();
        self.message = "Code block inserted (unsaved).".into();
    }

    /// Dispatch parameterized Ex commands without changing the fixed command catalog.
    pub(super) fn run_insertion_command(&mut self, input: &str) -> bool {
        let input = input.trim();
        let (name, argument) = input.split_once(char::is_whitespace).unwrap_or((input, ""));
        let argument = argument.trim();
        let result = match name {
            "chem" | "chemblock" => {
                if !self
                    .editor
                    .as_ref()
                    .is_some_and(|note| crate::markdown::is_markdown(&note.path))
                {
                    self.message = "Chemical notation requires a Markdown note.".into();
                    return true;
                }
                chemical_text(argument, name == "chemblock")
            }
            "lorem" => lorem_text(argument),
            "code" => {
                self.insert_code_block(argument);
                return true;
            }
            _ => return false,
        };
        match result {
            Ok(text) => self.insert_generated_text(&text, "Text inserted (unsaved)."),
            Err(message) => self.message = message.into(),
        }
        true
    }

    fn insert_generated_text(&mut self, text: &str, message: &str) {
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
                KeyEvent::new(
                    if ch == '\n' {
                        KeyCode::Enter
                    } else {
                        KeyCode::Char(ch)
                    },
                    KeyModifiers::NONE,
                ),
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
        self.message = message.into();
    }
}

/// An insertion below the current line (or into the current empty line),
/// separated from adjacent prose and with a cursor inside the empty body.
struct CodeBlockPlan {
    row: usize,
    col: usize,
    body_row: usize,
    text: String,
    expected: String,
}

fn code_block_plan(source: &str, row: usize, language: &str) -> Option<CodeBlockPlan> {
    if language.len() > 64
        || !language
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'+' | b'#' | b'.'))
    {
        return None;
    }
    let lines: Vec<&str> = source.split('\n').collect();
    let row = row.min(lines.len() - 1);
    let line = lines[row];
    let col = line.chars().count();
    let prefix = if !line.is_empty() {
        "\n\n"
    } else if row > 0 && !lines[row - 1].is_empty() {
        "\n"
    } else {
        ""
    };
    let suffix = if lines.get(row + 1).is_some_and(|line| !line.is_empty()) {
        "\n"
    } else {
        ""
    };
    let text = format!("{prefix}```{language}\n\n```{suffix}");
    let offset = lines[..row]
        .iter()
        .map(|line| line.len() + 1)
        .sum::<usize>()
        + line.len();
    let mut expected = source.to_owned();
    expected.insert_str(offset, &text);
    Some(CodeBlockPlan {
        row,
        col,
        body_row: row + prefix.len() + 1,
        text,
        expected,
    })
}

#[cfg(test)]
#[test]
fn code_block_plan_preserves_prose_and_validates_language() {
    let plan = code_block_plan("αβ\nNext", 0, "r").unwrap();
    assert_eq!(plan.expected, "αβ\n\n```r\n\n```\n\nNext");
    assert_eq!(plan.col, 2);
    assert_eq!(plan.body_row, 3);
    let plan = code_block_plan("", 0, "").unwrap();
    assert_eq!(plan.expected, "```\n\n```");
    assert_eq!(plan.body_row, 1);
    let plan = code_block_plan("First\n\nLast", 1, "python").unwrap();
    assert_eq!(plan.expected, "First\n\n```python\n\n```\n\nLast");
    assert!(code_block_plan("", 0, "c++").is_some());
    for invalid in ["r python", "r`", "r\n", "{r}"] {
        assert!(code_block_plan("", 0, invalid).is_none());
    }
}

/// Store the original mhchem notation, with balanced braces and no math delimiters.
fn chemical_text(formula: &str, display: bool) -> Result<String, &'static str> {
    if formula.is_empty() || formula.chars().any(|ch| ch.is_control() || ch == '$') {
        return Err("Usage: :chem H2O or :chemblock CO2 + C -> 2 CO (without $ delimiters).");
    }
    let mut depth = 0usize;
    let mut escaped = false;
    for ch in formula.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '{' => depth += 1,
            '}' if depth == 0 => return Err("Unbalanced chemical formula braces."),
            '}' => depth -= 1,
            _ => {}
        }
    }
    if depth != 0 || escaped {
        return Err("Unbalanced chemical formula braces or trailing backslash.");
    }
    let math = format!(r"\ce{{{formula}}}");
    Ok(if display {
        format!("\n\n$$\n{math}\n$$\n\n")
    } else {
        format!("${math}$")
    })
}

const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

fn lorem_text(argument: &str) -> Result<String, &'static str> {
    const USAGE: &str = "Usage: :lorem [1-1000 words or 1-20 paragraphs, e.g. 3p].";
    if let Some(count) = argument.strip_suffix('p') {
        let count: usize = count.parse().map_err(|_| USAGE)?;
        if !(1..=20).contains(&count) {
            return Err(USAGE);
        }
        return Ok(vec![LOREM; count].join("\n\n"));
    }
    let count: usize = if argument.is_empty() {
        30
    } else {
        argument.parse().map_err(|_| USAGE)?
    };
    if !(1..=1000).contains(&count) {
        return Err(USAGE);
    }
    let mut text = LOREM
        .split_whitespace()
        .cycle()
        .take(count)
        .collect::<Vec<_>>()
        .join(" ");
    if text.ends_with(',') {
        text.pop();
    }
    if !text.ends_with('.') {
        text.push('.');
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemistry_and_lorem_arguments_are_bounded_and_preserve_source() {
        assert_eq!(
            chemical_text("SO4^{2-}", false).unwrap(),
            r"$\ce{SO4^{2-}}$"
        );
        assert_eq!(
            chemical_text("CO2 + C -> 2 CO", true).unwrap(),
            "\n\n$$\n\\ce{CO2 + C -> 2 CO}\n$$\n\n"
        );
        assert!(chemical_text("", false).is_err());
        assert!(chemical_text("H2O}", false).is_err());
        assert!(chemical_text("H2O$", false).is_err());
        assert_eq!(lorem_text("").unwrap().split_whitespace().count(), 30);
        assert_eq!(lorem_text("100").unwrap().split_whitespace().count(), 100);
        assert_eq!(lorem_text("3p").unwrap().split("\n\n").count(), 3);
        for invalid in ["0", "1001", "0p", "21p", "abc"] {
            assert!(lorem_text(invalid).is_err());
        }
    }

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

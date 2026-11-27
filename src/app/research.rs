//! Read-only citation inspection and explicit local PDF launching.

mod zotero;

use super::App;
use crate::bibliography::{Bibliography, Reference};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::Index2;
use std::{
    collections::HashSet,
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CitationHit {
    pub key: String,
    pub row: usize,
    pub start: usize,
    pub end: usize,
}

pub(super) enum ResearchDialog {
    Preview {
        text: String,
        key: String,
        pdfs: Vec<PathBuf>,
        selected: usize,
        scroll: usize,
        status: String,
        lookup_status: String,
        lookup: Option<zotero::Lookup>,
    },
    Audit {
        hits: Vec<CitationHit>,
        selected: usize,
        total: usize,
    },
}

impl App {
    pub(super) fn preview_reference(&mut self, reference: &Reference) {
        let mut pdfs = Vec::new();
        if let Some(path) = self.bibliography_path.as_deref() {
            zotero::merge_paths(&mut pdfs, reference.pdf_paths(path));
        }
        let (lookup, lookup_status) = match zotero::start(&reference.key) {
            Ok(lookup) => (Some(lookup), "Looking up Zotero PDFs…".into()),
            Err(error) => (None, error),
        };
        self.research_dialog = Some(ResearchDialog::Preview {
            text: reference.preview(),
            key: reference.key.clone(),
            pdfs,
            selected: 0,
            scroll: 0,
            status: String::new(),
            lookup_status,
            lookup,
        });
    }

    /// Called before drawing; never block on the Zotero worker.
    pub(super) fn poll_research_attachments(&mut self) {
        use std::sync::mpsc::TryRecvError;
        let Some(ResearchDialog::Preview {
            pdfs,
            lookup_status,
            lookup,
            ..
        }) = &mut self.research_dialog
        else {
            return;
        };
        let Some(receiver) = lookup.as_ref() else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(paths)) => {
                let count = paths.len();
                zotero::merge_paths(pdfs, paths);
                *lookup_status = if count == 0 {
                    "Zotero has no local PDF paths for this key.".into()
                } else {
                    format!("Zotero: {count} PDF attachment(s)")
                };
            }
            Ok(Err(error)) => *lookup_status = error,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                *lookup_status = "Zotero lookup stopped; press r to retry.".into()
            }
        }
        *lookup = None;
    }

    fn research_bibliography(&mut self) -> Option<Bibliography> {
        let Some(path) = &self.bibliography_path else {
            self.message = "No bibliography configured.".into();
            return None;
        };
        match Bibliography::load(path) {
            Ok(bibliography) => Some(bibliography),
            Err(error) => {
                self.message = format!("Cannot load bibliography: {error}");
                None
            }
        }
    }

    pub(super) fn preview_citation_at_cursor(&mut self) {
        let Some(note) = self.editor.as_ref() else {
            self.message = "Open a note first.".into();
            return;
        };
        let cursor = note.state.cursor;
        let text = note.state.lines.to_string();
        let Some(hit) = citation_hits(&text)
            .into_iter()
            .find(|hit| hit.row == cursor.row && cursor.col >= hit.start && cursor.col < hit.end)
        else {
            self.message = "Place the cursor on a citation key such as @Example2026.".into();
            return;
        };
        let Some(bibliography) = self.research_bibliography() else {
            return;
        };
        if let Some(reference) = bibliography
            .references
            .iter()
            .find(|reference| reference.key == hit.key)
        {
            self.preview_reference(reference);
        } else {
            self.message = format!("Unresolved citation: @{}", hit.key);
        }
    }

    pub(super) fn check_citations(&mut self) {
        let Some(note) = self.editor.as_ref() else {
            self.message = "Open a note first.".into();
            return;
        };
        let text = note.state.lines.to_string();
        let Some(bibliography) = self.research_bibliography() else {
            return;
        };
        let keys: HashSet<_> = bibliography
            .references
            .iter()
            .map(|reference| reference.key.as_str())
            .collect();
        let mut hits = citation_hits(&text);
        let total = hits.len();
        hits.retain(|hit| !keys.contains(hit.key.as_str()));
        self.research_dialog = Some(ResearchDialog::Audit {
            hits,
            selected: 0,
            total,
        });
    }

    pub(super) fn handle_research_key(&mut self, key: KeyEvent) {
        let Some(mut dialog) = self.research_dialog.take() else {
            return;
        };
        if key.modifiers == KeyModifiers::NONE {
            if key.code == KeyCode::Esc {
                return;
            }
            match &mut dialog {
                ResearchDialog::Preview {
                    pdfs,
                    selected,
                    scroll,
                    status,
                    key: citation_key,
                    lookup_status,
                    lookup,
                    ..
                } => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => *scroll = scroll.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => *scroll = scroll.saturating_add(1),
                    KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
                    KeyCode::PageDown => *scroll = scroll.saturating_add(10),
                    KeyCode::Home => *scroll = 0,
                    KeyCode::Left | KeyCode::Char('h') => {
                        *selected = selected.saturating_sub(1);
                        status.clear();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        *selected = selected.saturating_add(1).min(pdfs.len().saturating_sub(1));
                        status.clear();
                    }
                    KeyCode::Char('r') => {
                        if lookup.is_none() {
                            status.clear();
                            match zotero::start(citation_key) {
                                Ok(receiver) => {
                                    *lookup = Some(receiver);
                                    *lookup_status = "Looking up Zotero PDFs…".into();
                                }
                                Err(error) => *lookup_status = error,
                            }
                        }
                    }
                    KeyCode::Char('o') => {
                        *status = match pdfs.get(*selected) {
                            Some(path) => match open_pdf(path) {
                                Ok(()) => "PDF opener launched.".into(),
                                Err(error) => format!("Cannot open PDF: {error}"),
                            },
                            None => String::new(),
                        };
                    }
                    _ => {}
                },
                ResearchDialog::Audit { hits, selected, .. } => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => *selected = selected.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => {
                        *selected = selected.saturating_add(1).min(hits.len().saturating_sub(1))
                    }
                    KeyCode::Enter => {
                        if let Some(hit) = hits.get(*selected)
                            && let Some(note) = self.editor.as_mut()
                        {
                            note.state.cursor = Index2::new(hit.row, hit.start);
                            self.message = format!("Unresolved citation: @{}", hit.key);
                            return;
                        }
                    }
                    _ => {}
                },
            }
        }
        self.research_dialog = Some(dialog);
    }
}

fn open_pdf(path: &Path) -> io::Result<()> {
    let path = path.canonicalize()?;
    if !path.is_file()
        || path
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("pdf"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Attachment is not a local PDF file",
        ));
    }
    // Pass the absolute filename as one argument; never invoke a shell.
    let mut child = Command::new("xdg-open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Reap the opener without blocking the TUI on the external application.
    drop(std::thread::spawn(move || {
        let _ = child.wait();
    }));
    Ok(())
}

/// Scan Zettui's explicit, single-line citation groups, including locators and
/// suppressed authors. Columns count Unicode scalar values, like the editor.
/// This is deliberately not a full Pandoc parser; bare narrative @keys are not audited.
fn citation_hits(text: &str) -> Vec<CitationHit> {
    let mut hits = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    let mut yaml = false;
    let mut comment = false;
    let mut code_ticks = 0;
    for (row, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if row == 0 && trimmed == "---" {
            yaml = true;
            continue;
        }
        if yaml {
            if matches!(trimmed, "---" | "...") {
                yaml = false;
            }
            continue;
        }
        let marker = trimmed.chars().next().unwrap_or(' ');
        if matches!(marker, '`' | '~') && !comment {
            let count = trimmed.chars().take_while(|&ch| ch == marker).count();
            if let Some((open, length)) = fence {
                if marker == open
                    && count >= length
                    && trimmed.chars().skip(count).all(char::is_whitespace)
                {
                    fence = None;
                }
                continue;
            }
            if count >= 3 {
                fence = Some((marker, count));
                code_ticks = 0;
                continue;
            }
        }
        if fence.is_some() || line.starts_with("    ") || line.starts_with('\t') {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut closing = chars
            .iter()
            .enumerate()
            .filter_map(|(index, &ch)| (ch == ']').then_some(index))
            .peekable();
        let mut i = 0;
        while i < chars.len() {
            let rest = &chars[i..];
            if comment {
                if rest.starts_with(&['-', '-', '>']) {
                    comment = false;
                    i += 3;
                } else {
                    i += 1;
                }
                continue;
            }
            if chars[i] == '`' {
                let count = rest.iter().take_while(|&&ch| ch == '`').count();
                if code_ticks == 0 {
                    code_ticks = count;
                } else if code_ticks == count {
                    code_ticks = 0;
                }
                i += count;
                continue;
            }
            if code_ticks != 0 {
                i += 1;
                continue;
            }
            if rest.starts_with(&['<', '!', '-', '-']) {
                comment = true;
                i += 4;
                continue;
            }
            if chars[i] == '\\' {
                i += 2;
                continue;
            }
            if chars[i] != '['
                || (i > 0 && matches!(chars[i - 1], '[' | '!'))
                || rest.get(1) == Some(&'[')
            {
                i += 1;
                continue;
            }
            while closing.peek().is_some_and(|&end| end < i) {
                closing.next();
            }
            let Some(&end) = closing.peek() else {
                i += 1;
                continue;
            };
            if matches!(chars.get(end + 1), Some(&('(' | '[' | ']'))) {
                i += 1;
                continue;
            }
            let mut segment = i + 1;
            let mut group = Vec::new();
            while segment < end {
                let stop = (segment..end)
                    .find(|&index| chars[index] == ';')
                    .unwrap_or(end);
                let mut start = segment;
                while start < stop && chars[start].is_whitespace() {
                    start += 1;
                }
                if chars.get(start) == Some(&'-') {
                    start += 1;
                }
                if start < stop && chars[start] == '@' {
                    let key_end = (start + 1..stop)
                        .find(|&index| chars[index].is_whitespace() || chars[index] == ',')
                        .unwrap_or(stop);
                    let key: String = chars[start + 1..key_end].iter().collect();
                    if !key.is_empty()
                        && !key.chars().any(|ch| {
                            ch.is_control()
                                || matches!(ch, '@' | '[' | ']' | '{' | '}' | '`' | '\\')
                        })
                    {
                        group.push(CitationHit {
                            key,
                            row,
                            start,
                            end: key_end,
                        });
                    }
                }
                if segment == i + 1 && group.is_empty() {
                    break;
                }
                segment = stop + 1;
            }
            if group.is_empty() {
                i += 1;
            } else {
                hits.extend(group);
                i = end + 1;
            }
        }
        if trimmed.is_empty() {
            code_ticks = 0;
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_unicode_columns_and_locators() {
        let hits = citation_hits("α [@A, p. 5; -@B] and [@A]");
        assert_eq!(
            hits.iter().map(|hit| hit.key.as_str()).collect::<Vec<_>>(),
            vec!["A", "B", "A"]
        );
        assert_eq!((hits[0].row, hits[0].start, hits[0].end), (0, 3, 5));
    }

    #[test]
    fn excludes_common_non_citation_contexts() {
        let text = "---\ntitle: '[@YAML]'\n---\n`[@inline]` \\[@escaped] [[@wiki]] [@link](url)\n```r\n[@fenced]\n```\n<!--\n[@comment]\n-->\n    [@indented]\nemail@example.org @narrative\n[@real]";
        let hits = citation_hits(text);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, "real");
    }

    #[test]
    fn missing_and_non_pdf_attachments_are_rejected_before_launch() {
        let dir = tempfile::tempdir().unwrap();
        assert!(open_pdf(&dir.path().join("missing.pdf")).is_err());
        let file = dir.path().join("not.pdf.txt");
        std::fs::write(&file, "test").unwrap();
        assert!(open_pdf(&file).is_err());
    }
}

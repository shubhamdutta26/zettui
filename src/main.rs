//! Zettui Lesson 5: browse, edit, link notes and insert Zotero citations.

mod bibliography;
mod config;
mod editor;
mod file_ops;
mod links;

use std::{env, fs, io, path::PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode, EditorView, LineNumbers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::bibliography::Bibliography;
use crate::config::Config;
use crate::editor::OpenNote;
use crate::links::{Index, LinkItem};

/// Temporary, single-line prompt shown over the file browser.
#[derive(Clone)]
enum PromptKind {
    NewNote,
    NewFolder,
    Rename { source: PathBuf },
    Move { source: PathBuf },
    Trash { source: PathBuf },
}

struct Prompt {
    kind: PromptKind,
    input: String,
}

/// A modal showing outgoing links or backlinks. Esc closes without changing notes.
struct LinkDialog {
    title: String,
    items: Vec<LinkItem>,
    selected: usize,
}

/// A searchable, read-only view of the JSON export loaded when the picker opens.
struct CitationDialog {
    bibliography: Bibliography,
    query: String,
    matches: Vec<usize>,
    selected: usize,
}

impl CitationDialog {
    fn new(bibliography: Bibliography) -> Self {
        let mut picker = Self {
            bibliography,
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
        };
        picker.filter();
        picker
    }

    fn filter(&mut self) {
        self.matches = self.bibliography.search(&self.query, 200);
        self.selected = 0;
    }
}

struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

/// The file browser and editor share one application state.
struct App {
    root: PathBuf,
    current_dir: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
    preview: String,
    editor: Option<OpenNote>,
    editor_keys: EditorEventHandler,
    command: Option<String>,
    prompt: Option<Prompt>,
    links: Option<LinkDialog>,
    citation_picker: Option<CitationDialog>,
    bibliography_path: Option<PathBuf>,
    history: Vec<PathBuf>,
    message: String,
}

impl App {
    fn new(root: PathBuf, bibliography_path: Option<PathBuf>) -> io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Notes path must be a directory",
            ));
        }
        let mut app = Self {
            root: root.clone(),
            current_dir: root,
            entries: Vec::new(),
            selected: 0,
            preview: String::new(),
            editor: None,
            editor_keys: EditorEventHandler::default(),
            command: None,
            prompt: None,
            links: None,
            citation_picker: None,
            bibliography_path,
            history: Vec::new(),
            message: String::new(),
        };
        app.refresh()?;
        Ok(app)
    }

    fn refresh(&mut self) -> io::Result<()> {
        let previously_selected = self
            .entries
            .get(self.selected)
            .map(|entry| entry.path.clone());
        let previous_index = self.selected;
        self.entries = fs::read_dir(&self.current_dir)?
            .filter_map(|item| {
                let item = item.ok()?;
                let kind = item.file_type().ok()?;
                if kind.is_symlink() || item.file_name() == ".git" {
                    return None;
                }
                Some(Entry {
                    name: item.file_name().to_string_lossy().into_owned(),
                    path: item.path(),
                    is_dir: kind.is_dir(),
                })
            })
            .collect();
        self.entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.selected = previously_selected
            .and_then(|path| self.entries.iter().position(|item| item.path == path))
            .unwrap_or_else(|| previous_index.min(self.entries.len().saturating_sub(1)));
        self.update_preview();
        Ok(())
    }

    fn select_path(&mut self, path: &std::path::Path) {
        if let Some(index) = self.entries.iter().position(|item| item.path == path) {
            self.selected = index;
            self.update_preview();
        }
    }

    fn update_preview(&mut self) {
        self.preview = match self.entries.get(self.selected) {
            None => "This directory is empty.".to_owned(),
            Some(entry) if entry.is_dir => {
                format!("Directory: {}\n\nPress Enter to open it.", entry.name)
            }
            Some(entry) => match fs::metadata(&entry.path) {
                Ok(info) if info.len() > 256_000 => {
                    "File too large to preview (256 KB limit).".to_owned()
                }
                Ok(_) => fs::read_to_string(&entry.path)
                    .unwrap_or_else(|e| format!("Cannot read UTF-8 text: {e}")),
                Err(e) => format!("Cannot inspect file: {e}"),
            },
        };
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            self.update_preview();
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.update_preview();
        }
    }

    fn enter(&mut self) -> io::Result<()> {
        let Some(entry) = self.entries.get(self.selected) else {
            return Ok(());
        };
        let path = entry.path.clone();
        if entry.is_dir {
            self.current_dir = path;
            self.refresh()?;
        } else {
            match OpenNote::load(path) {
                Ok(note) => {
                    self.editor = Some(note);
                    self.editor_keys = EditorEventHandler::default();
                    self.command = None;
                    self.message = "Editor opened. Press i to insert text.".to_owned();
                }
                Err(e) => self.message = format!("Cannot edit: {e}"),
            }
        }
        Ok(())
    }

    fn go_back(&mut self) -> io::Result<()> {
        if self.current_dir != self.root {
            if let Some(parent) = self.current_dir.parent() {
                let child = self.current_dir.clone();
                self.current_dir = parent.to_path_buf();
                self.refresh()?;
                self.select_path(&child);
            }
        }
        Ok(())
    }

    /// Render either the read-only preview or the editor in the right pane.
    fn draw(&mut self, frame: &mut Frame) {
        let areas =
            Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(frame.area());
        let panes = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(areas[0]);

        let files: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let prefix = if entry.is_dir { "[D] " } else { "    " };
                ListItem::new(format!("{prefix}{}", entry.name))
            })
            .collect();
        let list = List::new(files)
            .block(Block::bordered().title("Files"))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> ");
        let mut state = ListState::default();
        if !self.entries.is_empty() {
            state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, panes[0], &mut state);

        let status = if let Some(note) = &mut self.editor {
            let filename = note
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("note");
            let dirty = if note.is_dirty() { " [+]" } else { "" };
            let title = format!("Editing: {filename}{dirty}");
            let border = Block::bordered().title(title);
            let inner = border.inner(panes[1]);
            frame.render_widget(border, panes[1]);
            frame.render_widget(
                EditorView::new(&mut note.state)
                    .wrap(true)
                    .line_numbers(LineNumbers::Absolute),
                inner,
            );
            if let Some(cursor) = note.state.cursor_screen_position() {
                frame.set_cursor_position((cursor.x, cursor.y));
            }
            let instruction = if let Some(command) = &self.command {
                format!(":{command}   (Enter to run, Esc to cancel)")
            } else {
                "Esc: Normal | :w save | :q close | Ctrl+P cite | Ctrl+O links | Ctrl+B backlinks | Ctrl+T back".to_owned()
            };
            format!(
                "{} | {}{}\n{}\n{}",
                note.path.display(),
                note.state.mode.name(),
                dirty,
                instruction,
                self.message
            )
        } else {
            let preview = Paragraph::new(self.preview.as_str())
                .block(Block::bordered().title("Preview (press Enter to edit)"))
                .wrap(Wrap { trim: false });
            frame.render_widget(preview, panes[1]);
            format!(
                "{}\nj/k move | Enter open | h parent | o links | b backlinks | n note | N folder | r rename | m move | d Trash | q quit\n{}",
                self.current_dir.display(),
                self.message
            )
        };
        frame.render_widget(Paragraph::new(status), areas[1]);

        if let Some(prompt) = &self.prompt {
            let screen = frame.area();
            let width = screen.width.saturating_sub(2).min(74);
            let height = screen.height.saturating_sub(2).min(8);
            let popup = Rect::new(
                screen.x + (screen.width - width) / 2,
                screen.y + (screen.height - height) / 2,
                width,
                height,
            );
            let (title, help, input) = match &prompt.kind {
                PromptKind::NewNote => (
                    "Create note",
                    "Single filename. Add .json, .txt, etc. or leave blank for .md",
                    prompt.input.clone(),
                ),
                PromptKind::NewFolder => (
                    "Create folder",
                    "Enter a folder name without / or ..",
                    prompt.input.clone(),
                ),
                PromptKind::Rename { .. } => (
                    "Rename selected",
                    "Enter a new filename (including its extension)",
                    prompt.input.clone(),
                ),
                PromptKind::Move { .. } => (
                    "Move selected",
                    "Existing folder RELATIVE TO NOTES ROOT (e.g. Readings or .)",
                    prompt.input.clone(),
                ),
                PromptKind::Trash { source } => (
                    "Move to Trash?",
                    "Press y to confirm, n or Esc to cancel",
                    source
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "item".to_owned()),
                ),
            };
            let contents = format!(
                "{help}\n\n> {input}{}\n\n{}",
                if matches!(&prompt.kind, PromptKind::Trash { .. }) {
                    ""
                } else {
                    "▏"
                },
                if matches!(&prompt.kind, PromptKind::Trash { .. }) {
                    "Trash is recoverable using your desktop file manager"
                } else {
                    "Enter: confirm   Esc: cancel   Ctrl+U: clear"
                }
            );
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(contents)
                    .block(Block::bordered().title(title))
                    .wrap(Wrap { trim: false }),
                popup,
            );
        }
        if self.links.is_some() {
            self.draw_link_dialog(frame);
        }
        if self.citation_picker.is_some() {
            self.draw_citation_picker(frame);
        }
    }

    /// Display links on demand. Only this dialog captures j/k/Enter/Esc.
    fn draw_link_dialog(&mut self, frame: &mut Frame) {
        let Some(dialog) = &self.links else {
            return;
        };
        let screen = frame.area();
        let width = screen.width.saturating_sub(2).min(94);
        let height = screen.height.saturating_sub(2).min(18);
        if width < 12 || height < 5 {
            return;
        }
        let popup = Rect::new(
            screen.x + (screen.width - width) / 2,
            screen.y + (screen.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, popup);
        let block = Block::bordered().title(dialog.title.as_str());
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
        let entries: Vec<ListItem> = dialog
            .items
            .iter()
            .map(|item| {
                let symbol = if item.target.is_some() { "  " } else { "! " };
                ListItem::new(format!("{symbol}{}", item.label))
            })
            .collect();
        let list = List::new(entries)
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> ");
        let mut state = ListState::default();
        if !dialog.items.is_empty() {
            state.select(Some(dialog.selected));
        }
        frame.render_stateful_widget(list, areas[0], &mut state);
        frame.render_widget(
            Paragraph::new(
                "j/k select | Enter open | Esc close\n! means missing or ambiguous; no auto-create",
            ),
            areas[1],
        );
    }

    fn draw_citation_picker(&self, frame: &mut Frame) {
        let Some(dialog) = &self.citation_picker else {
            return;
        };
        let screen = frame.area();
        let width = screen.width.saturating_sub(2).min(110);
        let height = screen.height.saturating_sub(2).min(22);
        if width < 18 || height < 7 {
            return;
        }
        let popup = Rect::new(
            screen.x + (screen.width - width) / 2,
            screen.y + (screen.height - height) / 2,
            width,
            height,
        );
        frame.render_widget(Clear, popup);
        let block = Block::bordered().title("Zotero citations — type to search");
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let areas = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);
        frame.render_widget(
            Paragraph::new(format!("Search: {}▏", dialog.query)),
            areas[0],
        );
        let results: Vec<ListItem> = dialog
            .matches
            .iter()
            .map(|&index| ListItem::new(dialog.bibliography.references[index].summary()))
            .collect();
        let results = List::new(results)
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> ");
        let mut state = ListState::default();
        if !dialog.matches.is_empty() {
            state.select(Some(dialog.selected));
        }
        frame.render_stateful_widget(results, areas[1], &mut state);
        let count = dialog.matches.len();
        frame.render_widget(
            Paragraph::new(format!(
                "{count} results (max 200) | Up/Down select | Enter insert | Esc cancel
Ctrl+U clear search | Backspace delete"
            )),
            areas[2],
        );
    }

    /// Re-read the file every time Ctrl+P is pressed, so Zotero auto-exports
    /// become available immediately without restarting the application.
    fn open_citation_picker(&mut self) {
        if self.editor.is_none() {
            self.message = "Open a note before inserting a citation.".to_owned();
            return;
        }
        let Some(path) = &self.bibliography_path else {
            self.message = "No bibliography configured. Set bibliography in ~/.config/zettui/config.toml or pass --bibliography FILE".to_owned();
            return;
        };
        match Bibliography::load(path) {
            Ok(bibliography) => {
                self.citation_picker = Some(CitationDialog::new(bibliography));
                self.message.clear();
            }
            Err(e) => {
                self.message = format!("Cannot load bibliography {}: {e}", path.display());
            }
        }
    }

    fn insert_citation(&mut self, key: &str) {
        let Some(note) = &mut self.editor else {
            return;
        };
        if note.state.mode != EditorMode::Normal && note.state.mode != EditorMode::Insert {
            self.message =
                "Switch to Normal or Insert mode before inserting a citation.".to_owned();
            return;
        }
        let started_in_normal_mode = note.state.mode == EditorMode::Normal;
        // Use editor key events, not direct text replacement. EdTUI owns the
        // cursor, change history and undo stack.
        if started_in_normal_mode {
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
                &mut note.state,
            );
        }
        for character in format!("[@{key}]").chars() {
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                &mut note.state,
            );
        }
        if started_in_normal_mode {
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                &mut note.state,
            );
        }
        self.message = format!("Inserted [@{key}] — use :w to save your note.");
    }

    fn handle_citation_key(&mut self, key: KeyEvent) {
        let Some(mut dialog) = self.citation_picker.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                self.message = "Citation picker closed.".to_owned();
            }
            KeyCode::Up => {
                dialog.selected = dialog.selected.saturating_sub(1);
                self.citation_picker = Some(dialog);
            }
            KeyCode::Down => {
                if dialog.selected + 1 < dialog.matches.len() {
                    dialog.selected += 1;
                }
                self.citation_picker = Some(dialog);
            }
            KeyCode::Enter => {
                if let Some(&index) = dialog.matches.get(dialog.selected) {
                    let citekey = dialog.bibliography.references[index].key.clone();
                    self.insert_citation(&citekey);
                } else {
                    self.message = "No matching reference. Try another search.".to_owned();
                    self.citation_picker = Some(dialog);
                }
            }
            KeyCode::Backspace => {
                dialog.query.pop();
                dialog.filter();
                self.citation_picker = Some(dialog);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                dialog.query.clear();
                dialog.filter();
                self.citation_picker = Some(dialog);
            }
            // Typing is search input, so j/k are not navigation while searching.
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                dialog.query.push(c);
                dialog.filter();
                self.citation_picker = Some(dialog);
            }
            _ => self.citation_picker = Some(dialog),
        }
    }

    /// Build an index only when requested; it does not change any note files.
    fn show_links(&mut self, backlinks: bool) {
        let source = if let Some(note) = &self.editor {
            if note.is_dirty() {
                self.message =
                    "Save with :w before following links; unsaved edits are protected.".to_owned();
                return;
            }
            note.path.clone()
        } else {
            let Some(entry) = self.entries.get(self.selected) else {
                self.message = "Select a text note first.".to_owned();
                return;
            };
            if entry.is_dir {
                self.message = "Select a note, not a directory.".to_owned();
                return;
            }
            entry.path.clone()
        };
        let result = Index::build(&self.root).and_then(|index| {
            if backlinks {
                index.backlinks(&source)
            } else {
                index.outgoing(&source)
            }
        });
        match result {
            Ok(items) if !items.is_empty() => {
                self.links = Some(LinkDialog {
                    title: format!(
                        "{}: {}",
                        if backlinks {
                            "Backlinks"
                        } else {
                            "Outgoing links"
                        },
                        source
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "note".to_owned())
                    ),
                    items,
                    selected: 0,
                });
                self.message.clear();
            }
            Ok(_) => {
                self.message = if backlinks {
                    "No backlinks found."
                } else {
                    "No outgoing links found."
                }
                .to_owned()
            }
            Err(e) => self.message = format!("Link scan failed: {e}"),
        }
    }

    /// Open a linked note; refuse to discard unsaved edits. History is kept
    /// only for note-to-note navigation (Ctrl+T), not ordinary browser entry.
    fn open_link_target(&mut self, target: PathBuf, record_history: bool) {
        if self.editor.as_ref().is_some_and(OpenNote::is_dirty) {
            self.message = "Unsaved changes: save with :w before opening another note.".to_owned();
            return;
        }
        if self.editor.as_ref().is_some_and(|note| note.path == target) {
            self.message = "Already editing this note.".to_owned();
            return;
        }
        // Load BEFORE changing the file browser or current editor.
        let new_note = match OpenNote::load(target.clone()) {
            Ok(note) => note,
            Err(e) => {
                self.message = format!("Cannot open linked note: {e}");
                return;
            }
        };
        if record_history {
            if let Some(old) = &self.editor {
                self.history.push(old.path.clone());
            }
        }
        if let Some(parent) = target.parent() {
            self.current_dir = parent.to_path_buf();
            if let Err(e) = self.refresh() {
                self.message = format!("Link opened, but browser refresh failed: {e}");
            }
            self.select_path(&target);
        }
        self.editor = Some(new_note);
        self.editor_keys = EditorEventHandler::default();
        self.command = None;
        self.message = format!("Opened linked note: {}", target.display());
    }

    fn previous_note(&mut self) {
        if self.editor.as_ref().is_some_and(OpenNote::is_dirty) {
            self.message = "Save with :w before returning to the previous note.".to_owned();
            return;
        }
        let Some(path) = self.history.last().cloned() else {
            self.message = "No previous linked note in history.".to_owned();
            return;
        };
        if path.exists() {
            self.history.pop();
            self.open_link_target(path, false);
        } else {
            self.message = "Previous note no longer exists; history unchanged.".to_owned();
        }
    }

    fn handle_link_key(&mut self, key: KeyEvent) {
        let Some(dialog) = &mut self.links else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.links = None;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if dialog.selected + 1 < dialog.items.len() {
                    dialog.selected += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if dialog.selected > 0 {
                    dialog.selected -= 1;
                }
            }
            KeyCode::Enter => {
                let item = dialog.items.get(dialog.selected).cloned();
                if let Some(item) = item {
                    if let Some(target) = item.target {
                        self.links = None;
                        self.open_link_target(target, true);
                    } else {
                        self.message = "Unresolved/ambiguous link. Use a folder-qualified name or unique note ID.".to_owned();
                    }
                }
            }
            _ => {}
        }
    }

    fn begin_prompt(&mut self, kind: PromptKind) {
        let input = match &kind {
            PromptKind::Rename { source } => source
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            _ => String::new(),
        };
        self.prompt = Some(Prompt { kind, input });
        self.message.clear();
    }

    fn selected_path(&mut self) -> Option<PathBuf> {
        match self.entries.get(self.selected) {
            Some(entry) => Some(entry.path.clone()),
            None => {
                self.message = "No file or folder is selected.".to_owned();
                None
            }
        }
    }

    fn run_prompt(&mut self, prompt: Prompt) {
        let result = match &prompt.kind {
            PromptKind::NewNote => file_ops::create_note(&self.current_dir, &prompt.input),
            PromptKind::NewFolder => file_ops::create_folder(&self.current_dir, &prompt.input),
            PromptKind::Rename { source } => file_ops::rename_entry(source, &prompt.input),
            PromptKind::Move { source } => file_ops::move_entry(&self.root, source, &prompt.input),
            PromptKind::Trash { source } => file_ops::send_to_trash(source).map(|_| source.clone()),
        };
        match result {
            Ok(result_path) => {
                let action = match prompt.kind {
                    PromptKind::NewNote => "Created note",
                    PromptKind::NewFolder => "Created folder",
                    PromptKind::Rename { .. } => "Renamed",
                    PromptKind::Move { .. } => "Moved",
                    PromptKind::Trash { .. } => "Sent to Trash",
                };
                match self.refresh() {
                    Ok(()) => {
                        self.select_path(&result_path);
                        self.message = format!(
                            "{action}: {}{}",
                            result_path.display(),
                            if action == "Renamed" || action == "Moved" {
                                " (existing links are NOT automatically updated)"
                            } else {
                                ""
                            }
                        );
                    }
                    Err(e) => self.message = format!("{action}, but browser refresh failed: {e}"),
                }
            }
            Err(e) => {
                self.message = format!("Action failed: {e}");
                // Keep the prompt and typed text, so the name can be fixed.
                self.prompt = Some(prompt);
            }
        }
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) {
        let Some(mut prompt) = self.prompt.take() else {
            return;
        };
        if matches!(&prompt.kind, PromptKind::Trash { .. }) {
            match key.code {
                KeyCode::Char('y') if key.modifiers.is_empty() => self.run_prompt(prompt),
                KeyCode::Char('n') | KeyCode::Esc | KeyCode::Enter => {
                    self.message = "Canceled Trash action.".to_owned();
                }
                _ => self.prompt = Some(prompt),
            }
            return;
        }
        match key.code {
            KeyCode::Esc => self.message = "Canceled.".to_owned(),
            KeyCode::Enter => self.run_prompt(prompt),
            KeyCode::Backspace => {
                prompt.input.pop();
                self.prompt = Some(prompt);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                prompt.input.clear();
                self.prompt = Some(prompt);
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                prompt.input.push(c);
                self.prompt = Some(prompt);
            }
            _ => self.prompt = Some(prompt),
        }
    }

    fn save_editor(&mut self) -> bool {
        let Some(note) = &mut self.editor else {
            return false;
        };
        match note.save() {
            Ok(true) => {
                self.message = "Saved successfully.".to_owned();
                true
            }
            Ok(false) => {
                self.message = "No changes to save.".to_owned();
                true
            }
            Err(e) => {
                self.message = format!("SAVE FAILED: {e}");
                false
            }
        }
    }

    fn close_editor(&mut self, discard_changes: bool) {
        if self.editor.as_ref().is_some_and(|note| note.is_dirty()) && !discard_changes {
            self.message = "Unsaved changes! Use :w to save or :q! to discard.".to_owned();
            return;
        }
        self.editor = None;
        self.command = None;
        self.update_preview();
        self.message = "Returned to file browser.".to_owned();
    }

    /// Minimal Vim-style application commands (separate from edtui's text motions).
    fn run_command(&mut self, command: String) {
        match command.trim() {
            "w" => {
                self.save_editor();
            }
            "q" => self.close_editor(false),
            "q!" => self.close_editor(true),
            "wq" => {
                if self.save_editor() {
                    self.close_editor(false);
                }
            }
            other => self.message = format!("Unknown command :{other}"),
        }
    }

    fn handle_editor_key(&mut self, key: KeyEvent) {
        // Once a colon command is started, keys go into the command bar.
        if let Some(command) = &mut self.command {
            match key.code {
                KeyCode::Esc => {
                    self.command = None;
                }
                KeyCode::Enter => {
                    let cmd = self.command.take().unwrap_or_default();
                    self.run_command(cmd);
                }
                KeyCode::Backspace => {
                    command.pop();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    command.push(c)
                }
                _ => {}
            }
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('o') => {
                    self.show_links(false);
                    return;
                }
                KeyCode::Char('b') => {
                    self.show_links(true);
                    return;
                }
                KeyCode::Char('t') => {
                    self.previous_note();
                    return;
                }
                KeyCode::Char('p') => {
                    self.open_citation_picker();
                    return;
                }
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            self.save_editor();
            return;
        }

        // Only intercept ':' in Normal mode; it is literal text in Insert mode.
        if self
            .editor
            .as_ref()
            .is_some_and(|note| note.state.mode == EditorMode::Normal)
            && key.code == KeyCode::Char(':')
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            self.command = Some(String::new());
            return;
        }

        if let Some(note) = &mut self.editor {
            self.editor_keys.on_key_event(key, &mut note.state);
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if self.links.is_some() {
                self.handle_link_key(key);
                continue;
            }
            if self.citation_picker.is_some() {
                self.handle_citation_key(key);
                continue;
            }
            if self.prompt.is_some() {
                self.handle_prompt_key(key);
                continue;
            }
            if self.editor.is_some() {
                self.handle_editor_key(key);
                continue;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('o') => self.show_links(false),
                KeyCode::Char('b') => self.show_links(true),
                KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.previous_note()
                }
                KeyCode::Char('n') => self.begin_prompt(PromptKind::NewNote),
                KeyCode::Char('N') => self.begin_prompt(PromptKind::NewFolder),
                KeyCode::Char('r') => {
                    if let Some(source) = self.selected_path() {
                        self.begin_prompt(PromptKind::Rename { source });
                    }
                }
                KeyCode::Char('m') => {
                    if let Some(source) = self.selected_path() {
                        self.begin_prompt(PromptKind::Move { source });
                    }
                }
                KeyCode::Char('d') => {
                    if let Some(source) = self.selected_path() {
                        self.begin_prompt(PromptKind::Trash { source });
                    }
                }
                KeyCode::Char('R') => {
                    if let Err(e) = self.refresh() {
                        self.message = format!("Refresh failed: {e}");
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => self.move_down(),
                KeyCode::Char('k') | KeyCode::Up => self.move_up(),
                KeyCode::Char('l') | KeyCode::Enter | KeyCode::Right => self.enter()?,
                KeyCode::Char('h') | KeyCode::Backspace | KeyCode::Left => self.go_back()?,
                _ => {}
            }
        }
    }
}

/// Command line syntax:
///   zettui [NOTES_DIR] [--bibliography /path/to/references.json]
/// Configuration acts as a default; explicit CLI arguments win.
fn arguments() -> io::Result<(PathBuf, Option<PathBuf>)> {
    let config = Config::load()?;
    let mut notes_dir = None;
    let mut bibliography = None;
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let flag = arg.to_string_lossy().into_owned();
        match flag.as_str() {
            "--bibliography" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--bibliography requires a path",
                    )
                })?;
                bibliography = Some(PathBuf::from(value));
            }
            "-h" | "--help" => {
                println!(
                    "zettui [NOTES_DIR] [--bibliography FILE]\n\nConfiguration: ~/.config/zettui/config.toml"
                );
                std::process::exit(0);
            }
            other if other.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown option: {other}"),
                ));
            }
            _ if notes_dir.is_none() => notes_dir = Some(PathBuf::from(arg)),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Provide at most one NOTES_DIR",
                ));
            }
        }
    }
    Ok((
        notes_dir
            .or(config.notes_dir)
            .unwrap_or(env::current_dir()?),
        bibliography.or(config.bibliography),
    ))
}

fn main() -> io::Result<()> {
    let (root, bibliography) = arguments()?;
    let mut app = App::new(root, bibliography)?;
    ratatui::run(|terminal| app.run(terminal))
}

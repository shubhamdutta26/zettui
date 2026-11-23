//! Zettui Lesson 3: browse, edit, and manage local notes safely.

mod editor;
mod file_ops;

use std::{env, fs, io, path::PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode, EditorView, LineNumbers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::editor::OpenNote;

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
    message: String,
}

impl App {
    fn new(root: PathBuf) -> io::Result<Self> {
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
                "Esc: Normal | :w save | :q close | :q! discard | :wq save+close | Ctrl+S save"
                    .to_owned()
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
                "{}\nj/k move | Enter open | h parent | n note | N folder | r rename | m move | d Trash | R refresh | q quit\n{}",
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
                        self.message = format!("{action}: {}", result_path.display());
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

fn main() -> io::Result<()> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let mut app = App::new(path)?;
    ratatui::run(|terminal| app.run(terminal))
}

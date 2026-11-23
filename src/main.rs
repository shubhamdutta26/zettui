//! Zettui Lesson 2: a file browser plus an embedded Vim-style text editor.

mod editor;

use std::{env, fs, io, path::PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode, EditorView, LineNumbers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::editor::OpenNote;

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
            message: String::new(),
        };
        app.refresh()?;
        Ok(app)
    }

    fn refresh(&mut self) -> io::Result<()> {
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
        self.selected = 0;
        self.update_preview();
        Ok(())
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
                self.current_dir = parent.to_path_buf();
                self.refresh()?;
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
                "{}\nj/k: move | Enter/l: open | h: parent | q: quit\n{}",
                self.current_dir.display(),
                self.message
            )
        };
        frame.render_widget(Paragraph::new(status), areas[1]);
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
            if self.editor.is_some() {
                self.handle_editor_key(key);
                continue;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
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

//! Zettui lesson 1: read-only directory browser and text file preview.
//! Future lessons add editing, links, citations, themes, and configuration.

use std::{env, fs, io, path::PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

/// One file or directory shown in the left pane.
struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

/// All the information our UI needs to remember.
struct App {
    root: PathBuf,
    current_dir: PathBuf,
    entries: Vec<Entry>,
    selected: usize,
    preview: String,
}

impl App {
    fn new(root: PathBuf) -> io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The notes path must be a directory",
            ));
        }
        let mut app = Self {
            root: root.clone(),
            current_dir: root,
            entries: Vec::new(),
            selected: 0,
            preview: String::new(),
        };
        app.refresh()?;
        Ok(app)
    }

    fn refresh(&mut self) -> io::Result<()> {
        self.entries = fs::read_dir(&self.current_dir)?
            .filter_map(|item| {
                let item = item.ok()?;
                let kind = item.file_type().ok()?;
                // Avoid following symlinks; skip the Git internals.
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

        // Directories first, then alphabetical names.
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
                format!("Directory: {}\n\nPress Enter or l to open it.", entry.name)
            }
            Some(entry) => {
                // Limit previews to avoid pulling huge files into the TUI.
                match fs::metadata(&entry.path) {
                    Ok(info) if info.len() > 256_000 => {
                        "File too large to preview (limit: 256 KB).".to_owned()
                    }
                    Ok(_) => fs::read_to_string(&entry.path).unwrap_or_else(|err| {
                        format!("Cannot display this file as UTF-8 text: {err}")
                    }),
                    Err(err) => format!("Cannot inspect file: {err}"),
                }
            }
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
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                self.current_dir = entry.path.clone();
                self.refresh()?;
            }
        }
        Ok(())
    }

    fn go_back(&mut self) -> io::Result<()> {
        // Never navigate above the chosen notes root.
        if self.current_dir != self.root {
            if let Some(parent) = self.current_dir.parent() {
                self.current_dir = parent.to_path_buf();
                self.refresh()?;
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let areas = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());
        let panes = Layout::horizontal([
            Constraint::Percentage(35),
            Constraint::Percentage(65),
        ])
        .split(areas[0]);

        let files: Vec<ListItem> = self.entries.iter().map(|entry| {
            let prefix = if entry.is_dir { "[D] " } else { "    " };
            ListItem::new(format!("{prefix}{}", entry.name))
        }).collect();
        let list = List::new(files)
            .block(Block::bordered().title("Files"))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("> ");
        let mut state = ListState::default();
        if !self.entries.is_empty() {
            state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, panes[0], &mut state);

        let preview = Paragraph::new(self.preview.as_str())
            .block(Block::bordered().title("Preview (read-only)"))
            .wrap(Wrap { trim: false });
        frame.render_widget(preview, panes[1]);

        let status = format!(
            "{}\n j/k: move   Enter/l: open dir   h: parent   q: quit",
            self.current_dir.display()
        );
        frame.render_widget(Paragraph::new(status), areas[1]);
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;
            let Event::Key(key) = event::read()? else { continue };
            if key.kind != KeyEventKind::Press {
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
    // This helper handles terminal setup and restoration, even on errors.
    ratatui::run(|terminal| app.run(terminal))
}

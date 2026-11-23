//! Zettui: local notes, Vim editing, wiki-link completion, Zotero, and themes.

mod bibliography;
mod config;
mod editor;
mod file_ops;
mod links;
mod theme;

use std::{env, fs, io, path::PathBuf, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode, EditorTheme, EditorView, LineNumbers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::bibliography::Bibliography;
use crate::config::Config;
use crate::editor::OpenNote;
use crate::links::{Index, LinkItem, NoteSuggestion};
use crate::theme::{
    Appearance, ColorOverrides, THEME_CHOICES, Theme, ThemeChoice, omarchy_appearance,
};

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

/// Insert-mode wiki-link completion, opened immediately after typing `[[`.
/// Search text belongs to this dialog; the editor still contains literal `[[`.
struct WikiCompletionDialog {
    suggestions: Vec<NoteSuggestion>,
    query: String,
    matches: Vec<usize>,
    selected: usize,
}

impl WikiCompletionDialog {
    fn new(suggestions: Vec<NoteSuggestion>) -> Self {
        let mut dialog = Self {
            suggestions,
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
        };
        dialog.filter();
        dialog
    }

    fn filter(&mut self) {
        let query = self.query.to_lowercase();
        self.matches = self
            .suggestions
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.label.to_lowercase().contains(&query)
                    || item.link.to_lowercase().contains(&query)
            })
            .take(200)
            .map(|(index, _)| index)
            .collect();
        self.selected = 0;
    }
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

/// A non-destructive filename finder limited to the current directory.
/// It never touches file contents and only changes the browser selection.
struct FindDialog {
    query: String,
    matches: Vec<usize>,
    selected: usize,
}

impl FindDialog {
    fn new(entries: &[Entry]) -> Self {
        let mut dialog = Self {
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
        };
        dialog.filter(entries);
        dialog
    }

    fn filter(&mut self, entries: &[Entry]) {
        let query = self.query.to_lowercase();
        self.matches = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.name.to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect();
        self.selected = 0;
    }
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
    wiki_completion: Option<WikiCompletionDialog>,
    pending_open_bracket: bool,
    find_dialog: Option<FindDialog>,
    bibliography_path: Option<PathBuf>,
    theme: Theme,
    theme_choice: ThemeChoice,
    fallback_appearance: Appearance,
    theme_overrides: ColorOverrides,
    history: Vec<PathBuf>,
    message: String,
}

impl App {
    fn new(
        root: PathBuf,
        bibliography_path: Option<PathBuf>,
        theme_choice: ThemeChoice,
        theme_overrides: ColorOverrides,
        fallback_appearance: Appearance,
    ) -> io::Result<Self> {
        let appearance = theme_choice.resolve(omarchy_appearance().unwrap_or(fallback_appearance));
        let theme = Theme::named(appearance.as_str())
            .and_then(|theme| theme.with_overrides(&theme_overrides))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
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
            wiki_completion: None,
            pending_open_bracket: false,
            find_dialog: None,
            bibliography_path,
            theme,
            theme_choice,
            fallback_appearance,
            theme_overrides,
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
                    self.pending_open_bracket = false;
                    self.wiki_completion = None;
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

    /// Re-evaluate the Omarchy theme only when Auto is enabled. This is called
    /// periodically even without keyboard input, so theme switches are live.
    fn sync_system_theme(&mut self) {
        if self.theme_choice != ThemeChoice::Auto {
            return;
        }
        let appearance = omarchy_appearance().unwrap_or(self.fallback_appearance);
        if self.theme.name != appearance.as_str() {
            if let Ok(theme) = Theme::named(appearance.as_str())
                .and_then(|t| t.with_overrides(&self.theme_overrides))
            {
                self.theme = theme;
            }
        }
    }

    /// Auto -> Dark -> Light -> Auto. This never changes local settings.
    fn cycle_theme(&mut self) {
        let next = self.theme_choice.next();
        let appearance = next.resolve(omarchy_appearance().unwrap_or(self.fallback_appearance));
        match Theme::named(appearance.as_str())
            .and_then(|theme| theme.with_overrides(&self.theme_overrides))
        {
            Ok(theme) => {
                self.theme_choice = next;
                self.theme = theme;
                self.message = format!(
                    "Theme mode: {} ({}) — session only. Set theme = \"{}\" in config.toml.",
                    next.as_str(),
                    appearance.as_str(),
                    next.as_str()
                );
            }
            Err(error) => self.message = format!("Theme error: {error}"),
        }
    }

    /// All panels share one palette, including EdTUI's cursor and selection.
    fn draw(&mut self, frame: &mut Frame) {
        let theme = self.theme;
        frame.render_widget(Block::default().style(theme.base_style()), frame.area());
        let areas = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());
        let panes = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(areas[1]);

        let location = if self.editor.is_some() {
            "EDITOR"
        } else {
            "BROWSER"
        };
        let header = Line::from(vec![
            Span::styled(
                format!(" ZETTUI v{} ", env!("CARGO_PKG_VERSION")),
                Style::default()
                    .fg(theme.background)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {location}  "),
                Style::default().fg(theme.foreground),
            ),
            Span::styled(
                format!(
                    "theme: {} ({})   Ctrl+G: mode",
                    self.theme_choice.as_str(),
                    theme.name
                ),
                Style::default().fg(theme.muted),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(header).style(Style::default().bg(theme.surface)),
            areas[0],
        );

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let prefix = if entry.is_dir { "[D] " } else { "    " };
                let color = if entry.is_dir {
                    theme.directory
                } else {
                    theme.foreground
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix.to_owned(), Style::default().fg(color)),
                    Span::styled(entry.name.clone(), Style::default().fg(color)),
                ]))
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(" Files ")
                    .border_style(Style::default().fg(theme.border))
                    .title_style(Style::default().fg(theme.accent)),
            )
            .highlight_style(Style::default().bg(theme.selected_bg).fg(theme.selected_fg))
            .highlight_symbol("❯ ");
        let mut state = ListState::default();
        if !self.entries.is_empty() {
            state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, panes[0], &mut state);

        let footer = if let Some(note) = &mut self.editor {
            let filename = note
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("note");
            let dirty = if note.is_dirty() { " [+]" } else { "" };
            let title = format!(" {filename}{dirty} ");
            let border = Block::bordered()
                .title(title)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(if note.is_dirty() {
                    theme.warning
                } else {
                    theme.accent
                }));
            let inner = border.inner(panes[1]);
            frame.render_widget(border, panes[1]);
            let editor_theme = EditorTheme::default()
                .base(theme.base_style())
                .line_numbers_style(
                    Style::default()
                        .fg(theme.muted)
                        .bg(ratatui::style::Color::Reset),
                )
                .selection_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg))
                .cursor_style(Style::default().fg(theme.background).bg(theme.accent))
                .hide_status_line();
            frame.render_widget(
                EditorView::new(&mut note.state)
                    .wrap(true)
                    .line_numbers(LineNumbers::Absolute)
                    .theme(editor_theme),
                inner,
            );
            if let Some(cursor) = note.state.cursor_screen_position() {
                frame.set_cursor_position((cursor.x, cursor.y));
            }
            let help = if let Some(command) = &self.command {
                format!(":{command}   Enter: run | Esc: cancel")
            } else {
                "Insert: [[ link | Esc Normal | :w save | Ctrl+P cite | Ctrl+O links | Ctrl+B backlinks | Ctrl+T back".to_owned()
            };
            format!(
                "{} | {}{}\n{}\n{}",
                note.path.display(),
                note.state.mode.name(),
                dirty,
                help,
                self.message,
            )
        } else {
            let preview = Paragraph::new(self.preview.as_str())
                .block(
                    Block::bordered()
                        .title(" Preview — Enter to edit ")
                        .border_style(Style::default().fg(theme.border))
                        .title_style(Style::default().fg(theme.accent)),
                )
                .style(theme.base_style())
                .wrap(Wrap { trim: false });
            frame.render_widget(preview, panes[1]);
            format!(
                "{}\nj/k move | Enter open | h parent | o links | b backlinks | n/N create | r rename | m move | d Trash | / find | Ctrl+G mode | q quit\n{}",
                self.current_dir.display(),
                self.message,
            )
        };
        frame.render_widget(
            Paragraph::new(footer).style(Style::default().fg(theme.foreground).bg(theme.surface)),
            areas[2],
        );

        if let Some(prompt) = &self.prompt {
            let screen = frame.area();
            let width = screen.width.saturating_sub(2).min(74);
            let height = screen.height.saturating_sub(2).min(8);
            if width >= 12 && height >= 5 {
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
                        .block(
                            Block::bordered()
                                .title(title)
                                .border_style(Style::default().fg(theme.accent)),
                        )
                        .style(Style::default().fg(theme.foreground).bg(theme.surface))
                        .wrap(Wrap { trim: false }),
                    popup,
                );
            }
        }
        if self.links.is_some() {
            self.draw_link_dialog(frame);
        }
        if self.citation_picker.is_some() {
            self.draw_citation_picker(frame);
        }
        if self.find_dialog.is_some() {
            self.draw_find_dialog(frame);
        }
        if self.wiki_completion.is_some() {
            self.draw_wiki_completion(frame);
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
        let theme = self.theme;
        let block = Block::bordered()
            .title(dialog.title.as_str())
            .border_style(Style::default().fg(theme.accent));
        let inner = block.inner(popup);
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.surface)),
            popup,
        );
        frame.render_widget(block, popup);
        let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
        let entries: Vec<ListItem> = dialog
            .items
            .iter()
            .map(|item| {
                let symbol = if item.target.is_some() { "  " } else { "! " };
                let color = if item.target.is_some() {
                    theme.foreground
                } else {
                    theme.warning
                };
                ListItem::new(Line::from(vec![
                    Span::styled(symbol.to_owned(), Style::default().fg(color)),
                    Span::styled(item.label.clone(), Style::default().fg(color)),
                ]))
            })
            .collect();
        let list = List::new(entries)
            .highlight_style(Style::default().bg(theme.selected_bg).fg(theme.selected_fg))
            .highlight_symbol("> ");
        let mut state = ListState::default();
        if !dialog.items.is_empty() {
            state.select(Some(dialog.selected));
        }
        frame.render_stateful_widget(list, areas[0], &mut state);
        frame.render_widget(
            Paragraph::new(
                "j/k select | Enter open | Esc close\n! means missing or ambiguous; no auto-create",
            )
            .style(Style::default().fg(theme.muted).bg(theme.surface)),
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
        let theme = self.theme;
        let block = Block::bordered()
            .title("Zotero citations — type to search")
            .border_style(Style::default().fg(theme.accent));
        let inner = block.inner(popup);
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.surface)),
            popup,
        );
        frame.render_widget(block, popup);
        let areas = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);
        frame.render_widget(
            Paragraph::new(format!("Search: {}▏", dialog.query))
                .style(Style::default().fg(theme.accent).bg(theme.surface)),
            areas[0],
        );
        let results: Vec<ListItem> = dialog
            .matches
            .iter()
            .map(|&index| {
                ListItem::new(Line::styled(
                    dialog.bibliography.references[index].summary(),
                    Style::default().fg(theme.foreground),
                ))
            })
            .collect();
        let results = List::new(results)
            .highlight_style(Style::default().bg(theme.selected_bg).fg(theme.selected_fg))
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
            ))
            .style(Style::default().fg(theme.muted).bg(theme.surface)),
            areas[2],
        );
    }

    fn draw_wiki_completion(&self, frame: &mut Frame) {
        let Some(dialog) = &self.wiki_completion else {
            return;
        };
        let theme = self.theme;
        let screen = frame.area();
        let width = screen.width.saturating_sub(2).min(100);
        let height = screen.height.saturating_sub(2).min(20);
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
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.surface)),
            popup,
        );
        let block = Block::bordered()
            .title(" Link a note — [[ ")
            .border_style(Style::default().fg(theme.accent));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let areas = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);
        frame.render_widget(
            Paragraph::new(format!("Search note: {}▏", dialog.query))
                .style(Style::default().fg(theme.accent).bg(theme.surface)),
            areas[0],
        );
        let items: Vec<ListItem> = dialog
            .matches
            .iter()
            .map(|index| {
                let item = &dialog.suggestions[*index];
                ListItem::new(Line::styled(
                    item.label.clone(),
                    Style::default().fg(theme.foreground),
                ))
            })
            .collect();
        let mut state = ListState::default();
        if !dialog.matches.is_empty() {
            state.select(Some(dialog.selected));
        }
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg))
                .highlight_symbol("> "),
            areas[1],
            &mut state,
        );
        frame.render_widget(
            Paragraph::new(format!(
                "{} matches (max 200) | Type to filter | Up/Down select | Enter insert\nEsc cancel (keeps [[) | Backspace erase | Ctrl+U clear",
                dialog.matches.len()
            )).style(Style::default().fg(theme.muted).bg(theme.surface)), areas[2]
        );
    }

    fn draw_find_dialog(&self, frame: &mut Frame) {
        let Some(dialog) = &self.find_dialog else {
            return;
        };
        let theme = self.theme;
        let screen = frame.area();
        let width = screen.width.saturating_sub(2).min(84);
        let height = screen.height.saturating_sub(2).min(18);
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
        frame.render_widget(
            Block::default().style(Style::default().bg(theme.surface)),
            popup,
        );
        let block = Block::bordered()
            .title(" Find in current folder ")
            .border_style(Style::default().fg(theme.accent));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let areas = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);
        frame.render_widget(
            Paragraph::new(format!("Name: {}▏", dialog.query))
                .style(Style::default().fg(theme.accent).bg(theme.surface)),
            areas[0],
        );
        let items: Vec<ListItem> = dialog
            .matches
            .iter()
            .map(|index| {
                let entry = &self.entries[*index];
                let color = if entry.is_dir {
                    theme.directory
                } else {
                    theme.foreground
                };
                ListItem::new(Line::styled(entry.name.clone(), Style::default().fg(color)))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg))
            .highlight_symbol("❯ ");
        let mut state = ListState::default();
        if !dialog.matches.is_empty() {
            state.select(Some(dialog.selected));
        }
        frame.render_stateful_widget(list, areas[1], &mut state);
        frame.render_widget(
            Paragraph::new(format!(
                "{} matches | Up/Down: select | Enter: jump to file\nEsc: cancel | Ctrl+U: clear",
                dialog.matches.len()
            ))
            .style(Style::default().fg(theme.muted).bg(theme.surface)),
            areas[2],
        );
    }

    fn handle_find_key(&mut self, key: KeyEvent) {
        let Some(mut dialog) = self.find_dialog.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.message = "Filename search canceled.".to_owned(),
            KeyCode::Enter => {
                if let Some(&index) = dialog.matches.get(dialog.selected) {
                    let path = self.entries[index].path.clone();
                    self.select_path(&path);
                    self.message = format!("Selected {}. Press Enter to open.", path.display());
                } else {
                    self.message = "No matching filename in this folder.".to_owned();
                    self.find_dialog = Some(dialog);
                }
            }
            KeyCode::Up => {
                dialog.selected = dialog.selected.saturating_sub(1);
                self.find_dialog = Some(dialog);
            }
            KeyCode::Down => {
                if dialog.selected + 1 < dialog.matches.len() {
                    dialog.selected += 1;
                }
                self.find_dialog = Some(dialog);
            }
            KeyCode::Backspace => {
                dialog.query.pop();
                dialog.filter(&self.entries);
                self.find_dialog = Some(dialog);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                dialog.query.clear();
                dialog.filter(&self.entries);
                self.find_dialog = Some(dialog);
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                dialog.query.push(c);
                dialog.filter(&self.entries);
                self.find_dialog = Some(dialog);
            }
            _ => self.find_dialog = Some(dialog),
        }
    }

    /// Build the index on demand so newly created or renamed notes are visible.
    fn open_wiki_completion(&mut self) {
        let Some(note) = &self.editor else {
            return;
        };
        if note.state.mode != EditorMode::Insert {
            return;
        }
        let source = note.path.clone();
        match Index::build(&self.root) {
            Ok(index) => {
                let suggestions = index.suggestions(&source);
                if suggestions.is_empty() {
                    self.message = "No resolvable notes to link. Type a link manually.".to_owned();
                } else {
                    self.wiki_completion = Some(WikiCompletionDialog::new(suggestions));
                    self.message.clear();
                }
            }
            Err(e) => self.message = format!("Cannot scan notes for links: {e}"),
        }
    }

    /// The editor already contains the opening `[[` characters. Add only the
    /// resolved target and closing `]]`, through EdTUI for correct undo history.
    fn insert_wiki_completion(&mut self, link: &str) {
        let Some(note) = &mut self.editor else {
            return;
        };
        if note.state.mode != EditorMode::Insert {
            return;
        }
        for character in format!("{link}]]").chars() {
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                &mut note.state,
            );
        }
        self.message = format!("Inserted [[{link}]] — use :w to save.");
    }

    fn handle_wiki_completion_key(&mut self, key: KeyEvent) {
        let Some(mut dialog) = self.wiki_completion.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                self.message = "Link picker canceled; [[ remains in the note.".to_owned();
            }
            KeyCode::Up => {
                dialog.selected = dialog.selected.saturating_sub(1);
                self.wiki_completion = Some(dialog);
            }
            KeyCode::Down => {
                if dialog.selected + 1 < dialog.matches.len() {
                    dialog.selected += 1;
                }
                self.wiki_completion = Some(dialog);
            }
            KeyCode::Enter => {
                if let Some(&index) = dialog.matches.get(dialog.selected) {
                    let link = dialog.suggestions[index].link.clone();
                    self.insert_wiki_completion(&link);
                } else {
                    self.message = "No matching note. Change the search or press Esc.".to_owned();
                    self.wiki_completion = Some(dialog);
                }
            }
            KeyCode::Backspace => {
                dialog.query.pop();
                dialog.filter();
                self.wiki_completion = Some(dialog);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                dialog.query.clear();
                dialog.filter();
                self.wiki_completion = Some(dialog);
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                dialog.query.push(c);
                dialog.filter();
                self.wiki_completion = Some(dialog);
            }
            _ => self.wiki_completion = Some(dialog),
        }
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
        self.pending_open_bracket = false;
        self.wiki_completion = None;
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
        self.wiki_completion = None;
        self.pending_open_bracket = false;
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
        // Only two consecutive `[` keystrokes in Insert mode trigger a picker.
        // A navigation key, modifier shortcut or editor command resets it.
        if key.code != KeyCode::Char('[')
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            self.pending_open_bracket = false;
        }
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
                KeyCode::Char('g') => {
                    self.cycle_theme();
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

        let opening_bracket = self.editor.as_ref().is_some_and(|note| {
            note.state.mode == EditorMode::Insert
                && note
                    .path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown")
                    })
                && key.code == KeyCode::Char('[')
                && !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        });
        let open_picker = opening_bracket && self.pending_open_bracket;
        if let Some(note) = &mut self.editor {
            self.editor_keys.on_key_event(key, &mut note.state);
        }
        self.pending_open_bracket = opening_bracket && !open_picker;
        if open_picker {
            self.open_wiki_completion();
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            self.sync_system_theme();
            terminal.draw(|frame| self.draw(frame))?;
            // Poll instead of waiting forever: Omarchy may change its theme
            // while the user isn't pressing any keys.
            if !event::poll(Duration::from_millis(400))? {
                continue;
            }
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
            if self.wiki_completion.is_some() {
                self.handle_wiki_completion_key(key);
                continue;
            }
            if self.prompt.is_some() {
                self.handle_prompt_key(key);
                continue;
            }
            if self.find_dialog.is_some() {
                self.handle_find_key(key);
                continue;
            }
            if self.editor.is_some() {
                self.handle_editor_key(key);
                continue;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
                self.cycle_theme();
                continue;
            }
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('o') => self.show_links(false),
                KeyCode::Char('b') => self.show_links(true),
                KeyCode::Char('/') => {
                    self.find_dialog = Some(FindDialog::new(&self.entries));
                }
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
///   zettui [NOTES_DIR] [--bibliography PATH] [--theme auto|dark|light]
/// Configuration acts as a default; explicit CLI arguments win.
fn arguments() -> io::Result<(
    PathBuf,
    Option<PathBuf>,
    ThemeChoice,
    ColorOverrides,
    Appearance,
)> {
    let config = Config::load()?;
    let mut notes_dir = None;
    let mut bibliography = None;
    let mut theme_override = None;
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
            "--theme" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--theme requires a name")
                })?;
                theme_override = Some(value.to_string_lossy().into_owned());
            }
            "-h" | "--help" => {
                println!(
                    "zettui [NOTES_DIR] [--bibliography FILE] [--theme auto|dark|light]\n\nModes: {}\nConfig: ~/.config/zettui/config.toml",
                    THEME_CHOICES.join(", "),
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
    let name = theme_override
        .or(config.theme)
        .unwrap_or_else(|| "auto".to_owned());
    let choice = ThemeChoice::named(&name)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    let system = omarchy_appearance().unwrap_or_else(|| {
        // On other Linux desktops, use the terminal background at launch.
        // OSC color queries run here, before Ratatui enables raw input mode.
        match terminal_colorsaurus::theme_mode(terminal_colorsaurus::QueryOptions::default()) {
            Ok(terminal_colorsaurus::ThemeMode::Light) => Appearance::Light,
            _ => Appearance::Dark,
        }
    });
    Ok((
        notes_dir
            .or(config.notes_dir)
            .unwrap_or(env::current_dir()?),
        bibliography.or(config.bibliography),
        choice,
        config.colors,
        system,
    ))
}

fn main() -> io::Result<()> {
    let (root, bibliography, choice, overrides, fallback) = arguments()?;
    let mut app = App::new(root, bibliography, choice, overrides, fallback)?;
    ratatui::run(|terminal| app.run(terminal))
}

#[cfg(test)]
mod lesson6_tests {
    use super::*;

    #[test]
    fn wiki_completion_filters_by_path_title_and_id() {
        let mut dialog = WikiCompletionDialog::new(vec![
            NoteSuggestion {
                label: "Humanization — Antibody/VHH.md [123]".into(),
                link: "Antibody/VHH".into(),
            },
            NoteSuggestion {
                label: "SEC — Analytics/SEC.md [456]".into(),
                link: "Analytics/SEC".into(),
            },
        ]);
        assert_eq!(dialog.matches.len(), 2);
        dialog.query = "human".into();
        dialog.filter();
        assert_eq!(dialog.matches, vec![0]);
        dialog.query = "analytics".into();
        dialog.filter();
        assert_eq!(dialog.matches, vec![1]);
    }

    #[test]
    fn double_bracket_in_insert_mode_inserts_selected_existing_note() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Current.md");
        fs::write(&source, "# Current\n").unwrap();
        fs::write(dir.path().join("Target.md"), "# Target\n").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
        )
        .unwrap();
        app.select_path(&source);
        app.enter().unwrap();
        app.handle_editor_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        for _ in 0..2 {
            app.handle_editor_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        }
        assert!(app.wiki_completion.is_some());
        app.handle_wiki_completion_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.wiki_completion.is_none());
        assert!(
            app.editor
                .as_ref()
                .unwrap()
                .state
                .lines
                .to_string()
                .contains("[[Target]]")
        );
        assert!(app.editor.as_ref().unwrap().is_dirty());
    }

    #[test]
    fn canceling_wiki_completion_leaves_literal_brackets() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("Current.md");
        fs::write(&source, "# Current\n").unwrap();
        fs::write(dir.path().join("Target.md"), "# Target\n").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
        )
        .unwrap();
        app.select_path(&source);
        app.enter().unwrap();
        app.handle_editor_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        for _ in 0..2 {
            app.handle_editor_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        }
        app.handle_wiki_completion_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.wiki_completion.is_none());
        let text = app.editor.as_ref().unwrap().state.lines.to_string();
        assert!(text.contains("[["));
        assert!(!text.contains("[[Target]]"));
    }

    #[test]
    fn filename_search_is_case_insensitive_and_non_destructive() {
        let entries = vec![
            Entry {
                name: "CD4-SEC.md".into(),
                path: "CD4-SEC.md".into(),
                is_dir: false,
            },
            Entry {
                name: "VHH.json".into(),
                path: "VHH.json".into(),
                is_dir: false,
            },
            Entry {
                name: "Antibody".into(),
                path: "Antibody".into(),
                is_dir: true,
            },
        ];
        let mut dialog = FindDialog::new(&entries);
        dialog.query = "cd4".into();
        dialog.filter(&entries);
        assert_eq!(dialog.matches, vec![0]);
        dialog.query = "ANTIBODY".into();
        dialog.filter(&entries);
        assert_eq!(dialog.matches, vec![2]);
        assert_eq!(entries[0].name, "CD4-SEC.md");
    }
}

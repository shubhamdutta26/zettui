//! Zettui: local notes, Vim editing, wiki-link completion, Zotero, and themes.

mod bibliography;
mod config;
mod editor;
mod editor_search;
mod file_ops;
mod layout;
mod links;
mod search;
mod snippets;
mod theme;

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode, EditorTheme, EditorView, LineNumbers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap},
};

use crate::bibliography::Bibliography;
use crate::config::Config;
use crate::editor::OpenNote;
use crate::editor_search::{Direction, NoteSearch};
use crate::layout::{DEFAULT_BROWSER_PERCENT, resize_browser_percent, split_panes};
use crate::links::{Index, LinkItem, NoteSuggestion};
use crate::search::{SearchHit, SearchMode, VaultIndex};
use crate::snippets::{Field, Snippet, load_snippets};
use crate::theme::{
    Appearance, AutoThemes, ColorOverrides, THEME_CHOICES, Theme, ThemeChoice, omarchy_appearance,
};
use crossterm::cursor::SetCursorStyle;
use edtui::Index2;

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

struct SnippetPicker {
    items: Vec<Snippet>,
    query: String,
    matches: Vec<usize>,
    selected: usize,
}

impl SnippetPicker {
    fn new(items: Vec<Snippet>) -> Self {
        let mut picker = Self {
            items,
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
        };
        picker.filter();
        picker
    }

    fn filter(&mut self) {
        let query = self.query.to_lowercase();
        self.matches = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
    }
}

/// Each numbered snippet field is edited here before inserting the template.
/// This keeps existing note contents safe and avoids partially expanded YAML.
struct SnippetForm {
    snippet: Snippet,
    fields: Vec<Field>,
    active: usize,
    replace_on_type: bool,
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

/// Interactive picker for searching text lines or YAML frontmatter tags.
/// The index is loaded from disk when the picker opens; typing only filters RAM.
struct VaultSearchDialog {
    index: VaultIndex,
    mode: SearchMode,
    query: String,
    matches: Vec<SearchHit>,
    selected: usize,
}

impl VaultSearchDialog {
    fn new(index: VaultIndex, mode: SearchMode) -> Self {
        Self {
            index,
            mode,
            query: String::new(),
            matches: Vec::new(),
            selected: 0,
        }
    }

    fn filter(&mut self) {
        self.matches = self.index.search(self.mode, &self.query, 200);
        self.selected = 0;
    }
}

/// Keep filenames on one terminal row, including names with control characters.
fn display_component(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { '�' } else { c })
        .collect()
}

/// The workspace name followed by directory components relative to the notes root.
fn breadcrumb_parts(root: &Path, directory: &Path) -> Vec<String> {
    let label = |path: &Path| display_component(&path.to_string_lossy());
    match directory.strip_prefix(root) {
        Ok(relative) => {
            let mut parts = vec![
                root.file_name()
                    .map(|name| display_component(&name.to_string_lossy()))
                    .unwrap_or_else(|| label(root)),
            ];
            parts.extend(
                relative
                    .components()
                    .map(|part| display_component(&part.as_os_str().to_string_lossy())),
            );
            parts
        }
        // Do not misrepresent a directory outside the workspace as root-relative.
        Err(_) => directory
            .components()
            .map(|part| display_component(&part.as_os_str().to_string_lossy()))
            .collect(),
    }
}

/// Shorten a single label from the left, measuring terminal cells rather than bytes.
fn tail_label(label: &str, width: usize) -> String {
    if Span::raw(label).width() <= width {
        return label.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let mut start = label.len();
    for (index, _) in label.char_indices().rev() {
        if Span::raw(&label[index..]).width() + 1 > width {
            break;
        }
        start = index;
    }
    format!("…{}", &label[start..])
}

/// Hide whole ancestors first; always retain as much of the current folder as fits.
fn fit_breadcrumb(parts: &[String], width: usize) -> Vec<String> {
    const SEPARATOR: &str = " › ";
    if width == 0 || parts.is_empty() {
        return Vec::new();
    }
    let separator_width = Span::raw(SEPARATOR).width();
    let full_width = parts
        .iter()
        .map(|p| Span::raw(p.as_str()).width())
        .sum::<usize>()
        + separator_width * parts.len().saturating_sub(1);
    if full_width <= width {
        return parts.to_vec();
    }
    let last = parts.last().expect("nonempty breadcrumb");
    let last_width = Span::raw(last.as_str()).width();
    let omitted_width = 1 + separator_width;
    if last_width + omitted_width > width {
        return vec![tail_label(last, width)];
    }
    let mut first = parts.len() - 1;
    let mut used = last_width + omitted_width;
    while first > 0 {
        let extra = Span::raw(parts[first - 1].as_str()).width() + separator_width;
        if used + extra > width {
            break;
        }
        used += extra;
        first -= 1;
    }
    let mut result = vec!["…".to_owned()];
    result.extend_from_slice(&parts[first..]);
    result
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
    note_search: NoteSearch,
    prompt: Option<Prompt>,
    links: Option<LinkDialog>,
    citation_picker: Option<CitationDialog>,
    snippet_picker: Option<SnippetPicker>,
    snippet_form: Option<SnippetForm>,
    snippets_dir: PathBuf,
    wiki_completion: Option<WikiCompletionDialog>,
    pending_open_bracket: bool,
    find_dialog: Option<FindDialog>,
    vault_search: Option<VaultSearchDialog>,
    bibliography_path: Option<PathBuf>,
    theme: Theme,
    theme_choice: ThemeChoice,
    fallback_appearance: Appearance,
    auto_themes: AutoThemes,
    theme_overrides: ColorOverrides,
    browser_width_percent: u16,
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
        auto_themes: AutoThemes,
    ) -> io::Result<Self> {
        let appearance = auto_themes.resolve(
            theme_choice,
            omarchy_appearance().unwrap_or(fallback_appearance),
        );
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
            note_search: NoteSearch::default(),
            prompt: None,
            links: None,
            citation_picker: None,
            snippet_picker: None,
            snippet_form: None,
            snippets_dir: snippets::default_snippets_dir(),
            wiki_completion: None,
            pending_open_bracket: false,
            find_dialog: None,
            vault_search: None,
            bibliography_path,
            theme,
            theme_choice,
            fallback_appearance,
            auto_themes,
            theme_overrides,
            browser_width_percent: DEFAULT_BROWSER_PERCENT,
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
        let appearance = self.auto_themes.resolve(
            self.theme_choice,
            omarchy_appearance().unwrap_or(self.fallback_appearance),
        );
        if self.theme.name != appearance.as_str() {
            if let Ok(theme) = Theme::named(appearance.as_str())
                .and_then(|t| t.with_overrides(&self.theme_overrides))
            {
                self.theme = theme;
            }
        }
    }

    /// Cycle all built-in palettes. This never changes local settings.
    fn cycle_theme(&mut self) {
        let next = self.theme_choice.next();
        let appearance = self.auto_themes.resolve(
            next,
            omarchy_appearance().unwrap_or(self.fallback_appearance),
        );
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
    fn resize_columns(&mut self, key: KeyEvent) -> bool {
        if key.modifiers != KeyModifiers::ALT {
            return false;
        }
        let grow = match key.code {
            KeyCode::Char('h') | KeyCode::Left => false,
            KeyCode::Char('l') | KeyCode::Right => true,
            _ => return false,
        };
        self.browser_width_percent = resize_browser_percent(self.browser_width_percent, grow);
        self.message = format!(
            "Files: {}% | Notes: {}% (session only; set browser_width_percent in config.toml)",
            self.browser_width_percent,
            100 - self.browser_width_percent,
        );
        true
    }

    /// Check whether a modal window currently has input focus.
    fn has_active_popup(&self) -> bool {
        self.prompt.is_some()
            || self.links.is_some()
            || self.citation_picker.is_some()
            || self.snippet_picker.is_some()
            || self.snippet_form.is_some()
            || self.wiki_completion.is_some()
            || self.find_dialog.is_some()
            || self.vault_search.is_some()
    }

    fn draw(&mut self, frame: &mut Frame) {
        let show_editor_cursor = !self.has_active_popup() && self.note_search.input.is_none();
        let theme = self.theme;
        frame.render_widget(Block::default().style(theme.base_style()), frame.area());
        let areas = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());
        let panes = split_panes(areas[1], self.browser_width_percent);

        let browser_active = self.editor.is_none() && !self.has_active_popup();
        let editor_active = self.editor.is_some() && !self.has_active_popup();
        // Show the actual directory of the open note, or the current browser folder.
        let directory = self
            .editor
            .as_ref()
            .and_then(|note| note.path.parent())
            .unwrap_or(&self.current_dir);
        let parts = breadcrumb_parts(&self.root, directory);
        let mut header = vec![
            Span::styled(
                " Zettui",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" v{}  /  ", env!("CARGO_PKG_VERSION")),
                Style::default().fg(theme.muted),
            ),
        ];
        let prefix_width = header.iter().map(Span::width).sum::<usize>();
        let available = usize::from(areas[0].width).saturating_sub(prefix_width);
        let visible = fit_breadcrumb(&parts, available);
        for (index, part) in visible.iter().enumerate() {
            if index > 0 {
                header.push(Span::styled(" › ", Style::default().fg(theme.muted)));
            }
            let style = if part == "…" {
                Style::default().fg(theme.muted)
            } else if index + 1 == visible.len() {
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            };
            header.push(Span::styled(part.as_str(), style));
        }
        frame.render_widget(Paragraph::new(Line::from(header)), areas[0]);

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let prefix = if entry.is_dir { "▸ " } else { "  " };
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
                    .padding(Padding::horizontal(1))
                    .border_style(Style::default().fg(if browser_active {
                        theme.accent
                    } else {
                        theme.border
                    }))
                    .title_style(Style::default().fg(if browser_active {
                        theme.accent
                    } else {
                        theme.muted
                    })),
            )
            .highlight_style(Style::default().bg(theme.selected_bg).fg(theme.selected_fg))
            .highlight_symbol("▏ ");
        let mut state = ListState::default();
        if !self.entries.is_empty() {
            state.select(Some(self.selected));
        }
        frame.render_stateful_widget(list, panes[0], &mut state);

        let popup_active = self.has_active_popup();
        let (mode, location, help, stats, dirty_state) = if let Some(note) = &mut self.editor {
            let filename = note
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("note")
                .to_owned();
            let modified = note.is_dirty();
            let dirty = if modified { " [+]" } else { "" };
            let title = format!(" {filename}{dirty} ");
            let border = Block::bordered()
                .title(title)
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(if editor_active {
                    theme.accent
                } else {
                    theme.border
                }))
                .title_style(Style::default().fg(if modified {
                    theme.warning
                } else if editor_active {
                    theme.accent
                } else {
                    theme.muted
                }));
            let inner = border.inner(panes[1]);
            frame.render_widget(border, panes[1]);
            let editor_theme = EditorTheme::default()
                .base(theme.base_style())
                .line_numbers_style(
                    Style::default().fg(theme.muted).bg(theme
                        .base_style()
                        .bg
                        .unwrap_or(ratatui::style::Color::Reset)),
                )
                .selection_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg))
                .cursor_style(if note.state.mode == EditorMode::Insert {
                    // Neutralize EdTUI's painted block cursor so the real
                    // terminal can render its blinking vertical bar.
                    theme.base_style()
                } else {
                    Style::default().fg(theme.background).bg(theme.accent)
                })
                .hide_status_line();
            frame.render_widget(
                EditorView::new(&mut note.state)
                    .wrap(true)
                    .line_numbers(LineNumbers::Absolute)
                    .theme(editor_theme),
                inner,
            );
            // Only show the hardware cursor when the editor has input focus.
            if show_editor_cursor {
                if let Some(cursor) = note.state.cursor_screen_position() {
                    frame.set_cursor_position((cursor.x, cursor.y));
                }
            }
            let mode = if self.note_search.input.is_some() {
                "SEARCH".to_owned()
            } else if self.command.is_some() {
                "COMMAND".to_owned()
            } else {
                note.state.mode.name().to_uppercase()
            };
            let help = if let Some(input) = &self.note_search.input {
                format!(
                    "{}{}▏  [{}/{}] | Enter confirm | Esc cancel",
                    input.direction.prefix(),
                    input.query,
                    self.note_search.active,
                    self.note_search.count
                )
            } else if let Some(command) = &self.command {
                format!(":{command}▏   Enter run | Esc cancel")
            } else if mode == "NORMAL" {
                "i Insert | v Visual | / Find | :w Save | :q Close | Ctrl+F Search | Alt+T Tags"
                    .to_owned()
            } else if mode == "INSERT" {
                "Esc Normal | Ctrl+S Save | Ctrl+P Cite | Ctrl+E Snippet | Ctrl+F Search".to_owned()
            } else {
                "Esc Normal | Ctrl+S Save | Ctrl+F Search | Alt+T Tags".to_owned()
            };
            let stats = format!(
                "{} words  │  Ln {}, Col {}",
                note.state.lines.to_string().split_whitespace().count(),
                note.state.cursor.row + 1,
                note.state.cursor.col + 1
            );
            (mode, filename, help, stats, Some(modified))
        } else {
            let preview = Paragraph::new(self.preview.as_str())
                .block(
                    Block::bordered()
                        .title(" Preview ")
                        .padding(Padding::horizontal(1))
                        .border_style(Style::default().fg(theme.border))
                        .title_style(Style::default().fg(theme.muted)),
                )
                .style(theme.base_style())
                .wrap(Wrap { trim: false });
            frame.render_widget(preview, panes[1]);
            (
                "BROWSER".to_owned(),
                {
                    let relative = self
                        .current_dir
                        .strip_prefix(&self.root)
                        .unwrap_or(&self.current_dir);
                    if relative.as_os_str().is_empty() {
                        ".".to_owned()
                    } else {
                        display_component(&relative.to_string_lossy())
                    }
                },
                "Enter Open | n New note | N New folder | / Find | Ctrl+F Search | q Quit"
                    .to_owned(),
                String::new(),
                None,
            )
        };
        let mode = if popup_active {
            "DIALOG"
        } else {
            mode.as_str()
        };
        let mode_color = match mode {
            "INSERT" => theme.directory,
            "VISUAL" => theme.foreground,
            "COMMAND" | "SEARCH" | "DIALOG" => theme.warning,
            "NORMAL" => theme.success,
            _ => theme.accent,
        };
        let mut status = vec![Span::styled(
            format!(" {mode} "),
            Style::default()
                .fg(theme.background)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        )];
        if let Some(modified) = dirty_state {
            status.push(Span::styled(
                if modified {
                    "  ● Modified"
                } else {
                    "  ✓ Saved"
                },
                Style::default().fg(if modified {
                    theme.warning
                } else {
                    theme.success
                }),
            ));
        }
        let location_width = usize::from(areas[2].width)
            .saturating_sub(status.iter().map(Span::width).sum::<usize>() + 2);
        if location_width > 0 {
            status.push(Span::styled(
                format!("  {}", tail_label(&location, location_width)),
                Style::default().fg(theme.muted),
            ));
        }
        let footer_rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(areas[2]);
        let status_line = Line::from(status);
        // Give the mode and save state priority on narrow terminals.
        let stats_line = Line::from(Span::styled(stats, Style::default().fg(theme.muted)));
        let stats_width = stats_line.width();
        if stats_width > 0
            && status_line.width() + stats_width + 2 <= usize::from(footer_rows[0].width)
        {
            let columns =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(stats_width as u16)])
                    .split(footer_rows[0]);
            frame.render_widget(
                Paragraph::new(status_line).style(theme.base_style()),
                columns[0],
            );
            frame.render_widget(
                Paragraph::new(stats_line).style(theme.base_style()),
                columns[1],
            );
        } else {
            frame.render_widget(
                Paragraph::new(status_line).style(theme.base_style()),
                footer_rows[0],
            );
        }
        let mut shortcuts = Vec::new();
        if popup_active {
            shortcuts.push(Span::styled(
                "See dialog for controls",
                Style::default().fg(theme.muted),
            ));
        } else if self.command.is_some() || self.note_search.input.is_some() {
            shortcuts.push(Span::styled(
                help.as_str(),
                Style::default().fg(theme.accent),
            ));
        } else {
            for (i, item) in help.split(" | ").enumerate() {
                if i > 0 {
                    shortcuts.push(Span::raw("   "));
                }
                let (key, description) = item.split_once(' ').unwrap_or((item, ""));
                shortcuts.push(Span::styled(
                    key,
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD),
                ));
                shortcuts.push(Span::styled(
                    format!(" {description}"),
                    Style::default().fg(theme.muted),
                ));
            }
        }
        frame.render_widget(
            Paragraph::new(Line::from(shortcuts)).style(theme.base_style()),
            footer_rows[1],
        );
        frame.render_widget(
            Paragraph::new(self.message.as_str()).style(theme.base_style()),
            footer_rows[2],
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
        if self.snippet_picker.is_some() {
            self.draw_snippet_picker(frame);
        }
        if self.snippet_form.is_some() {
            self.draw_snippet_form(frame);
        }
        if self.vault_search.is_some() {
            self.draw_vault_search(frame);
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

    fn draw_snippet_picker(&self, frame: &mut Frame) {
        let Some(dialog) = &self.snippet_picker else {
            return;
        };
        let theme = self.theme;
        let screen = frame.area();
        let width = screen.width.saturating_sub(2).min(86);
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
            .title(" Apply snippet — Ctrl+E ")
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
            Paragraph::new(format!("Search: {}▏", dialog.query))
                .style(Style::default().fg(theme.accent).bg(theme.surface)),
            areas[0],
        );
        let items: Vec<ListItem> = dialog
            .matches
            .iter()
            .map(|index| ListItem::new(dialog.items[*index].name.clone()))
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
            Paragraph::new(
                "Type to filter | Up/Down select | Enter choose\nEsc cancel | Ctrl+U clear",
            )
            .style(Style::default().fg(theme.muted).bg(theme.surface)),
            areas[2],
        );
    }

    fn draw_snippet_form(&self, frame: &mut Frame) {
        let Some(form) = &self.snippet_form else {
            return;
        };
        let theme = self.theme;
        let screen = frame.area();
        let width = screen.width.saturating_sub(2).min(88);
        let height = screen.height.saturating_sub(2).min(16);
        if width < 20 || height < 8 {
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
            .title(format!(" Snippet: {} ", form.snippet.name))
            .border_style(Style::default().fg(theme.accent));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(inner);
        let rows: Vec<Line> = form
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                Line::styled(
                    format!(
                        "{} ${}: {}{}",
                        if i == form.active { ">" } else { " " },
                        field.index,
                        field.value,
                        if i == form.active { "▏" } else { "" }
                    ),
                    Style::default().fg(if i == form.active {
                        theme.accent
                    } else {
                        theme.foreground
                    }),
                )
            })
            .collect();
        frame.render_widget(Paragraph::new(rows).wrap(Wrap { trim: false }), areas[0]);
        frame.render_widget(Paragraph::new(
            "Type to replace default | Tab/Enter next or apply\nShift+Tab previous | Ctrl+U clear | Esc cancel"
        ).style(Style::default().fg(theme.muted).bg(theme.surface)), areas[1]);
    }

    fn draw_vault_search(&self, frame: &mut Frame) {
        let Some(dialog) = &self.vault_search else {
            return;
        };
        let theme = self.theme;
        let screen = frame.area();
        let width = screen.width.saturating_sub(2).min(106);
        let height = screen.height.saturating_sub(2).min(23);
        if width < 24 || height < 7 {
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
            .title(format!(" Search vault — {} ", dialog.mode.label()))
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
            Paragraph::new(format!(
                "{} search: {}▏  (Tab changes mode)",
                dialog.mode.label(),
                dialog.query
            ))
            .style(Style::default().fg(theme.accent).bg(theme.surface)),
            areas[0],
        );
        let items: Vec<ListItem> = dialog
            .matches
            .iter()
            .map(|hit| {
                let relative = hit
                    .path
                    .strip_prefix(&dialog.index.root)
                    .unwrap_or(&hit.path);
                ListItem::new(format!(
                    "{}:{}  {}",
                    relative.display(),
                    hit.line,
                    hit.excerpt
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
                .highlight_symbol("❯ "),
            areas[1],
            &mut state,
        );
        frame.render_widget(Paragraph::new(format!(
            "{} matches | {} indexed | {} skipped{} | ↑↓ select Enter open Esc cancel\nTab toggle Text/Tags | Ctrl+U clear",
            dialog.matches.len(), dialog.index.count(), dialog.index.skipped,
            if dialog.index.truncated { " | file limit reached" } else { "" }
        )).style(Style::default().fg(theme.muted).bg(theme.surface)), areas[2]);
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

    fn open_vault_search(&mut self, mode: SearchMode) {
        match VaultIndex::load(&self.root) {
            Ok(index) => {
                let count = index.count();
                self.vault_search = Some(VaultSearchDialog::new(index, mode));
                self.message =
                    format!("Indexed {count} notes. Type a query or Tab to change search mode.");
            }
            Err(error) => self.message = format!("Cannot index notes: {error}"),
        }
    }

    fn open_search_hit(&mut self, hit: SearchHit) -> bool {
        if self
            .editor
            .as_ref()
            .is_some_and(|note| note.path != hit.path && note.is_dirty())
        {
            self.message =
                "Unsaved changes: save with :w before opening a search result.".to_owned();
            return false;
        }
        if !self
            .editor
            .as_ref()
            .is_some_and(|note| note.path == hit.path)
        {
            self.open_link_target(hit.path.clone(), true);
        }
        if let Some(note) = &mut self.editor {
            if note.path == hit.path {
                note.state.cursor = Index2::new(hit.line.saturating_sub(1), 0);
                note.state
                    .set_viewport_offset(0, hit.line.saturating_sub(2));
                self.message = format!("Search result: {}:{}", hit.path.display(), hit.line);
                return true;
            }
        }
        false
    }

    fn handle_vault_search_key(&mut self, key: KeyEvent) {
        let Some(mut dialog) = self.vault_search.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                self.message = "Vault search closed.".to_owned();
            }
            KeyCode::Tab => {
                dialog.mode = dialog.mode.toggle();
                dialog.filter();
                self.vault_search = Some(dialog);
            }
            KeyCode::Up => {
                dialog.selected = dialog.selected.saturating_sub(1);
                self.vault_search = Some(dialog);
            }
            KeyCode::Down => {
                if dialog.selected + 1 < dialog.matches.len() {
                    dialog.selected += 1;
                }
                self.vault_search = Some(dialog);
            }
            KeyCode::Enter => {
                if let Some(hit) = dialog.matches.get(dialog.selected).cloned() {
                    if !self.open_search_hit(hit) {
                        self.vault_search = Some(dialog);
                    }
                } else {
                    self.message = "No result selected; type a query or press Esc.".to_owned();
                    self.vault_search = Some(dialog);
                }
            }
            KeyCode::Backspace => {
                dialog.query.pop();
                dialog.filter();
                self.vault_search = Some(dialog);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                dialog.query.clear();
                dialog.filter();
                self.vault_search = Some(dialog);
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                dialog.query.push(ch);
                dialog.filter();
                self.vault_search = Some(dialog);
            }
            _ => self.vault_search = Some(dialog),
        }
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
    fn open_snippet_picker(&mut self) {
        let Some(note) = &self.editor else {
            return;
        };
        if !note.state.lines.to_string().is_empty() {
            self.message =
                "Snippets currently apply only to completely empty notes (no content changed)."
                    .to_owned();
            return;
        }
        match load_snippets(&self.snippets_dir) {
            Ok(items) if !items.is_empty() => {
                self.snippet_picker = Some(SnippetPicker::new(items));
                self.message.clear();
            }
            Ok(_) => self.message = format!("No snippets found in {}", self.snippets_dir.display()),
            Err(e) => self.message = format!("Snippet error: {e}"),
        }
    }

    fn apply_snippet(&mut self, snippet: &Snippet, fields: &[Field]) {
        let expanded = match snippets::expand(&snippet.source, fields, chrono::Local::now()) {
            Ok(value) => value,
            Err(e) => {
                self.message = format!("Snippet expansion failed: {e}");
                return;
            }
        };
        let Some(note) = &mut self.editor else {
            return;
        };
        // Never erase or merge with an existing note, even if it changed while
        // the dialog was open. All insertion goes through EdTUI's event handler.
        if !note.state.lines.to_string().is_empty() {
            self.message = "Note is no longer empty; snippet was not applied.".to_owned();
            return;
        }
        if note.state.mode != EditorMode::Insert {
            self.editor_keys.on_key_event(
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
                &mut note.state,
            );
        }
        for ch in expanded.text.chars() {
            let code = match ch {
                '\n' => KeyCode::Enter,
                '\t' => KeyCode::Tab,
                c => KeyCode::Char(c),
            };
            self.editor_keys
                .on_key_event(KeyEvent::new(code, KeyModifiers::NONE), &mut note.state);
        }
        note.state.cursor = Index2::new(expanded.cursor_row, expanded.cursor_col);
        self.pending_open_bracket = false;
        self.message = format!(
            "Inserted snippet '{}' (unsaved). Press Esc, then :w to save.",
            snippet.name
        );
    }

    fn handle_snippet_picker_key(&mut self, key: KeyEvent) {
        let Some(mut picker) = self.snippet_picker.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                self.message = "Snippet picker canceled.".to_owned();
            }
            KeyCode::Up => {
                picker.selected = picker.selected.saturating_sub(1);
                self.snippet_picker = Some(picker);
            }
            KeyCode::Down => {
                if picker.selected + 1 < picker.matches.len() {
                    picker.selected += 1;
                }
                self.snippet_picker = Some(picker);
            }
            KeyCode::Enter => {
                if let Some(&index) = picker.matches.get(picker.selected) {
                    let snippet = picker.items[index].clone();
                    match snippets::fields(&snippet.source) {
                        Ok(fields) if fields.is_empty() => self.apply_snippet(&snippet, &fields),
                        Ok(fields) => {
                            self.snippet_form = Some(SnippetForm {
                                snippet,
                                fields,
                                active: 0,
                                replace_on_type: true,
                            })
                        }
                        Err(e) => self.message = format!("Invalid snippet: {e}"),
                    }
                } else {
                    self.snippet_picker = Some(picker);
                }
            }
            KeyCode::Backspace => {
                picker.query.pop();
                picker.filter();
                self.snippet_picker = Some(picker);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                picker.query.clear();
                picker.filter();
                self.snippet_picker = Some(picker);
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                picker.query.push(ch);
                picker.filter();
                self.snippet_picker = Some(picker);
            }
            _ => self.snippet_picker = Some(picker),
        }
    }

    fn handle_snippet_form_key(&mut self, key: KeyEvent) {
        let Some(mut form) = self.snippet_form.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                self.message = "Snippet canceled (no changes made).".to_owned();
            }
            KeyCode::BackTab => {
                form.active = form.active.saturating_sub(1);
                form.replace_on_type = true;
                self.snippet_form = Some(form);
            }
            KeyCode::Enter | KeyCode::Tab => {
                if form.active + 1 < form.fields.len() {
                    form.active += 1;
                    form.replace_on_type = true;
                    self.snippet_form = Some(form);
                } else {
                    self.apply_snippet(&form.snippet, &form.fields);
                }
            }
            KeyCode::Backspace => {
                let field = &mut form.fields[form.active];
                if form.replace_on_type {
                    field.value.clear();
                } else {
                    field.value.pop();
                }
                form.replace_on_type = false;
                self.snippet_form = Some(form);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                form.fields[form.active].value.clear();
                form.replace_on_type = false;
                self.snippet_form = Some(form);
            }
            KeyCode::Char(ch)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let field = &mut form.fields[form.active];
                if form.replace_on_type {
                    field.value.clear();
                }
                field.value.push(ch);
                form.replace_on_type = false;
                self.snippet_form = Some(form);
            }
            _ => self.snippet_form = Some(form),
        }
    }

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

    fn handle_note_search_key(&mut self, key: KeyEvent) {
        let Some(note) = &mut self.editor else {
            self.note_search.input = None;
            return;
        };
        let style = Style::default()
            .fg(self.theme.selected_fg)
            .bg(self.theme.selected_bg);
        let current = Style::default()
            .fg(self.theme.background)
            .bg(self.theme.accent);
        match key.code {
            KeyCode::Esc => {
                self.note_search.cancel(&mut note.state);
                self.message = "Search canceled.".into();
                return;
            }
            KeyCode::Enter => {
                if let Some(input) = self.note_search.input.take() {
                    let query = if input.query.is_empty() {
                        self.note_search
                            .last
                            .as_ref()
                            .map(|(q, _)| q.clone())
                            .unwrap_or_default()
                    } else {
                        input.query
                    };
                    if !query.is_empty() {
                        self.note_search.jump(
                            &mut note.state,
                            &query,
                            input.origin,
                            input.direction,
                            style,
                            current,
                        );
                        self.note_search.last = Some((query, input.direction));
                    }
                }
                self.message = if self.note_search.count == 0 {
                    "No match.".into()
                } else {
                    format!(
                        "Match {}/{} | n next | N previous",
                        self.note_search.active, self.note_search.count
                    )
                };
                return;
            }
            KeyCode::Backspace => {
                if let Some(input) = &mut self.note_search.input {
                    input.query.pop();
                }
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(input) = &mut self.note_search.input {
                    input.query.clear();
                }
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                if let Some(input) = &mut self.note_search.input {
                    input.query.push(c);
                }
            }
            _ => return,
        }
        self.note_search.preview(&mut note.state, style, current);
    }

    fn intercept_note_search(&mut self, key: KeyEvent) -> bool {
        if self.note_search.input.is_some() {
            self.handle_note_search_key(key);
            return true;
        }
        let Some(note) = &mut self.editor else {
            return false;
        };
        if note.state.mode != EditorMode::Normal
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        match key.code {
            KeyCode::Char('/') | KeyCode::Char('?') => {
                let direction = if key.code == KeyCode::Char('/') {
                    Direction::Forward
                } else {
                    Direction::Backward
                };
                note.state.clear_highlights();
                self.note_search.begin(&note.state, direction);
                self.message.clear();
                true
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if let Some((query, direction)) = self.note_search.last.clone() {
                    let direction = if key.code == KeyCode::Char('N') {
                        direction.reverse()
                    } else {
                        direction
                    };
                    let origin = note.state.cursor.clone();
                    self.note_search.jump(
                        &mut note.state,
                        &query,
                        origin,
                        direction,
                        Style::default()
                            .fg(self.theme.selected_fg)
                            .bg(self.theme.selected_bg),
                        Style::default()
                            .fg(self.theme.background)
                            .bg(self.theme.accent),
                    );
                    self.message = if self.note_search.count == 0 {
                        "No match.".into()
                    } else {
                        format!(
                            "Match {}/{} | n next | N previous",
                            self.note_search.active, self.note_search.count
                        )
                    };
                } else {
                    self.message = "No previous search. Press / or ?.".into();
                }
                true
            }
            _ => false,
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

        if self.intercept_note_search(key) {
            return;
        }
        // Ordinary editing can invalidate the previous match coordinates.
        if let Some(note) = &mut self.editor {
            note.state.clear_highlights();
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
                KeyCode::Char('e') => {
                    self.open_snippet_picker();
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
        let result = self.run_loop(terminal);
        // Restore the cursor configured by the user in Foot/other terminals.
        let reset = crossterm::execute!(io::stdout(), SetCursorStyle::DefaultUserShape);
        match result {
            Err(error) => Err(error),
            Ok(()) => reset,
        }
    }

    fn run_loop(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut previous_insert_mode = None;
        loop {
            let in_insert = self
                .editor
                .as_ref()
                .is_some_and(|note| note.state.mode == EditorMode::Insert);
            if previous_insert_mode != Some(in_insert) {
                let shape = if in_insert {
                    SetCursorStyle::BlinkingBar
                } else {
                    SetCursorStyle::BlinkingBlock
                };
                crossterm::execute!(io::stdout(), shape)?;
                previous_insert_mode = Some(in_insert);
            }
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
            if self.snippet_form.is_some() {
                self.handle_snippet_form_key(key);
                continue;
            }
            if self.snippet_picker.is_some() {
                self.handle_snippet_picker_key(key);
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
            if self.vault_search.is_some() {
                self.handle_vault_search_key(key);
                continue;
            }
            if self.note_search.input.is_some() {
                self.handle_note_search_key(key);
                continue;
            }
            if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.open_vault_search(SearchMode::Text);
                continue;
            }
            if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::ALT) {
                self.open_vault_search(SearchMode::Tags);
                continue;
            }
            if self.resize_columns(key) {
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
                KeyCode::Enter => self.enter()?,

                KeyCode::Char('l') | KeyCode::Right => {
                    if self
                        .entries
                        .get(self.selected)
                        .is_some_and(|entry| entry.is_dir)
                    {
                        self.enter()?;
                    }
                }
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
    AutoThemes,
    u16,
    PathBuf,
)> {
    let config = Config::load()?;
    let browser_width = config.browser_width()?;
    let auto_themes = config.auto_themes()?;
    let snippets_dir = config
        .snippets_dir
        .clone()
        .unwrap_or_else(snippets::default_snippets_dir);
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
        auto_themes,
        browser_width,
        snippets_dir,
    ))
}

fn main() -> io::Result<()> {
    let (root, bibliography, choice, overrides, fallback, auto_themes, browser_width, snippets_dir) =
        arguments()?;
    let mut app = App::new(root, bibliography, choice, overrides, fallback, auto_themes)?;
    app.browser_width_percent = browser_width;
    app.snippets_dir = snippets_dir;
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
            AutoThemes::default(),
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
            AutoThemes::default(),
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

#[cfg(test)]
mod lesson73_app_tests {
    use super::*;
    #[test]
    fn hotkeys_resize_in_both_directions_without_editing_notes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Note.md");
        fs::write(&path, "# Original\n").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap();
        assert!(app.resize_columns(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT)));
        assert_eq!(app.browser_width_percent, 40);
        assert!(app.resize_columns(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT)));
        assert_eq!(app.browser_width_percent, 35);
        assert!(!app.resize_columns(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)));
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Original\n");
    }
}

#[cfg(test)]
mod lesson74_snippet_app_tests {
    use super::*;

    #[test]
    fn applies_template_to_empty_note_without_automatic_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Empty.md");
        fs::write(&path, "").unwrap();
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
        app.enter().unwrap();
        let snippet = Snippet {
            name: "test".into(),
            source: "---
title: ${1:Untitled}
id: $ZKN_ID
---

$0"
            .into(),
        };
        let mut fields = snippets::fields(&snippet.source).unwrap();
        fields[0].value = "New Note".into();
        app.apply_snippet(&snippet, &fields);
        let note = app.editor.as_ref().unwrap();
        assert!(note.is_dirty());
        assert!(note.state.lines.to_string().contains("title: New Note"));
        assert_eq!(fs::read_to_string(path).unwrap(), "");
    }

    #[test]
    fn refuses_template_on_existing_note() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Existing.md");
        fs::write(
            &path,
            "# Preserve me
",
        )
        .unwrap();
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
        app.enter().unwrap();
        app.apply_snippet(
            &Snippet {
                name: "test".into(),
                source: "Changed".into(),
            },
            &[],
        );
        assert_eq!(
            app.editor.as_ref().unwrap().state.lines.to_string(),
            "# Preserve me
"
        );
        assert!(!app.editor.as_ref().unwrap().is_dirty());
    }
}

#[cfg(test)]
mod lesson75_search_app_tests {
    use super::*;

    #[test]
    fn text_search_opens_note_and_closes_dialog() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("Target.md");
        fs::write(
            &target,
            "---\ntags: [vhh]\n---\n# Title\nFramework details\n",
        )
        .unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap();
        app.open_vault_search(SearchMode::Text);
        let dialog = app.vault_search.as_mut().unwrap();
        dialog.query = "framework".into();
        dialog.filter();
        assert_eq!(dialog.matches.len(), 1);
        app.handle_vault_search_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.vault_search.is_none());
        assert_eq!(app.editor.unwrap().path, target);
    }

    #[test]
    fn search_refuses_to_discard_unsaved_changes() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("First.md");
        let second = dir.path().join("Second.md");
        fs::write(&first, "# First\n").unwrap();
        fs::write(&second, "# Second\nFind a unique antibody\n").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap();
        app.select_path(&first);
        app.enter().unwrap();
        app.handle_editor_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        app.handle_editor_key(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        assert!(app.editor.as_ref().unwrap().is_dirty());
        app.open_vault_search(SearchMode::Text);
        let dialog = app.vault_search.as_mut().unwrap();
        dialog.query = "unique antibody".into();
        dialog.filter();
        assert_eq!(dialog.matches.len(), 1);
        app.handle_vault_search_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.vault_search.is_some());
        assert_eq!(app.editor.as_ref().unwrap().path, first);
    }
}

#[cfg(test)]
mod breadcrumb_tests {
    use super::*;

    #[test]
    fn workspace_and_nested_directory() {
        let root = Path::new("/home/sam/Notes");
        assert_eq!(breadcrumb_parts(root, root), vec!["Notes"]);
        assert_eq!(
            breadcrumb_parts(root, &root.join("Antibody/VHH")),
            vec!["Notes", "Antibody", "VHH"]
        );
        assert_eq!(
            breadcrumb_parts(root, Path::new("/tmp/Other")),
            vec!["/", "tmp", "Other"]
        );
    }

    #[test]
    fn long_paths_keep_current_folder() {
        let parts = ["Notes", "Antibody", "VHH"].map(str::to_owned);
        assert_eq!(fit_breadcrumb(&parts, 80), parts.to_vec());
        assert_eq!(fit_breadcrumb(&parts, 7), vec!["…", "VHH"]);
        assert_eq!(fit_breadcrumb(&parts, 2), vec!["…H"]);
        assert!(fit_breadcrumb(&parts, 0).is_empty());
    }

    #[test]
    fn unicode_and_control_characters_are_safe() {
        let parts = ["Notes", "抗体研究"].map(str::to_owned);
        for width in 0..25 {
            let fitted = fit_breadcrumb(&parts, width).join(" › ");
            assert!(Span::raw(fitted).width() <= width);
        }
        assert_eq!(display_component("a\nb"), "a�b");
    }
}

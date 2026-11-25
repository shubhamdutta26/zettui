//! Application state and coordination. Child modules share private state through App.

mod appearance;
mod browser;
mod dialogs;
mod editing;
mod events;
mod ui;

use std::{
    io,
    path::{Path, PathBuf},
};

use edtui::EditorEventHandler;
use ratatui::text::Span;

use crate::bibliography::Bibliography;
use crate::cli::{StartupOptions, arguments};
use crate::editor::OpenNote;
use crate::editor_search::NoteSearch;
use crate::layout::DEFAULT_BROWSER_PERCENT;
use crate::links::{LinkItem, NoteSuggestion};
use crate::search::{SearchHit, SearchMode, VaultIndex};
use crate::snippets::{Field, Snippet};
use crate::theme::{
    Appearance, AutoThemes, ColorOverrides, Theme, ThemeChoice, omarchy_appearance,
};
use crate::{focus, markdown, snippets, system_theme};

#[cfg(test)]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
#[cfg(test)]
use std::fs;

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
    system_theme: system_theme::Settings,
    terminal_palette: Option<system_theme::TerminalPalette>,
    last_theme_check: std::time::Instant,
    theme_snapshot: Option<system_theme::ThemeSnapshot>,
    fallback_appearance: Appearance,
    auto_themes: AutoThemes,
    theme_overrides: ColorOverrides,
    browser_width_percent: u16,
    writing: focus::Writing,
    markdown_settings: markdown::MarkdownSettings,
    highlight_cache: markdown::HighlightCache,
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
            system_theme: system_theme::Settings::default(),
            terminal_palette: None,
            last_theme_check: std::time::Instant::now(),
            theme_snapshot: None,
            fallback_appearance,
            auto_themes,
            theme_overrides,
            browser_width_percent: DEFAULT_BROWSER_PERCENT,
            writing: focus::Writing::default(),
            markdown_settings: markdown::MarkdownSettings::default(),
            highlight_cache: markdown::HighlightCache::default(),
            history: Vec::new(),
            message: String::new(),
        };
        app.refresh()?;
        Ok(app)
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
}

pub(crate) fn start() -> io::Result<()> {
    let StartupOptions {
        root,
        bibliography,
        choice,
        system_theme,
        terminal_palette,
        overrides,
        fallback,
        auto_themes,
        browser_width,
        snippets_dir,
        writing,
        markdown_settings,
    } = arguments()?;
    let mut app = App::new(root, bibliography, choice, overrides, fallback, auto_themes)?;
    app.system_theme = system_theme;
    app.terminal_palette = terminal_palette;
    app.theme = app
        .resolve_theme(choice)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    app.browser_width_percent = browser_width;
    app.snippets_dir = snippets_dir;
    app.writing = writing;
    app.markdown_settings = markdown_settings;
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

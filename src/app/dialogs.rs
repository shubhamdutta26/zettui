//! Search, completion, bibliography, snippets, links and file-operation prompts.

use super::{
    App, CitationDialog, LinkDialog, Prompt, PromptKind, SnippetForm, SnippetPicker,
    VaultSearchDialog, WikiCompletionDialog,
};
use crate::bibliography::Bibliography;
use crate::editor::OpenNote;
use crate::links::Index;
use crate::search::{SearchHit, SearchMode, VaultIndex};
use crate::snippets::{Field, Snippet, load_snippets};
use crate::{file_ops, snippets};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::{EditorEventHandler, EditorMode, Index2};
use std::path::PathBuf;

impl App {
    pub(super) fn open_vault_search(&mut self, mode: SearchMode) {
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
        if let Some(note) = &mut self.editor
            && note.path == hit.path
        {
            self.outline_view.focused = false;
            note.state.cursor = Index2::new(hit.line.saturating_sub(1), 0);
            note.state
                .set_viewport_offset(0, hit.line.saturating_sub(2));
            self.message = format!("Search result: {}:{}", hit.path.display(), hit.line);
            return true;
        }
        false
    }

    pub(super) fn handle_vault_search_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_find_key(&mut self, key: KeyEvent) {
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
    pub(super) fn open_wiki_completion(&mut self) {
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

    pub(super) fn handle_wiki_completion_key(&mut self, key: KeyEvent) {
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
    pub(super) fn open_snippet_picker(&mut self) {
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

    pub(super) fn apply_snippet(&mut self, snippet: &Snippet, fields: &[Field]) {
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

    pub(super) fn handle_snippet_picker_key(&mut self, key: KeyEvent) {
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

    pub(super) fn handle_snippet_form_key(&mut self, key: KeyEvent) {
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

    pub(super) fn open_citation_picker(&mut self) {
        if self.editor.is_none() {
            self.message = "Open a note before inserting a citation.".to_owned();
            return;
        }
        let Some(path) = &self.bibliography_path else {
            self.message = "No bibliography configured. Set workspace.bibliography in ~/.config/zettui/config.toml or pass --bibliography FILE".to_owned();
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

    pub(super) fn handle_citation_key(&mut self, key: KeyEvent) {
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
    pub(super) fn show_links(&mut self, backlinks: bool) {
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
        if record_history && let Some(old) = &self.editor {
            self.history.push(old.path.clone());
        }
        if let Some(parent) = target.parent() {
            self.current_dir = parent.to_path_buf();
            if let Err(e) = self.refresh() {
                self.message = format!("Link opened, but browser refresh failed: {e}");
            }
            self.select_path(&target);
        }
        self.note_search.reset_matches();
        self.outline_view.reset();
        self.editor = Some(new_note);
        self.editor_keys = EditorEventHandler::default();
        self.pending_open_bracket = false;
        self.wiki_completion = None;
        self.command = None;
        self.message = format!("Opened linked note: {}", target.display());
    }

    pub(super) fn previous_note(&mut self) {
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

    pub(super) fn handle_link_key(&mut self, key: KeyEvent) {
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

    pub(super) fn begin_prompt(&mut self, kind: PromptKind) {
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

    pub(super) fn selected_path(&mut self) -> Option<PathBuf> {
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

    pub(super) fn handle_prompt_key(&mut self, key: KeyEvent) {
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
}

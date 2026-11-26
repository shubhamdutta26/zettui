//! Application-level command identifiers and existing keyboard bindings.
//! Modal input and edtui text motions remain in their own handlers.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    SearchText,
    SearchTags,
    NarrowBrowser,
    WidenBrowser,
    NarrowOutline,
    WidenOutline,
    ToggleFiles,
    TogglePreview,
    ToggleOutline,
    FocusOutline,
    SearchHeadings,
    CycleTheme,
    Quit,
    OutgoingLinks,
    Backlinks,
    FindFilename,
    PreviousNote,
    NewNote,
    NewFolder,
    Rename,
    Move,
    Trash,
    Refresh,
    MoveDown,
    MoveUp,
    OpenSelection,
    EnterFolder,
    ParentFolder,
    CitationPicker,
    SnippetPicker,
    Save,
    Close,
    DiscardAndClose,
    SaveAndClose,
    Writing(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyScope {
    Global,
    Browser,
    Editor,
    Leader,
}

impl Command {
    /// Resolve a key at its existing point in the event-routing precedence.
    pub(crate) fn from_key(scope: KeyScope, key: KeyEvent) -> Option<Self> {
        use Command::*;
        use KeyCode::*;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        match scope {
            KeyScope::Global => match key.code {
                F(2) if key.modifiers.is_empty() => Some(ToggleFiles),
                F(3) if key.modifiers.is_empty() => Some(TogglePreview),
                F(4) if key.modifiers.is_empty() => Some(ToggleOutline),
                F(6) if key.modifiers.is_empty() => Some(FocusOutline),
                F(7) if key.modifiers.is_empty() => Some(SearchHeadings),
                Up if key.modifiers == KeyModifiers::ALT => Some(WidenOutline),
                Down if key.modifiers == KeyModifiers::ALT => Some(NarrowOutline),
                Char('f') if ctrl => Some(SearchText),
                Char('t') if alt => Some(SearchTags),
                Char('h') | Left if key.modifiers == KeyModifiers::ALT => Some(NarrowBrowser),
                Char('l') | Right if key.modifiers == KeyModifiers::ALT => Some(WidenBrowser),
                _ => None,
            },
            KeyScope::Browser => match key.code {
                Char('g') if ctrl => Some(CycleTheme),
                Char('q') => Some(Quit),
                Char('o') => Some(OutgoingLinks),
                Char('b') => Some(Backlinks),
                Char('/') => Some(FindFilename),
                Char('t') if ctrl => Some(PreviousNote),
                Char('n') => Some(NewNote),
                Char('N') => Some(NewFolder),
                Char('r') => Some(Rename),
                Char('m') => Some(Move),
                Char('d') => Some(Trash),
                Char('R') => Some(Refresh),
                Char('j') | Down => Some(MoveDown),
                Char('k') | Up => Some(MoveUp),
                Enter => Some(OpenSelection),
                Char('l') | Right => Some(EnterFolder),
                Char('h') | Backspace | Left => Some(ParentFolder),
                _ => None,
            },
            KeyScope::Editor if ctrl => match key.code {
                Char('o') => Some(OutgoingLinks),
                Char('b') => Some(Backlinks),
                Char('t') => Some(PreviousNote),
                Char('p') => Some(CitationPicker),
                Char('e') => Some(SnippetPicker),
                Char('g') => Some(CycleTheme),
                Char('s') => Some(Save),
                _ => None,
            },
            KeyScope::Leader if key.modifiers.is_empty() => match key.code {
                Char('e') => Some(ToggleFiles),
                Char('p') => Some(TogglePreview),
                Char('o') => Some(ToggleOutline),
                Char('l') => Some(FocusOutline),
                Char('h') => Some(SearchHeadings),
                Char('z') => Some(Writing("zen")),
                Char('f') => Some(Writing("focus")),
                Char('t') => Some(Writing("typewriter")),
                Char('1') => Some(Writing("focus line")),
                Char('2') => Some(Writing("focus paragraph")),
                Char('3') => Some(Writing("focus section")),
                Char('n') => Some(Writing("toggle number")),
                Char('s') => Some(Writing("toggle status")),
                _ => None,
            },
            _ => None,
        }
    }

    /// Fixed Ex commands; parameterized writing commands stay in Writing::command.
    pub(crate) fn from_ex(input: &str) -> Option<Self> {
        use Command::*;
        match input.trim() {
            "w" => Some(Save),
            "q" => Some(Close),
            "q!" => Some(DiscardAndClose),
            "wq" => Some(SaveAndClose),
            "toggle files" => Some(ToggleFiles),
            "toggle preview" => Some(TogglePreview),
            "toggle outline" => Some(ToggleOutline),
            "outline" => Some(FocusOutline),
            "headings" => Some(SearchHeadings),
            "zen" => Some(Writing("zen")),
            "focus" => Some(Writing("focus")),
            "focus off" => Some(Writing("focus off")),
            "focus line" => Some(Writing("focus line")),
            "focus paragraph" => Some(Writing("focus paragraph")),
            "focus section" => Some(Writing("focus section")),
            "typewriter" => Some(Writing("typewriter")),
            "set number" => Some(Writing("set number")),
            "set nonumber" => Some(Writing("set nonumber")),
            "set wrap" => Some(Writing("set wrap")),
            "set nowrap" => Some(Writing("set nowrap")),
            "set status" => Some(Writing("set status")),
            "set nostatus" => Some(Writing("set nostatus")),
            "toggle number" => Some(Writing("toggle number")),
            "toggle status" => Some(Writing("toggle status")),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_focus_bindings_do_not_replace_editor_motions() {
        let plain = KeyModifiers::NONE;
        assert_eq!(
            Command::from_key(KeyScope::Global, KeyEvent::new(KeyCode::F(6), plain)),
            Some(Command::FocusOutline)
        );
        assert_eq!(
            Command::from_key(KeyScope::Leader, KeyEvent::new(KeyCode::Char('l'), plain)),
            Some(Command::FocusOutline)
        );
        assert_eq!(Command::from_ex("outline"), Some(Command::FocusOutline));
        assert_eq!(
            Command::from_key(KeyScope::Editor, KeyEvent::new(KeyCode::Char('l'), plain)),
            None
        );
    }

    #[test]
    fn heading_search_bindings_preserve_existing_search_commands() {
        let plain = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;
        assert_eq!(
            Command::from_key(KeyScope::Global, KeyEvent::new(KeyCode::F(7), plain)),
            Some(Command::SearchHeadings)
        );
        assert_eq!(
            Command::from_key(KeyScope::Leader, KeyEvent::new(KeyCode::Char('h'), plain)),
            Some(Command::SearchHeadings)
        );
        assert_eq!(Command::from_ex("headings"), Some(Command::SearchHeadings));
        assert_eq!(
            Command::from_key(KeyScope::Editor, KeyEvent::new(KeyCode::Char('p'), ctrl)),
            Some(Command::CitationPicker)
        );
        assert_eq!(
            Command::from_key(KeyScope::Editor, KeyEvent::new(KeyCode::Char('h'), plain)),
            None
        );
    }

    #[test]
    fn shortcuts_keep_their_context() {
        let key = |code, modifiers| KeyEvent::new(code, modifiers);
        let plain = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;
        let alt = KeyModifiers::ALT;
        assert_eq!(
            Command::from_key(KeyScope::Editor, key(KeyCode::Char('p'), ctrl)),
            Some(Command::CitationPicker)
        );
        assert_eq!(
            Command::from_key(KeyScope::Global, key(KeyCode::Char('p'), ctrl)),
            None
        );
        assert_eq!(
            Command::from_key(KeyScope::Browser, key(KeyCode::Char('l'), plain)),
            Some(Command::EnterFolder)
        );
        assert_eq!(
            Command::from_key(KeyScope::Browser, key(KeyCode::Enter, plain)),
            Some(Command::OpenSelection)
        );
        assert_eq!(
            Command::from_key(KeyScope::Editor, key(KeyCode::Char('l'), plain)),
            None
        );
        assert_eq!(
            Command::from_key(KeyScope::Editor, key(KeyCode::Char('s'), ctrl)),
            Command::from_ex("w")
        );
        assert_eq!(
            Command::from_key(KeyScope::Global, key(KeyCode::Left, alt)),
            Some(Command::NarrowBrowser)
        );
        assert_eq!(
            Command::from_key(KeyScope::Global, key(KeyCode::Char('f'), ctrl)),
            Some(Command::SearchText)
        );
        assert_eq!(
            Command::from_key(KeyScope::Leader, key(KeyCode::Char('z'), plain)),
            Command::from_ex("zen")
        );
        assert_eq!(
            Command::from_key(KeyScope::Leader, key(KeyCode::Char('z'), ctrl)),
            None
        );
        assert_eq!(Command::from_ex("q"), Some(Command::Close));
        assert_eq!(Command::from_ex("q!"), Some(Command::DiscardAndClose));
        assert_eq!(Command::from_ex("set textwidth=100"), None);
    }

    #[test]
    fn panel_shortcuts_share_command_ids() {
        for (function, leader, ex) in [
            (2, 'e', "toggle files"),
            (3, 'p', "toggle preview"),
            (4, 'o', "toggle outline"),
        ] {
            let global = Command::from_key(
                KeyScope::Global,
                KeyEvent::new(KeyCode::F(function), KeyModifiers::NONE),
            );
            assert!(global.is_some());
            assert_eq!(global, Command::from_ex(ex));
            assert_eq!(
                global,
                Command::from_key(
                    KeyScope::Leader,
                    KeyEvent::new(KeyCode::Char(leader), KeyModifiers::NONE)
                )
            );
        }
        assert_eq!(
            Command::from_key(
                KeyScope::Global,
                KeyEvent::new(KeyCode::Up, KeyModifiers::ALT)
            ),
            Some(Command::WidenOutline)
        );
        assert_eq!(
            Command::from_key(
                KeyScope::Global,
                KeyEvent::new(KeyCode::Down, KeyModifiers::ALT)
            ),
            Some(Command::NarrowOutline)
        );
    }
}

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
    ToggleMetadata,
    FocusMetadata,
    SearchHeadings,
    OpenPalette,
    WritingAnalytics,
    CycleTheme,
    Quit,
    QuitAll,
    SaveQuitAll,
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
    InsertLink,
    InsertImage,
    InsertCodeBlock,
    CitationPreview,
    CitationCheck,
    ExportDocument,
    SnippetPicker,
    Save,
    Hide,
    BufferPicker,
    NextBuffer,
    PreviousBuffer,
    AlternateBuffer,
    CloseBuffer,
    CloseOtherBuffers,
    ToggleTabs,
    ToggleHiddenFolders,
    SaveAll,
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
                F(5) if key.modifiers.is_empty() => Some(BufferPicker),
                F(6) if key.modifiers.is_empty() => Some(FocusOutline),
                F(7) if key.modifiers.is_empty() => Some(SearchHeadings),
                F(8) if key.modifiers.is_empty() => Some(OpenPalette),
                F(9) if key.modifiers.is_empty() => Some(ToggleHiddenFolders),
                F(10) if key.modifiers.is_empty() => Some(WritingAnalytics),
                F(11) if key.modifiers.is_empty() => Some(FocusMetadata),
                Up if key.modifiers == KeyModifiers::ALT => Some(WidenOutline),
                Down if key.modifiers == KeyModifiers::ALT => Some(NarrowOutline),
                Char('f') if ctrl => Some(SearchText),
                Char('t') if alt => Some(SearchTags),
                Char('h') | Left if key.modifiers == KeyModifiers::ALT => Some(NarrowBrowser),
                Char('l') | Right if key.modifiers == KeyModifiers::ALT => Some(WidenBrowser),
                _ => None,
            },
            KeyScope::Browser if key.modifiers == KeyModifiers::CONTROL => match key.code {
                Char('g') => Some(CycleTheme),
                Char('t') => Some(PreviousNote),
                _ => None,
            },
            // A modified key must not fall through to a plain file operation.
            // Shift is allowed for terminals that report it with uppercase letters.
            KeyScope::Browser
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                match key.code {
                    Char('q') => Some(Quit),
                    Char('o') => Some(OutgoingLinks),
                    Char('b') => Some(Backlinks),
                    Char('/') => Some(FindFilename),
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
                }
            }
            KeyScope::Editor if key.modifiers == KeyModifiers::ALT => match key.code {
                Char('i') => Some(InsertImage),
                Char('c') => Some(InsertCodeBlock),
                _ => None,
            },
            KeyScope::Editor if ctrl => match key.code {
                Char('o') => Some(OutgoingLinks),
                Char('b') => Some(Backlinks),
                Char('t') => Some(PreviousNote),
                Char('p') => Some(CitationPicker),
                Char('k') if key.modifiers == KeyModifiers::CONTROL => Some(InsertLink),
                Char('e') => Some(SnippetPicker),
                Char('g') => Some(CycleTheme),
                Char('s') => Some(Save),
                _ => None,
            },
            KeyScope::Leader if key.modifiers.is_empty() => match key.code {
                Char('e') => Some(ToggleFiles),
                Char('p') => Some(TogglePreview),
                Char('o') => Some(ToggleOutline),
                Char('m') => Some(ToggleMetadata),
                Char('x') => Some(ExportDocument),
                Char('l') => Some(FocusOutline),
                Char('h') => Some(SearchHeadings),
                Char('c') => Some(OpenPalette),
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
            "cite" => Some(CitationPicker),
            "link" => Some(InsertLink),
            "image" => Some(InsertImage),
            "cite-preview" => Some(CitationPreview),
            "cite-check" => Some(CitationCheck),
            "export" => Some(ExportDocument),
            "hide" => Some(Hide),
            "buffers" | "ls" => Some(BufferPicker),
            "bn" => Some(NextBuffer),
            "bp" => Some(PreviousBuffer),
            "b#" => Some(AlternateBuffer),
            "bd" => Some(CloseBuffer),
            "bonly" => Some(CloseOtherBuffers),
            "toggle tabs" => Some(ToggleTabs),
            "toggle hidden" | "toggle hidden folders" => Some(ToggleHiddenFolders),
            "wa" => Some(SaveAll),
            "qa" | "qall" => Some(QuitAll),
            "wqa" | "wqall" => Some(SaveQuitAll),
            "q" => Some(Close),
            "q!" => Some(DiscardAndClose),
            "wq" => Some(SaveAndClose),
            "toggle files" => Some(ToggleFiles),
            "toggle preview" => Some(TogglePreview),
            "toggle outline" => Some(ToggleOutline),
            "outline" => Some(FocusOutline),
            "metadata" | "toggle metadata" => Some(ToggleMetadata),
            "metadata-edit" => Some(FocusMetadata),
            "headings" => Some(SearchHeadings),
            "commands" => Some(OpenPalette),
            "analytics" | "stats" => Some(WritingAnalytics),
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

/// Discoverable application actions. Key hints are descriptive; execution always
/// goes through Command, never simulated keystrokes or shell commands.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CommandInfo {
    pub command: Command,
    pub title: &'static str,
    pub shortcut: &'static str,
    pub aliases: &'static str,
}

pub(crate) const CATALOG: &[CommandInfo] = &[
    CommandInfo {
        command: Command::ExportDocument,
        title: "Export document or folder",
        shortcut: "Space x / :export",
        aliases: "pandoc pdf docx html epub latex export",
    },
    CommandInfo {
        command: Command::WritingAnalytics,
        title: "Writing analytics and calendar",
        shortcut: "F10",
        aliases: ":analytics :stats writing goals statistics streak heatmap",
    },
    CommandInfo {
        command: Command::BufferPicker,
        title: "Switch open buffers",
        shortcut: "F5 / Space b b",
        aliases: ":buffers :ls",
    },
    CommandInfo {
        command: Command::NextBuffer,
        title: "Next buffer",
        shortcut: "]b",
        aliases: ":bn",
    },
    CommandInfo {
        command: Command::PreviousBuffer,
        title: "Previous buffer",
        shortcut: "[b",
        aliases: ":bp",
    },
    CommandInfo {
        command: Command::AlternateBuffer,
        title: "Previously active buffer",
        shortcut: "Space b p",
        aliases: ":b# alternate",
    },
    CommandInfo {
        command: Command::CloseBuffer,
        title: "Close current buffer",
        shortcut: "Space b d",
        aliases: ":bd",
    },
    CommandInfo {
        command: Command::CloseOtherBuffers,
        title: "Close other buffers",
        shortcut: "Space b o",
        aliases: ":bonly",
    },
    CommandInfo {
        command: Command::ToggleTabs,
        title: "Toggle buffer tab bar",
        shortcut: "Space b t",
        aliases: ":toggle tabs",
    },
    CommandInfo {
        command: Command::ToggleHiddenFolders,
        title: "Toggle hidden folders",
        shortcut: "F9",
        aliases: ":toggle hidden dot directories show hide",
    },
    CommandInfo {
        command: Command::SaveAll,
        title: "Save all buffers",
        shortcut: "Space b s",
        aliases: ":wa",
    },
    CommandInfo {
        command: Command::SearchText,
        title: "Search workspace text",
        shortcut: "Ctrl+F",
        aliases: "vault full text find",
    },
    CommandInfo {
        command: Command::SearchTags,
        title: "Search workspace tags",
        shortcut: "Alt+T",
        aliases: "yaml metadata vault",
    },
    CommandInfo {
        command: Command::SearchHeadings,
        title: "Search document headings",
        shortcut: "F7",
        aliases: "outline sections :headings",
    },
    CommandInfo {
        command: Command::FindFilename,
        title: "Find filename in current folder",
        shortcut: "/",
        aliases: "file search",
    },
    CommandInfo {
        command: Command::OpenSelection,
        title: "Open selected file or folder",
        shortcut: "Enter",
        aliases: "browse",
    },
    CommandInfo {
        command: Command::NewNote,
        title: "Create new note",
        shortcut: "n",
        aliases: "file add",
    },
    CommandInfo {
        command: Command::NewFolder,
        title: "Create new folder",
        shortcut: "N",
        aliases: "directory add",
    },
    CommandInfo {
        command: Command::Rename,
        title: "Rename selected file or folder",
        shortcut: "r",
        aliases: "name",
    },
    CommandInfo {
        command: Command::Move,
        title: "Move selected file or folder",
        shortcut: "m",
        aliases: "relocate",
    },
    CommandInfo {
        command: Command::Trash,
        title: "Send selected file or folder to Trash",
        shortcut: "d",
        aliases: "delete remove",
    },
    CommandInfo {
        command: Command::ParentFolder,
        title: "Go to parent folder",
        shortcut: "h",
        aliases: "directory back",
    },
    CommandInfo {
        command: Command::Refresh,
        title: "Refresh file browser",
        shortcut: "R",
        aliases: "reload directory",
    },
    CommandInfo {
        command: Command::Save,
        title: "Save note",
        shortcut: "Ctrl+S / :w",
        aliases: "write",
    },
    CommandInfo {
        command: Command::SaveAndClose,
        title: "Save and close note",
        shortcut: ":wq",
        aliases: "write exit",
    },
    CommandInfo {
        command: Command::Hide,
        title: "Return to browser (keep buffer open)",
        shortcut: ":hide",
        aliases: "hide park buffer browse notes",
    },
    CommandInfo {
        command: Command::Close,
        title: "Close note",
        shortcut: ":q",
        aliases: "return to browser",
    },
    CommandInfo {
        command: Command::OutgoingLinks,
        title: "Show outgoing links",
        shortcut: "o / Ctrl+O",
        aliases: "wiki links",
    },
    CommandInfo {
        command: Command::Backlinks,
        title: "Show backlinks",
        shortcut: "b / Ctrl+B",
        aliases: "incoming links",
    },
    CommandInfo {
        command: Command::PreviousNote,
        title: "Return to previous linked note",
        shortcut: "Ctrl+T",
        aliases: "history back",
    },
    CommandInfo {
        command: Command::InsertLink,
        title: "Insert Markdown link",
        shortcut: "Ctrl+K / :link",
        aliases: "url hyperlink",
    },
    CommandInfo {
        command: Command::InsertImage,
        title: "Insert Markdown image",
        shortcut: "Alt+I / :image",
        aliases: "figure picture image path",
    },
    CommandInfo {
        command: Command::InsertCodeBlock,
        title: "Insert Markdown code block",
        shortcut: "Alt+C / :code LANG",
        aliases: "fenced code chunk snippet language programming",
    },
    CommandInfo {
        command: Command::CitationPicker,
        title: "Insert Zotero citation",
        shortcut: "Ctrl+P / :cite",
        aliases: "bibliography references",
    },
    CommandInfo {
        command: Command::CitationPreview,
        title: "Preview citation under cursor",
        shortcut: ":cite-preview",
        aliases: "reference details abstract pdf",
    },
    CommandInfo {
        command: Command::CitationCheck,
        title: "Check unresolved citations in current note",
        shortcut: ":cite-check",
        aliases: "missing references bibliography audit",
    },
    CommandInfo {
        command: Command::SnippetPicker,
        title: "Insert note snippet",
        shortcut: "Ctrl+E",
        aliases: "template",
    },
    CommandInfo {
        command: Command::ToggleFiles,
        title: "Toggle Files panel",
        shortcut: "F2",
        aliases: "sidebar :toggle files",
    },
    CommandInfo {
        command: Command::TogglePreview,
        title: "Toggle Preview panel",
        shortcut: "F3",
        aliases: "sidebar :toggle preview",
    },
    CommandInfo {
        command: Command::ToggleOutline,
        title: "Toggle Outline panel",
        shortcut: "F4",
        aliases: "sidebar :toggle outline",
    },
    CommandInfo {
        command: Command::ToggleMetadata,
        title: "Open Metadata Editor",
        shortcut: "Space m / F11 / :metadata",
        aliases: "yaml frontmatter properties tags title",
    },
    CommandInfo {
        command: Command::FocusMetadata,
        title: "Open Metadata Editor (alias)",
        shortcut: "F11 / :metadata-edit",
        aliases: "yaml fields properties edit",
    },
    CommandInfo {
        command: Command::FocusOutline,
        title: "Switch focus between editor and Outline",
        shortcut: "F6",
        aliases: "navigate :outline",
    },
    CommandInfo {
        command: Command::NarrowBrowser,
        title: "Decrease Files panel width",
        shortcut: "Alt+Left",
        aliases: "resize narrow sidebar",
    },
    CommandInfo {
        command: Command::WidenBrowser,
        title: "Increase Files panel width",
        shortcut: "Alt+Right",
        aliases: "resize widen sidebar",
    },
    CommandInfo {
        command: Command::NarrowOutline,
        title: "Decrease Outline panel width",
        shortcut: "Alt+Down",
        aliases: "resize narrow sidebar",
    },
    CommandInfo {
        command: Command::WidenOutline,
        title: "Increase Outline panel width",
        shortcut: "Alt+Up",
        aliases: "resize widen sidebar",
    },
    CommandInfo {
        command: Command::CycleTheme,
        title: "Cycle color theme",
        shortcut: "Ctrl+G",
        aliases: "appearance palette dark light",
    },
    CommandInfo {
        command: Command::Writing("zen"),
        title: "Toggle Zen mode",
        shortcut: "Space z",
        aliases: "distraction free :zen",
    },
    CommandInfo {
        command: Command::Writing("focus"),
        title: "Toggle Focus mode",
        shortcut: "Space f",
        aliases: ":focus dim",
    },
    CommandInfo {
        command: Command::Writing("focus line"),
        title: "Focus current line",
        shortcut: "Space 1",
        aliases: ":focus line",
    },
    CommandInfo {
        command: Command::Writing("focus paragraph"),
        title: "Focus current paragraph",
        shortcut: "Space 2",
        aliases: ":focus paragraph",
    },
    CommandInfo {
        command: Command::Writing("focus section"),
        title: "Focus current section",
        shortcut: "Space 3",
        aliases: ":focus section",
    },
    CommandInfo {
        command: Command::Writing("focus off"),
        title: "Disable Focus mode",
        shortcut: ":focus off",
        aliases: "undim",
    },
    CommandInfo {
        command: Command::Writing("typewriter"),
        title: "Toggle Typewriter mode",
        shortcut: "Space t",
        aliases: ":typewriter center",
    },
    CommandInfo {
        command: Command::Writing("toggle number"),
        title: "Toggle line numbers",
        shortcut: "Space n",
        aliases: ":toggle number gutter",
    },
    CommandInfo {
        command: Command::Writing("toggle status"),
        title: "Toggle Zen status bar",
        shortcut: "Space s",
        aliases: ":toggle status footer",
    },
    CommandInfo {
        command: Command::Writing("set wrap"),
        title: "Enable word wrapping",
        shortcut: ":set wrap",
        aliases: "wrap lines",
    },
    CommandInfo {
        command: Command::Writing("set nowrap"),
        title: "Disable word wrapping",
        shortcut: ":set nowrap",
        aliases: "unwrap lines",
    },
    CommandInfo {
        command: Command::QuitAll,
        title: "Quit and remember session",
        shortcut: ":qa",
        aliases: "qall exit session",
    },
    CommandInfo {
        command: Command::SaveQuitAll,
        title: "Save all buffers and quit",
        shortcut: ":wqa",
        aliases: "wqall save exit session",
    },
    CommandInfo {
        command: Command::Quit,
        title: "Quit Zettui",
        shortcut: "q",
        aliases: "exit application",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_modified_keys_do_not_fall_through_to_plain_commands() {
        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ] {
            for code in ['q', 'd', 'm', 'r', 'n', 'N', 'j', 'k', 'o', 'b', '/'] {
                assert_eq!(
                    Command::from_key(
                        KeyScope::Browser,
                        KeyEvent::new(KeyCode::Char(code), modifiers)
                    ),
                    None
                );
            }
            assert_eq!(
                Command::from_key(KeyScope::Browser, KeyEvent::new(KeyCode::Enter, modifiers)),
                None
            );
        }
        for (code, modifiers, expected) in [
            ('d', KeyModifiers::NONE, Command::Trash),
            ('q', KeyModifiers::NONE, Command::Quit),
            ('N', KeyModifiers::SHIFT, Command::NewFolder),
            ('R', KeyModifiers::SHIFT, Command::Refresh),
            ('g', KeyModifiers::CONTROL, Command::CycleTheme),
            ('t', KeyModifiers::CONTROL, Command::PreviousNote),
        ] {
            assert_eq!(
                Command::from_key(
                    KeyScope::Browser,
                    KeyEvent::new(KeyCode::Char(code), modifiers)
                ),
                Some(expected)
            );
        }
    }

    #[test]
    fn documented_ex_commands_resolve_to_expected_actions() {
        use Command::*;
        for (input, expected) in [
            ("w", Save),
            ("cite", CitationPicker),
            ("link", InsertLink),
            ("image", InsertImage),
            ("cite-preview", CitationPreview),
            ("cite-check", CitationCheck),
            ("hide", Hide),
            ("buffers", BufferPicker),
            ("ls", BufferPicker),
            ("bn", NextBuffer),
            ("bp", PreviousBuffer),
            ("b#", AlternateBuffer),
            ("bd", CloseBuffer),
            ("bonly", CloseOtherBuffers),
            ("toggle tabs", ToggleTabs),
            ("toggle hidden", ToggleHiddenFolders),
            ("toggle hidden folders", ToggleHiddenFolders),
            ("wa", SaveAll),
            ("qa", QuitAll),
            ("qall", QuitAll),
            ("wqa", SaveQuitAll),
            ("wqall", SaveQuitAll),
            ("q", Close),
            ("q!", DiscardAndClose),
            ("wq", SaveAndClose),
            ("toggle files", ToggleFiles),
            ("toggle preview", TogglePreview),
            ("toggle outline", ToggleOutline),
            ("outline", FocusOutline),
            ("metadata", ToggleMetadata),
            ("toggle metadata", ToggleMetadata),
            ("metadata-edit", FocusMetadata),
            ("headings", SearchHeadings),
            ("commands", OpenPalette),
            ("analytics", WritingAnalytics),
            ("stats", WritingAnalytics),
            ("zen", Writing("zen")),
            ("focus", Writing("focus")),
            ("focus off", Writing("focus off")),
            ("focus line", Writing("focus line")),
            ("focus paragraph", Writing("focus paragraph")),
            ("focus section", Writing("focus section")),
            ("typewriter", Writing("typewriter")),
            ("set number", Writing("set number")),
            ("set nonumber", Writing("set nonumber")),
            ("set wrap", Writing("set wrap")),
            ("set nowrap", Writing("set nowrap")),
            ("set status", Writing("set status")),
            ("set nostatus", Writing("set nostatus")),
            ("toggle number", Writing("toggle number")),
            ("toggle status", Writing("toggle status")),
        ] {
            assert_eq!(Command::from_ex(input), Some(expected), "{input}");
            assert_eq!(Command::from_ex(&format!("  {input}  ")), Some(expected));
        }
        for input in ["qa!", "W", "!ls", "", "set textwidth=80"] {
            assert_eq!(Command::from_ex(input), None, "{input}");
        }
    }

    #[test]
    fn hidden_folder_shortcuts_share_command() {
        assert_eq!(
            Command::from_key(
                KeyScope::Global,
                KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE)
            ),
            Some(Command::ToggleHiddenFolders)
        );
        assert_eq!(
            Command::from_ex("toggle hidden"),
            Some(Command::ToggleHiddenFolders)
        );
        assert_eq!(
            Command::from_ex("toggle hidden folders"),
            Some(Command::ToggleHiddenFolders)
        );
    }

    #[test]
    fn palette_shortcuts_preserve_zotero() {
        let plain = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;
        let shift = KeyModifiers::SHIFT;
        assert_eq!(
            Command::from_key(KeyScope::Global, KeyEvent::new(KeyCode::F(8), plain)),
            Some(Command::OpenPalette)
        );
        for ch in ['p', 'P'] {
            assert_eq!(
                Command::from_key(
                    KeyScope::Global,
                    KeyEvent::new(KeyCode::Char(ch), ctrl | shift)
                ),
                None
            );
        }
        assert_eq!(
            Command::from_key(KeyScope::Global, KeyEvent::new(KeyCode::Char('p'), ctrl)),
            None
        );
        assert_eq!(
            Command::from_key(KeyScope::Editor, KeyEvent::new(KeyCode::Char('p'), ctrl)),
            Some(Command::CitationPicker)
        );
        assert_eq!(
            Command::from_key(KeyScope::Leader, KeyEvent::new(KeyCode::Char('c'), plain)),
            Some(Command::OpenPalette)
        );
        assert_eq!(Command::from_ex("commands"), Some(Command::OpenPalette));
    }

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
    fn export_leader_preserves_files_and_editor_keys() {
        let plain = KeyModifiers::NONE;
        let x = KeyEvent::new(KeyCode::Char('x'), plain);
        assert_eq!(
            Command::from_key(KeyScope::Leader, x),
            Command::from_ex("export")
        );
        assert_eq!(
            Command::from_key(KeyScope::Leader, x),
            Some(Command::ExportDocument)
        );
        assert_eq!(Command::from_key(KeyScope::Editor, x), None);
        assert_eq!(
            Command::from_key(KeyScope::Leader, KeyEvent::new(KeyCode::Char('e'), plain)),
            Some(Command::ToggleFiles)
        );
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

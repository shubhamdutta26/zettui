# Buffers and Sessions (v0.7.13)

Zettui keeps multiple notes open without changing the plain-file storage model. A buffer is an in-memory editing state for a file. A tab represents an open buffer, not a separate copy of a file. A session is workspace-specific metadata used to reopen saved notes on the next launch.

## Getting started

1. Select a note in Files and press Enter; selecting a file alone only previews it.
2. Edit normally (`i`, then Esc to return to Normal mode).
3. Use `:hide` to return to Files without closing the note. Open another note.
4. Switch between the open notes using `]b`, `[b`, or `F5`. Unsaved edits remain in their respective buffers. Opening an already open file activates its existing buffer.
5. Use `:wa` to save all modified notes, or `:wqa` to save and exit. Restart the same workspace to restore its session.

Tabs use the current theme, indicate the active buffer and mark modified notes with `[+]`. Duplicate filenames use workspace-relative paths for disambiguation. The visible tab range follows the active note when the terminal is narrow. Tabs are keyboard-driven and hidden in Zen mode. The editor title and footer filename are omitted when tabs are visible.

## Keyboard and Ex command reference

Use buffer prefix/leader shortcuts in Normal mode. The F5 picker and F8 palette also work from the browser; an active editor must be in Normal mode to open them. Modal prompts own keyboard input until closed.

| Key | Ex command | Action |
| --- | --- | --- |
| `F5`, `Space b b` | `:buffers`, `:ls` | Fuzzy buffer picker; Up/Down (or Ctrl+P/Ctrl+N), Enter, Esc/F5, Ctrl+U to clear |
| `]b` | `:bn` | Next buffer (wraps) |
| `[b` | `:bp` | Previous buffer (wraps) |
| `Space b p` | `:b#` | Previously active buffer |
| `Space b d` | `:bd` | Close current buffer if clean; activate an adjacent buffer if present |
| `Space b o` | `:bonly` | Close all other buffers; refuse if any other buffer is modified |
| `Space b t` | `:toggle tabs` | Toggle tab visibility |
| `Space b s` | `:wa` | Save all modified buffers |
| `Ctrl+S` | `:w` | Save active note |
| — | `:hide` | Return to Files without closing the current buffer |
| — | `:q` | Close current note and return to Files; refuse if modified |
| — | `:q!` | Explicitly discard active note edits and close it |
| — | `:wq` | Save current note and close it if save succeeds |
| — | `:qa`, `:qall` | Quit application if all buffers are clean |
| — | `:wqa`, `:wqall` | Save modified buffers and quit only if all are clean |
| `q` (browser) | — | Quit application if all buffers are clean |
| `F9` | `:toggle hidden`, `:toggle hidden folders` | Toggle visibility of dot-prefixed folders |
| `F8` | — | Command palette; search for buffer, session exit and folder visibility actions |

`:q` and `:bd` have different navigation behavior. `Ctrl+T` follows note navigation history, whereas `:b#` follows recent buffer activation. Explicit discard (`:q!`) affects only the active note. There is no forced application-wide quit command in this release.

## State and safety

| State | Switch buffers | Restart after clean exit |
| --- | --- | --- |
| Note contents | Saved and unsaved | Saved file contents only |
| Cursor and viewport | Yes | Yes, if enabled |
| Undo/redo | Yes (per buffer) | No |
| Last in-note search query and direction | Yes (shared across buffers) | No |
| Open set, order and active buffer | Yes | Yes, if enabled |
| Browser directory/selection, panel and tab preferences | Workspace-wide | Yes (layout preferences are optional) |
| Insert/Visual mode, pending keys and modal dialogs | Reset on switching | No |

Viewport offsets can adjust with terminal resizing, word wrapping and Typewriter mode. Search and outline displays are rebuilt for the active buffer. The last in-note search query and direction are application-wide, not independent per buffer. The outline and heading picker use the live editor text; vault text/tag search and link indexing use files on disk, not unsaved contents.

Existing safe-save rules apply to every buffer, including checks for external modifications. `:wa` attempts each save independently; a failure can leave some notes saved and others modified. `:wqa` does not exit if dirty buffers remain. File-browser rename, move and Trash operations are refused for open files and folders containing open files; close affected buffers before changing their paths.

## Configuration

These optional sections in `config.toml` have the following defaults. Old configurations continue to work without them.

```toml
[buffers]
show_tab_bar = true

[session]
enabled = true
restore_on_startup = true
restore_cursor = true
restore_layout = true

[browser]
show_hidden_folders = false
```

- `enabled`: allow loading and saving session state. Disabling it does not delete an existing session.
- `restore_on_startup`: if false, begin without a restored session; a subsequent clean exit can still replace it if `enabled` is true.
- `restore_cursor`: restore cursor and scroll offsets; otherwise open notes at the default position.
- `restore_layout`: restore Files, Preview and Outline visibility, pane widths, tabs and the hidden-folder preference. With this disabled, configuration defaults apply to those preferences. Browser location and open buffers can still be restored.
- `show_tab_bar` and `show_hidden_folders`: startup defaults, overridden by restored preferences when available and layout restoration is enabled.

## Session files

Sessions are saved on successful application exit, outside the notes directory:

```text
$XDG_STATE_HOME/zettui/sessions/<workspace-id>.json
```

If `XDG_STATE_HOME` is unset or not absolute, the default is `~/.local/state/zettui/sessions/`. The identifier is a deterministic hash of the canonical workspace directory. The full workspace path and schema version are also checked on load. Session JSON includes paths, cursor/viewport positions and layout/browser metadata; it does not contain note text. It may reveal filenames and directory paths, so treat it as local application state.

Session writes use a temporary file and atomic replacement. Missing or unreadable notes are skipped; restored cursor positions are clamped to the current contents. Invalid or unsupported session files are ignored with a message. A session write failure is reported after leaving the terminal; it does not undo successful note saves. There is no crash recovery or persisted undo history. Multiple instances in the same workspace share a session file; the last successful exit wins.

To start fresh without deleting session files, set `restore_on_startup = false`. For a one-off clean start without overwriting an existing session, set `enabled = false` as well; restore the previous configuration afterward. The application never rewrites `config.toml`.

## Hidden folders

F9 refreshes the current Files directory. Dot-prefixed folders are hidden by default; `.git` and symbolic links remain excluded even when hidden folders are shown. Dot-prefixed regular files retain their previous browser behavior. If hiding folders while inside one, the browser moves to the nearest visible ancestor. Already-open notes are not closed or changed. The toggle does not broaden vault search or link indexing.

## Implementation overview

- `src/buffers.rs`: ordered `OpenNote` collection, active/alternate buffer tracking, lifecycle and bulk saving.
- `src/editor.rs`: per-note editor state and safe saving; `src/editor_search.rs` handles search interactions.
- `src/buffer_switcher.rs` and `src/app/buffer_navigation.rs`: fuzzy buffer picker and key handling.
- `src/app/ui.rs`: theme-aware tab rendering and picker UI.
- `src/session.rs` and `src/app/session.rs`: metadata format, XDG storage, capture and restoration.
- `src/app/browser.rs`: visibility filtering and browser refresh; `src/commands.rs` and the app command/palette modules integrate the actions.

For final release validation, see [v0.7.13 integration checklist](RELEASE-CHECKLIST-0.7.13.md).

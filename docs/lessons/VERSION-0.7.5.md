# Zettui v0.7.5 — Full-Vault Search, Tag Search, Insert Cursor

## What you will learn

- Use a small read-only `VaultIndex` to search nested folders.
- Search YAML `tags` with inline and multiline syntax.
- Show results in a Ratatui modal and jump to a matching line.
- Distinguish EdTUI's painted cursor from the terminal cursor shape.

## Try it

1. Run `cargo fmt`, `cargo check`, and `cargo test` from your repository.
2. Launch `cargo run -- examples/notes`, or use the sample files in this kit under `examples/search-notes`.
3. Press `Ctrl+F` from browser or editor. Type `framework` to search note contents.
4. Use `↑`/`↓` and `Enter` to open the match at the selected line.
5. Press `Alt+T` or press `Tab` in the search dialog to switch to tags. Search `antibody`.
6. Enter Insert mode (`i`) in a note: the terminal should show a blinking horizontal underscore. Press `Esc` for the block cursor again.

## Design notes

`src/search.rs` is isolated from the UI so tests can exercise the index without launching a terminal. Indexing happens each time the search picker opens, never continuously. Only ordinary UTF-8 note formats are read; symlinks and hidden folders are ignored. Files larger than 1 MiB are skipped. The scan is capped at 20,000 files or 128 MiB of indexed text and shows when it reaches that limit.

Search matches are case-insensitive and match all space-separated terms within one line. Tags are read from YAML frontmatter fields `tags: [a, b]`, `tags:` with `- a` lines, or `tags: a`. This is a deliberately limited YAML parser, not full YAML schema support. A tag result opens at the start of the note. Search reflects the **saved file on disk**, not unsaved editor changes.

For the cursor, `SetCursorStyle::BlinkingUnderScore` (Crossterm) changes the *real terminal cursor* in Insert mode; `EditorTheme::cursor_style` stops EdTUI from drawing the solid highlight at the same cell. On exit, `DefaultUserShape` restores the configured terminal cursor. Terminal support determines the exact blinking appearance.

## Keyboard commands

| Shortcut | Effect |
| --- | --- |
| `Ctrl+F` | Search text, any pane |
| `Alt+T` | Search tags, any pane |
| `Tab` | Toggle text/tags while search picker is open |
| `Up` / `Down` | Choose a result |
| `Enter` | Open note, jump to matching line for text results |
| `Ctrl+U` | Clear search |
| `Esc` | Close search without changing the open note |

## Git milestone

Create `feature/search-and-cursor` off updated main. After local checks and GitHub CI succeed, merge into main, then tag `v0.7.5`. Avoid releasing from a branch where Cargo.lock is out of date. Keep the test sample notes out of your actual notes vault unless you want them there.

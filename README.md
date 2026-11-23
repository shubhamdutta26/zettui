# Zettui — Lesson 2: Vim editing + safe save

This is the second lesson of Zettui, a local-first TUI for plain-text notes. Your YAML front matter, note IDs, and tags are **ordinary text** — the app does not parse, delete, or rearrange them. In Lesson 2, edits are limited to UTF-8 files no larger than 256 KB.

## Try it (on a disposable copy first)

```bash
cd ~/Projects/zettui
cargo check
cargo test
cargo run -- examples/notes
```

You can point Zettui at any note directory:

```bash
cargo run -- /path/to/notes
```

### File browser (left pane)

| Key | Action |
|---|---|
| `j` / `k` | Move up/down |
| `l` or `Enter` | Open selected directory or **open a file in the editor** |
| `h` | Parent directory |
| `q` | Quit Zettui (only while in file browser) |

### Editor (right pane)

| Key/Command | Action |
|---|---|
| `i` | Insert mode |
| `Esc` | Return to Normal mode |
| `h j k l` | Move cursor in Normal mode |
| `w`, `b`, `0`, `$`, `gg`, `G` | Word/line/file motions |
| `x`, `dd`, `o`, `u`, `Ctrl+r` | Delete char/line, new line, undo, redo |
| `v` | Visual selection |
| `Ctrl+S` | Save anytime |
| `:w` + Enter | Save |
| `:q` + Enter | Close file **if no unsaved changes** |
| `:q!` + Enter | Discard unsaved changes, close file |
| `:wq` + Enter | Save and return to file browser |

> Important: `:q` is an application command that closes the file, not Zettui. To quit the application, close the editor and press `q` in the file browser. If you are in Insert mode, press `Esc` **before typing a colon command**.

## Safer saving

- Zettui compares your edited text to the version saved from disk, so it warns before closing dirty files.
- If another app changed the on-disk contents since opening, save is refused, not overwritten.
- A save writes to a temporary file in the same directory, then renames it over the original.
- File permissions are preserved. UTF-8 BOM and CRLF line endings are preserved for typical files. Mixed line-ending files may be normalized, so test with copies first.
- Saving via rename can result in a new inode; other apps holding the old inode might need to reopen the file. Always keep a proper backup.

## Files explained

- `src/main.rs`: file browser, window drawing, key routing, command bar.
- `src/editor.rs`: note loading, change tracking, safe saving, and automated tests.
- `Cargo.toml`: dependency configuration (`edtui` and `tempfile` added in this lesson).

## Tests

```bash
cargo fmt --check
cargo check
cargo test
```

## Next

Lesson 3: create, rename, move, and delete notes and directories. Then note linking, bibliography search, configuration and themes, and Linux release packaging.

Do **not** commit real Zettlr notes, Zotero files, or personal configuration into a public GitHub repository.

# Zettui (Lesson 6, v0.6.0)

Zettui is a **local-first Rust terminal notes manager** built in incremental lessons.
Lesson 6 adds color themes and a filename finder to the existing file browser,
Vim-style editor, wiki links/backlinks, safe file management, and Zotero citation picker.

## Quick start

```bash
cargo fmt --check
cargo check
cargo test
cargo run -- examples/notes --bibliography examples/references.json --theme catppuccin
```

### Browser shortcuts

- `j`/`k` move, `Enter` open, `h` parent, `q` exit.
- `/` search filenames **in the current folder** (type to filter, Up/Down to select,
  Enter jumps to the file in the browser, then Enter opens it; Esc cancels).
- `Ctrl+G` cycle Gruvbox Dark, Catppuccin Mocha, Tokyo Night, Paper Light.
- `n` new note, `N` new folder, `r` rename, `m` move, `d` Trash.
- `o` outgoing links, `b` backlinks, `R` refresh.

### Editor shortcuts

- `i` Insert; `Esc` Normal; Vim motions from EdTUI; `:w`, `:q`, `:q!`, `:wq`.
- `Ctrl+S` save; `Ctrl+O` outgoing links; `Ctrl+B` backlinks;
  `Ctrl+T` back in link navigation history; `Ctrl+P` Zotero picker.
- `Ctrl+G` cycle themes without altering note contents.

## Theme configuration

Create `~/.config/zettui/config.toml` (or `$XDG_CONFIG_HOME/zettui/config.toml`):

```toml
notes_dir = "/home/YOUR_USER/Documents/Notes"
bibliography = "/home/YOUR_USER/Documents/Zotero/references.json"
theme = "tokyo-night"

[colors]
accent = "#B89CF4"
selected_bg = "#364A82"
```

The `[colors]` keys are optional overrides; use `#RRGGBB` syntax.
Other names: `gruvbox`, `catppuccin`, `paper`. For a temporary theme, use
`--theme NAME`. Changing the theme via `Ctrl+G` is **session-only**; Zettui
never edits your configuration file. Restarting restores the selected
configuration/CLI theme. The terminal must support true color for accurate RGB.

For a documented update from Lesson 5, see [LESSON-6.md](LESSON-6.md).

## Important limitations

- The filename finder searches only the *current folder*, not note contents
  or nested directories. Full-vault content search is planned for later.
- Themes style the application and the editor's base text, cursor, line numbers
  and selections, but do **not** implement Markdown/YAML syntax highlighting.
- Plain citation insertion (`[@citekey]`) is not citation rendering.
- Link updates after rename/move are not automatic.
- No Linux installer/package yet (Lesson 7).
- Do not push personal notes, bibliographies, or local config files to GitHub.
- Test the updated application on copied notes first.

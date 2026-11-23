# Zettui

A local-first terminal notes manager with Vim editing, wiki links, Zotero citations, and snippets. Your Markdown and other text files stay on disk.

## Installation

Download Linux packages from [GitHub Releases](https://github.com/shubhamdutta26/zettui/releases): `.deb` for Debian/Ubuntu, `.rpm` for Fedora, or the `PKGBUILD` plus archive for Arch/Omarchy. Alternatively, with Rust 1.88+: `cargo install --path . --locked`.

Run: `zettui ~/Documents/Notes`

## Configuration

`~/.config/zettui/config.toml`:

```toml
notes_dir = "/home/USER/Documents/Notes"
bibliography = "/home/USER/Documents/Zotero/references.json"
theme = "auto"  # auto | dark | light
browser_width_percent = 35  # 20–70
# Optional; defaults to ~/.config/zettui/snippets
# snippets_dir = "/home/USER/.config/zettui/snippets"
```

Put reusable UTF-8 templates in `~/.config/zettui/snippets/`, for example `new-note.md`. Open an **empty** note and press `Ctrl+E` to pick one. Edit `${1:...}` fields with `Tab`/`Enter`; `$0` sets the final cursor position. The date macros and `$ZKN_ID` expand automatically.

## Keys

| Browser | Action | Editor | Action |
| --- | --- | --- | --- |
| `j`/`k`, `Enter` | Move, open | `i`, `Esc` | Insert, Normal |
| `n` / `N` | Create note / folder | `:w`, `:wq`, `:q!` | Save, close, discard |
| `r`, `m`, `d` | Rename, move, Trash | `Ctrl+E` | Snippet picker (empty note) |
| `/` | Find filename | `[[` | Link autocomplete (Insert mode) |
| `o`, `b` | Outgoing links, backlinks | `Ctrl+O`, `Ctrl+B` | Outgoing links, backlinks |
| `q` | Quit | `Ctrl+P`, `Ctrl+T` | Citations, link history |

In both panes: `Alt+h`/`Alt+l` resize, `Ctrl+G` switches light/dark/auto. Width and theme adjustments from hotkeys are session-only.

[Roadmap](ROADMAP.md) · [Lessons](docs/lessons/) · MIT license

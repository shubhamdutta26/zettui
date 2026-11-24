# Zettui

A local-first, Vim-style terminal notes manager written in Rust. Browse and edit Markdown and other UTF-8 text files, connect notes with wiki links, insert Zotero citations, use reusable snippets, and search across your notes. Your files stay on disk.

## Installation

Download Linux releases from [GitHub Releases](https://github.com/shubhamdutta26/zettui/releases) (portable x86-64 archive, `.deb` for Debian/Ubuntu, `.rpm` for Fedora, or `PKGBUILD` for Arch/Omarchy), or build from source with a compatible Rust toolchain:

```bash
git clone https://github.com/shubhamdutta26/zettui.git
cd zettui
cargo install --path . --locked
```

Run with `zettui ~/Documents/Notes`, or simply `zettui` when `notes_dir` is configured.

## Configuration

Edit `~/.config/zettui/config.toml`:

```toml
notes_dir = "/home/USER/Documents/Notes"
bibliography = "/home/USER/Documents/Zotero/references.json"
theme = "auto"                    # auto | dark | light | midnight | ...
# light_theme = "catppuccin-latte"
# dark_theme = "everforest"
browser_width_percent = 35       # 20–70
# snippets_dir = "/home/USER/.config/zettui/snippets"
```

`auto` follows your system's light/dark appearance, including Omarchy. Theme and pane-width keyboard adjustments are session-only; edit TOML to make them permanent. Zotero exports should include citation keys (Better CSL JSON recommended).

## Appearance (v0.7.7)

The interface uses a compact workspace header, accent-colored active panel borders, muted inactive borders, a vertical file selection marker, compact directory markers, padded panels and a lighter status area. The browser remains a current-directory list; Enter opens a folder and h returns to its parent.

Set `theme = "midnight"` for the new Noodle-inspired dark palette, or cycle themes with `Ctrl+G`. Existing palettes and custom color overrides are retained. Available themes: `auto`, `dark`, `light`, `nord`, `dracula`, `everforest`, `rose-pine`, `kanagawa`, `solarized-dark`, `catppuccin-latte`, `rose-pine-dawn`, `gruvbox`, `tokyo-night`, `midnight`.

In Auto mode, optional `light_theme` and `dark_theme` select fixed palettes for the corresponding system appearance. Omitting them preserves the original defaults.

## Interface refinement (v0.7.8)

The status bar shows the current mode, saved or modified state, contextual shortcuts, live word count and one-based cursor line and column. Word count uses whitespace-separated tokens in the entire live buffer, including Markdown and YAML. On narrow terminals, cursor statistics may be hidden to preserve the main status.

## Keyboard shortcuts

| Context | Keys | Action |
| --- | --- | --- |
| Browser | `j` / `k`, `↑` / `↓` | Move through files |
| Browser | `Enter`, `l`, `h` | Open file/folder; enter folder only; go to parent folder |
| Browser | `n` / `N` | Create note / folder |
| Browser | `r`, `m`, `d` | Rename, move, send to Trash (confirmation) |
| Browser | `R`, `/`, `q` | Refresh; search filenames in current folder; quit |
| Browser | `o` / `b` | Outgoing links / backlinks for selected note |
| Editor (Normal) | `/`, `?`, `n`, `N` | In-note search forward/backward and repeat matches |
| Editor | `i`, `Esc` | Enter Insert mode / return to Normal mode |
| Editor | `h` / `j` / `k` / `l`, `w` / `b`, `gg` / `G` | Vim cursor navigation |
| Editor | `u`, `Ctrl+R`, `v` | Undo, redo, Visual mode |
| Editor | `Ctrl+S`, `:w`, `:wq`, `:q`, `:q!` | Save; save; save and close; close; discard and close |
| Editor (Insert) | `[[` | Search and insert a wiki link to an existing note |
| Editor | `Ctrl+O`, `Ctrl+B`, `Ctrl+T` | Outgoing links, backlinks, return to previously linked note |
| Editor | `Ctrl+P`, `Ctrl+E` | Zotero citation picker; snippet picker (empty note only) |
| Anywhere | `Ctrl+F`, `Alt+T` | Full-text search; search YAML tags |
| Anywhere | `Alt+h` / `Alt+l` (or `Alt+←` / `Alt+→`) | Narrow / widen file-browser pane |
| Anywhere | `Ctrl+G` | Cycle all built-in themes |
| Search popup | `Tab`, `↑` / `↓`, `Enter`, `Esc` | Toggle text/tags, select result, open, cancel |
| Pickers | `↑` / `↓`, `Enter`, `Esc`, `Ctrl+U` | Select, confirm, cancel, clear search (where supported) |
| Snippet fields | `Tab` / `Enter`, `Shift+Tab` | Next / previous field |

**Cursor:** Normal mode uses a blinking block; Insert mode uses a **blinking vertical bar** (subject to terminal support).

## Notes and snippets

- `[[Note Name]]` links, outgoing links, and backlinks work across your notes; duplicate filenames may require folder-qualified links.
- Place `.md` snippets in `~/.config/zettui/snippets/` (or `snippets_dir`). In an empty note, press `Ctrl+E`, pick a snippet, and fill its numbered fields. Date variables and `$ZKN_ID` expand automatically.
- `Ctrl+P` searches your Zotero bibliography and inserts Pandoc-style citations such as `[@Jain2017]`.
- `Ctrl+F` searches saved text in nested folders; `Alt+T` searches YAML frontmatter `tags:`. Search does not include unsaved edits or inline `#hashtags`. Files larger than 1 MiB are skipped.

[Roadmap](ROADMAP.md) · [Development lessons](docs/lessons/) · MIT License

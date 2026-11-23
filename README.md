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
theme = "auto"                    # auto | dark | light
browser_width_percent = 35       # 20–70
# snippets_dir = "/home/USER/.config/zettui/snippets"
```

`auto` follows your system's light/dark appearance, including Omarchy. Theme and pane-width keyboard adjustments are session-only; edit TOML to make them permanent. Zotero exports should include citation keys (Better CSL JSON recommended).

## Keyboard shortcuts

| Context | Keys | Action |
| --- | --- | --- |
| Browser | `j` / `k`, `↑` / `↓` | Move through files |
| Browser | `Enter` / `l`, `h` | Open file/folder; go to parent folder |
| Browser | `n` / `N` | Create note / folder |
| Browser | `r`, `m`, `d` | Rename, move, send to Trash (confirmation) |
| Browser | `R`, `/`, `q` | Refresh; search filenames in current folder; quit |
| Browser | `o` / `b` | Outgoing links / backlinks for selected note |
| Editor | `i`, `Esc` | Enter Insert mode / return to Normal mode |
| Editor | `h` / `j` / `k` / `l`, `w` / `b`, `gg` / `G` | Vim cursor navigation |
| Editor | `u`, `Ctrl+R`, `v` | Undo, redo, Visual mode |
| Editor | `Ctrl+S`, `:w`, `:wq`, `:q`, `:q!` | Save; save; save and close; close; discard and close |
| Editor (Insert) | `[[` | Search and insert a wiki link to an existing note |
| Editor | `Ctrl+O`, `Ctrl+B`, `Ctrl+T` | Outgoing links, backlinks, return to previously linked note |
| Editor | `Ctrl+P`, `Ctrl+E` | Zotero citation picker; snippet picker (empty note only) |
| Anywhere | `Ctrl+F`, `Alt+T` | Full-text search; search YAML tags |
| Anywhere | `Alt+h` / `Alt+l` (or `Alt+←` / `Alt+→`) | Narrow / widen file-browser pane |
| Anywhere | `Ctrl+G` | Cycle Auto / Dark / Light themes |
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

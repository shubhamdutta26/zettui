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

Copy the documented [config.example.toml](config.example.toml) to `~/.config/zettui/config.toml` (or `$XDG_CONFIG_HOME/zettui/config.toml`) and edit it:

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/zettui"
cp -n config.example.toml "${XDG_CONFIG_HOME:-$HOME/.config}/zettui/config.toml"
```

The canonical configuration uses TOML tables:

```toml
[workspace]
notes_dir = "~/Documents/Notes"
bibliography = "~/Zotero/references.json"

[appearance]
theme = "auto"
light_theme = "catppuccin-latte"
dark_theme = "everforest"

[layout]
browser_width_percent = 35

[markdown]
syntax_highlighting = true
highlight_code_blocks = true
highlight_frontmatter = true
dim_delimiters = true

[markdown.heading_colors]
# h1 = "#CBA6F7" # Optional; inherits adaptive color if omitted.
```

### Configuration reference

All keys are optional. Restart Zettui after changing the file. Session commands do not write it. The CLI notes path, `--bibliography` and `--theme` override the corresponding configured values. Relative configured paths resolve against the config directory; `~/` expands the home directory.

| Table | Key | Default / meaning |
| --- | --- | --- |
| `workspace` | `notes_dir` | Current directory if no CLI path |
| `workspace` | `bibliography` | Unset; Zotero JSON with citation keys (Better CSL JSON recommended) |
| `workspace` | `snippets_dir` | Zettui config directory's `snippets/` |
| `appearance` | `theme` | `"auto"`; any built-in theme |
| `appearance` | `light_theme` | `"light"`; fixed palette for Auto light appearance |
| `appearance` | `dark_theme` | `"dark"`; fixed palette for Auto dark appearance |
| `layout` | `browser_width_percent` | `35`; integer 20–70 |
| `editor` | `line_numbers` | `true` |
| `editor` | `word_wrap` | `true`; visual wrapping |
| `markdown` | `syntax_highlighting` | `true`; master switch for editor and preview |
| `markdown` | `highlight_code_blocks` | `true`; fenced code and its markers |
| `markdown` | `highlight_frontmatter` | `true`; YAML frontmatter |
| `markdown` | `dim_delimiters` | `true`; mute formatting delimiters, fence and table markers; heading markers retain heading color |
| `markdown.heading_colors` | `h1`, `h2`, `h3`, `h4`, `h5`, `h6` | Unset; each uses adaptive light/dark color; optional quoted `#RRGGBB` overrides |
| `zen` | `enabled` | `false` |
| `zen` | `width` | `80`; effective range 20–240 columns |
| `zen` | `show_line_numbers` | `false` |
| `zen` | `show_status_bar` | `true` |
| `zen` | `show_title` | `true` |
| `focus` | `enabled` | `false` |
| `focus` | `mode` | `"paragraph"`; `"line"`, `"paragraph"`, `"section"` |
| `focus` | `dim_strength` | `0.65`; clamped to 0–0.85 |
| `focus` | `typewriter` | `false`; independent of focus |
| `colors` | `background`, `surface`, `foreground`, `muted`, `border`, `accent`, `directory`, `selected_bg`, `selected_fg`, `warning`, `success` | Unset; optional quoted `#RRGGBB` global theme overrides |

Heading overrides apply across themes and to both preview and editor. Choose colors readable on your background. Missing heading overrides use adaptive colors. Disabling fenced code or YAML highlighting leaves those blocks plain, without interpreting their contents as Markdown. Inline code remains highlighted unless the master switch is off. Search and focus highlighting are independent of the master switch. Invalid heading colors are configuration errors.

Legacy top-level workspace, theme and browser width keys are still accepted; if both forms specify a setting, the sectioned value wins. Keep legacy root keys before any TOML table headers. `[colors]`, `[editor]`, `[zen]`, and `[focus]` retain their existing locations.

## Appearance (v0.7.7)

The interface uses a compact workspace header, accent-colored active panel borders, muted inactive borders, a vertical file selection marker, compact directory markers, padded panels and a lighter status area. The browser remains a current-directory list; Enter opens a folder and h returns to its parent.

Set `theme = "midnight"` for the new Noodle-inspired dark palette, or cycle themes with `Ctrl+G`. Existing palettes and custom color overrides are retained. Available themes: `auto`, `dark`, `light`, `nord`, `dracula`, `everforest`, `rose-pine`, `kanagawa`, `solarized-dark`, `catppuccin-latte`, `rose-pine-dawn`, `gruvbox`, `tokyo-night`, `midnight`.

In Auto mode, optional `light_theme` and `dark_theme` select fixed palettes for the corresponding system appearance. Omitting them preserves the original defaults.

## Interface refinement (v0.7.8)

The status bar shows the current mode, saved or modified state, contextual shortcuts, live word count and one-based cursor line and column. Word count uses whitespace-separated tokens in the entire live buffer, including Markdown and YAML. On narrow terminals, cursor statistics may be hidden to preserve the main status.

The header shows the notes workspace and current directory as a breadcrumb (for example, `Notes › Antibody › VHH`). Opening a note through a link or search uses the note's actual parent directory. The filename remains in the editor panel title. On narrow terminals, older ancestors are omitted and the current folder is prioritized. The browser status bar uses a root-relative path (`.` at the workspace root). Breadcrumbs are display-only; existing navigation keys are unchanged.

Search, tag, filename, citation, wiki-link and snippet pickers share a theme-aware popup layout with a query field, result counts, highlighted selection and contextual help. Vault results show note excerpts and root-relative locations; citation results show citation keys. The snippet picker shows a read-only template preview on wide terminals, and the snippet form keeps the active field visible. Smaller terminals use a compact layout. Search behavior, insertion semantics and keyboard shortcuts are unchanged.

## Markdown highlighting (v0.7.9 development)

Markdown notes use theme-aware source highlighting in the browser preview and editor. Headings, emphasis, inline code, fenced code, YAML frontmatter, links, wiki links, tags, Pandoc citations, math, task markers, blockquotes and comments have distinct styles. Heading levels H1–H6 use six distinct colors (including the heading markers), with separate light and dark palettes. Other formatting delimiters remain visible but muted. Fenced code uses lightweight language-aware lexical coloring for common languages; unknown languages remain plain. This is source highlighting, not a complete Markdown renderer or language parser. Existing themes and `[colors]` overrides supply the palette; `[markdown]` and `[markdown.heading_colors]` offer optional customization.

The active Files panel or opened-note editor uses heavy Unicode border characters; the read-only Preview remains inactive. Enter still opens notes, and `l` still only enters directories. Search matches are overlaid on syntax highlighting. No changes to the stored Markdown are made by highlighting.

## Distraction-free writing (v0.7.9 development)

Open a note with Enter, then use the following commands in Normal mode. Zen, Focus and Typewriter are independent session settings. Zen hides the browser and borders, centers the editor at a configurable width, and replaces the full footer with a minimal status area. Commands, searches and notices remain visible when needed. Closing a note restores the browser; reopening a note uses the current writing settings.

| Command | Normal-mode shortcut | Action |
| --- | --- | --- |
| `:zen` | `Space z` | Toggle Zen |
| `:focus` | `Space f` | Toggle focus using last selected level |
| `:focus line` | `Space 1` | Focus current logical line |
| `:focus paragraph` | `Space 2` | Focus current paragraph |
| `:focus section` | `Space 3` | Focus current Markdown section |
| `:focus off` | | Disable focus |
| `:typewriter` | `Space t` | Toggle experimental logical-line centering |
| `:set number` / `:set nonumber` | `Space n` (toggle) | Show/hide line numbers in current layout |
| `:set status` / `:set nostatus` | `Space s` (toggle) | Show/hide Zen status bar |
| `:set wrap` / `:set nowrap` | | Toggle visual wrapping |
| `:set textwidth=80` | | Set Zen pane width (20–240 columns, including gutter if enabled) |

Space is reserved as a leader in Normal mode; Escape cancels a pending leader. Insert-mode spaces are unchanged. Focus dimming is suspended in Visual mode and search matches retain their colors. Paragraph focus uses blank lines and headings; section focus uses the nearest preceding ATX heading and ends at the next heading of equal or higher rank. YAML and fenced code headings are ignored. Typewriter mode centers logical lines approximately; long wrapped lines may move the cursor away from the center. It does not insert padding into your notes.

Optional writing settings in `~/.config/zettui/config.toml`:

```toml
[editor]
line_numbers = true
word_wrap = true

[zen]
enabled = false
width = 80
show_line_numbers = false
show_status_bar = true
show_title = true

[focus]
enabled = false
mode = "paragraph" # line | paragraph | section
dim_strength = 0.65 # clamped to 0–0.85
typewriter = false
```

Missing settings use these defaults. Command changes are session-only. Heading colors use contrast-aware light/dark palettes unless explicitly overridden in `[markdown.heading_colors]`.

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

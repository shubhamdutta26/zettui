# Keyboard and writing commands

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
| Browser / Editor | `F2` / `F3` / `F4` | Toggle Files / Preview / Outline preferences |
| Editor (Normal) | `F6`, `Space l`, `:outline` | Show and focus Outline; F6 returns to editor |
| Outline | `j` / `k`, `↑` / `↓`, `Home` / `End` | Move through visible headings |
| Outline | `h` / `←`, `l` / `→` | Collapse or select parent; expand or select first child |
| Outline | `Enter`, `Esc` | Jump to heading and return to editor; return without moving cursor |
| Browser / Editor | `Alt+↑` / `Alt+↓` | Widen / narrow outline preference |
| Anywhere | `Ctrl+G` | Cycle native, terminal and built-in themes |
| Search popup | `Tab`, `↑` / `↓`, `Enter`, `Esc` | Toggle text/tags, select result, open, cancel |
| Pickers | `↑` / `↓`, `Enter`, `Esc`, `Ctrl+U` | Select, confirm, cancel, clear search (where supported) |
| Snippet fields | `Tab` / `Enter`, `Shift+Tab` | Next / previous field |

**Cursor:** Normal mode uses a blinking block; Insert mode uses a **blinking vertical bar** (subject to terminal support).


## Panel visibility

F2 toggles Files while editing; closing the note always restores the file browser without changing the preference. F3 toggles the browser Preview. F4 toggles the Outline of the open note. In Normal mode, `Space e`, `Space p`, `Space o` and `:toggle files`, `:toggle preview`, `:toggle outline` perform the same actions. Function keys work in browser and editor modes (except when a modal picker owns input). The editor always remains available and Preview never receives keyboard focus. F6 (Normal mode), `Space l` or `:outline` opens and focuses the Outline. Its selection is independent of the editor cursor until Enter jumps; Esc or F6 returns without moving the cursor. Collapsing a branch only hides outline entries, not note text. A collapsed ancestor represents the current section when its active descendant is hidden. Collapse state is per open document and resets if heading rows, levels or titles change. Hiding the Outline or shrinking the terminal until it cannot be displayed returns focus to the editor. Sidebars temporarily disappear in Zen; narrow terminals temporarily omit the Outline if necessary. Panel width changes and toggles last for the session only; settings in `[layout]` set startup preferences. Outline width changes by five percentage points, within 15–50%. Files resizing is unchanged.

## Distraction-free writing

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

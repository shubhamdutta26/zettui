# Keyboard and writing commands

## Keyboard shortcuts

Shortcuts are context-sensitive. Active dialogs, pickers, in-note search input and
the Ex command bar take precedence over global shortcuts. Editor text motions
not intercepted by Zettui are delegated to the pinned `edtui` editing engine.
See [Vim-style commands](VIM-COMMANDS.md) for the Ex reference, inherited editor keys and known conflicts. Browser file actions require plain keys (Shift is allowed for uppercase keys); Ctrl+G and Ctrl+T remain browser shortcuts. Global shortcuts are resolved before browser actions.

| Context | Keys | Action |
| --- | --- | --- |
| Browser | `j` / `k`, `↑` / `↓` | Move through files |
| Browser | `Enter` | Open selected file or folder |
| Browser | `l` / `→` | Enter folder only; never focus Preview |
| Browser | `h` / `←` / `Backspace` | Go to parent folder |
| Browser | `n` / `N` | Create note / folder |
| Browser | `r`, `m`, `d` | Rename, move, send to Trash (confirmation) |
| Browser | `R`, `/`, `q` | Refresh; search filenames in current folder; quit |
| Browser | `o` / `b` | Outgoing links / backlinks for selected note |
| Browser | `Ctrl+T` | Return to previously linked note |
| Editor (Normal) | `/`, `?`, `n`, `N` | In-note search forward/backward and repeat matches |
| Editor | `i`, `Esc` | Enter Insert mode / return to Normal mode |
| Editor | `h` / `j` / `k` / `l`, `w` / `b`, `gg` / `G` | Vim cursor navigation |
| Editor | `u`, `Ctrl+R`, `v` | Undo, redo, Visual mode |
| Editor / Outline | `Ctrl+S` | Save active note |
| Editor (Normal) | `:w`, `:wq`, `:q`, `:q!` | Save; save and close; close; discard and close |
| Editor (Insert, Markdown) | `[[` | Search and insert a wiki link to an existing note |
| Editor | `Ctrl+O`, `Ctrl+B`, `Ctrl+T` | Outgoing links, backlinks, return to previously linked note |
| Editor | `Ctrl+P`, `Ctrl+E` | Zotero citation picker; snippet picker (empty note only) |
| Global (outside modal input) | `Ctrl+F`, `Alt+T` | Full-text search; search YAML tags |
| Global (outside modal input) | `Alt+h` / `Alt+l` (or `Alt+←` / `Alt+→`) | Narrow / widen file-browser pane |
| Global (outside modal input) | `F2` / `F3` / `F4` | Toggle Files / Preview / Outline preferences |
| Browser / Editor (Normal) / Outline | `F5` | Fuzzy-switch open buffers |
| Editor (Normal) | `Space b b` | Fuzzy-switch open buffers |
| Editor (Normal) | `]b` / `[b` | Next / previous buffer |
| Editor (Normal) | `Space b p` | Activate previously active buffer |
| Editor (Normal) | `Space b d` / `Space b o` | Close current / other buffers subject to dirty-buffer checks |
| Editor (Normal) | `Space b t` / `Space b s` | Toggle tabs / save all buffers |
| Global (outside modal input) | `F9` | Toggle dot-prefixed folders |
| Global (outside modal input) | `F10` | Open writing analytics |
| Editor (Normal) | `F6`, `Space l`, `:outline` | Show and focus Outline; F6 returns to editor |
| Editor (Normal) / Outline | `F7` | Search headings in the open Markdown note |
| Editor (Normal) | `Space h`, `:headings` | Search headings without showing the Outline |
| Browser / Editor (Normal) / Outline | `F8` | Global command palette |
| Editor (Normal) | `Space c`, `:commands` | Global command palette |
| Editor (Normal, Markdown) | `Space x`, `:export` | Export document |
| Command palette | `↑` / `↓`, `Ctrl+P` / `Ctrl+N`, `Home` / `End` | Select previous/next or first/last action |
| Command palette | `Enter`, `Esc` / `F8`, `Ctrl+U` | Execute, cancel, clear query |
| Heading search | `↑` / `↓`, `Ctrl+P` / `Ctrl+N`, `Home` / `End` | Select previous/next or first/last result |
| Heading search | `Enter`, `Esc` / `F7`, `Ctrl+U` | Jump, cancel, clear query |
| Outline | `j` / `k`, `↑` / `↓`, `Home` / `End` | Move through visible headings |
| Outline | `h` / `←`, `l` / `→` | Collapse or select parent; expand or select first child |
| Outline | `Enter`, `Esc` | Jump to heading and return to editor; return without moving cursor |
| Global (outside modal input) | `Alt+↑` / `Alt+↓` | Widen / narrow outline preference |
| Browser / Editor / Outline / Writing Analytics | `Ctrl+G` | Cycle native, terminal and built-in themes |
| Vault search | `Tab`, `↑` / `↓`, `Enter`, `Esc` | Switch text/tags, select, open, cancel |
| Filename search | `↑` / `↓`, `Enter`, `Esc` | Select result; confirm selection in Files (Enter again opens); cancel |
| Vault / filename / wiki-link / snippet / citation pickers | Type, `Backspace`, `↑` / `↓`, `Enter`, `Esc`, `Ctrl+U` | Filter, edit query, select, confirm, cancel, clear query |
| Citation picker | `Ctrl+T`, `Ctrl+R`, `Ctrl+V` | Toggle selected citation mark; reload bibliography; preview highlighted reference |
| Citation preview | `↑` / `↓`, `PgUp` / `PgDn`, `←` / `→` or `h` / `l`, `o`, `r`, `Esc` | Scroll, select PDF, launch PDF, retry Zotero lookup, close preview |
| Citation audit | `↑` / `↓`, `j` / `k`, `Enter`, `Esc` | Select unresolved occurrence, jump, close |
| Buffer picker | Type, `Backspace`, `↑` / `↓`, `Ctrl+P` / `Ctrl+N`, `Enter`, `Esc` / `F5`, `Ctrl+U` | Filter, edit query, select, activate, cancel, clear query |
| In-note search input | Type, `Backspace`, `Ctrl+U`, `Enter`, `Esc` | Edit query, clear, confirm or cancel |
| Ex command bar | Type, `Backspace`, `Enter`, `Esc` | Edit command, execute or cancel |
| Link dialog | `j` / `k`, `↓` / `↑`, `Enter`, `Esc` / `q` | Select, follow or cancel |
| File-operation prompt | Type, `Backspace`, `Ctrl+U`, `Enter`, `Esc` | Edit path/name, clear, confirm or cancel |
| Trash confirmation | `y`, `n` / `Esc` / `Enter` | Confirm Trash or cancel |
| Snippet fields | Type, `Backspace`, `Tab` / `Enter`, `Shift+Tab`, `Ctrl+U`, `Esc` | Edit current field, next/finish, previous, clear current field, cancel |

**Cursor:** Normal mode uses a blinking block; Insert mode uses a **blinking vertical bar** (subject to terminal support).


## Panel visibility

F2 toggles Files while editing; closing the note always restores the file browser without changing the preference. F3 toggles the browser Preview. F4 toggles the Outline of the open note. In Normal mode, `Space e`, `Space p`, `Space o` and `:toggle files`, `:toggle preview`, `:toggle outline` perform the same actions. F2/F3/F4 work in browser and editor modes (except when modal input owns the keys). F5–F8 have the mode restrictions in the table above. The editor always remains available and Preview never receives keyboard focus. F2/F3/F4 change session preferences even when the corresponding panel is not currently displayed. F6 (Normal mode), `Space l` or `:outline` opens and focuses the Outline. Its selection is independent of the editor cursor until Enter jumps; Esc or F6 returns without moving the cursor. Collapsing a branch only hides outline entries, not note text. A collapsed ancestor represents the current section when its active descendant is hidden. Collapse state is per open document and resets if heading rows, levels or titles change. Hiding the Outline or shrinking the terminal until it cannot be displayed returns focus to the editor. Sidebars temporarily disappear in Zen; narrow terminals temporarily omit the Outline if necessary. Panel width changes and toggles last for the session only; settings in `[layout]` set startup preferences. Outline width changes by five percentage points, within 15–50%. Files resizing is unchanged.

## Heading search

F7, `Space h` or `:headings` opens a searchable list of the current Markdown note's headings in Normal mode. F7 also works with the Outline focused. Empty queries show source order; typing ranks case-insensitive exact, prefix, substring and ordered-subsequence matches, so `sec` can match `Size exclusion chromatography`. Results show heading level and one-based line number; duplicate titles are independent. Use arrows or Ctrl+N/Ctrl+P to move, Home/End for the first/last result, Ctrl+U to clear, Enter to jump or Esc/F7 to cancel. Ctrl+P still opens Zotero outside this picker. Search includes unsaved and collapsed headings, works in Zen and does not change panel visibility or save the note. It searches raw heading titles in the open note, not all files. Esc restores the previous focus; Enter returns to the editor. No new configuration is needed.

## Global command palette

F8 opens a searchable list of available application commands from the browser, Normal-mode editor or focused Outline. `Space c` and `:commands` also open it from the Normal-mode editor. Ctrl+P remains the Zotero shortcut in the editor. There is no Ctrl+Shift+P palette binding: terminals may encode it identically to Ctrl+P. The palette cannot interrupt Insert/Visual mode or an active dialog, search or `:` command bar. Search command titles, shortcut hints or aliases (e.g. `delete`, `zotero`, `f7`, `srchhdngs`). Empty queries retain catalog order; ranked matches use the same fuzzy matcher as heading search. Use arrows or Ctrl+N/Ctrl+P, Home/End, Backspace and Ctrl+U; Enter executes, Esc or F8 cancels. No match means no action. Canceling restores the prior focus; executing returns to the editor if the Outline was focused, except for the Outline focus toggle itself.

Only context-appropriate actions are listed: file operations and Quit in browser mode, Save and writing modes in the editor, heading navigation in Markdown notes, citations with a configured bibliography, snippets in empty notes, and link navigation when unsaved edits permit it. File/Trash actions reuse existing prompts; closing and saving still enforce the existing safety checks. Explicit forced discard (`:q!`) is deliberately not offered. The palette does not execute arbitrary shell commands and does not modify the configuration file.

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
| `:set number` / `:set nonumber` / `:toggle number` | `Space n` (toggle) | Show/hide line numbers in current layout |
| `:set status` / `:set nostatus` / `:toggle status` | `Space s` (toggle) | Show/hide Zen status bar |
| `:set wrap` / `:set nowrap` | | Toggle visual wrapping |
| `:set textwidth=80` | | Set Zen pane width (20–240 columns, including gutter if enabled) |

Space is reserved as a leader in Normal mode; Escape cancels a pending leader. Insert-mode spaces are unchanged. Focus dimming is suspended in Visual mode and search matches retain their colors. Paragraph focus uses blank lines and headings; section focus uses the nearest preceding ATX heading and ends at the next heading of equal or higher rank. YAML frontmatter, fenced and indented code, multiline HTML comments and standalone display-math blocks are excluded. Setext headings and headings inside list items or blockquotes are not supported. Typewriter mode centers logical lines approximately; long wrapped lines may move the cursor away from the center. It does not insert padding into your notes.

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


## Writing analytics

| Key | Action |
| --- | --- |
| `F10`, `:analytics`, `:stats` | Open writing analytics (`F8` palette also available) |
| `Tab` / `Shift+Tab`, `1`–`3` | Cycle views / choose Overview, Month, Year |
| `j` / `k`, Up / Down | Scroll overview or move calendar by week |
| `h` / `l`, Left / Right | Move calendar by day |
| `[` / `]`, PageUp / PageDown | Previous / next month or year (Page keys scroll overview) |
| `t` / Home | Return to today / top of overview |
| `r` | Refresh statistics |
| `Ctrl+G` | Cycle themes without leaving the dashboard |
| `Esc` / `q` / `F10` | Close dashboard |

### Research metadata (Phase III)

| Shortcut / command | Action |
| --- | --- |
| `Space m` / `F11` / `:metadata` / `:metadata-edit` | Open Metadata Editor |
| `i` / Esc | Insert / Normal mode |
| `:w` / `:wq` | Apply to note / apply and close |
| `:q` / `:q!` | Close clean popup / discard unapplied edits |
| Ctrl+arrows (Normal mode) | Resize popup |

See [Research Toolkit](RESEARCH-TOOLKIT.md#phase-iii--metadata-editor).

### Document export (Research Toolkit)

Use `Space x` or `:export` in Normal mode, or F8 → **Export document or folder**. In the
export dialog: Tab/Up/Down selects fields, Left/Right changes format, Space
or Left/Right toggles citation processing, Ctrl+u clears a text field, Enter
exports, Ctrl+o opens the last successful output, Ctrl+d toggles diagnostics, PageUp/PageDown scroll details, and Esc closes when idle. PDF engine selection uses Left/Right in the PDF engine field. See [Document export](DOCUMENT-EXPORT.md).

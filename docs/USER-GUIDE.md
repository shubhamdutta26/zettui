# User guide

## Appearance

The interface uses a compact workspace header, accent-colored active panel borders, muted inactive borders, a vertical file selection marker, compact directory markers, padded panels and a lighter status area. The browser remains a current-directory list; Enter opens a folder and h returns to its parent.

Set `theme = "midnight"` under `[appearance]` for the Noodle-inspired dark palette, or cycle themes with `Ctrl+G`. Existing palettes and custom color overrides are retained. Available themes: `system`, `terminal`, `auto`, `dark`, `light`, `nord`, `dracula`, `everforest`, `rose-pine`, `kanagawa`, `solarized-dark`, `catppuccin-latte`, `rose-pine-dawn`, `gruvbox`, `tokyo-night`, `midnight`.

In Auto mode, optional `light_theme` and `dark_theme` select fixed palettes for the corresponding system appearance. Omitting them preserves the original defaults.

## Interface refinement

The status bar shows the current mode, saved or modified state, contextual shortcuts, live word count and one-based cursor line and column. Word count uses whitespace-separated tokens in the entire live buffer, including Markdown and YAML. On narrow terminals, cursor statistics may be hidden to preserve the main status.

The header shows the notes workspace and current directory as a breadcrumb (for example, `Notes › Antibody › VHH`). Opening a note through a link or search uses the note's actual parent directory. The filename appears in the editor panel title when buffer tabs are not shown. On narrow terminals, older ancestors are omitted and the current folder is prioritized. The browser status bar uses a root-relative path (`.` at the workspace root). Breadcrumbs are display-only; existing navigation keys are unchanged.

Search, tag, filename, citation, wiki-link and snippet pickers share a theme-aware popup layout with a query field, result counts, highlighted selection and contextual help. Vault results show note excerpts and root-relative locations; citation results show citation keys. The snippet picker shows a read-only template preview on wide terminals, and the snippet form keeps the active field visible. Smaller terminals use a compact layout. Search behavior, insertion semantics and keyboard shortcuts are unchanged.

## Markdown highlighting

Markdown notes use theme-aware source highlighting in the browser preview and editor. Headings, emphasis, inline code, fenced code, YAML frontmatter, links, wiki links, tags, Pandoc citations, math, task markers, blockquotes and comments have distinct styles. Heading levels H1–H6 use six distinct colors (including the heading markers), with separate light and dark palettes. Other formatting delimiters remain visible but muted. Fenced code uses lightweight language-aware lexical coloring for common languages; unknown languages remain plain. This is source highlighting, not a complete Markdown renderer or language parser. Existing themes and `[colors]` overrides supply the palette; `[markdown]` and `[markdown.heading_colors]` offer optional customization.

The active Files panel or opened-note editor uses heavy Unicode border characters; the read-only Preview remains inactive. Enter still opens notes, and `l` still only enters directories. Search matches are overlaid on syntax highlighting. No changes to the stored Markdown are made by highlighting.

## Panel layout

Use F2 to hide/show Files while editing, F3 to hide/show Preview in browser mode, and F4 to show/hide the Outline of the open note. The outline shows current unsaved Markdown headings and follows the cursor. Press F6 in Normal mode to focus it, use j/k to select headings, h/l to collapse or expand branches (or move to parent/child), and Enter to jump to the heading. Esc or F6 returns without moving the cursor. Alt+Up/Down adjusts its preferred width. The editor stays visible; closing a note restores Files so browsing remains possible. Zen and narrow terminals temporarily hide sidebars without forgetting their preferences. See [keyboard shortcuts](KEYBINDINGS.md) for Normal-mode alternatives.

Press F7 (or `Space h` / `:headings` in Normal mode) to fuzzy-search headings in the current note, even with the Outline hidden or in Zen. Type a partial title or abbreviation, select a result with arrows, and press Enter to jump. Search uses the live unsaved buffer, includes collapsed branches and distinguishes repeated titles by line number. Esc cancels without moving the cursor.

## Global command palette

Press F8 from the browser, Normal-mode editor or focused Outline. `Space c` and `:commands` also work in the Normal-mode editor. Search by title, shortcut or alias (for example `delete`, `zotero`, `resize`, or `srchhdngs`). Use arrows or Ctrl+N/Ctrl+P to select, Enter to execute and Esc to cancel. Ctrl+P still opens Zotero outside the navigation pickers; Ctrl+Shift+P is not a separate palette shortcut.

Only context-appropriate commands are offered. Trash still requires confirmation, closing a modified note still requires saving or explicit discard, and a file changed externally is not overwritten. The palette cannot run shell commands or bypass an active dialog, Insert mode or Visual mode.

Heading navigation recognizes top-level ATX headings (`#` through `######`), not Setext or list/blockquote headings. It ignores YAML frontmatter, fenced and indented code, multiline HTML comments and standalone display math. See [keyboard shortcuts](KEYBINDINGS.md) for all navigation bindings.

## Notes and snippets

- `[[Note Name]]` links, outgoing links, and backlinks work across your notes; duplicate filenames may require folder-qualified links.
- Place `.md` snippets in `~/.config/zettui/snippets/` (or `snippets_dir`). In an empty note, press `Ctrl+E`, pick a snippet, and fill its numbered fields. Date variables and `$ZKN_ID` expand automatically.
- `Ctrl+P` searches your Zotero bibliography and inserts Pandoc-style citations such as `[@Jain2017]`.
- `Ctrl+F` searches saved text in nested folders; `Alt+T` searches YAML frontmatter `tags:`. Search does not include unsaved edits or inline `#hashtags`. Files larger than 1 MiB are skipped. The editor separately limits opening files to 256,000 bytes (256 KB).

See [writing modes and shortcuts](KEYBINDINGS.md), [Vim-style commands](VIM-COMMANDS.md) and [CLI reference](CLI.md).


## Buffers and sessions

Use `:hide` to return to Files without closing the current note, then open another
with Enter. Switch with F5, `]b` or `[b`; save all with `:wa` and save all and
quit with `:wqa`. A clean quit restores saved notes and metadata on the next
launch when session restoration is enabled, not unsaved note contents. See
[Buffers and Sessions](BUFFERS-AND-SESSIONS.md) for safety rules and configuration.

## Writing analytics

Press F10 outside modal input (`:analytics`, `:stats` or
F8 command palette also work). Select `1` Overview, `2` Month or `3` Year;
Tab cycles the views. Use `h`/`l` by day, `j`/`k` by week, `[`/`]` by period,
`t` for today, `r` to refresh and Esc to close. Overview scrolls with `j`/`k`
or PageUp/PageDown. Closing the dashboard preserves the current note and edits.

To enable word goals, add to your Zettui configuration and restart:

```toml
[writing_goals]
daily_words = 1000
weekly_words = 5000
```

Zero disables a goal. Counters track gross edit activity, including accepted
insertions and undo/redo, rather than the saved length of your notes. Statistics
are installation-wide; the dashboard includes pending activity and the editor
flushes periodically and on graceful exit.

Import historical Zettlr data (use a stable source label per independent history):

```sh
zettui --stats-import-zettlr "$HOME/.config/Zettlr/stats.json" desktop
```

For source builds use `cargo run --locked --` instead of `zettui`. Export/import
snapshots to exchange history; never share the live state file. Real multi-computer
exchange has not yet been tested by the maintainer. See [writing analytics](WRITING-ANALYTICS.md)
for counting rules, storage, source labels, backups, transfer instructions and limitations.

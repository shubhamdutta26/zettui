# User guide

## Appearance

The interface uses a compact workspace header, accent-colored active panel borders, muted inactive borders, a vertical file selection marker, compact directory markers, padded panels and a lighter status area. The browser remains a current-directory list; Enter opens a folder and h returns to its parent.

Set `theme = "midnight"` for the Noodle-inspired dark palette, or cycle themes with `Ctrl+G`. Existing palettes and custom color overrides are retained. Available themes: `system`, `terminal`, `auto`, `dark`, `light`, `nord`, `dracula`, `everforest`, `rose-pine`, `kanagawa`, `solarized-dark`, `catppuccin-latte`, `rose-pine-dawn`, `gruvbox`, `tokyo-night`, `midnight`.

In Auto mode, optional `light_theme` and `dark_theme` select fixed palettes for the corresponding system appearance. Omitting them preserves the original defaults.

## Interface refinement

The status bar shows the current mode, saved or modified state, contextual shortcuts, live word count and one-based cursor line and column. Word count uses whitespace-separated tokens in the entire live buffer, including Markdown and YAML. On narrow terminals, cursor statistics may be hidden to preserve the main status.

The header shows the notes workspace and current directory as a breadcrumb (for example, `Notes › Antibody › VHH`). Opening a note through a link or search uses the note's actual parent directory. The filename remains in the editor panel title. On narrow terminals, older ancestors are omitted and the current folder is prioritized. The browser status bar uses a root-relative path (`.` at the workspace root). Breadcrumbs are display-only; existing navigation keys are unchanged.

Search, tag, filename, citation, wiki-link and snippet pickers share a theme-aware popup layout with a query field, result counts, highlighted selection and contextual help. Vault results show note excerpts and root-relative locations; citation results show citation keys. The snippet picker shows a read-only template preview on wide terminals, and the snippet form keeps the active field visible. Smaller terminals use a compact layout. Search behavior, insertion semantics and keyboard shortcuts are unchanged.

## Markdown highlighting

Markdown notes use theme-aware source highlighting in the browser preview and editor. Headings, emphasis, inline code, fenced code, YAML frontmatter, links, wiki links, tags, Pandoc citations, math, task markers, blockquotes and comments have distinct styles. Heading levels H1–H6 use six distinct colors (including the heading markers), with separate light and dark palettes. Other formatting delimiters remain visible but muted. Fenced code uses lightweight language-aware lexical coloring for common languages; unknown languages remain plain. This is source highlighting, not a complete Markdown renderer or language parser. Existing themes and `[colors]` overrides supply the palette; `[markdown]` and `[markdown.heading_colors]` offer optional customization.

The active Files panel or opened-note editor uses heavy Unicode border characters; the read-only Preview remains inactive. Enter still opens notes, and `l` still only enters directories. Search matches are overlaid on syntax highlighting. No changes to the stored Markdown are made by highlighting.

## Panel layout

Use F2 to hide/show Files while editing, F3 to hide/show Preview in browser mode, and F4 to show/hide the Outline of the open note. The outline shows current unsaved Markdown headings and follows the cursor. Press F6 in Normal mode to focus it, use j/k to select headings, h/l to collapse or expand branches (or move to parent/child), and Enter to jump to the heading. Esc or F6 returns without moving the cursor. Alt+Up/Down adjusts its preferred width. The editor stays visible; closing a note restores Files so browsing remains possible. Zen and narrow terminals temporarily hide sidebars without forgetting their preferences. See [keyboard shortcuts](KEYBINDINGS.md) for Normal-mode alternatives.

Press F7 (or `Space h` / `:headings` in Normal mode) to fuzzy-search headings in the current note, even with the Outline hidden or in Zen. Type a partial title or abbreviation, select a result with arrows, and press Enter to jump. Search uses the live unsaved buffer, includes collapsed branches and distinguishes repeated titles by line number. Esc cancels without moving the cursor.

## Notes and snippets

- `[[Note Name]]` links, outgoing links, and backlinks work across your notes; duplicate filenames may require folder-qualified links.
- Place `.md` snippets in `~/.config/zettui/snippets/` (or `snippets_dir`). In an empty note, press `Ctrl+E`, pick a snippet, and fill its numbered fields. Date variables and `$ZKN_ID` expand automatically.
- `Ctrl+P` searches your Zotero bibliography and inserts Pandoc-style citations such as `[@Jain2017]`.
- `Ctrl+F` searches saved text in nested folders; `Alt+T` searches YAML frontmatter `tags:`. Search does not include unsaved edits or inline `#hashtags`. Files larger than 1 MiB are skipped.

See [writing modes and shortcuts](KEYBINDINGS.md).

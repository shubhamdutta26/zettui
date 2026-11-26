# Architecture

`main.rs` declares modules and delegates startup to `app::start`. `cli.rs` resolves startup options before Ratatui initializes the terminal. `app.rs` owns the private application state, picker state, and startup coordination. Its child modules extend `App` with focused implementations; the event loop remains the single owner of application-state mutation.

| Application module | Responsibility |
| --- | --- |
| `app/browser.rs` | Directory listing, selection, navigation and previews |
| `app/appearance.rs` | Theme resolution, live refresh, cycling and pane resizing |
| `app/dialogs.rs` | Picker actions, search, links, citations, snippets and file prompts |
| `app/editing.rs` | Saving, editor command input and in-note search interception |
| `app/commands.rs` | Shared application-command execution |
| `app/events.rs` | Terminal lifecycle, cursor shape and input routing |
| `app/ui.rs` | Main interface and popup rendering |

These modules share `App` privately rather than passing large collections of state between unrelated public interfaces. Methods used by sibling modules have `pub(super)` visibility; implementation helpers remain private.

| Module | Responsibility |
| --- | --- |
| `commands` | Application command identifiers and context-specific keyboard/Ex bindings |
| `config` | Deserialize configuration, expand paths and validate options |
| `theme`, `system_theme` | Built-in palettes, native/terminal providers, user overrides |
| `editor` | Open note state and safe saving |
| `editor_search` | In-note search and match positions |
| `markdown` | Source-preserving styling using character offsets |
| `outline` | Top-level ATX headings, parent indices, section boundaries and cursor lookup |
| `focus`, `layout` | Writing modes, dimming and pane geometry |
| `search`, `links` | Vault search and note relationships |
| `bibliography`, `snippets` | Read-only reference and template loading |
| `file_ops` | Filesystem operations |

## Invariants

- Editing operates on raw text; rendering must not rewrite Markdown.
- External modifications must not be silently overwritten.
- Terminal palette queries run before the event loop, not while reading key events.
- Preserve character offsets for Unicode text and editor search overlays.
- Keep system theme fallback and explicit user override precedence intact.

## Heading navigation

`outline::extract` works on an immutable source snapshot and returns headings in source order with zero-based logical rows, nearest shallower parent indices, exclusive section ends, and borrowed raw titles. Focus mode uses the same parser. Extraction does not depend on syntax highlighting or theme settings. The parser handles top-level H1–H6 ATX headings, BOM/CRLF, YAML frontmatter, fenced and indented code, multiline HTML comments and standalone `$$` math blocks. It is a lightweight block scanner, not a full Markdown parser: Setext and nested list/blockquote headings are not supported, and inline HTML/code handling is limited. The read-only outline uses the same source snapshot as editor rendering; interactive outline navigation and heading search are subsequent phases.

## Panel layout

`layout::Panels` holds session visibility preferences and preferred outline width. Its geometry method returns `[Files, Editor/Preview, Outline]`, reserving the active pane and suppressing the outline on small terminals. Browser mode always retains Files; editor mode always retains the editor. Zen temporarily suppresses sidebars without changing preferences. The event loop retains its existing browser/editor input ownership; Preview and Outline are read-only. Panel commands flow through the shared command dispatcher and do not mutate the note buffer or configuration file.

## Rendering and theme caches

- `HighlightCache` holds one document's lexical styles and word count. Exact source text, resolved theme, Markdown status and settings form the cache key. Search and focus overlays operate on copies of base styles so cursor movements cannot contaminate the cache. The editor still serializes its buffer once per draw; this is not incremental parsing.
- `ThemeSnapshot` records actual palette file contents and legacy light-mode markers. The live-theme poll skips resolution when the snapshot is unchanged. Reading contents (rather than relying on timestamps) detects symlink replacements. Theme queries still happen before the terminal event loop.
- Existing periodic rendering and vault indexing behavior are unchanged. Benchmark before introducing further optimizations.

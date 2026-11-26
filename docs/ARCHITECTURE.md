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
| `app/palette.rs` | Context checks, command palette lifecycle and keyboard routing |
| `app/navigation.rs` | Outline and heading-search input, focus and shared cursor jumps |
| `app/ui.rs` | Main interface and popup rendering |

These modules share `App` privately rather than passing large collections of state between unrelated public interfaces. Methods used by sibling modules have `pub(super)` visibility; implementation helpers remain private.

| Module | Responsibility |
| --- | --- |
| `commands` | Application command identifiers, discoverable command catalog and context-specific keyboard/Ex bindings |
| `command_palette` | Context-filtered command snapshot, fuzzy ranking and selection |
| `config` | Deserialize configuration, expand paths and validate options |
| `theme`, `system_theme` | Built-in palettes, native/terminal providers, user overrides |
| `editor` | Open note state and safe saving |
| `editor_search` | In-note search and match positions |
| `markdown` | Source-preserving styling using character offsets |
| `fuzzy`, `heading_search` | Dependency-free fuzzy ranking and current-document heading picker snapshot |
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

`outline::extract` works on an immutable source snapshot and returns headings in source order with zero-based logical rows, nearest shallower parent indices, exclusive section ends, and borrowed raw titles. Focus mode uses the same parser. Extraction does not depend on syntax highlighting or theme settings. The parser handles top-level H1–H6 ATX headings, BOM/CRLF, YAML frontmatter, fenced and indented code, multiline HTML comments and standalone `$$` math blocks. It is a lightweight block scanner, not a full Markdown parser: Setext and nested list/blockquote headings are not supported, and inline HTML/code handling is limited. The outline uses the same source snapshot as editor rendering. `OutlineView` tracks keyboard focus, selection and collapsed branches; a signature of heading rows, levels and titles invalidates stale indices. `app/navigation.rs` coordinates outline keys and cursor jumps without modifying text. `heading_search::HeadingSearch` owns a compact heading snapshot extracted from the unsaved editor buffer when the picker opens; it does not depend on outline visibility or collapse state. `fuzzy::score` ranks exact, prefix, substring and ordered-subsequence matches using Unicode lowercase characters and source-order tie-breaking. The event loop routes picker input before editor/global actions, so its snapshot remains stable. Outline and heading search use the same cursor jump helper.

## Global command palette

`commands::CATALOG` stores typed command identifiers, display titles, shortcut hints and aliases. `App::palette_available` filters this catalog by application context when opening the picker and checks availability again before dispatch. `command_palette::CommandPalette` ranks titles, hints and aliases via `fuzzy::score` and preserves catalog ordering for ties and empty queries. The popup owns keyboard input before global or editor routing; Enter removes it before invoking the existing `execute_command` dispatcher (including the browser quit signal). No simulated keystrokes or duplicated filesystem actions are involved. Existing prompts and dirty-buffer/external-change checks remain authoritative. Forced discard is not cataloged. Canceling leaves the editor cursor, text and prior Outline focus untouched.

## Panel layout

`layout::Panels` holds session visibility preferences and preferred outline width. Its geometry method returns `[Files, Editor/Preview, Outline]`, reserving the active pane and suppressing the outline on small terminals. Browser mode always retains Files; editor mode always retains the editor. Zen temporarily suppresses sidebars without changing preferences. The event loop routes modal keys first, then the in-note search and Ex command bar, then global commands, then focused Outline keys before editor/browser input. `App::route_key` provides a testable single-key path used by the real terminal loop. Global shortcuts clear pending leader and wiki-link prefix state. Preview remains read-only; the Outline does not forward editing motions to edtui. Panel commands flow through the shared command dispatcher and do not mutate the note buffer or configuration file.

## Rendering and theme caches

- `HighlightCache` holds one document's lexical styles and word count. Exact source text, resolved theme, Markdown status and settings form the cache key. Search and focus overlays operate on copies of base styles so cursor movements cannot contaminate the cache. The editor still serializes its buffer once per draw; this is not incremental parsing.
- `ThemeSnapshot` records actual palette file contents and legacy light-mode markers. The live-theme poll skips resolution when the snapshot is unchanged. Reading contents (rather than relying on timestamps) detects symlink replacements. Theme queries still happen before the terminal event loop.
- Existing periodic rendering and vault indexing behavior are unchanged. Benchmark before introducing further optimizations.

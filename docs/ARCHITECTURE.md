# Architecture

`main.rs` declares modules and delegates startup to `app::start`. `cli.rs` resolves startup options before Ratatui initializes the terminal. `app.rs` owns the private application state, picker state, and startup coordination. Its child modules extend `App` with focused implementations; the event loop remains the single owner of application-state mutation.

| Application module | Responsibility |
| --- | --- |
| `app/browser.rs` | Directory listing, selection, navigation and previews |
| `app/appearance.rs` | Theme resolution, live refresh, cycling and pane resizing |
| `app/dialogs.rs` | Picker actions, search, links, citations, snippets and file prompts |
| `app/editing.rs` | Saving, editor commands and in-note search interception |
| `app/events.rs` | Terminal lifecycle, cursor shape and input routing |
| `app/ui.rs` | Main interface and popup rendering |

These modules share `App` privately rather than passing large collections of state between unrelated public interfaces. Methods used by sibling modules have `pub(super)` visibility; implementation helpers remain private.

| Module | Responsibility |
| --- | --- |
| `config` | Deserialize configuration, expand paths and validate options |
| `theme`, `system_theme` | Built-in palettes, native/terminal providers, user overrides |
| `editor` | Open note state and safe saving |
| `editor_search` | In-note search and match positions |
| `markdown` | Source-preserving styling using character offsets |
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

## Rendering and theme caches

- `HighlightCache` holds one document's lexical styles and word count. Exact source text, resolved theme, Markdown status and settings form the cache key. Search and focus overlays operate on copies of base styles so cursor movements cannot contaminate the cache. The editor still serializes its buffer once per draw; this is not incremental parsing.
- `ThemeSnapshot` records actual palette file contents and legacy light-mode markers. The live-theme poll skips resolution when the snapshot is unchanged. Reading contents (rather than relying on timestamps) detects symlink replacements. Theme queries still happen before the terminal event loop.
- Existing periodic rendering and vault indexing behavior are unchanged. Benchmark before introducing further optimizations.

# Architecture

`main.rs` owns application state, event routing, dialogs and rendering. `cli.rs` resolves startup options before Ratatui initializes the terminal. The event loop coordinates the browser, editor and modal pickers; it owns mutation of application state.

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

Future extraction of UI/event modules and caching should be separate changes with regression tests. Do not optimize before measuring representative notes.

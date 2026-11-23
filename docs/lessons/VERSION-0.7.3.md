# v0.7.3 — Resizable panes

## What changed

Previously, the file pane always occupied 35% of the available width. Now `browser_width_percent` in `config.toml` specifies the starting ratio. The editor/preview uses the remaining width. A value outside 20–70 produces a clear configuration error.

```toml
# ~/.config/zettui/config.toml (above [colors] if present)
browser_width_percent = 40
```

The ratio is loaded when Zettui starts. During a session, use:

- `Alt+h` or `Alt+Left` — reduce file-browser width by 5 percentage points
- `Alt+l` or `Alt+Right` — increase file-browser width by 5 percentage points

These keys work in the browser and editor, but not while a prompt or picker is active. The application shows the current requested percentage in the header. Changes from shortcuts are not written to your TOML file.

On narrow terminals, minimum pane widths override the preferred ratio. On extremely narrow screens neither pane can be made fully readable, but the layout never allocates more than the available width.

## Implementation overview

`src/layout.rs` defines the split policy and pure tests. `src/config.rs` validates the configured percentage. `src/main.rs` stores the current ratio in `App`, passes the panel dimensions to Ratatui, and intercepts Alt key combinations before the Vim editor handles them.

## Try it

1. Set `browser_width_percent = 50` in the TOML file. Restart Zettui and verify a roughly 50/50 split.
2. In the browser, press `Alt+h` twice: the requested split becomes 40/60.
3. Open a note. Use `Alt+l`: the requested split becomes 45/55. Confirm normal `h` and `l` still work as Vim motions.
4. Exit and restart: the ratio returns to the configured 50/50.
5. Narrow the terminal. Both panels should remain bounded by the terminal width.

Test with `cargo fmt && cargo check && cargo test` before committing.

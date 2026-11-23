# Lesson 6 — Themes, automatic light/dark mode, and filename search

This is the **consolidated** tutorial for Zettui v0.6.0 and its v0.6.1
appearance correction. It replaces the two separate Lesson 6 guides.

## What was added

Lesson 6 introduced a theme module (`src/theme.rs`), configurable theme
colors (`src/config.rs`), and `/` filename search in the current directory
(`src/main.rs`). The original v0.6.0 design included Gruvbox Dark,
Catppuccin Mocha, Tokyo Night, and Paper Light. These independent palettes
were **superseded in v0.6.1** by three system-aware modes:

| Mode | Actual behavior |
|---|---|
| `auto` (default) | Follow Omarchy's current light/dark appearance while running |
| `dark` | Force a dark Zettui palette |
| `light` | Force a light Zettui palette |

The historical names `gruvbox`, `catppuccin`, and `tokyo-night` are accepted
as aliases for dark; `paper` is accepted as an alias for light. These are no
longer four separately selectable palettes.

## Why the v0.6.1 correction was needed

Foot/Omarchy paints the terminal's underlying background and outer margins.
The original Zettui canvas used a fixed RGB background, producing a pale strip
at its edges when Foot and Zettui used different colors. Zettui now uses
Ratatui `Color::Reset` for its canvas, inheriting the terminal background.
It still styles the status bar, selections, borders and editor cursor.
The TUI cannot paint pixels outside the terminal's character grid.

In `auto` mode Zettui checks the current Omarchy color settings about every
400 ms. Supported locations include:

- `~/.local/state/omarchy/current/theme/colors.toml`
- `~/.config/omarchy/current/theme/colors.toml` (older installations)

The setting `mode = "light"` / `mode = "dark"` is preferred, and the older
`light.mode` marker is supported. On other terminals, Zettui detects the
background at startup using `terminal-colorsaurus`; live switching outside
Omarchy is not currently guaranteed.

## Configuration

Create or edit `~/.config/zettui/config.toml` (or its XDG equivalent):

```toml
notes_dir = "/home/USER/Documents/Notes"
bibliography = "/home/USER/Documents/Zotero/references.json"
theme = "auto"

[colors]
accent = "#B89CF4"
directory = "#8BD5CA"
selected_bg = "#394A76"
selected_fg = "#FFFFFF"
```

The `[colors]` table is optional. Also supported: `background`, `surface`,
`foreground`, `muted`, `border`, `warning`, and `success`. Hex values must be
valid. Overrides apply to *both* appearances, so custom light/dark contrast
may be poor; temporarily remove the overrides while testing Auto mode.

Launch with `zettui /path/to/notes --theme auto`, or override with `--theme dark`
or `--theme light`. `Ctrl+G` cycles Auto → Dark → Light → Auto for the
**current session only**; config is not overwritten.

## Filename finder

In the **browser** (outside the Vim editor), press `/` to find files or
folders in the *current directory only*. Type a case-insensitive part of the
filename, use Up/Down to select, press Enter to highlight the item in the
browser, and Enter again to open it. Backspace erases a character, Ctrl+U
clears the query, and Esc cancels. The finder never alters files or searches
inside their contents.

## Verify the lesson

```bash
cargo fmt --all -- --check
cargo check
cargo test
cargo run -- examples/notes --theme auto
```

Try `Ctrl+G`, change Omarchy between light and dark while Zettui remains open,
and test `/` in the browser. Existing Vim edits, wiki links, and Zotero
citations should continue working.

## How the Rust modules fit together

- `src/theme.rs`: defines `Theme`, `ThemeChoice`, appearance detection,
  overrides, and editor/terminal colors.
- `src/config.rs`: loads the TOML `theme` and `[colors]` settings.
- `src/main.rs`: stores the active theme, redraws it, and implements the
  filename finder and its keyboard input.

*Historical note:* v0.6.0 originally cycled among four independent palettes.
This guide documents their replacement by v0.6.1 rather than recommending the
outdated four-palette commands or Git patch instructions.

# Lesson 6 update — Follow Omarchy light/dark mode (v0.6.1)

This is a **correction to Lesson 6**, not Lesson 7. It retains your Vim editor,
file operations, wiki links, Zotero integration, and filename finder.

## Why the screenshot has pale edges

Ratatui paints terminal character cells, not pixels in Foot's outer margins.
The original Lesson 6 set a hard-coded RGB background, while Foot used the
background configured by Omarchy. Switching Zettui palettes didn't change
Foot's underlying background.

Version 0.6.1 paints the editor/browser canvas with `Color::Reset` — the
terminal default background. Thus Foot's character cells match its margins,
while the app retains styled status bars, selections and dialogs.
**Zettui does not modify Foot settings or your Omarchy theme.**

## Modes

| Setting | Behavior |
|---|---|
| `auto` (default) | Follows active Omarchy light/dark, checking every 400 ms |
| `dark` | Forces a Catppuccin-inspired dark interface |
| `light` | Forces a paper-like light interface |

Omarchy Quattro's current theme is read from
`~/.local/state/omarchy/current/theme/colors.toml`; older installs also
use `~/.config/omarchy/current/theme/colors.toml`.
The `mode = "light"` / `mode = "dark"` key is preferred; the old `light.mode`
marker is supported too. On other terminals, Zettui queries the terminal's
background **at launch** via `terminal-colorsaurus`. Live updates without
restarting are currently **Omarchy-specific**; if probing is unavailable,
Zettui defaults to dark until manually overridden.

Older names (`gruvbox`, `catppuccin`, `tokyo-night`, `paper`) remain accepted
as aliases (`dark` for the first three, `light` for `paper`) so existing
configs still load. Enable Auto explicitly if your config contains an older
name.

## 1. Protect your working files

Keep working on `feature/themes-and-find` (or your existing Lesson 6 branch):

```bash
cd ~/Projects/zettui
git status
```

For an **unmodified Lesson 6** project, apply the patch:

```bash
unzip ~/Downloads/zettui-lesson-6-auto.zip -d /tmp
git apply --check /tmp/zettui-lesson-6-auto.patch
git apply /tmp/zettui-lesson-6-auto.patch
```

If you customized any of the source files, **do not force the patch**.
Compare the corresponding files inside `zettui-lesson-6-auto/` and integrate
your changes or test on a fresh Lesson 6 project.

## 2. Enable automatic appearance

Open `~/.config/zettui/config.toml`. Keep your real notes and bibliography
paths, then change only the theme key:

```toml
notes_dir = "/home/USER/Documents/Notes"
bibliography = "/home/USER/Documents/Zotero/references.json"
theme = "auto"
```

Previously configured `[colors]` overrides are still supported, but fixed
foreground and surface overrides designed for a dark palette may not be
readable in light mode. Temporarily remove `[colors]` if contrast is poor.
In auto mode, the canvas inherits the **terminal** background;
`[colors].background` affects contrast elements, not Foot's outer pixels.

You can also try Auto just for one launch, without modifying config:

```bash
cargo run -- examples/notes --theme auto
```

## 3. Compile and test

```bash
cargo fmt
cargo check
cargo test
cargo run -- examples/notes --theme auto
```

Expected behavior:

1. Header shows `theme: auto (dark)` or `theme: auto (light)`.
2. The editor/preview canvas matches Foot's surrounding background.
3. Switch Omarchy to a light theme. Zettui should adopt the light palette
   automatically while running, after Omarchy completes the switch.
4. Switch to a dark Omarchy theme. Zettui returns to dark.
5. `Ctrl+G` cycles `auto -> dark -> light -> auto`.
6. Confirm `Ctrl+P` citations, `Ctrl+O` links and `/` filename search still work.

Foot's transparency or explicit `pad` colors can affect its outermost pixels;
Zettui cannot paint outside the character grid. If a tiny fringe remains,
inspect `pad`/`alpha` in `foot.ini` instead of adding more Rust colors.

## 4. Commit once checks pass

```bash
git add Cargo.toml Cargo.lock src/main.rs src/theme.rs src/config.rs
git add README.md LESSON-6.md LESSON-6-AUTO.md
git commit -m "Follow Omarchy light/dark theme and terminal background"
```

Then merge your feature branch as described in the original Lesson 6 guide.

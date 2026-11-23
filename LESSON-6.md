# Lesson 6 — Themes and a filename finder (beginner edition)

We are upgrading the **working Lesson 5** project. Your editor, safe file
operations, note links, backlinks, Zotero picker and citation insertion remain.
This lesson adds four selectable palettes, optional custom colors and a
non-destructive filename finder. No new dependencies are required.

## 1. Protect your working app

From your existing project:

```bash
cd ~/Projects/zettui
git status
git switch -c feature/themes-and-find
```

Commit or set aside uncommitted changes before starting. If this branch name
already exists, choose another name. Test on sample notes.

## 2. Extract and copy the update

```bash
unzip ~/Downloads/zettui-lesson-6.zip -d /tmp
```

**Before copying**, compare any files you customized with the updated versions:

```bash
diff -u src/main.rs /tmp/zettui-lesson-6/src/main.rs | less
diff -u src/config.rs /tmp/zettui-lesson-6/src/config.rs | less
```

If your project still matches the previous lessons, apply the changes:

```bash
cp /tmp/zettui-lesson-6/Cargo.toml .
cp /tmp/zettui-lesson-6/src/main.rs src/
cp /tmp/zettui-lesson-6/src/config.rs src/
cp /tmp/zettui-lesson-6/src/theme.rs src/
cp /tmp/zettui-lesson-6/README.md .
cp /tmp/zettui-lesson-6/ROADMAP.md .
cp /tmp/zettui-lesson-6/LESSON-6.md .
```

**Do not overwrite** `src/editor.rs`, `src/file_ops.rs`, `src/links.rs`, or
`src/bibliography.rs`: Lesson 6 leaves those files unchanged. If you
customized the files being copied, integrate the changes rather than
blindly replacing your work.

## 3. Format, compile, and test

```bash
cargo fmt
cargo check
cargo test
```

If you encounter an error, copy the **first** compiler error and the surrounding
lines. Do not proceed to your original notes until these pass. The archive was
checked in the preparation environment, but Cargo compilation could not be run
there.

## 4. Launch the sample vault

```bash
cargo run -- examples/notes --bibliography examples/references.json --theme gruvbox
```

You should see a themed app header, blue-ish directory names, a highlighted
selection, a themed file-preview panel, and a colored status line. Open a note:
its line numbers, cursor and selection now inherit the theme.

## 5. Test the four built-in themes

Press `Ctrl+G` in the **browser** or **editor** to cycle:

1. `gruvbox` — warm dark browns, yellow accent
2. `catppuccin` — cool dark colors with lavender accent
3. `tokyo-night` — deep navy, violet accent
4. `paper` — light background, dark text

When you reach Paper, one more `Ctrl+G` returns to Gruvbox. Switching themes
never writes your notes or `config.toml`. With an editor open, the text,
selections, line numbers and cursor should switch together.

**Tip:** Your terminal emulator must support 24-bit/true-color output for the
intended colors. If your terminal is limited to 256 colors, the results may
look less accurate.

## 6. Set a permanent theme

Open your local config file:

```bash
mkdir -p ~/.config/zettui
nano ~/.config/zettui/config.toml
```

Keep the paths you already set in Lesson 5 and add `theme` to the **top level**:

```toml
notes_dir = "/home/YOUR_USER/Documents/Notes"
bibliography = "/home/YOUR_USER/Documents/Zotero/references.json"
theme = "catppuccin"
```

Now a plain `cargo run` uses Catppuccin on launch. A command-line option
(e.g., `cargo run -- --theme paper`) overrides the theme for that launch.
`Ctrl+G` still cycles during the current session, without changing your config.

## 7. Customize colors

Put custom colors **after** the top-level keys:

```toml
notes_dir = "/home/YOUR_USER/Documents/Notes"
bibliography = "/home/YOUR_USER/Documents/Zotero/references.json"
theme = "tokyo-night"

[colors]
accent = "#B89CF4"
directory = "#8BD5CA"
selected_bg = "#394A76"
selected_fg = "#FFFFFF"
```

Supported optional fields are `background`, `surface`, `foreground`, `muted`,
`border`, `accent`, `directory`, `selected_bg`, `selected_fg`, `warning`, and
`success`. Zettui reports an error for bad hex colors or unknown theme names
instead of quietly ignoring them. Custom colors apply to every built-in theme
you cycle to, until you remove or change the overrides and restart.

## 8. Search filenames in your current directory

1. Return to the file browser (`:q`, save first if needed).
2. Press `/`. This opens a filename-search popup.
3. Type `vhh` or `citation` (or any part of a filename).
4. The list shrinks to matching files and folders, ignoring case.
5. Use **Up/Down** to select. Press `Enter` to highlight that item in the
   browser; press `Enter` again to open it.
6. Press `Esc` to cancel without moving your selection.

`Ctrl+U` clears the search; Backspace removes the last character.
This finder does **not** search the contents of notes or other folders.

## 9. What Rust code did we add?

`src/theme.rs` defines a `Theme` struct. A struct groups colors for each
interface element together. `Theme::named()` chooses a built-in palette;
`with_overrides()` replaces individual colors from `[colors]` in TOML.
`Theme::next()` advances to the next built-in theme.

`src/config.rs` now reads two additional settings: `theme` and `[colors]`.

`src/main.rs` stores the active palette in `App`. The `draw()` function applies
it to the header, browser, editor and dialogs. The editor uses EdTUI's
`EditorTheme` for base text, cursor, line numbers and visual selection.
`FindDialog` searches the current `Vec<Entry>` without changing or reading
file contents.

Try reading one small function at a time:

```rust
pub fn next(self) -> Self {
    let index = BUILTIN_THEMES.iter()
        .position(|name| *name == self.name)
        .unwrap_or(0);
    Self::named(BUILTIN_THEMES[(index + 1) % BUILTIN_THEMES.len()])
        .expect("all built-in theme names are valid")
}
```

`%` is the **remainder operator**. When the next index reaches 4, `4 % 4`
becomes `0`, restarting the cycle with Gruvbox.

## 10. Commit your completed milestone

After tests and the hands-on checks succeed:

```bash
git add Cargo.toml Cargo.lock src/main.rs src/config.rs src/theme.rs LESSON-6.md README.md ROADMAP.md
git commit -m "Add color themes and filename finder"
git switch main
git merge --ff-only feature/themes-and-find
git branch -d feature/themes-and-find
git push origin main
```

If `Cargo.lock` is not yet present, run `cargo check` first, which ordinarily
generates it for a binary crate.

## What remains for Lesson 7?

We'll prepare GitHub Actions to test Linux builds, then distribute a standalone
binary, `.deb`, `.rpm`, and Arch Linux packaging. Real full-vault Markdown
syntax highlighting and link-aware renaming are separate future enhancements;
they are **not** part of this version.

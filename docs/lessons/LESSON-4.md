# Lesson 4 — Wiki links, outgoing links and backlinks

## What you are building

A **read-only link index** that scans Markdown notes on demand and lets you follow links without rewriting your Zettlr files. Your original YAML `id`, `title` and `tags` remain unchanged. Use the included sample notes first.

- `[[VHH]]`: link by note filename stem (without `.md`).
- `[[Antibody/VHH]]`: link by folder path under the notes root.
- `[[202611220003]]`: link by YAML `id` field.
- `[[VHH|display text]]`: alias text after `|` (the target is still `VHH`).
- `[[VHH#Heading]]`: resolve the note (jumping to the heading is not implemented).
- `[Reading](Readings/Paper.md)`: ordinary relative Markdown link.

A note may also be found by its YAML `title`. If multiple notes have the same name, title, or ID, the resolver tries the current directory first; otherwise it shows **ambiguous** instead of picking one at random. Folder-qualified wiki links are resolved relative to the note's folder first and then to the notes root.

## Step 1 — Create a branch

```bash
cd ~/Projects/zettui
git status
git switch -c feature/note-links
```

If there are uncommitted edits, commit or stash them before copying the lesson files.

## Step 2 — Update your existing project

Extract the ZIP you downloaded to `/tmp`:

```bash
unzip ~/Downloads/zettui-lesson-4.zip -d /tmp
cp /tmp/zettui-lesson-4/Cargo.toml .
cp /tmp/zettui-lesson-4/src/main.rs src/
cp /tmp/zettui-lesson-4/src/links.rs src/
cp /tmp/zettui-lesson-4/LESSON-4.md .
```

Keep your current `src/editor.rs` and `src/file_ops.rs`. If you've customized `main.rs`, compare the changes before replacing it. For example, `diff -u src/main.rs /tmp/zettui-lesson-4/src/main.rs`.

For testing, use the **extracted** example notes (they have links and YAML IDs). This avoids overwriting samples or personal notes in your existing repository.

## Step 3 — Build and test

```bash
cargo fmt
cargo check
cargo test
cargo run -- /tmp/zettui-lesson-4/examples/notes
```

In case of errors, copy the *first* compiler error, its filename and line number. Do not make arbitrary changes to your personal notes to fix application errors.

## Step 4 — Follow a wiki link

1. In the browser, highlight `welcome.md` and press `Enter`.
2. Press `Ctrl+O` to open the **Outgoing links** list.
3. Select `Antibody/VHH` using `j` or `k`, then press `Enter`.
4. You are now editing `Antibody/VHH.md` in its own directory.
5. Press `Ctrl+T` to return to `welcome.md`.

A link list is a picker, not an automatic note creator. Missing links have `!` and cannot be opened in this version.

## Step 5 — Find backlinks

1. Navigate to `Antibody/VHH.md` and open it.
2. Press `Ctrl+B` for **Backlinks**.
3. You should see `welcome.md` (two references) and `Readings/Paper.md` (one reference).
4. Select a backlink and press `Enter` to open the referring note.

In the **file browser**, press `o` for outgoing links or `b` for backlinks on the selected note. Press `Esc` to close the picker without opening a link.

| Context | Key | Action |
| --- | --- | --- |
| Browser | `o` | Outgoing links for selected note |
| Browser | `b` | Backlinks to selected note |
| Editor (any mode) | `Ctrl+O` | Outgoing links for open note |
| Editor (any mode) | `Ctrl+B` | Backlinks to open note |
| Editor or browser | `Ctrl+T` | Previous linked note, if available |
| Link picker | `j` / `k` | Select link |
| Link picker | `Enter` | Open resolved target |
| Link picker | `Esc` | Close |

## Step 6 — Verify unsaved-change protection

1. Edit `welcome.md` with `i`, add a line and `Esc`.
2. Press `Ctrl+O`. Zettui should refuse to navigate until the note is saved.
3. Save with `:w`, then press `Ctrl+O`. You may now open another note.
4. Try a nonexistent `[[Missing Note]]` link and confirm that Zettui never creates a file without asking you.

## Architecture: what changed?

`src/links.rs` knows how to **discover notes**, **extract link text**, **resolve a link to a unique local file**, and **find backlinks**. It does not draw the interface, edit a file, or save your notes.

`src/main.rs` adds `LinkDialog` to the app's state, draws the picker, handles its keys, and navigates between open notes. It uses the existing `OpenNote::load` for file loading. It refuses to navigate away from unsaved changes, and `Ctrl+T` provides basic link-navigation history.

### The central data type

```rust
pub struct LinkItem {
    pub label: String,
    pub target: Option<PathBuf>,
}
```

`label` is what you see in the picker. `target` is `Some(path)` when a link points unambiguously to an existing note. It is `None` when a note is missing or ambiguous. The picker never guesses a missing target.

## Important limitations

- The first version only extracts links from `.md`, `.markdown`, and `.txt` files. It can target UTF-8 `.md`, `.txt`, `.json`, `.yaml`, `.yml`, and `.toml` files.
- It understands the listed simple link forms, **not all Markdown syntax**. Complex nested brackets, parentheses in URLs, reference-style links, percent-encoded paths, inline-code links, and complex/multiline YAML are not fully supported.
- `[[Note#Heading]]` opens the note but does not jump to a heading.
- It ignores YAML front matter and fenced code blocks while scanning links.
- Links to external web pages are listed as unresolved; opening web URLs is future work.
- Renaming or moving a note **does not update other links**. Do not batch-rename interconnected notes yet.
- The scan runs when you request links/backlinks, not continuously. Files larger than 256 KB are not parsed for links, and indexing stops at 12,000 eligible text files.
- This initial version does not show a graph, offer completion suggestions while typing `[[`, or create a new note by following a missing link.

## Commit the finished lesson

After `cargo test` passes and you have practiced on the extracted examples:

```bash
git add Cargo.toml Cargo.lock src/main.rs src/links.rs LESSON-4.md
git commit -m "Add wiki-link navigation and backlinks"
git switch main
git merge --ff-only feature/note-links
git branch -d feature/note-links
git push origin main
```

Lesson 5 will add Zotero/Better CSL JSON configuration, reference lookup and citation insertion.

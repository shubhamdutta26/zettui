# Lesson 3 — Create, rename, move and Trash notes (Linux)

This lesson builds on Lesson 2. The same embedded Vim editor is still present.

## Part 1 — Start from a working Git branch

```bash
cd ~/Projects/zettui
git status
git switch -c feature/file-management
```

Commit or stash unrelated changes before replacing files.

## Part 2 — Install the lesson in your existing repository

Extract `zettui-lesson-3.zip` to `/tmp`:

```bash
unzip ~/Downloads/zettui-lesson-3.zip -d /tmp
cp /tmp/zettui-lesson-3/Cargo.toml ./
cp /tmp/zettui-lesson-3/src/main.rs ./src/
cp /tmp/zettui-lesson-3/src/file_ops.rs ./src/
cp /tmp/zettui-lesson-3/LESSON-3.md ./
```

Keep your original `src/editor.rs`, README, personal notes, and any other custom files.

If you customized `src/main.rs` since Lesson 2, review the diff and merge your changes rather than blindly replacing it.

## Part 3 — Build and run

```bash
cargo fmt
cargo check
cargo test
cargo run -- examples/notes
```

The first build needs Internet access to download the new `trash` and `rustix` crates.

## Part 4 — Practice in the file browser

| Key | Behavior |
| --- | --- |
| `n` | Create note; default extension `.md`, or type `.json`, `.yaml`, `.txt`, etc. |
| `N` | Create a directory |
| `r` | Rename selected file or directory |
| `m` | Move selection to an existing folder, specified relative to the **notes root** |
| `d` | Ask for confirmation, then send the selection to the Linux Trash |
| `R` | Refresh the browser |
| `Enter` / `l` | Open a note or enter a directory |
| `h` | Go to parent folder |
| `j` / `k` | Move browser selection |
| `q` | Quit (from browser only) |

For prompts: Enter confirms, Esc cancels, Backspace erases a character, and Ctrl+U clears the entire input (useful for rename). `d` needs an extra `y` confirmation.

### Exercise A: create a folder and note

1. Press `N`, type `Experiments`, Enter.
2. Enter the new folder, press `n`, type `CD4-SEC`, Enter.
3. Zettui creates `CD4-SEC.md`.
4. Open the note, press `i` to enter Insert mode, add a few lines, Esc, `:wq` + Enter.

New notes start EMPTY; no metadata template is inserted yet. Existing Zettlr YAML stays as it was. We will build configurable templates and metadata support in a later lesson.

### Exercise B: rename and move

1. Select `CD4-SEC.md`, press `r`, replace the name with `CD4-SEC-2026.md`, Enter.
2. Press `m`, type `.` and Enter. A single dot means the root notes folder.
3. Return to the root using `h`, then confirm the note is visible there.
4. Create `Readings` in the root if it is not present. Select your note and press `m` -> `Readings` -> Enter.
5. Open `Readings` and verify the note still has its contents.

Move destinations must ALREADY EXIST. A folder path such as `Antibody/Humanization` is relative to the root you passed at app startup. Absolute paths and `..` are rejected.

### Exercise C: collision safety

1. Create notes `one.md` and `two.md` in the same directory.
2. Select `one.md`, press `r`, try to rename it `two.md`.
3. The operation must fail. Neither file should be replaced.
4. Press Esc to close the prompt.

This uses Linux `renameat2(RENAME_NOREPLACE)` for no-clobber behavior. On a filesystem that does not support it, rename/move fail safely instead of falling back to an overwriting rename.

### Exercise D: send to Trash

1. Select a disposable test file.
2. Press `d` and then `n`: file should remain.
3. Press `d` and then `y`: file should disappear from Zettui.
4. Find it in your desktop Trash (or the file manager's Trash folder) and restore it if you wish.

If the operating system Trash is unavailable or reports an error, Zettui will NOT permanently delete the file as a fallback.

## Part 5 — Key Rust ideas

- `mod file_ops;` includes the new Rust module.
- `enum PromptKind` lists the type of operation the prompt will execute.
- `Option<Prompt>` holds either an active prompt (`Some`) or no prompt (`None`).
- `Result<PathBuf, io::Error>` is the outcome of a filesystem action: success with a path or an error.
- `OpenOptions::create_new(true)` guarantees creating a note does not replace an existing path.
- `renameat_with(..., RenameFlags::NOREPLACE)` requests an atomic no-replace rename from Linux.
- `trash::delete(source)` moves an item to system Trash, rather than permanently deleting it.
- Unit tests in `src/file_ops.rs` test collisions, directory escape attempts, and basic operations.

## Part 6 — Commit

```bash
cargo fmt && cargo check && cargo test
git diff --stat
git add Cargo.toml Cargo.lock src/main.rs src/file_ops.rs LESSON-3.md
git commit -m "Add safe note and folder management"
git switch main
git merge --ff-only feature/file-management
git branch -d feature/file-management
git push origin main
```

If the project has not been pushed to a remote yet, configure your GitHub origin before the final push.

## Current limits

- Rename/move do NOT update Markdown links or wiki-links pointing to the old file; we add link indexing and optional link updates in Lesson 4.
- File creation does not yet generate Zettelkasten IDs, YAML front matter, or tags.
- Move paths are inside the selected notes root; moving across roots/filesystems is not supported.
- Files are hidden from the browser if they are symlinks; this is intentional for this lesson.
- Filesystem actions should be tested on sample/copy directories before your actual Zettlr archive.
- There is no automatic undo for a rename or move yet (they can be manually reversed).

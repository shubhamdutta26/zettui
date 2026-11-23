# Lesson 2 — Guided changes from your Lesson 1 repository

## Step A — Protect Lesson 1

In your existing `zettui` repository, run:

```bash
git status
git switch -c feature/vim-editor
```

Do this before copying files. The new branch keeps the first working version intact.

## Step B — Copy the updated files

Copy from this lesson archive into your existing project, replacing:

- `Cargo.toml`
- `src/main.rs`
- `README.md`

And adding:

- `src/editor.rs`
- `LESSON-2.md` (this document)

Do not overwrite your private notes or any custom files you've added since Lesson 1.

## Step C — Read the code in three small pieces

1. `OpenNote::load()` in `src/editor.rs` reads a file; `EditorState::new(Lines::from(...))` prepares the Vim editor.
2. `handle_editor_key()` in `src/main.rs` routes keystrokes either to Zettui's command bar or to EdTUI's editing engine.
3. `OpenNote::save()` in `src/editor.rs` checks whether the file changed externally and writes safely.

## Step D — Build and test

```bash
cargo fmt
cargo check
cargo test
cargo run -- examples/notes
```

Open `welcome.md`, press `i` to type, `Esc`, then `:wq` and Enter. Reopen the file to see your changes.

## Step E — Verify the unsaved-changes warning

Open a sample note, edit it, and run `:q` (no exclamation mark). You should stay in the editor and see an unsaved warning. Use `:q!` to discard, or `:wq` to save.

## Step F — Commit your work

```bash
git add Cargo.toml Cargo.lock src README.md LESSON-2.md
git commit -m "Add Vim-style note editing and safe saves"
git switch main
git merge --ff-only feature/vim-editor
git branch -d feature/vim-editor
```

Commit the `Cargo.lock` file if Cargo created one. Never merge if `cargo check` or `cargo test` fail.

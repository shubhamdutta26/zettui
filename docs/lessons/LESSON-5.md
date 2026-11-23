# Lesson 5 — Zotero citations in Zettui

This lesson adds a searchable bibliography to the Lesson 4 note editor. The Zotero
export is **read-only**. Zettui inserts citekeys into the currently open note,
and only saves notes when you request `:w` or `Ctrl+S`.

## What you should have working before you begin

Lessons 1–4: file browser, EdTUI Vim editor, file/folder operations, and wiki links.
A current Rust toolchain, Git, and Cargo. Make a separate sample copy of notes
before testing new code.

## Step 1 — Create a feature branch

```bash
git status
git switch -c feature/zotero-citations
```

If you already have uncommitted work, commit or stash it before replacing files.

## Step 2 — Install the lesson files

Assuming you extracted the ZIP to `/tmp/zettui-lesson-5`, run these commands
from your existing `zettui/` repository:

```bash
cp /tmp/zettui-lesson-5/Cargo.toml .
cp /tmp/zettui-lesson-5/src/main.rs src/
cp /tmp/zettui-lesson-5/src/config.rs src/
cp /tmp/zettui-lesson-5/src/bibliography.rs src/
cp /tmp/zettui-lesson-5/LESSON-5.md .
```

The existing `src/editor.rs`, `src/file_ops.rs`, and `src/links.rs` are retained.
If you've customized `main.rs`, review the diff first.

## Step 3 — Build and test

```bash
cargo fmt
cargo check
cargo test
```

The archive was packaged without a Rust compiler in the authoring environment.
Please rely on your local `cargo check` and `cargo test` to verify compilation.
If a build error occurs, keep the exact output so we can address it before merging.

## Step 4 — Try the sample bibliography

The ZIP contains `examples/references.json` (Better CSL JSON) and
`examples/betterbibtex-debug-example.json` (debug format). Run directly from
the extracted folder so you don't overwrite your own examples:

```bash
cargo run -- /tmp/zettui-lesson-5/examples/notes \
  --bibliography /tmp/zettui-lesson-5/examples/references.json
```

In the left browser pane, open `Readings/Citation-Exercise.md`. Move to a blank
spot in the note (e.g. `G` in Normal mode goes to the last line), then:

1. Press `Ctrl+P` in Normal or Insert mode.
2. Type `Jain` or `antibody` (search is case-insensitive).
3. Use **Up/Down arrow keys** to choose a result. `j` and `k` are typed as
   search text in this modal.
4. Press `Enter`; `[@Jain2017]` is inserted at the editor cursor.
5. Save using `:w` or `Ctrl+S`.
6. Reopen the note and check that your citation is still present and YAML
   metadata is unchanged.

To test the older BetterBibTeX JSON `items` structure:

```bash
cargo run -- /tmp/zettui-lesson-5/examples/notes \
  --bibliography /tmp/zettui-lesson-5/examples/betterbibtex-debug-example.json
```

Search `Example`, and check that the inserted key is `[@DemoDebug2026]`.

## Step 5 — Configure your real bibliography

### Zotero export

In Zotero, right-click *My Library* (or a collection) and choose **Export**.
Choose **Better CSL JSON** from the Better BibTeX add-on, then enable
**Keep updated** (if available), and save it to a directory of your choice.

If you already have a working Better CSL JSON export for Zettlr, you can point
Zettui to that same file without creating a duplicate.

Do not commit a personal bibliography to your public Git repository: it may
contain private titles, notes, local paths, or metadata.

### Config file

```bash
mkdir -p ~/.config/zettui
nano ~/.config/zettui/config.toml
```

Example (replace `YOUR_USER` and the rest of the path):

```toml
notes_dir = "/home/YOUR_USER/Documents/Notes"
bibliography = "/home/YOUR_USER/Documents/Zotero/references.json"
```

Then `cargo run` defaults to the configured notes directory and bibliography.
You can still override the notes directory on the command line:

```bash
cargo run -- /path/to/notes
```

And you can override the bibliography for a single launch:

```bash
cargo run -- /path/to/notes --bibliography /path/to/other-refs.json
```

The config location follows `$XDG_CONFIG_HOME/zettui/config.toml` if that
environment variable is set, otherwise `~/.config/zettui/config.toml`.
Paths inside config may use `~/`, absolute paths, or be relative to the config
file. Command-line relative paths are resolved against your shell's current
directory.

## What the Rust code does

- `config.rs` loads TOML settings using `serde` and `toml`. You can think of
  `Config` as a data structure describing your preferences.
- `bibliography.rs` parses JSON using `serde_json`, converts items to a smaller
  `Reference` struct, rejects duplicate/unsafe keys and searches the results.
- `main.rs` handles `Ctrl+P`, opens a modal search list, and inserts the chosen
  citation through EdTUI's existing key-event interface.

Read `bibliography.rs` first. Find the `Reference` struct and its `matches()`
method. Then look at `Bibliography::parse()` and `Bibliography::search()`. In
`main.rs`, study `open_citation_picker()` and `insert_citation()`.

### Two intentional limitations

This milestone only inserts citekeys; it does not generate a formatted
bibliography, download PDFs, or contact Zotero's online API. Use Pandoc,
Quarto, or Zettlr's citation processor for rendered references.

The picker refreshes the JSON each time it opens; it does not watch the file
continuously or modify Zotero. If Zotero hasn't finished an automatic export,
close the picker and press `Ctrl+P` again.

## Step 6 — Commit after tests pass

```bash
git add Cargo.toml Cargo.lock src/main.rs src/config.rs src/bibliography.rs LESSON-5.md
git commit -m "Add Zotero bibliography search and citation insertion"
git switch main
git merge --ff-only feature/zotero-citations
git branch -d feature/zotero-citations
git push origin main
```

For beginners: `git status` before every branch switch and merge is a good
habit. Never include your private Zotero export in this commit.

## Troubleshooting

- **Picker says no bibliography configured:** check your TOML location and
  spelling: `bibliography = "/absolute/path/file.json"`.
- **Could not load bibliography:** confirm the path exists, the file is valid
  UTF-8 JSON, and Zotero has completed the export.
- **No citation keys:** export as *Better CSL JSON*. Standard Zotero JSON item
  exports may have a different structure and no Better BibTeX citekeys.
- **Blank search results:** press `Ctrl+U` in the picker to clear your query.
- **Citation inserted in the wrong position:** close the picker, move the Vim
  cursor where you want the citation, and open the picker again.
- **YAML metadata unexpectedly modified:** test with sample notes and ensure
  the cursor was outside the YAML header before inserting.

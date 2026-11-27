# Research Toolkit (planned v8.0.0)

## Phase I — Zotero citation search

Configure `workspace.bibliography` with a Better CSL JSON or Better BibTeX JSON export, or pass `--bibliography FILE`.
Open a note and press `Ctrl+P`. The bibliography is read again on opening the picker.

- Search by citation key, title, author surname, year, journal or DOI. Every query token must match a field; exact and contiguous matches rank ahead of fuzzy subsequences.
- Use Up/Down to navigate, `Ctrl+T` to mark/unmark, and Enter to insert the marked references in marking order. With no marks, Enter inserts the highlighted reference. Marks persist when changing the search query.
- Multiple references are inserted as `[@Alpha; @Beta]`; a single reference uses `[@Alpha]`.
- `Ctrl+R` refreshes the open picker after the Zotero export changes. Valid refreshes retain existing marked keys and report removed keys. Invalid refreshes retain the last loaded bibliography. The query is preserved.
- Escape cancels without changing the note. Save with `:w`.

For automatic updates on the Zotero side, configure Better BibTeX's automatic export (Keep updated). Zettui reads the exported file, not Zotero's database. No background file watcher is introduced in this phase.

## Manual checks

1. Open the picker and search for a known reference by title, DOI and abbreviated title.
2. Mark two references across different queries, insert, and verify the citation syntax and undo behavior.
3. Reopen and mark a reference. Modify the bibliography export and refresh. Confirm the query and valid marks survive.
4. Refresh with temporarily malformed JSON; verify the earlier results remain and an error is shown.
5. Cancel a picker with marked references; confirm the note remains unchanged.

## Phase II — Citation inspection

- `Ctrl+V` in the citation picker previews the highlighted reference without changing marks, query, or editor contents. Escape returns to the picker. Space remains a searchable character.
- In Normal mode, put the cursor on a citation key in `[@Key]` or `[@A; @B]` and run `:cite-preview` (also available from F8). `:cite` opens the insertion picker.
- The preview shows full author names, title, year, journal, DOI and the abstract when present in the JSON export. Scroll with Up/Down, j/k or PageUp/PageDown.
- Left/Right (h/l) selects among PDF attachments; `o` opens the selected file via `xdg-open`; `r` retries live lookup. Exported paths (including a custom CSL `file` field with plain paths) are available even when Zotero is closed. Relative paths resolve against the bibliography directory. For live lookup, run Zotero with Better BibTeX and install `curl`; the preview queries `item.attachments` at `127.0.0.1:23119` on a worker thread, with a five-second timeout. It uses the personal Zotero library by default; group-specific lookup is not implemented. Only local PDF paths are accepted, and duplicate paths are merged. If Zotero is unavailable, the existing bibliography and exported paths still work. A path reported by Zotero may not yet be downloaded: open/download it in Zotero and press `r`. The app reports launch errors, not whether the external viewer displayed the file.
- `:cite-check` (or F8 → Check unresolved citations) scans the current live note, including unsaved edits, against a freshly loaded bibliography. It shows each missing occurrence with its line number; Enter jumps to that occurrence, Escape closes without moving the cursor.
- The checker recognizes single-line bracketed citation groups beginning with `@` or `-@`, including semicolon-separated keys and locators. It skips common fenced/inline code, YAML frontmatter, comments and links. It is not a complete Pandoc parser: bare narrative citations, multiline groups and complex Markdown constructs are outside its scope.

### Phase II manual checks

1. Preview a reference with an abstract and another without; scroll and resize the terminal.
2. Mark a reference, preview another, return with Esc and confirm marks and search remain unchanged.
3. With Zotero running and a Better CSL JSON export without paths, preview an item with two downloaded PDFs. Wait for the count, switch with h/l, and open both with o. Close Zotero, reopen the preview, and confirm the error and offline citation functionality. Reopen Zotero and press r. Also test exported paths, missing files, and items without PDFs. Check that the terminal remains responsive.
4. Create `[@Known; @Missing]` in an unsaved note; check that only the missing key is listed and Enter jumps to it. Try a citation in a code fence and confirm it is ignored.
5. Preview a known key under the cursor; confirm Esc leaves the note unchanged. Test an unresolved key and a malformed bibliography.

## Phase III — Metadata Editor

Open a Markdown note in Normal mode, including Zen mode. `:metadata`, `Space m`,
`F11` and `:metadata-edit` open the same centered editor. The command palette
also provides **Open Metadata Editor**. It uses an independent EdTUI buffer
with Vim motions, Insert/Visual modes, undo/redo and the configured YAML
syntax highlighting. It preserves comments, order, indentation and custom
fields. A note without frontmatter opens a minimal editable YAML block.

- `i`, `a`, `o`, Vim motions and Esc work inside the popup.
- `:w` applies the draft to the active note and stays open; `:wq` applies and
  closes. Neither saves to disk. Use `:w` in the main editor afterwards.
- `:q` closes only if there are no unapplied edits; `:q!` discards unapplied
  changes. F11 closes a clean popup in Normal mode.
- In Normal mode, Ctrl+Left/Right changes width and Ctrl+Up/Down changes
  height. The popup automatically fits the terminal when resized.

Only the leading YAML block is replaced. The note body and existing final
newline are preserved. The editor checks the opening and closing delimiters
and rejects extra lines outside the block; it does not parse or validate YAML
semantics. Unclosed frontmatter must be repaired in the main editor first.
An attempted application that cannot reproduce the expected text leaves the
original note unchanged. The apply operation uses EdTUI edits; depending on
which edit path is needed, undo in the note may take multiple steps.

### Phase III manual checks

1. Edit a title, tags, nested YAML, and comments in the popup; check that the
   note stays unchanged until `:w` or `:wq`. Save separately in the main editor.
2. Test `:q` protection, `:q!` discard, independent undo/redo, repeated apply,
   a new note without metadata, and malformed delimiters.
3. Test the popup in Zen mode, with Outline visible, and while resizing the
   terminal. Verify F11, the command palette, and `Space m`.
4. Confirm bibliography search, citation preview, attachment opening and
   normal note editing still work after closing the popup.

## Phase IV — Document export

Open a Markdown note and press `Space x` in Normal mode or enter `:export`. The F8 command palette also exports a selected note or directory from the browser. PDF is the default; DOCX, HTML, EPUB and LaTeX are supported. See [Document export](DOCUMENT-EXPORT.md) for setup, citations, image resolution, diagnostics and limitations.

# Writing analytics (v0.7.14 development)

## Phase 1: statistics engine

The standalone `zettui::statistics` library module provides local, installation-wide
storage independent of notes, buffers, sessions, and workspaces. Phase 2 connects it to the terminal application; a launch without
writing still does not create statistics.

- State: `$XDG_STATE_HOME/zettui/statistics.json` or
  `~/.local/state/zettui/statistics.json` when no absolute XDG state path is set.
- Schema version: `1`. Each installation generates a random 128-bit identifier
  on initialization, and keeps its own cumulative counters by local calendar day.
- Four unsigned counters: words added, words deleted, characters added, characters
  deleted. Deletions never subtract from the addition counters. A zero delta does
  not create a day. Phase 2 defines edit counting below.
- `Store::load` initializes missing state; `Store::record` adds a delta and persists
  it under an advisory lock. `Statistics::daily_totals` sums devices by date.
- A separate `statistics.json.lock` file serializes read-modify-write operations
  between cooperating local Zettui processes. Do not delete it while in use.
- Writes use a synced temporary file and atomic replacement in the same directory.
  Invalid data, unsupported versions, oversized files and counter overflow are
  errors; existing data is not silently reset. This protects against interrupted
  writes, but does not promise persistence of the last rename across power loss.
- No document paths, note text, hostnames or hardware IDs are stored.

Illustrative state (not written until the engine is called):

```json
{
  "version": 1,
  "local_device_id": "0123456789abcdef0123456789abcdef",
  "devices": {
    "0123456789abcdef0123456789abcdef": {
      "2026-11-26": {
        "words_added": 150,
        "words_deleted": 10,
        "characters_added": 900,
        "characters_deleted": 60
      }
    }
  }
}
```

Future export/import will preserve device identities and merge cumulative counters
by source and date rather than adding the same snapshot twice. Do not use a shared
or copied live statistics file as the state of multiple active installations:
that would reuse one device identity. The separate exchange format and legacy
Zettlr source handling are deferred to later phases.

## Phase 1 verification

```sh
cargo test --locked --lib statistics::tests
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

The tests exercise state creation, stable device identity, daily accumulation,
reopening, multiple-device totals, zero activity, invalid-state protection,
overflow protection, concurrent updates, and XDG paths.

## Phase 2: editor activity and automatic persistence

The application enables a tracker after startup session restoration. It compares
active buffer text before and after routed keyboard events, only when the same
note remains active. Opening, switching, restoring, searching, saving and moving
around notes do not create activity. Direct external changes do not count.

Counting rules (also apply to undo/redo and editor paste commands):

- Words: positive and negative changes in the whitespace-delimited token count,
  matching the existing editor status bar. Adding letters to a word does not
  create another word; splitting or joining a word changes the count. A same-word-
  count replacement records character changes but zero word changes.
- Characters: Unicode scalar values, including spaces and line breaks; common
  prefix/suffix are excluded to estimate the changed span. Disjoint changes in
  one editor action may count the intervening unchanged characters as part of
  the replacement. This is not a full diff or a grapheme-cluster count.
- Undo/redo and user-accepted insertions (citations, wiki links, snippets and
  paste through the existing editor key handling) are visible edits, not special
  cases. Undoing a deletion or redoing an insertion can add to gross additions.
  Separate terminal bracketed-paste events are not newly supported in this phase.
- All editable text counts, including Markdown syntax and YAML. Activity measures
  work performed in the editor, not newly authored prose or saved note length.
  Discarding an unsaved note does not erase recorded writing activity.

Deltas are assigned to the local date when the edit occurs and accumulated in
memory. The event loop attempts to persist a batch every two seconds (also when
idle); on graceful exit it flushes again, even if the terminal loop returns an
I/O error. Each batch commits all dates under one lock, preventing partial batch
replays. A successful save clears pending counters. On failure, the footer shows
`Stats unsaved`, and the tracker retries without discarding pending activity.
Note saving and editing are unaffected by ordinary statistics file errors. A
persistent save failure is reported on exit. A crash, forced termination, or
exit while persistence is failing may lose the pending in-memory activity.

State remains installation-wide and is not created until the first tracked edit
is flushed. No note text or filename is persisted. The analytics dashboard and
writing-goal settings are deferred to later phases. No new dependencies or
schema/version changes are required.

### Phase 2 verification

```sh
cargo test --locked --lib
cargo test --locked statistics_track_edits_not_navigation_search_or_save
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Manual check: launch with an isolated state directory, create an empty note,
enter Insert mode and type `hello world`, then Backspace once and press Esc.
Wait for at least two seconds. Inspect the state file: words added = 2,
characters added = 11, characters deleted = 1, words deleted = 0. Saving,
switching buffers, searching and quitting should not change those counts.
Reopening Zettui and adding further text should accumulate activity rather than
reset the device or the previous counters. Use an isolated test directory to
avoid contaminating actual writing statistics.

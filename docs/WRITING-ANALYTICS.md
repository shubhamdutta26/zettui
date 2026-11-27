# Writing analytics (introduced in v0.7.14)

## Quick reference

- Open the dashboard: `F10`, `:analytics`, `:stats` or the command palette.
- Cycle themes inside any dashboard view: `Ctrl+G` (session only).
- Exchange snapshots: `zettui --stats-export FILE` and `zettui --stats-import FILE`.
- Migrate Zettlr: `zettui --stats-import-zettlr "$HOME/.config/Zettlr/stats.json" desktop`.
- Inspect the local identity: `zettui --stats-device-id`.

Keep independent live state on each installation. Physical multi-computer
exchange remains untested by the maintainer. The implementation history and
validation procedures below are organized by writing-analytics development phase,
not by the later codebase-maintenance phases.

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

Export/import preserves device identities and merges cumulative counters
by source and date rather than adding the same snapshot twice. Do not use a shared
or copied live statistics file as the state of multiple active installations:
that would reuse one device identity. See Phase 5 below. Legacy Zettlr history uses separate, named sources; see Phase 6 below.

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
writing-goal settings are described below. No new dependencies or
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

## Phase 3: analytics and writing goals

`zettui::analytics` computes inclusive date-range summaries, calendar-day
averages, writing-day counts and current/longest streaks. `Tracker::report`
combines all saved devices with the local pending edits without forcing a save.
Reading a missing state file returns an empty history and does not initialize
an installation. Invalid state and aggregation overflow are reported as errors.

Reports include today, Monday-start week to date, month to date, year to date
and all time (first recorded date through today, or today for empty history).
Future dates are excluded. Averages use all elapsed calendar days, including
zero-activity days. A writing day contains a positive word or character addition;
a deletion-only day does not extend a streak. A current streak can end yesterday
if today has no additions yet. Longest streaks are historical through today.

Optional goals are loaded at startup from the normal configuration:

```toml
[writing_goals]
daily_words = 1000
weekly_words = 5000
```

Both default to zero (disabled independently). Targets are nonnegative integers.
Goals measure gross word additions, including accepted insertions and undo/redo
as described in Phase 2; deleted words do not reduce progress. The report includes
actual words, target, remaining words, reached status and a capped percentage
(floored at whole percent). Over-target writing remains visible in actual totals.
Goals are configuration, not statistics data, and do not modify past records.
The interactive display is described in Phase 4.

### Phase 3 verification

```sh
cargo test --locked --lib analytics::tests
cargo test --locked --lib writing_activity::tests
cargo test --locked writing_goals_default_to_disabled_and_accept_independent_targets
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

## Phase 4: interactive dashboard and calendar

Open the full-screen, read-only dashboard with `F10`, the `F8` command palette
(search for writing analytics), or `:analytics` / `:stats` from a note. Closing
returns to the current buffer or browser without saving/discarding edits.

- Overview: daily/weekly goal bars, current and longest streaks, period totals,
  calendar-day averages and a last-seven-days bar chart. Scroll with `j`/`k`
  or PageUp/PageDown.
- Month: Monday-start date grid, with daily word-addition intensity and selected
  date details (all four counters). Navigate by day (`h`/`l` or Left/Right),
  week (`j`/`k` or Up/Down), month (`[`/`]` or PageUp/PageDown).
- Year: Monday-start contribution heatmap, month labels and selected-day details.
  Same day/week navigation; `[`/`]` or PageUp/PageDown move by year. The heatmap
  shows a window around the selected week when the terminal is narrow.
- `Tab` / Shift+Tab cycle views, `1`–`3` select a view, `t` / Home returns to
  today, `r` refreshes, `Esc` / `q` / `F10` closes. Future dates cannot be selected.

Fixed intensity bands: 0, 1–249, 250–499, 500–999 and 1000+ words added.
The full calendars use theme-derived shading; the compact fallback uses
`·`, `░`, `▒`, `▓`, `█` respectively. Character-only and deletion-only activity
use `+` and `-` markers (unless the selection/today marker takes precedence).
Future dates are not counted. See the selected-day panel for exact counters. Short terminals show selected-date details instead of
clipping the calendar grid; terminals smaller than 30x8 request resizing.

The dashboard reads one consistent snapshot (persisted devices plus pending local
edits) when opened, when refreshed with `r`, and at a local date change. It does
not force a write or read the state file on every redraw. External writes require
refresh or reopening. Corrupt or unreadable history is shown as an error, not
zero totals. Pending persistence warnings remain visible. Goal values are still
configured in `config.toml`; no interactive goal editor is added in this phase.

### Phase 4 verification

```sh
cargo test --locked analytics_dashboard::tests
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Check the dashboard from the browser and an unsaved note, switch views and dates,
resize the terminal, refresh and close. Check goal bars with configured goals and
verify that dashboard navigation does not change the note or writing counters.

### Phase 4A: overview visual refinement

The Overview now uses bordered metric cards, full-width themed goal bars,
vertical seven-day activity bars, lifetime statistics and detailed period
summaries. Wide terminals arrange goals and the chart side by side; narrow
terminals stack the panels. A scrollbar and `j`/`k` or PageUp/PageDown expose
all content without clipping the header or footer. Terminals 30–31 columns
wide retain the compact original overview. Month and Year are unchanged.
No counters, goal calculations, storage or shortcuts change in this phase.

### Phase 4B: monthly calendar redesign

The Month tab now shows a Monday-start grid of bordered day cells, using only
as many weeks as the selected month requires. Past and current days show word
additions; activity colors are derived from the theme with contrast-adjusted
text. The selected day has a double border, today is underlined, and future
days display a dash (not a zero). Character-only and deletion-only activity
remain marked `0+` and `0-`. Full selected-day counters and an activity legend
appear below the grid. Counts may be abbreviated in small cells, but the
selected-day counters are exact in the bordered layout.

At 58 or more terminal columns, with enough rows for the month, the calendar
adapts its cell height to the viewport. Smaller terminals use the original
compact calendar or selected-day details if necessary. Existing navigation,
statistics storage, goal calculations, Overview and Year behavior are unchanged.

```sh
cargo test --locked analytics_dashboard::month_ui::tests
```

### Phase 4C: yearly heatmap redesign

The Year tab uses a bordered contribution graph with separate filled day tiles,
Monday-first rows, aligned month labels and a legend shared in meaning and color
with the Month tab. Fixed word-count bands allow comparison across years.
Character-only and deletion-only tiles have `+` and `-` markers. The selected
cell shows `*` without changing its activity color; today shows `o` when not
selected. Future and out-of-year dates remain blank. All four counters appear
in the selected-date panel below the graph.

Wide terminals use symmetric half-cell edges around centered ASCII markers;
smaller terminals use single-character tiles or a week window that follows
the selected date.
Very short terminals show selected-date details instead of clipping the grid.
Existing day/week/year shortcuts, the Overview, and the Month grid are unchanged.

```sh
cargo test --locked analytics_dashboard::year_ui::tests
```

### Phase 4D: final visual polish

The Overview uses wider, dynamically spaced activity columns and a horizontal
lifetime summary when space permits, with stacked fallbacks for narrow terminals.
Goal labels show fractional progress (or `<0.1%`), and the bar uses partial
terminal-cell blocks for small nonzero values. Percentages remain capped at 100%
and never round up to completion. Panel borders use a slightly stronger
theme-derived contrast. Singular day labels and compact footer text are corrected.
The Month fill stays inside its borders, and the Year markers retain their
symmetric half-cell rendering. No storage, tracking, navigation or goal semantics
change.

Run `cargo test --locked analytics_dashboard` and check the three views in both
light and dark themes, at wide and narrow terminal sizes.

### v0.8.1: theme consistency

Month and Year activity bands and their legends use the active theme accent.
The compact calendar uses the same accent for positive activity; deletion-only
activity retains the warning color. Fixed activity thresholds, markers, contrast
adjustment and all counters are unchanged. Press `Ctrl+G` in any dashboard view
to cycle themes without leaving the dashboard or resetting the selected date or
view. The theme change is session-only, just as in the editor.

## Phase 5: multi-computer statistics exchange

**Validation status (v0.7.15):** the maintainer has not yet tested this workflow
on multiple physical computers. Automated tests simulate independent installations
using isolated directories on one host. This is not evidence of real-world
interoperability; keep original backups and use the physical-device checklist
in [release validation](RELEASE-CHECKLIST-0.7.14.md).

Transfer complete version-1 JSON snapshots with standalone terminal commands.
They use the same installation-wide XDG state as the editor and do not load
notes, configuration, or the TUI. They work even if the editor's configuration
is invalid. During development, prefix each flag with `cargo run --locked --`
instead of `zettui`.

1. Close Zettui on computer A to flush pending writing activity. Export to a
   new file: `zettui --stats-export ~/Downloads/stats-A.json`.
2. Transfer the exported JSON to computer B (USB, file sharing or a cloud
   folder). Import with `zettui --stats-import ~/Downloads/stats-A.json`.
3. Close Zettui on B, export its combined snapshot to a new filename such as
   `stats-B.json`, transfer it to A and import there. Both now know both histories.
   Repeat with a new export after subsequent writing sessions.
4. Open the dashboard (`F10`) to see combined totals. If a dashboard was already
   open during an import, press `r` to refresh it.

`zettui --stats-device-id` prints (or initializes) the persistent local identity.
Each installation must have its own ID. A new installation can import an old
backup without taking its old identity. Never copy a live `statistics.json`
into the state directories of multiple actively writing installations, or point
multiple computers at a shared live state directory. Divergent counters for a
shared ID cannot generally be distinguished from old snapshots; the importer
rejects detectable non-monotonic conflicts, but cannot detect all cloned IDs.
If two computers have already shared an ID, stop and keep both original files
before attempting recovery; automated conflict repair is not supported.

For each device and date, a newer snapshot supersedes an older one only when
all four cumulative counters are greater than or equal to the earlier values.
Distinct devices are summed for analytics. An identical or older snapshot
is a no-op; importing a third computer's merged snapshot also does not count
previously known history twice. A mixture of increases and decreases for the
same device/day is rejected. The local identity is always retained. Dates
remain the local dates originally recorded by each device; no timezone
conversion is applied.

Imports validate the entire input and resulting daily totals before committing
under the existing local advisory lock. Invalid JSON, unsupported schemas,
invalid dates or identifiers, overflows and detected conflicts leave existing
state unchanged. The source file is never modified. Exports include all known
devices, use an atomic no-clobber write, and require an existing destination
directory. An export or explicit device-ID query initializes missing local state.
Both files use the existing 32 MiB limit. No note contents or paths are included;
the files do contain writing dates, counts and pseudonymous device IDs.

This is manual snapshot exchange, not continuous sync. A CLI export includes
only persisted activity: edits still buffered in another process may be missing.
Always exit the editor cleanly before exporting. Importing while an editor is
open is safe for persisted and subsequent local edits (the same lock is used),
but refresh the dashboard afterward. No changes to the UI or goal settings.
Zettlr JSON has a different schema; use the dedicated migration command below.

### Phase 5 verification

```sh
cargo test --locked --lib statistics::transfer::tests
cargo test --locked --test statistics_transfer
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

The tests exercise same-day aggregation, idempotency, old and new snapshots,
three-device round trips, conflict/overflow/corrupt input protection, source
preservation, no-clobber exports, concurrent local writes, and the standalone
CLI using isolated state directories.

## Phase 6: import historical Zettlr statistics

Import the `stats.json` file from each independent Zettlr installation using
an explicit, stable source label:

```sh
zettui --stats-import-zettlr ~/Downloads/stats-desktop.json desktop
zettui --stats-import-zettlr ~/Downloads/stats-laptop.json laptop
zettui --stats-import-zettlr ~/Downloads/stats-macbook.json macbook
```

During development use `cargo run --locked --` before the flags instead of
`zettui`. The label is required, case-sensitive, and must consist of 1–48
lowercase ASCII letters, digits, hyphens or underscores. Use the **same** label
for all copies or later exports of one history, even on different computers.
Use different labels only for genuinely independent histories. If a file was
already combined from several computers, import it once with a single label
instead of also importing the individual histories. Do not import overlapping
or already-synchronized Zettlr histories as independent sources: their original
provenance is not encoded in Zettlr's JSON and cannot be inferred reliably.
Labels are included in exported snapshots; avoid personal or identifying names.

The importer reads `wordCount` and `charCount` maps, checks canonical dates and
nonnegative integer counters, and preserves the union of both date sets,
including zero-count and character-only days. A missing count for a date is
zero. Legacy values are mapped to added words and characters as reported by
Zettlr; deletion counts are set to zero because they are unknown. These metrics
are not necessarily identical to Zettui's native edit counters. `pomodoros`,
targets, tags and dictionary data are not migrated. Historical dates are not
shifted, and future dates, if present, are retained (the analytics layer filters
future activity as before). Existing writing goals are not modified.

Each source is stored as a distinct `zettlr:<label>` entry in the existing
version-1 `devices` map, never as the local installation ID. This extends the
allowed identifier format; Phase 5 development builds cannot read migrated
snapshots, so upgrade all computers before exchanging them. Normal Zettui
snapshots carry these sources and retain their identities. Reimporting an
identical or older snapshot with the same label is a no-op; a later snapshot
updates cumulative counts, and conflicting increases/decreases are rejected.
A legacy correction that reduces counters is not applied automatically. Keep
original files and do not try to bypass a conflict by changing the source label.
The same transactional validation and locking as native imports apply. Source
files are never modified. No changes to native device IDs, dependencies or UI.

### Phase 6 verification

```sh
cargo test --locked --lib statistics::zettlr::tests
cargo test --locked --test zettlr_migration
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

To test without touching existing writing data, set `XDG_STATE_HOME` to an
absolute temporary directory before running the migration or dashboard.

## Phase 7: integration and release validation

The `writing_analytics_integration` test combines the legacy import CLI, native
edit tracking, pending and persisted activity, writing goals, streaks and
round-trip snapshot merging using temporary independent state directories.
Run `cargo test --locked --test writing_analytics_integration` and follow the
[release checklist](RELEASE-CHECKLIST-0.7.14.md). Actual multi-computer testing
remains pending; do not mark that check as passed based on simulated tests.

# Writing analytics (v0.7.14 development)

## Phase 1: statistics engine

The standalone `zettui::statistics` library module provides local, installation-wide
storage independent of notes, buffers, sessions, and workspaces. The terminal
application does not yet call it; ordinary app launches will not create statistics.

- State: `$XDG_STATE_HOME/zettui/statistics.json` or
  `~/.local/state/zettui/statistics.json` when no absolute XDG state path is set.
- Schema version: `1`. Each installation generates a random 128-bit identifier
  on initialization, and keeps its own cumulative counters by local calendar day.
- Four unsigned counters: words added, words deleted, characters added, characters
  deleted. Deletions never subtract from the addition counters. A zero delta does
  not create a day. Phase 2 will define edit counting, undo/redo and paste behavior.
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

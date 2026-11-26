# Zettui v0.7.14 release validation

Run on the current maintenance/development branch before the release version bump. The checks
below are instructions, not a record of completed tests. In particular, the
maintainer has **not tested statistics exchange on multiple physical computers**.
Single-host simulated device tests do not establish physical-device compatibility.

## Automated checks

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --locked
cargo test --locked --test cli_configuration
cargo test --locked --test statistics_transfer
cargo test --locked --test zettlr_migration
cargo test --locked --test writing_analytics_integration
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked
cargo build --release --locked
git diff --check
```

The new integration test runs the CLI and library in temporary directories;
it does not modify real notes or statistics. A successful run is required before
release. Record the actual test count and outcomes; do not infer success from
patch application. Also rerun the existing [buffer/session checks](RELEASE-CHECKLIST-0.7.13.md)
where relevant to guard against editor and navigation regressions.

## Isolated manual dashboard and migration checks

Run the following in a disposable **Bash** shell (`bash`), from the repository
root. Closing that shell restores your normal XDG paths. The temporary files
are intentionally kept until you finish reviewing the results.

```bash
demo=$(mktemp -d)
export XDG_STATE_HOME="$demo/state"
export XDG_CONFIG_HOME="$demo/config"
mkdir -p "$demo/notes" "$XDG_CONFIG_HOME/zettui"
touch "$demo/notes/test.md" "$demo/notes/second.md"
cat > "$XDG_CONFIG_HOME/zettui/config.toml" <<'TOML'
[writing_goals]
daily_words = 4
weekly_words = 10
TOML
cargo run --locked -- "$demo/notes" --theme dark
```

| Check | Action and expected behavior |
| --- | --- |
| Empty history | F10; zero writing totals, no activity days and no spurious counts. |
| Native edits | Open empty test.md, type `hello world` in Insert mode, Backspace once, Esc. F10: 2 words added, 11 characters added, 1 character deleted, 50% daily goal. |
| Persistence | Wait at least two seconds; inspect state file after graceful quit. Restart and verify counters and device ID are retained. |
| Navigation | Save, switch buffers, open existing note, use search and dashboard navigation; counts do not increase unless text changes. |
| Unsaved edits | Open F10 from a modified note, Esc; text and modified indicator remain intact. |
| Calendar | Keys 2 and 3; navigate h/l, j/k, [/], t. Verify selected-day counters, today and future-date handling. |
| Responsive UI | Resize to wide, narrow and short dimensions; check overview scrolling, calendar fallback and heatmap selection. |
| Theme | Compare dark and light; inspect calendar borders, centered ASCII markers and goal bars. |
| Regression | Exercise F5, :hide, :wa, :qa, session restore and F8 palette. Verify no unwanted statistics from session restore. |

After exiting the TUI, test migration and export in the **same isolated shell**:

```bash
printf '%s\n' '{"wordCount":{"2024-02-29":123},"charCount":{"2024-02-29":456},"pomodoros":{}}' > "$demo/legacy.json"
cargo run --locked -- --stats-import-zettlr "$demo/legacy.json" test-source
cargo run --locked -- --stats-import-zettlr "$demo/legacy.json" test-source
cargo run --locked -- --stats-export "$demo/export.json"
cargo run --locked -- --stats-import "$demo/export.json"
cargo run --locked -- "$demo/notes"
```

The second legacy import and reimport of the exported snapshot must report zero
updated records. Lifetime words should be 125 if exactly the native edit above
was performed; February 29, 2024 has 123 words and 456 characters with zero
recorded deletions. Verify this date in the historical calendar. Try exporting
again to the same filename; the command must fail without changing the file.
Exit Zettui and then `exit` the disposable shell. No real state is involved.

## Physical multi-computer exchange — pending

This test is **not yet performed by the maintainer**. It is a separate validation
item, not a claim that transfer is broken or that it has passed. Use two machines
running the same completed release build, each with an independent temporary
XDG state and configuration directory, and a disposable notes workspace.
Do not copy a live state file or use shared live state. Keep original backups.

1. Check `zettui --stats-device-id` on both installations; the IDs must differ.
2. On A write exactly `alpha beta` in an empty note; on B write exactly
   `gamma delta epsilon` in a separate empty note. Exit both editors cleanly.
3. On A export to a new filename with `zettui --stats-export /path/to/A.json`.
   Transfer that snapshot to B. Import twice with
   `zettui --stats-import /path/to/A.json`. The second import must update zero
   records. B's lifetime total should be five words.
4. Export B to a new `B.json`, transfer it to A and import. Both lifetime totals
   should now be five words and each local device ID should remain unchanged.
   If both devices recorded edits on the same local date, that day's total
   should also be five words; otherwise the original dates remain separate.
5. Add one word on A, close and export to a **new** filename. Import on B,
   then reimport the original A snapshot. Totals must remain six words.
6. If testing migrated history, import the same synthetic legacy history under
   the same label on both machines; exchange snapshots and confirm it counts
   once. Different labels are only for independent histories.
7. Record OS, version, device-ID distinction, observed totals, duplicate and
   stale-import behavior, and pass/fail. Keep the status as pending until these
   checks are actually performed. If releasing first, retain the limitation
   in the README and release notes; do not describe it as physically validated.

## Release handoff

- [ ] Automated checks passed (record actual results).
- [ ] Manual isolated dashboard/migration checks completed.
- [ ] Physical multi-computer exchange completed, or explicitly documented as pending.
- [ ] README, keybindings, user guide and configuration reference reviewed.
- [ ] No user statistics, device snapshots or temporary test files committed.
- [ ] Review the Unreleased changelog and roadmap; bump the package version and create the release tag only after approval and validation.

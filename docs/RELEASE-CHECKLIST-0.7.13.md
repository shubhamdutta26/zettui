# Zettui v0.7.13 integration checklist

Run from the feature branch before the version bump. Use temporary notes and isolated state/configuration, never production notes. The reported Phase 6 baseline is 172 passing tests; Phase 7 adds a combined buffer/session/hidden-folder test. A clean run, rather than the expected count alone, is the release gate.

## Automated validation

```bash
git status
cargo fmt --all
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked
cargo build --release --locked
git diff --check
```

## Disposable workspace

```bash
demo=$(mktemp -d)
mkdir -p "$demo/notes/Research" "$demo/notes/.archive" "$demo/notes/.git" \
    "$demo/notes/Other" "$demo/state" "$demo/config"
printf '# Alpha\n\n## One\n\nSearchable alpha\n' > "$demo/notes/Research/Alpha.md"
printf '# Beta\n\n## Two\n\nSearchable beta\n' > "$demo/notes/Research/Beta.md"
printf '# Archived\n\narchive-only-token\n' > "$demo/notes/.archive/Archived.md"
printf '# Other Alpha\n' > "$demo/notes/Other/Alpha.md"
for i in $(seq 1 120); do printf 'Line %03d: %s\n' "$i" 'abcdefghijklmnopqrstuvwxyz0123456789'; done > "$demo/notes/Long.md"
export XDG_STATE_HOME="$demo/state"
export XDG_CONFIG_HOME="$demo/config"
cargo run --locked -- "$demo/notes" --theme dark
```

Run the exported variables and subsequent launch commands in the same shell, or use a disposable shell and close it at the end. The temporary configuration avoids changing your actual preferences. For horizontal viewport checks create `$XDG_CONFIG_HOME/zettui/config.toml` with `[editor]` and `word_wrap = false`, then restart.

## Manual release gates

| Check | Action and expected behavior |
| --- | --- |
| Open and switch | Open Alpha, edit, `:hide`, open Beta. F5 / `]b` / `[b` switch without losing edits; reopening Alpha does not duplicate it. |
| Independent state | Edit both notes; undo (`u`) and redo (Ctrl+R) affect only the active note. Repeat `/` searches with `n` independently. Move cursors to different positions and switch back. |
| Tab appearance | Verify active/inactive backgrounds, `[+]`, duplicate names, narrow terminal clipping, `Space b t`, and Zen (`Space z`). Compare Foot and COSMIC Terminal if available. |
| Save and close | `:bd` refuses to close a dirty buffer; `:bonly` refuses if any other buffer is dirty. `:wa` saves all. `:q` returns to the browser; `:q!` discards only the current note. |
| External edits | Modify an inactive note in Zettui and externally on disk. `:wqa` must refuse exit, preserve the disk version and retain unsaved edits. Use disposable data to resolve the conflict. |
| Clean restart | Open Alpha, Beta and Long, activate Beta and set distinct cursor positions. `:qa`, then rerun the same command. Verify open set, order, active note, cursor and scrolling. |
| Browser restart | `:hide`, navigate to Research, select a file, press browser `q`. Restart: browser directory and selection restore, buffers remain accessible with F5. |
| Layout | Change F2/F3/F4, pane widths (Alt+arrows), tab visibility and F9. Quit/restart; preferences restore. Check with `restore_layout = false` separately. |
| Hidden folders | Toggle F9, open Archived, modify it, then F9 again; browser returns to a visible parent, open edits survive. `.git` never appears. Ctrl+F cannot find archive-only-token even when the folder is shown. |
| Missing note | Quit cleanly with a disposable note open, remove it externally, then restart; other buffers still restore. |
| Corrupt session | While Zettui is closed, inspect the JSON under the isolated state directory and replace that temporary session file with invalid JSON. Startup remains usable. |
| Workspace isolation | Launch with a different temporary root. It must not reopen the original workspace's buffers. |
| Existing features | Verify wiki completion (`[[`), outgoing links/backlinks, Zotero (Ctrl+P with bibliography), snippets (Ctrl+E in an empty note), F7 heading picker, F8 palette, F6 outline, search/tags, Markdown/YAML/code highlighting, Zen/Focus/Typewriter and panel resizing. |
| Native themes | Verify system theme on COSMIC and Omarchy when those environments are available, including live theme changes. Test a built-in light theme separately. |

Inspect session metadata after an exit:

```bash
ls -l "$XDG_STATE_HOME/zettui/sessions/"
```

Session files should contain metadata only, no note text. Do not publish them: paths may contain personal information.

## Release preparation

- Run the automated and manual gates; resolve any failures.
- Review README, configuration, keyboard shortcuts, architecture and changelog against the implemented commands and the feature guide.
- Verify Cargo.toml/Cargo.lock still report 0.7.12 during Phase 7.
- Commit the documentation and integration test on the feature branch. Perform the official version bump and release tag only in Phase 8.

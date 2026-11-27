# Vim-style commands

Zettui embeds `edtui` (pinned to `0.11.7`) for modal text editing. It is not
Vim or Neovim itself and does not load Vim plugins or a vimrc. This reference
covers the application-owned Ex commands and intercepted search/leader keys.
The inherited commands below were checked against the [edtui 0.11.7 reference](https://docs.rs/edtui/0.11.7/edtui/#keybindings). They are source/documentation-verified, not runtime-certified; the full upstream keymap is not vendored.

## Application Ex commands

Open a note, press Esc to enter Normal mode, type `:` followed by a command,
and press Enter. Esc cancels and Backspace edits the command. Commands are
case-sensitive, with surrounding whitespace trimmed. This reference covers
the fixed application command spellings (including aliases). These are the fixed
commands in `Command::from_ex` plus the parameterized width command in
`Writing::command`. There is no arbitrary shell execution through Ex.

| Command | Action |
| --- | --- |
| `:w` | Save active note (refuses unsafe overwrite). |
| `:cite` | Open Zotero citation picker. |
| `:cite-preview` | Preview the bracketed citation key under the cursor. |
| `:cite-check` | Inspect unresolved bracketed citations in the active note. |
| `:export` | Open the document export dialog. |
| `:q` | Close active note and return to Files; refuse if modified. |
| `:q!` | Discard active note changes and close it. |
| `:wq` | Save and close active note if save succeeds. |
| `:hide` | Return to Files without closing active buffer. |
| `:buffers`, `:ls` | Open fuzzy buffer picker. |
| `:bn`, `:bp` | Next / previous buffer. |
| `:b#` | Previously active buffer. |
| `:bd` | Close current buffer if clean; activate an adjacent buffer if present. |
| `:bonly` | Close other buffers; refuse if any other buffer is dirty. |
| `:wa` | Save all modified buffers. |
| `:qa`, `:qall` | Quit only if all buffers are clean. |
| `:wqa`, `:wqall` | Save all and quit only if all buffers are clean. |
| `:toggle tabs` | Toggle buffer tab bar. |
| `:toggle hidden`, `:toggle hidden folders` | Toggle hidden folders. |
| `:toggle files` | Toggle Files preference. |
| `:toggle preview` | Toggle Preview preference. |
| `:toggle outline` | Toggle Outline preference. |
| `:outline` | Focus Outline, or return to editor. |
| `:headings` | Fuzzy-search headings in current Markdown note. |
| `:commands` | Open global command palette. |
| `:analytics`, `:stats` | Open writing analytics dashboard. |
| `:zen` | Toggle Zen. |
| `:focus` | Toggle Focus mode. |
| `:focus off` | Disable Focus mode. |
| `:focus line`, `:focus paragraph`, `:focus section` | Enable selected Focus mode. |
| `:typewriter` | Toggle Typewriter mode. |
| `:set number`, `:set nonumber`, `:toggle number` | Show, hide or toggle line numbers for current layout. |
| `:set wrap`, `:set nowrap` | Enable or disable visual wrapping. |
| `:set status`, `:set nostatus`, `:toggle status` | Show, hide or toggle Zen status bar. |
| `:set textwidth=80` | Set Zen pane width; integer 20–240 accepted, no spaces around `=`. |

No application-wide forced-discard command such as `:qa!` is implemented.
Unrecognized commands display an error rather than being delegated to Vim.

## Application-owned Normal-mode keys

| Keys | Action |
| --- | --- |
| `/`, `?` | Incremental forward/backward in-note search. |
| `n`, `N` | Repeat last search in the same/opposite direction. |
| `]b`, `[b` | Next / previous buffer. |
| `Space b b`, `Space b p` | Buffer picker / previous active buffer. |
| `Space b d`, `Space b o` | Close current / other buffers. |
| `Space b t`, `Space b s` | Toggle tabs / save all. |
| `Space e`, `Space p`, `Space o` | Toggle Files / Preview / Outline. |
| `Space x` | Open document export. |
| `Space l`, `Space h`, `Space c` | Focus Outline / heading search / command palette. |
| `Space z`, `Space f`, `Space t` | Toggle Zen / Focus / Typewriter. |
| `Space 1`, `Space 2`, `Space 3` | Line / paragraph / section Focus. |
| `Space n`, `Space s` | Toggle line numbers / Zen status. |

`Space` is reserved as a leader in Normal mode, but inserts a literal space in
Insert mode. Escape cancels pending leader and buffer-prefix sequences. The
application handles search before passing ordinary keys to edtui. Global keys
and modal inputs have higher routing priority. Editor Ctrl shortcuts also
intercept keys, including `Ctrl+O`, `Ctrl+B`, `Ctrl+T`, `Ctrl+P`, `Ctrl+E`,
`Ctrl+G`, and `Ctrl+S`; these need not behave like Neovim's defaults.

See [Keyboard shortcuts](KEYBINDINGS.md) for browser, picker and editor bindings,
and [Buffers and Sessions](BUFFERS-AND-SESSIONS.md) for save and discard semantics.

## Inherited Vim editing keys

Outside Zettui's own modal controls and reserved bindings, the pinned editing
engine supplies the following common keys. This is a grouped reference, not a
claim of complete Neovim compatibility.

| Mode | Keys | Purpose |
| --- | --- | --- |
| Normal | `h`, `j`, `k`, `l` | Cursor movement |
| Normal | `w`, `e`, `b` | Word navigation |
| Normal | `0`, `_`, `$`, `Home`, `End` | Line positions |
| Normal | `gg`, `G`, `%` | Document boundaries / matching delimiter |
| Normal | `Ctrl+D`, `Ctrl+U`, `PageDown`, `PageUp` | Half/full-page navigation |
| Normal | `i`, `a`, `A`, `o`, `O` | Begin inserting or open a line |
| Normal | `x`, `dd`, `dw`, `dW`, `diw`, `diW`, `D` | Delete text |
| Normal | `cw`, `cW`, `ciw`, `ciW` | Change text |
| Normal | `J`, `.`, `u`, `Ctrl+R` | Join, repeat change, undo, redo |
| Normal / Visual | `v`, `viw`, `y`, `yy`, `p`, `P` | Select, yank and paste (according to mode) |
| Insert | `Esc`, `Enter`, `Backspace`, `Ctrl+H`, `Delete` | Leave Insert, newline, delete |
| Insert | Arrow keys, `Home`, `End`, `Ctrl+Left`, `Ctrl+Right` | Cursor navigation |
| Insert | `PageUp`, `PageDown`, `Ctrl+U` | Page navigation / delete toward line start |

### Known routing limitations

- Normal-mode `n`, `N`, `/`, `?`, `:`, Space and `[` / `]` are intercepted
  by Zettui before edtui sees them. This can also interfere with multi-key
  motions and text objects, e.g. `fn` or `di[`. Character-target motions
  (`f`, `t`, `df`, `dt`, `cf`, `ct`) are inherited but not fully verified.
- `Ctrl+E` opens the snippet picker, not an external editor. `Ctrl+F` opens
  vault search; the other reserved Ctrl keys are listed above.
- The pinned editor's pending key sequence is private. Changing its routing
  requires further integration testing; Phase 4 does not attempt to emulate
  it in Zettui or change the existing reserved keys.
- The application consumes key-press events only; explicit terminal Repeat
  and Release events are ignored. Holding a key depends on terminal encoding.

### Research metadata

`:metadata` and `:metadata-edit` open a modal YAML editor. Inside it, `:w`
applies to the active note, `:wq` applies and closes, `:q` closes only if clean,
and `:q!` discards unapplied changes. Save the note to disk separately.
See [Research Toolkit](RESEARCH-TOOLKIT.md#phase-iii--metadata-editor).

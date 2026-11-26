# Vim-style commands

Zettui embeds `edtui` (pinned to `0.11.7`) for modal text editing. It is not
Vim or Neovim itself and does not load Vim plugins or a vimrc. This reference
covers the application-owned Ex commands and intercepted search/leader keys.
The underlying editing engine's full motion/operator/keymap inventory is not
vendored in this repository and is not certified by this reference.

## Application Ex commands

Open a note, press Esc to enter Normal mode, type `:` followed by a command,
and press Enter. Esc cancels and Backspace edits the command. Commands are
case-sensitive, with surrounding whitespace trimmed. These are the fixed
commands in `Command::from_ex` plus the parameterized width command in
`Writing::command`. There is no arbitrary shell execution through Ex.

| Command | Action |
| --- | --- |
| `:w` | Save active note (refuses unsafe overwrite). |
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
| `:analytics`, `:stats` | Open writing analytics dashboard (development feature). |
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

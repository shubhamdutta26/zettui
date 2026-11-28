# Zettui

A local-first, Vim-style terminal notes manager written in Rust. Browse and edit Markdown and other UTF-8 text files, connect notes with wiki links, insert Zotero citations, use reusable snippets, and search across your notes. Your files stay on disk.

## Command and editing enhancements (v0.8.2 development)

Press `:` in Normal mode for a floating Ex command window; F8 remains the
searchable command palette. In Markdown notes, `Ctrl+K` / `:link` inserts a
link and `Alt+I` / `:image` inserts an image reference using a two-field form.
See the [user guide](docs/USER-GUIDE.md) and [keyboard shortcuts](docs/KEYBINDINGS.md).

## Installation

Zettui **v0.8.1 — UI and Theme Refinements** aligns Outline heading colors with
Markdown syntax highlighting, uses the active theme accent in the writing
calendar and yearly heatmap, and enables `Ctrl+G` theme cycling inside Writing
Analytics. The v0.8.0 Research Toolkit includes richer Zotero citation search,
reference previews, local PDF attachments, a standalone YAML metadata editor,
and optional Pandoc-powered document export. See the [changelog](CHANGELOG.md)
and [release checklist](docs/RELEASE-CHECKLIST-0.8.1.md). COSMIC and physical
multi-computer statistics checks remain unverified unless retested.

Download Linux releases from [GitHub Releases](https://github.com/shubhamdutta26/zettui/releases) (portable x86-64 archive, `.deb` for Debian/Ubuntu, `.rpm` for Fedora, or `PKGBUILD` for Arch/Omarchy), or build from source with a compatible Rust toolchain:

```bash
git clone https://github.com/shubhamdutta26/zettui.git
cd zettui
cargo install --path . --locked
```

Rust 1.88 is the declared minimum; the current lockfile has not been verified
against that exact compiler. CI uses stable Rust. See [Development](docs/DEVELOPMENT.md).

Run with `zettui ~/Documents/Notes`, or simply `zettui` when `notes_dir` is configured.

## Quick start

Copy the [example configuration](config.example.toml) to your Zettui configuration directory and edit the paths. The default appearance follows COSMIC, Omarchy or the terminal, falling back to built-in themes.

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/zettui"
cp -n config.example.toml "${XDG_CONFIG_HOME:-$HOME/.config}/zettui/config.toml"
zettui --help
zettui --version
zettui --check-config
```

Try a disposable copy of the [example vault](examples/notes/):

```bash
demo=$(mktemp -d)
cp -R examples/notes "$demo/notes"
cargo run --locked -- "$demo/notes" --bibliography examples/bibliography/references.json
```

Use `j`/`k` to browse, Enter to open, `i` to edit, Escape for Normal mode, and `:w` to save. `Space z` toggles Zen mode. See the full references below.

## Research toolkit

Search and mark Zotero citations with `Ctrl+P`, preview with `Ctrl+V`, inspect a citation under the cursor with `:cite-preview`, and audit missing references using `:cite-check`. Open the modal YAML Metadata Editor with `:metadata`, `Space m` or `F11`; apply with `:wq` without saving the note to disk. See [Research Toolkit](docs/RESEARCH-TOOLKIT.md) for usage, supported exports and PDF attachment requirements.

## Syntax highlighting

Since v0.7.11, Zettui provides theme-aware source highlighting in the editor and browser preview without changing note contents.

- Markdown: six heading levels, nested emphasis, strikethrough, inline code, links and images, wiki links, tags, citations, footnotes, math, task lists, tables and HTML comments.
- YAML: keys, strings, numbers (including scientific notation), dates, booleans, nulls, comments, nested mappings, collections, anchors, aliases and multiline strings.
- Fenced code: lightweight language-specific highlighting for R, Python, Rust, Bash, JavaScript, TypeScript, C, C++, SQL, YAML, TOML, JSON, Typst and LaTeX, plus basic HTML and CSS. Standard Markdown fences and Quarto-style fences such as ` ```{r, echo=false} ` select the same R lexer. Unknown languages remain plain.

Syntax colors inherit the active theme (including COSMIC and Omarchy). Optional `[markdown.heading_colors]`, `[markdown.syntax_colors]`, `[markdown.yaml_colors]` and `[markdown.code_colors]` tables allow individual overrides; no new configuration is required. Automatic heading colors avoid duplicates, and inline code has a separate color role. See [Configuration](docs/CONFIGURATION.md#syntax-color-overrides) for details.

Highlighting is source-preserving and lexical, not a full Markdown renderer, YAML validator or programming-language parser. Quarto chunks are highlighted, not executed.

## Intelligent Navigation

Zettui v0.7.12 adds keyboard-driven navigation without changing the plain-text storage model.

| Shortcut | Action |
| --- | --- |
| `F2` / `F3` / `F4` | Toggle Files / Preview / Outline preferences |
| `F6` | Focus Outline or return to editor |
| `F7` | Fuzzy-search headings in the open Markdown note |
| `F8` | Search and execute available application commands |
| `Alt+↑` / `Alt+↓` | Resize Outline |

The outline and heading search use live, unsaved Markdown. Outline headings share the editor’s H1–H6 colors, including custom overrides when syntax highlighting is enabled; the selected entry retains its selection style. Collapse outline branches with `h`/`l`, select with `j`/`k`, and jump with Enter. The outline is hidden by default. The palette reuses existing commands and safety checks; `Ctrl+P` remains Zotero. See the [user guide](docs/USER-GUIDE.md), [keybindings](docs/KEYBINDINGS.md) and [configuration](docs/CONFIGURATION.md).

## Writing Analytics (since v0.7.14)

Open the writing dashboard with `F10`, the command palette or `:analytics` / `:stats`.
View writing goals, streaks, activity totals and a monthly calendar or yearly
heatmap. `Tab` / `Shift+Tab` cycle views (`1` Overview, `2` Month, `3` Year),
`h`/`l` move by day and `j`/`k` by week in calendar views (or scroll the Overview),
`[`/`]` move periods, `t` returns to today, `r` refreshes, `Ctrl+G` cycles themes
and `Esc` closes. Month and Year activity colors follow the active theme accent.
Exchange writing history between computers using duplicate-safe snapshot merging:

```bash
zettui --stats-export ~/Downloads/writing-stats.json
zettui --stats-import ~/Downloads/writing-stats.json
zettui --stats-device-id
```

Exit the editor before exporting to include all pending edits. Keep each computer's
live statistics file separate; exchange snapshots instead. Exports require a new
filename and never replace an existing file.

**Validation status:** multi-computer snapshot exchange has automated test coverage
using isolated state directories, but the maintainer has not yet tested it on
multiple physical computers. Keep backups of your original statistics and test
with disposable state before relying on cross-computer exchange.

Import legacy Zettlr history with `zettui --stats-import-zettlr FILE SOURCE`
(e.g. `zettui --stats-import-zettlr "$HOME/.config/Zettlr/stats.json" desktop`). Reuse the same source label for the same history; never import
a combined history alongside its original component files.
See [Writing analytics](docs/WRITING-ANALYTICS.md) for configuration and details.

## Buffers and Sessions

Since v0.7.13, Zettui keeps multiple notes open in memory, with independent editing state, theme-aware tabs and a fuzzy buffer switcher. Use `:hide` to return to the Files browser without closing a note, then Enter to open another note or `F5` to switch back to an existing buffer.

| Shortcut | Action |
| --- | --- |
| `F5` / `Space b b` | Fuzzy-switch open buffers |
| `]b` / `[b` | Next / previous buffer |
| `Space b p` | Previously active buffer |
| `Space b d` / `Space b o` | Close current / other buffers, subject to unsaved-change checks |
| `Space b t` | Toggle buffer tabs |
| `Space b s` / `:wa` | Save all modified buffers |
| `:qa` / `:wqa` | Quit all / save all and quit, keeping the open-buffer set for the next session |
| `F9` | Show or hide dot-prefixed folders (`.git` remains excluded) |

Sessions restore open notes, their order, the active note, cursor and viewport positions, and browser/panel preferences on restart. They are stored locally under `$XDG_STATE_HOME/zettui/sessions/` (default `~/.local/state/zettui/sessions/`) and contain metadata, not unsaved note contents. `:qa` refuses to quit when any buffer is dirty; `:wqa` refuses to quit if any changes cannot be saved. `:q` still closes the active note rather than quitting the application.

See [Buffers and Sessions](docs/BUFFERS-AND-SESSIONS.md) for the full command reference, configuration, recovery limitations and testing procedure.

## Documentation

- [User guide](docs/USER-GUIDE.md)
- [Buffers and Sessions](docs/BUFFERS-AND-SESSIONS.md)
- [Configuration](docs/CONFIGURATION.md)
- [Keyboard shortcuts and writing modes](docs/KEYBINDINGS.md)
- [Vim commands, inherited editing keys and limitations](docs/VIM-COMMANDS.md)
- [Command-line reference](docs/CLI.md)
- [Writing analytics](docs/WRITING-ANALYTICS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Development](docs/DEVELOPMENT.md)
- [Releasing](docs/RELEASING.md)
- [Changelog](CHANGELOG.md) · [Roadmap](ROADMAP.md) · [Historical lessons](docs/lessons/)

For installed release packages, the complete documentation is also available in the [GitHub repository](https://github.com/shubhamdutta26/zettui).

MIT License.

## Research toolkit: document export

Press `Space x` in Normal mode or enter `:export` in a Markdown note, or use F8 → **Export document or folder** in the
file browser. Export PDF, DOCX, standalone HTML, EPUB or LaTeX using optional
Pandoc. See [Document export](docs/DOCUMENT-EXPORT.md) for configuration, images,
citation processing and limitations.

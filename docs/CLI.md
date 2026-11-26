# Command-line reference

Commands below reflect `src/cli.rs` and `src/statistics_cli.rs` in the v0.7.13
source with v0.7.14 writing analytics development. The displayed binary version
is the version from `Cargo.toml`, not a separate feature version.

## Start Zettui

```text
zettui [NOTES_DIR] [--bibliography FILE] [--theme NAME]
zettui --help
zettui -h
zettui --version
zettui -V
zettui --check-config
```

| Argument | Meaning |
| --- | --- |
| `NOTES_DIR` | Workspace path; overrides `[workspace].notes_dir`. If neither is supplied, use the working directory. At most one positional path. |
| `--bibliography FILE` | Override `[workspace].bibliography` with a Zotero JSON bibliography. |
| `--theme NAME` | Override `[appearance].theme`. |
| `--help`, `-h` | Print usage and theme names. |
| `--version`, `-V` | Print the compiled package version. |
| `--check-config` | Validate configuration without opening the TUI or writing state. Run alone. |
| `--` | End options; treat the remaining argument as a literal notes directory. |

Configuration is read from `$XDG_CONFIG_HOME/zettui/config.toml` when the
variable is nonempty; otherwise from `~/.config/zettui/config.toml`.
Relative configured workspace paths are resolved against the configuration
directory; relative CLI paths are resolved from the working directory. Quote
paths containing spaces. The parser does not support `--flag=value`.
Use `--` before a dash-leading notes directory, or `./` before a relative
path beginning with `-` when passing an option value. Options requiring values
reject an empty value or a dash-leading token. Help and version do not load
configuration or initialize the terminal. The first help/version action
encountered ends startup argument processing (invalid earlier arguments can
still produce an error). `--check-config` must be the first and only argument.

`--check-config` checks TOML parsing, supported values, panel widths, theme
names, fallback palettes and explicit colors. It uses defaults if no file
exists. It does not check whether workspace files or directories exist, validate
live desktop palettes, or open a terminal. It exits nonzero on validation errors, but is not a strict
unknown-key validator for every nested TOML section.

Supported themes: `system`, `terminal`, `auto`, `dark`, `light`, `nord`,
`dracula`, `everforest`, `rose-pine`, `kanagawa`, `solarized-dark`,
`catppuccin-latte`, `rose-pine-dawn`, `gruvbox`, `tokyo-night`, `midnight`. Legacy aliases:
`catppuccin` (dark) and `paper` (light).

```sh
zettui --check-config
zettui --theme system -- -notes
zettui "$HOME/Documents/Notes" --theme system
zettui "$HOME/Documents/Notes" --bibliography "$HOME/Zotero/references.json"
```

## Writing statistics (v0.7.14 development)

These standalone commands must be the first argument and do not open the TUI
or load the note configuration. Extra arguments are rejected. For a relative
snapshot filename beginning with `-`, use `./filename`.

| Command | Meaning |
| --- | --- |
| `zettui --stats-export FILE` | Export a snapshot to a new file; never overwrite an existing destination. |
| `zettui --stats-import FILE` | Merge a Zettui snapshot into local statistics. |
| `zettui --stats-import-zettlr FILE SOURCE` | Import legacy Zettlr history using a stable label for that independent source. |
| `zettui --stats-device-id` | Print the local device identifier; may initialize missing statistics state. |

```sh
zettui --stats-export "$HOME/Downloads/writing-stats.json"
zettui --stats-import "$HOME/Downloads/writing-stats.json"
zettui --stats-import-zettlr "$HOME/.config/Zettlr/stats.json" desktop
zettui --stats-device-id
```

Exit Zettui before exporting so pending activity can be flushed. Keep separate
live statistics on each computer and transfer snapshots, not live state files.
Reuse the same Zettlr source label for the same history; do not import both a
combined history and its components. Physical multi-computer exchange has not
been manually tested by the maintainer. See [Writing analytics](WRITING-ANALYTICS.md)
for merge behavior, state paths, backup instructions and limitations.

For a source checkout, replace `zettui` in these examples with `cargo run --locked --`.
See also [Configuration](CONFIGURATION.md) and [Vim-style commands](VIM-COMMANDS.md).

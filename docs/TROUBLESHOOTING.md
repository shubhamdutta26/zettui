# Troubleshooting

## Startup and configuration

Run `zettui --version`, `zettui --help` and `zettui --check-config` to identify
the build and check configuration. Help/version do not load configuration.
Check `$XDG_CONFIG_HOME/zettui/config.toml` (default
`~/.config/zettui/config.toml`). Use sectioned TOML; old flat keys are not
supported. A successful configuration check does not prove paths exist or
that a native theme provider is available. See [Configuration](CONFIGURATION.md)
and [CLI](CLI.md).

## Files, search and templates

| Symptom | What to check |
| --- | --- |
| File does not open | Regular UTF-8 files only, up to 256,000 bytes on load. Symlinks are rejected. |
| Search result is missing | Save the note first; vault search reads disk, not live buffers. Hidden entries, symlinks, unsupported extensions and files larger than 1 MiB are skipped. Search stops at 20,000 files or 128 MiB of file contents. |
| Link index is incomplete | Hidden directories and symlinks are excluded; the index errors above 12,000 supported files. Source metadata is read only for eligible files up to 256,000 bytes. Narrow the workspace if necessary. |
| Snippet not listed | Check the configured snippet directory; files must be top-level, non-hidden, regular files with no extension or `.md`, `.txt`, `.snippet`, up to 32 KiB. The picker requires an empty note. |
| Zotero citations unavailable | Check the configured bibliography path and JSON citation keys. The bibliography must be no larger than 32 MiB. |
| Save refused after external edits | Preserve your unsaved text separately before closing or reopening. Zettui refuses to overwrite changed disk contents; it does not merge external edits automatically. |
| Rename, move or Trash refused | Save and close affected open buffers first, including any inside the selected folder. |

## Keyboard and appearance

- Press Esc to leave modal input or return to Normal mode before invoking
  mode-restricted shortcuts. See [Keybindings](KEYBINDINGS.md).
- Some inherited multi-key Vim sequences conflict with reserved keys; see
  [Vim limitations](VIM-COMMANDS.md#known-routing-limitations).
- If automatic theme detection is unsuitable, try `zettui --theme dark` or
  `zettui --theme light`. Native palettes and cursor rendering depend on
  desktop/terminal support.
- The Preview never receives keyboard focus. Zen or limited terminal space
  can temporarily hide sidebars; resize the terminal or disable Zen.

## Sessions and writing history

- For a fresh session without replacing saved metadata, temporarily set
  `[session] enabled = false`; see [Buffers and Sessions](BUFFERS-AND-SESSIONS.md).
- For `Stats unsaved`, preserve the existing statistics files and check write
  permissions and available disk space. The tracker retries pending activity;
  a persistent failure can lose unflushed counts on exit.
- Close Zettui before exporting statistics and use a new destination filename.
  Do not share a live statistics file between devices. Back up original files
  before imports; never change a source label just to bypass a merge conflict.
- Physical multi-computer exchange remains manually unverified. See
  [Writing Analytics](WRITING-ANALYTICS.md) for data semantics and recovery limits.

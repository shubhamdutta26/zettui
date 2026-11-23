# Lesson 7.4 — Snippet templates

## Goal

Apply a reusable Zettlr-style note template to a new, empty note without typing front matter repeatedly. Zettui reads snippet files from `$XDG_CONFIG_HOME/zettui/snippets` or `~/.config/zettui/snippets` by default.

## Install

Use the kit's installer from a clean v0.7.3 feature branch. It updates only the necessary source anchors and makes a backup. Run `--check` first. `cargo check` will add the `chrono` dependency to `Cargo.lock`.

## Set up a snippet

```bash
mkdir -p ~/.config/zettui/snippets
cp /tmp/zettui-v0.7.4-kit/examples/snippets/new-note.md ~/.config/zettui/snippets/
```

Any UTF-8 extensionless, `.md`, `.txt`, or `.snippet` file in that folder becomes a template. Files larger than 32 KiB and symlinks are ignored. Snippets are reloaded each time you open the picker. You can also set `snippets_dir = "~/my-templates"` in `config.toml` (relative paths are resolved from the config directory).

## Use

1. In the file browser, press `n` to create `test-note.md`, then open it with `Enter`.
2. Press `Ctrl+E`, type `new-note`, and press `Enter`.
3. The three numbered fields appear in an input form. Type the title; `Tab` or `Enter` goes to the tag; then the heading. `Shift+Tab` moves backward; `Ctrl+U` clears a field.
4. Press `Tab` or `Enter` on the final field to insert the expanded template. You will be in Insert mode at `$0`.
5. Press `Esc`, then `:w` to save, or `:q!` to discard.

### Supported Zettlr macros

| Token | Behavior |
| --- | --- |
| `${1:Untitled}` | Prompt for numbered field 1; default `Untitled` |
| `${2:tag}` | Prompt for field 2; numbering controls order |
| `$CURRENT_YEAR` | Local four-digit year |
| `$CURRENT_MONTH` | Local zero-padded month |
| `$CURRENT_DATE` | Local zero-padded day |
| `$ZKN_ID` | Local timestamp in default `YYYYMMDDHHMMSS` format |
| `$0` | Cursor location after insertion |

Repeated `${n:default}` markers share the same supplied value. Unknown `$VARIABLE` strings remain unchanged. In this first version, the numbered fields are filled **before** insertion, not via Tab inside the editor, and only **completely empty notes** accept templates. That protects existing YAML and data. `Esc` in either dialog cancels without modifying the note.

The ID is generated when the template is applied. This version does not scan the vault for duplicate IDs or support custom Zettlr ID patterns. Be careful about generating multiple IDs in the same second. Your note is not saved until you explicitly use `:w`.

## Verify

```bash
cargo fmt
cargo check
cargo test
```

Check generated YAML, title, tag, date, 14-digit ID, cursor position, unsaved marker, and successful reopen after `:w`. Also test `Ctrl+E` on a nonempty note: it must refuse insertion. If your shell's terminal captures `Ctrl+E`, inspect Foot bindings before changing Zettui.

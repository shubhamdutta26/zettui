# Configuration reference

## Configuration

Copy the documented [config.example.toml](../config.example.toml) to `~/.config/zettui/config.toml` (or `$XDG_CONFIG_HOME/zettui/config.toml`) and edit it:

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/zettui"
cp -n config.example.toml "${XDG_CONFIG_HOME:-$HOME/.config}/zettui/config.toml"
```

The canonical configuration uses TOML tables:

```toml
[workspace]
notes_dir = "~/Documents/Notes"
bibliography = "~/Zotero/references.json"

[appearance]
theme = "system"
source = "auto"
live_reload = true
ensure_contrast = true
light_theme = "catppuccin-latte"
dark_theme = "everforest"

[layout]
browser_width_percent = 35

[markdown]
syntax_highlighting = true
highlight_code_blocks = true
highlight_frontmatter = true
dim_delimiters = true

[markdown.heading_colors]
# h1 = "#CBA6F7" # Optional; inherits adaptive color if omitted.
```

### Configuration reference

All keys are optional. Restart Zettui after changing the file. Session commands do not write it. The CLI notes path, `--bibliography` and `--theme` override the corresponding configured values. Relative configured paths resolve against the config directory; `~/` expands the home directory.

| Table | Key | Default / meaning |
| --- | --- | --- |
| `workspace` | `notes_dir` | Current directory if no CLI path |
| `workspace` | `bibliography` | Unset; Zotero JSON with citation keys (Better CSL JSON recommended) |
| `workspace` | `snippets_dir` | Zettui config directory's `snippets/` |
| `appearance` | `theme` | `"system"`; native, terminal, auto or built-in preset |
| `appearance` | `source` | `"auto"`; `cosmic`, `omarchy` or `terminal` restricts provider |
| `appearance` | `live_reload` | `true`; periodically refresh native desktop colors |
| `appearance` | `ensure_contrast` | `true`; adjust low-contrast RGB text |
| `appearance` | `light_theme` | `"light"`; fixed palette for Auto light appearance |
| `appearance` | `dark_theme` | `"dark"`; fixed palette for Auto dark appearance |
| `layout` | `browser_width_percent` | `35`; integer 20–70 |
| `editor` | `line_numbers` | `true` |
| `editor` | `word_wrap` | `true`; visual wrapping |
| `markdown` | `syntax_highlighting` | `true`; master switch for editor and preview |
| `markdown` | `highlight_code_blocks` | `true`; fenced code and its markers |
| `markdown` | `highlight_frontmatter` | `true`; YAML frontmatter |
| `markdown` | `dim_delimiters` | `true`; mute formatting delimiters, fence and table markers; heading markers retain heading color |
| `markdown.heading_colors` | `h1`, `h2`, `h3`, `h4`, `h5`, `h6` | Unset; each uses adaptive light/dark color; optional quoted `#RRGGBB` overrides |
| `zen` | `enabled` | `false` |
| `zen` | `width` | `80`; effective range 20–240 columns |
| `zen` | `show_line_numbers` | `false` |
| `zen` | `show_status_bar` | `true` |
| `zen` | `show_title` | `true` |
| `focus` | `enabled` | `false` |
| `focus` | `mode` | `"paragraph"`; `"line"`, `"paragraph"`, `"section"` |
| `focus` | `dim_strength` | `0.65`; clamped to 0–0.85 |
| `focus` | `typewriter` | `false`; independent of focus |
| `colors` | `background`, `surface`, `foreground`, `muted`, `border`, `accent`, `directory`, `selected_bg`, `selected_fg`, `warning`, `success` | Unset; optional quoted `#RRGGBB` global theme overrides |

Heading overrides apply across themes and to both preview and editor. Choose colors readable on your background. Missing heading overrides use adaptive colors. Disabling fenced code or YAML highlighting leaves those blocks plain, without interpreting their contents as Markdown. Inline code remains highlighted unless the master switch is off. Search and focus highlighting are independent of the master switch. Invalid heading colors are configuration errors.

The sectioned configuration is required starting with v0.7.10. Old top-level workspace, theme and browser width keys are rejected rather than silently ignored. Move these settings into `[workspace]`, `[appearance]` and `[layout]` respectively. Zettui never rewrites the configuration. The current example is the canonical reference.


## Native system theming

On a COSMIC desktop, `appearance.theme = "system"` reads the active resolved v2 RON palette from `$XDG_CONFIG_HOME/cosmic` (or `~/.config/cosmic`), with per-file defaults from `/usr/share/cosmic` where present. It follows the desktop mode and custom palette while running. Set `source = "cosmic"` to explicitly select this provider. Missing or unsupported palettes fall back; explicit COSMIC mode uses the configured light/dark fallback. Terminal cells approximate alpha with opaque RGB.

On other desktops the default reads Omarchy's active `colors.toml` (under `$XDG_STATE_HOME` or `~/.local/state`, then `$XDG_CONFIG_HOME` or `~/.config`). It accepts named semantic colors and older ANSI-style keys. Background, foreground, selection, surface, border, syntax and H1–H6 colors follow the active palette where available. Missing fields use derived or fallback colors. Existing `auto` retains light/dark preset selection; fixed themes and `Ctrl+G` remain available.

| Setting | Default | Meaning |
| --- | --- | --- |
| `appearance.theme` | `system` | Native colors; `terminal`, `auto`, and fixed presets also supported |
| `appearance.source` | `auto` | `auto` tries COSMIC in a COSMIC session, otherwise Omarchy, then terminal; `cosmic`, `omarchy` or `terminal` restricts the provider |
| `appearance.live_reload` | `true` | Poll native themes at most every 400 ms; no restart for palette changes |
| `appearance.ensure_contrast` | `true` | Adjust low-contrast RGB foreground/semantic text against the background |
| `appearance.light_theme` / `dark_theme` | `light` / `dark` | Fixed light/dark fallback palettes |

Terminal mode samples RGB background and foreground at startup using terminal-colorsaurus, and uses the terminal's ANSI color slots for semantic colors. Restart to refresh sampled terminal RGB colors. Full ANSI RGB querying and older COSMIC v1 palettes are not implemented. ANSI semantic colors cannot be contrast-checked or hue-preservingly dimmed without their RGB values. Unsupported terminal queries fall back to built-in palettes. `[colors]` and `[markdown.heading_colors]` overrides still take priority, including over contrast correction. The configuration file is never rewritten.

Test on Omarchy by switching between two dark themes, then between dark and light themes while a note is open. Verify editor, preview, search, selection, dialogs, focus and headings update without modifying the note.

Test on COSMIC by toggling light/dark and changing the accent or custom palette while a note is open. Existing overrides remain in effect.

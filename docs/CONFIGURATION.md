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
outline_width_percent = 25
show_files = true
show_preview = true
show_outline = false

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
| `layout` | `browser_width_percent` | `35`; preferred percent, integer 20–70 |
| `layout` | `outline_width_percent` | `25`; preferred percent, integer 15–50 |
| `layout` | `show_files` | `true`; while editing; always visible in browser |
| `layout` | `show_preview` | `true`; browser mode only |
| `layout` | `show_outline` | `false`; editor mode only; interactive Markdown outline |
| `writing_goals` | `daily_words` | `0` (disabled); positive gross word-addition target per local day |
| `writing_goals` | `weekly_words` | `0` (disabled); positive gross word-addition target per Monday-start week |
| `editor` | `line_numbers` | `true` |
| `editor` | `word_wrap` | `true`; visual wrapping |
| `markdown` | `syntax_highlighting` | `true`; master switch for editor and preview |
| `markdown` | `highlight_code_blocks` | `true`; fenced code and its markers |
| `markdown` | `highlight_frontmatter` | `true`; YAML frontmatter |
| `markdown` | `dim_delimiters` | `true`; mute formatting delimiters, fence and table markers; heading markers retain heading color |
| `markdown.heading_colors` | `h1`, `h2`, `h3`, `h4`, `h5`, `h6` | Unset; each uses adaptive light/dark color; optional quoted `#RRGGBB` overrides |
| `markdown.syntax_colors` | See [syntax color overrides](#syntax-color-overrides) | Unset; inherit theme colors |
| `markdown.yaml_colors` | See [syntax color overrides](#syntax-color-overrides) | Unset; inherit theme colors |
| `markdown.code_colors` | See [syntax color overrides](#syntax-color-overrides) | Unset; inherit theme colors |
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

Heading overrides apply across themes and to both preview and editor. Choose colors readable on your background. Missing heading overrides use native theme heading colors where available, otherwise adaptive light/dark colors. Duplicate automatic heading colors are replaced with unused fallback colors; explicit overrides are preserved. Disabling fenced code or YAML highlighting leaves those blocks plain, without interpreting their contents as Markdown. Inline code remains highlighted unless the master switch is off. Search and focus highlighting are independent of the master switch. Invalid heading or syntax colors and unknown keys in these color tables are configuration errors.

The sectioned configuration is required starting with v0.7.10. Old top-level workspace, theme and browser width keys are rejected rather than silently ignored. Move these settings into `[workspace]`, `[appearance]` and `[layout]` respectively. Zettui never rewrites the configuration. The current example is the canonical reference.


## Syntax color overrides

Starting with v0.7.11, individual syntax roles can be overridden without changing the global UI palette. All entries are optional, accept quoted `#RRGGBB` values, and apply to both the editor and preview. Restart after editing the configuration. Omitted roles follow the active resolved theme, including live desktop palette changes. Explicit overrides take precedence over global `[colors]` overrides and are not contrast-corrected; choose colors suitable for your background.

The following tables list all supported keys. The values in the example are suggestions for dark backgrounds, not fixed defaults.

### `markdown.syntax_colors`

Markdown emphasis, references, links, math, comments, list markers and formatting punctuation. Inline code uses its own color role.

```toml
[markdown.syntax_colors]
# Optional #RRGGBB overrides; omitted entries inherit the active theme.
# bold = "#F38BA8"
# italic = "#A6E3A1"
# strikethrough = "#A6ADC8"
# highlight = "#F38BA8"
# inline_code = "#CBA6F7"
# link = "#89B4FA"
# link_destination = "#CBA6F7"
# wiki_link = "#89B4FA"
# tag = "#A6E3A1"
# citation = "#CBA6F7"
# footnote = "#CBA6F7"
# math = "#F38BA8"
# comment = "#A6ADC8"
# blockquote = "#A6ADC8"
# list_marker = "#CBA6F7"
# task_done = "#A6E3A1"
# task_pending = "#F38BA8"
# delimiter = "#A6ADC8"
# horizontal_rule = "#A6ADC8"
```

### `markdown.yaml_colors`

YAML frontmatter and the contents of fenced YAML blocks share these roles. Dates and timestamps use `number`; anchors, aliases and explicit tags use `reference`.

```toml
[markdown.yaml_colors]
# Optional #RRGGBB overrides; omitted entries inherit the active theme.
# key = "#89B4FA"
# string = "#A6E3A1"
# number = "#F38BA8"
# boolean = "#CBA6F7"
# null = "#CBA6F7"
# comment = "#A6ADC8"
# marker = "#F38BA8"
# reference = "#CBA6F7"
# delimiter = "#A6ADC8"
```

### `markdown.code_colors`

Fenced programming-language and data-format blocks use these roles, except YAML contents. Fence markers use the code delimiter role even for YAML fences.

```toml
[markdown.code_colors]
# Optional #RRGGBB overrides; omitted entries inherit the active theme.
# keyword = "#CBA6F7"
# constant = "#CBA6F7"
# function = "#89B4FA"
# string = "#A6E3A1"
# number = "#F38BA8"
# comment = "#A6ADC8"
# operator = "#CBA6F7"
# delimiter = "#A6ADC8"
```

### Behavior and precedence

- `syntax_highlighting = false` disables lexical highlighting in both panes. Search and focus overlays remain independent.
- `highlight_frontmatter = false` leaves frontmatter plain; `highlight_code_blocks = false` leaves fenced blocks plain, including fenced YAML. Neither switch disables inline code.
- `dim_delimiters = false` uses the theme foreground for formatting delimiters instead of their muted or overridden colors. Heading markers retain the heading color. Horizontal rules have their own role.
- Explicit heading colors are reserved first; automatic heading colors avoid exact duplicates using adaptive fallback colors. Deliberately identical explicit heading overrides are allowed.
- Unspecified syntax roles inherit semantic colors from the resolved palette (such as accent, directory, success, warning and muted). The inline-code default uses accent, or foreground if accent matches the bold color.

### Supported fenced code

Use ordinary fences such as ` ```r `, or Quarto-style fences such as ` ```{r} ` and ` ```{r, echo=false} `. The language identifier selects the lexer; Quarto itself is not a language and Zettui does not execute chunks.

Supported identifiers include `rust` (`rs`), `python` (`py`), `r`, `bash` (`sh`, `shell`, `zsh`), `javascript` (`js`), `typescript` (`ts`), `c`, `cpp` (`c++`, `cc`, `cxx`), `sql`, `yaml` (`yml`), `toml`, `json`, `typst`, `latex` (`tex`), `html` and `css`. Unknown identifiers are left plain. HTML, CSS and Typst have basic lexical coverage. Numeric literals, functions, comments and strings are highlighted where recognized, including selected multiline strings and comments.

This is a lightweight lexer, not a validator or full grammar implementation. Nested Markdown emphasis is line-based; table recognition is heuristic. Explicit YAML tags do not fully resolve scalar types, and advanced code constructs such as nested block comments, Rust raw strings, Bash heredocs and interpolation are not fully supported.

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

Terminal mode samples RGB background and foreground at startup using terminal-colorsaurus, and uses the terminal's ANSI color slots for semantic colors. Restart to refresh sampled terminal RGB colors. Full ANSI RGB querying and older COSMIC v1 palettes are not implemented. ANSI semantic colors cannot be contrast-checked or hue-preservingly dimmed without their RGB values. Unsupported terminal queries fall back to built-in palettes. `[colors]` and explicit Markdown heading and syntax color overrides still take priority, including over contrast correction. The configuration file is never rewritten.

Test on Omarchy by switching between two dark themes, then between dark and light themes while a note is open. Verify editor, preview, search, selection, dialogs, focus and headings update without modifying the note.

Test on COSMIC by toggling light/dark and changing the accent or custom palette while a note is open. Existing overrides remain in effect.

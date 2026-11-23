# Zettui

A local-first, Vim-style terminal notes manager. Browse and edit Markdown or other text files, navigate wiki links and backlinks, autocomplete `[[note links]]`, and insert citations from Zotero JSON exports. Your notes stay on disk.

## Install

Get x86-64 Linux packages from [GitHub Releases](https://github.com/shubhamdutta26/zettui/releases):

| System | Command |
| --- | --- |
| Debian / Ubuntu | `sudo apt install ./zettui_0.7.3_linux_x86_64.deb` |
| Fedora | `sudo dnf install ./zettui_0.7.3_linux_x86_64.rpm` |
| Arch / Omarchy | Download `PKGBUILD` and matching `.tar.gz`, inspect, run `makepkg -si` |

For other Linux distributions, extract `zettui_0.7.3_linux_x86_64.tar.gz` and install the `zettui` executable into `~/.local/bin`. From source: `cargo install --path . --locked` (Rust 1.88+).

## Usage and settings

```bash
zettui ~/Documents/Notes
```

Configure `~/.config/zettui/config.toml`:

```toml
notes_dir = "/home/USER/Documents/Notes"
bibliography = "/home/USER/Documents/Zotero/references.json"
theme = "auto" # auto | dark | light
browser_width_percent = 35 # width of file pane; 20 to 70; note pane takes the rest
```

Use Zotero Better CSL JSON (or a compatible BetterBibTeX JSON export). In Auto theme mode, Omarchy theme changes are followed while Zettui runs.

## Keybindings

| Browser | Action | Editor | Action |
| --- | --- | --- | --- |
| `j` / `k` | Select | `i` / `Esc` | Insert / Normal |
| `Enter` / `h` | Open / parent | `hjkl`, `w`, `b`, `gg`, `G` | Move cursor |
| `n` / `N` | Create note / folder | `:w`, `:wq`, `:q!` | Save, close, discard |
| `r` / `m` / `d` | Rename / move / Trash | `[[` in Insert mode | Link autocomplete |
| `/` | Search current folder | `Ctrl+P` | Search Zotero citations |
| `o` / `b` | Outgoing / backlinks | `Ctrl+O` / `Ctrl+B` | Outgoing / backlinks |
| `R` / `q` | Refresh / quit | `Ctrl+T` | Previous linked note |

**In both browser and editor:** `Alt+h` / `Alt+Left` narrows the file pane; `Alt+l` / `Alt+Right` widens it (5% steps). `Ctrl+G` cycles Auto/Dark/Light themes. Hotkey changes last until Zettui exits; edit `config.toml` for a permanent default.

See [ROADMAP.md](ROADMAP.md) and [docs/lessons](docs/lessons/). Licensed under [MIT](LICENSE).

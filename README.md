# Zettui

A local-first, Vim-style terminal app for managing plain-text notes. Browse folders, edit Markdown and other text files, follow wiki links, view backlinks, and insert Zotero citations. Your notes stay on disk.

## Features

- Vim-style editing with unsaved-change and external-modification protection
- Create, rename, move, and Trash notes or directories
- `[[wiki links]]`, Markdown links, backlinks, and link navigation history
- Zotero/Better BibTeX JSON citation search and `[@citekey]` insertion
- Current-folder filename search and automatic light/dark themes (including Omarchy)

## Install

**Linux x86-64:** Download the matching files from the [GitHub Releases page](https://github.com/OWNER/zettui/releases) (replace `OWNER` with the repository owner).

| System | Install |
| --- | --- |
| Debian / Ubuntu | `sudo apt install ./zettui_0.7.1_linux_x86_64.deb` |
| Fedora | `sudo dnf install ./zettui_0.7.1_linux_x86_64.rpm` |
| Arch / Omarchy | Download `PKGBUILD` and the matching `.tar.gz` into one directory; review the recipe and run `makepkg -si` |

**Other x86-64 Linux distributions:** Download `zettui_0.7.1_linux_x86_64.tar.gz`, then:

```bash
tar -xzf zettui_0.7.1_linux_x86_64.tar.gz
mkdir -p ~/.local/bin
install -m755 zettui ~/.local/bin/zettui
```

Make sure `~/.local/bin` is in your `PATH`.

**From source (Rust 1.88+):** In the cloned repository, run `cargo install --path . --locked`.

## Run and configure

```bash
zettui ~/Documents/Notes
zettui ~/Documents/Notes --bibliography ~/Documents/Zotero/references.json --theme auto
```

Alternatively, create `~/.config/zettui/config.toml` (or `$XDG_CONFIG_HOME/zettui/config.toml`):

```toml
notes_dir = "/home/USER/Documents/Notes"
bibliography = "/home/USER/Documents/Zotero/references.json"
theme = "auto" # auto, dark, or light
```

Use a **Better CSL JSON** export from Zotero's Better BibTeX extension, or a compatible BetterBibTeX JSON export containing citation keys. Zettui reads the export; it does not edit it. `auto` follows Omarchy's theme changes while running; elsewhere it detects the terminal's appearance at startup.

## Keybindings

| Browser | Action | Editor | Action |
| --- | --- | --- | --- |
| `j` / `k` | Move selection | `i` / `Esc` | Insert / Normal |
| `Enter` / `l` | Open | `hjkl`, `w`, `b`, `gg`, `G` | Vim navigation |
| `h` | Parent directory | `:w` / `Ctrl+S` | Save |
| `n` / `N` | New note / folder | `:q` / `:q!` | Close / discard changes |
| `r` / `m` / `d` | Rename / move / Trash | `:wq` | Save and close |
| `/` | Find in current folder | `Ctrl+P` | Zotero citations |
| `o` / `b` | Outgoing links / backlinks | `Ctrl+O` / `Ctrl+B` | Outgoing links / backlinks |
| `R` / `q` | Refresh / quit | `Ctrl+T` | Previous linked note |
| `Ctrl+G` | Cycle auto / dark / light | `Ctrl+G` | Cycle theme mode |

**Current limitations:** No full-vault content search or Markdown syntax highlighting yet; renaming/moving a note does not update incoming links. See [ROADMAP.md](ROADMAP.md).

Licensed under [MIT](LICENSE).

# Zettui

A local-first, Vim-style terminal notes manager written in Rust. Browse and edit Markdown and other UTF-8 text files, connect notes with wiki links, insert Zotero citations, use reusable snippets, and search across your notes. Your files stay on disk.

## Installation

Download Linux releases from [GitHub Releases](https://github.com/shubhamdutta26/zettui/releases) (portable x86-64 archive, `.deb` for Debian/Ubuntu, `.rpm` for Fedora, or `PKGBUILD` for Arch/Omarchy), or build from source with a compatible Rust toolchain:

```bash
git clone https://github.com/shubhamdutta26/zettui.git
cd zettui
cargo install --path . --locked
```

Run with `zettui ~/Documents/Notes`, or simply `zettui` when `notes_dir` is configured.

## Quick start

Copy the [example configuration](config.example.toml) to your Zettui configuration directory and edit the paths. The default appearance follows Omarchy or the terminal, falling back to built-in themes.

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/zettui"
cp -n config.example.toml "${XDG_CONFIG_HOME:-$HOME/.config}/zettui/config.toml"
zettui --help
zettui --version
```

Try a disposable copy of the [example vault](examples/notes/):

```bash
demo=$(mktemp -d)
cp -R examples/notes "$demo/notes"
cargo run --locked -- "$demo/notes" --bibliography examples/bibliography/references.json
```

Use `j`/`k` to browse, Enter to open, `i` to edit, Escape for Normal mode, and `:w` to save. `Space z` toggles Zen mode. See the full references below.

## Documentation

- [User guide](docs/USER-GUIDE.md)
- [Configuration](docs/CONFIGURATION.md)
- [Keyboard shortcuts and writing modes](docs/KEYBINDINGS.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development](docs/DEVELOPMENT.md)
- [Releasing](docs/RELEASING.md)
- [Changelog](CHANGELOG.md) · [Roadmap](ROADMAP.md) · [Historical lessons](docs/lessons/)

For installed release packages, the complete documentation is also available in the [GitHub repository](https://github.com/shubhamdutta26/zettui).

MIT License.

# Document export

The optional Pandoc integration does not change Markdown notes, save open buffers,
or require Zotero to be running. It uses the configured Better CSL JSON bibliography
when citation processing is enabled. No new Rust dependencies are required.

## Setup

Install Pandoc (3.x recommended, including support for `--embed-resources`). PDF
also requires a PDF engine, such as Typst (with a compatible Pandoc version) or TeX Live's `pdflatex` or `xelatex`.

```bash
# Arch / Omarchy
sudo pacman -S pandoc-cli
# Debian / Ubuntu / Pop!_OS
sudo apt install pandoc
pandoc --version
```

Check your distribution's available Pandoc version; older builds may not support
all options. PDF engines are installed separately.

## Use

- Open a note and press `Space x` or enter `:export` in Normal mode; alternatively search for
  `export` in the F8 command palette.
- In the Files browser, highlight a note or folder and use F8 → Export document
  or folder. Folder export is recursive, one document per note, with the relative
  directory structure preserved. Hidden entries and symlinks are skipped. It is
  not a combined manuscript.
- Tab/Up/Down changes field; Left/Right changes format or toggles citations;
  Space toggles citations. For PDF, select the engine field and use Left/Right. Edit the destination and CSL path with typing,
  Backspace, or Ctrl+u to clear. Relative paths use the notes root; `~/` works.
- Enter starts export; the modal shows progress, warnings and errors. Esc closes
  when idle. Warnings and failures expand the compact dialog only when the wrapped diagnostics exceed the available status rows. Ctrl+d
  toggles details; PageUp/PageDown scroll the expanded diagnostics (PageDown
  opens collapsed details). The destination is abbreviated with `~` for display;
  an inactive CSL field shows only its filename. Full underlying paths are unchanged. Ctrl+o requests opening the last successful output (or folder) via `xdg-open`. The dialog cannot be closed during export. A conversion is limited to
  120 seconds per document. The note's live buffer is used if open, including
  unsaved edits. Other files in a folder are read from disk.
- Destination parent must exist. The selected output file or folder must not
  exist. A folder destination must be outside the selected source directory.
  Outputs are staged before publication and an existing destination is never
  overwritten. Folder entries with colliding output names are rejected.

## Configuration

```toml
[export]
format = "pdf"
# output_dir = "~/Documents/Exports"
# csl = "~/Zotero/styles/nature.csl"
# resource_dirs = ["~/Documents/Notes/assets"]
# pdf_engine = "auto"
```

The output directory, CSL and resource paths are relative to config.toml when
not absolute. If `output_dir` is omitted, the default is next to the source note
or folder. For citations, the existing `[workspace] bibliography` setting is
used. Enable Citations in the dialog to run `--citeproc`; a blank CSL field uses
Pandoc's default citation style (or note metadata). The exported file may contain
Pandoc warnings, e.g. an unresolved citation; review the dialog after completion.

The image search order starts with the source note directory and its `assets/`, then each ancestor directory and its `assets/` up to the vault root, then configured resource directories. DOCX, EPUB, HTML and PDF incorporate images
supported by their writer. LaTeX produces `.tex` source; it references external
images rather than copying them. Keep source images accessible when compiling it.
Normal Markdown image links are supported; unresolved wiki links are not rewritten.
Quarto code cells are exported as Markdown, not executed. Only export trusted
notes: Pandoc can read linked files or fetch linked resources; PDF engines process
LaTeX. Zettui does not execute shell commands from filenames or dialog inputs.

PDF is the default unless an existing configuration explicitly sets another format.
An omitted engine or `pdf_engine = "auto"` checks PATH in this order: Typst, XeLaTeX, LuaLaTeX, pdfLaTeX, Tectonic, wkhtmltopdf. The dialog lists installed known engines and any explicitly configured engine. Explicit missing engines fail preflight; Auto can be selected instead. This detects executables, not engine packages or Pandoc-version compatibility. Successful conversions with diagnostics are reported separately from failures; diagnostic capture is bounded.


## Chemical equations (v0.8.2)

Use inline `$\ce{H2O}$` or display `$$` math containing `\ce{...}`. The
`:chem` and `:chemblock` commands insert this source syntax. For LaTeX source
and TeX-based PDF export, the filter loads `mhchem`; install that package for
PDF compilation. Typst export uses Whalogen 0.3.0, which may need to be downloaded
on first use. Keep each `\ce{...}` in its own math expression for Typst.
Whalogen does not implement all mhchem syntax. Other output formats report an
unsupported chemistry error when chemical math is present. The filter leaves
ordinary math and literal code examples alone. Notes are never rewritten.

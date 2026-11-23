# Zettui (Lesson 1)

A Rust terminal notes app, built one beginner-friendly lesson at a time.

## What works now

- Open an existing notes directory (does NOT change your files).
- Navigate directories with `j`, `k`, `h`, `l` or the arrow keys.
- Read-only UTF-8 file previews for Markdown, JSON, and other text files.
- Leave the notes root with `q` (does not write any data).

## Install Rust

Use [rustup](https://rustup.rs/) for a recent stable Rust toolchain.
Run `cargo --version` to check installation.

## Run

```bash
cargo run -- examples/notes
```

Or point to your own notes folder:

```bash
cargo run -- /absolute/path/to/your/notes
```

## Build a reusable binary

```bash
cargo build --release
./target/release/zettui examples/notes
```

You can also install it locally (once the project is cloned):

```bash
cargo install --path .
zettui /absolute/path/to/your/notes
```

## Next lessons

1. Modal Vim editing and safe save.
2. File and folder creation/renaming.
3. Wiki links and backlinks.
4. Zotero Better CSL JSON citations.
5. TOML configuration and themes.
6. Linux packaging and releases.

Never commit personal notes or a full Zotero library to a public repository.

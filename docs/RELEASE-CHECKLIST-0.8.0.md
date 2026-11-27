# Zettui v0.8.0 — Release checklist

Run these checks on the exact release candidate. This file is a checklist, not a record of completed tests.

## Automated gates

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --locked
cargo build --release --locked
./target/release/zettui --version
./target/release/zettui --check-config
git diff --check
```

- [ ] All commands succeed and version reports 0.8.0; record actual test results.
- [ ] Verify declared Rust 1.88 support (`cargo +1.88.0 check --locked --all-targets`) or correct the declared minimum if needed.
- [ ] CI passes on the exact main commit, including static Linux package and checksum checks.

## Manual regression checks

Use a disposable vault and test bibliography. Do not modify real notes or overwrite exports.

- [ ] Citation search, marking across queries, reload, preview, and unresolved-citation audit; unsaved changes remain intact.
- [ ] Live Zotero PDF lookup, switch between attachments, open PDFs, retry, and offline fallback.
- [ ] Metadata popup: edit/apply/discard, undo/redo, missing and malformed delimiters, Zen and narrow terminal.
- [ ] Export: `Space x`, `:export`, and F8; `Space e` still toggles Files; Insert-mode Space/x still inserts text.
- [ ] PDF (Typst or selected engine), DOCX (LibreOffice Writer), HTML, EPUB and LaTeX with citations, images, footnotes and unsaved edits.
- [ ] Batch export, output path editing, no-overwrite, missing Pandoc/engine errors, one-line errors without unnecessary resize, scrolling long diagnostics, Ctrl+o opening.
- [ ] Existing editor, search, links, buffers, sessions, themes, navigation and analytics smoke tests.
- [ ] COSMIC terminal rendering/theme switching (previously unverified).
- [ ] Physical multi-computer statistics exchange (previously unverified).

The last two checks may be deferred if their status is disclosed. Do not infer success from simulated-device tests or compilation. See [prior package checks](RELEASE-CHECKLIST-0.7.14.md#package-and-minimum-rust-version-checks).

## Publishing

Review the diff and ensure no private data, test exports or generated build files are included. Commit, rebase, merge, push main and verify CI. Only then create the annotated `v0.8.0` tag, push it and verify the generated Linux release assets and checksums. See [Releasing](RELEASING.md).

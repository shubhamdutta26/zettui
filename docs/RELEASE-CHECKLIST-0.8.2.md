# Zettui v0.8.2 — Release checklist

Run on the exact release candidate. This checklist is not a record of completed tests.

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

- [ ] All commands pass; the binary reports `zettui 0.8.2`.
- [ ] Verify the declared Rust 1.88 minimum (`cargo +1.88.0 check --locked --all-targets`) or record its unverified status.
- [ ] CI passes on the exact main commit, including Linux packaging and checksum checks.

## Manual regression checks

Use a disposable vault and sample statistics. Preserve real notes and state.

- [ ] `:` opens the compact 64 × 4 floating Ex input in Normal mode; Enter runs a command, Esc cancels, Ctrl+U clears, and a narrow terminal falls back safely. Insert-mode `:` remains literal. Test light/dark themes and save/quit guards with multiple dirty buffers.
- [ ] `Ctrl+K` / `:link`, `Alt+I` / `:image`, and F8 entries insert valid Markdown with escaped labels and destinations. Tab/Enter navigate fields; cancellation leaves the note unchanged. Verify Normal/Insert mode preservation, undo/redo, local paths containing spaces, and existing wiki-link completion.
- [ ] `:chem` and `:chemblock` insert portable mhchem math, reject malformed input and leave the note unsaved. Export a note with inline and displayed chemical math to Typst PDF and TeX PDF/LaTeX; inspect actual output, including subscripts, charges and arrows. Check ordinary math and literal fenced code are unchanged. Unsupported chemistry formats must report an error. Record any missing engines/packages or network limitations.
- [ ] `:lorem`, `:lorem 100`, and `:lorem 3p` insert 30 words, 100 words and three paragraphs respectively. Verify invalid and oversized arguments are rejected, and undo/redo and word counts behave correctly.
- [ ] `Alt+C`, `:code`, `:code r`, `:code python` and F8 insert fenced blocks with the cursor inside in Insert mode. Verify surrounding text, syntax highlighting, cancellation, invalid language identifiers, undo/redo and empty/end-of-note cases.
- [ ] Verify earlier v0.8.1 theme/analytics features, existing export and editor commands, sessions and search using the prior regression checklist. Export destination parent must already exist; no overwrite is allowed.
- [ ] COSMIC terminal/theme switching and physical multi-computer statistics exchange (historically unverified); record deferred checks rather than claiming success.

## Publishing

Review the diff for private data and generated files. Commit, rebase onto main,
merge and push. Verify CI before tagging `v0.8.2`. Push the annotated tag, then
verify the Linux Release workflow, published assets and checksums. Delete the
merged feature branch after verification. See [Releasing](RELEASING.md).

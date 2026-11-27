# Zettui v0.8.1 — Release checklist

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

- [ ] All commands pass; the binary reports `zettui 0.8.1`.
- [ ] Verify the declared Rust 1.88 minimum (`cargo +1.88.0 check --locked --all-targets`) or record its unverified status.
- [ ] CI passes on the exact main commit, including Linux packaging and checksum checks.

## Manual regression checks

Use a disposable vault and sample statistics. Preserve real notes and state.

- [ ] H1–H6 colors match between editor and unselected Outline entries in light and dark themes; custom heading overrides and disabled syntax highlighting behave consistently. Selected entries remain readable, and collapse, navigation and heading jumps still work.
- [ ] Month and Year activity bands and legends use the current theme accent; zero, future, character-only and deletion-only days retain their meanings. Verify contrast in light/dark themes and compact layouts.
- [ ] `Ctrl+G` cycles themes in Overview, Month and Year without resetting the selected date or view. Esc returns to the same note/browser; dirty contents and counters are unchanged. Other modified keys remain isolated.
- [ ] Footer remains readable at 30-column and wide layouts. Existing editor, buffer tab layout, search, navigation and Research Toolkit commands pass smoke checks.
- [ ] COSMIC terminal/theme switching and physical multi-computer statistics exchange (historically unverified); record deferred checks rather than claiming success.

## Publishing

Review the diff for private data and generated files. Commit, rebase onto main,
merge and push. Verify CI before tagging `v0.8.1`. Push the annotated tag, then
verify the Linux Release workflow, published assets and checksums. Delete the
merged feature branch after verification. See [Releasing](RELEASING.md).

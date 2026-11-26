# Development

Use a feature branch and commit focused changes. Run from the repository root:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
cargo doc --no-deps --document-private-items
```

`Cargo.lock` is committed; `target/` and `dist/` are generated and ignored. `cargo clean` removes build artifacts, not source. Test interactive changes in a disposable vault, including unsaved changes, search, Unicode, themes and terminal resizing. Rust 1.88 is declared in the manifest; the current CI uses stable Rust and the exact minimum remains to be verified against the lockfile.

Keep comments focused on invariants, edge cases and design decisions. Historical lessons are retained as historical records, not current setup instructions.

For the Intelligent Navigation integration and terminal smoke tests, see [Navigation regression checklist](NAVIGATION-TESTS.md). Run it before tagging v0.7.12.

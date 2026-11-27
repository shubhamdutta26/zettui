# Development

Use a feature branch and commit focused changes. Run from the repository root:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --locked
git diff --check
```

`Cargo.lock` is committed; `target/` and `dist/` are generated and ignored. `cargo clean` removes build artifacts, not source. Test interactive changes in a disposable vault, including unsaved changes, search, Unicode, themes and terminal resizing. Rust 1.88 is declared in the manifest; the current CI uses stable Rust and the exact minimum remains to be verified against the lockfile. CI also tests static Linux packages and their CLI on pull requests, main and manual runs; see the [release checklist](RELEASE-CHECKLIST-0.7.15.md).

For documentation changes, compare CLI flags against `src/cli.rs` and
`src/statistics_cli.rs`, Ex commands and key scopes against `src/commands.rs`,
modal behavior against `src/app/`, and configuration against `src/config.rs`
and the settings types. Update the example configuration and relevant guides
when behavior changes. Record actual test results separately from planned checks.

Keep comments focused on invariants, edge cases and design decisions. Historical lessons are retained as historical records, not current setup instructions.

For the Intelligent Navigation integration and terminal smoke tests, see [Navigation regression checklist](NAVIGATION-TESTS.md). Use it for navigation regressions in subsequent releases as well. See also the [v0.7.13](RELEASE-CHECKLIST-0.7.13.md) and [v0.7.14](RELEASE-CHECKLIST-0.7.14.md) checklists for historical and reusable test procedures. Multi-computer statistics exchange remains manually unverified on separate physical computers.

For UI and Theme Refinements, run the [v0.8.1 release checklist](RELEASE-CHECKLIST-0.8.1.md). The [v0.8.0 Research Toolkit checklist](RELEASE-CHECKLIST-0.8.0.md) remains a historical reference.

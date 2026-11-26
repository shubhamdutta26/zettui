# Zettui v0.7.15 release validation

Codebase Maintenance and Stability. Run these checks on the exact release candidate before tagging.

## Required checks

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

- [ ] Manifest, local lockfile package and executable report 0.7.15.
- [ ] Automated checks pass on release candidate; record actual results.
- [ ] Declared Rust 1.88 minimum verified or corrected based on actual results (`cargo +1.88.0 check --locked --all-targets`).
- [ ] CI on the exact release commit passes, including musl build, tarball, Debian, RPM and checksum verification. See [packaging procedures](RELEASE-CHECKLIST-0.7.14.md#package-and-minimum-rust-version-checks).
- [ ] Release notes retain outstanding validation caveats; no private state or test files committed.
- [ ] Annotated tag matches manifest; only push after the required checks pass.
- [ ] After release, verify workflow and published asset checksums.

## Manual regression status

Maintainer reports the local manual regression suite passed before the version
bump, except tests on COSMIC and multiple physical computers. This is not a
record of a fresh v0.7.15 build or CI run.

- [ ] COSMIC rendering, theme switching and terminal smoke test — pending.
- [ ] [Physical multi-computer statistics exchange](RELEASE-CHECKLIST-0.7.14.md#physical-multi-computer-exchange--pending) — pending.

These two checks may be deferred if their status is disclosed in the README and
release notes. Known inherited Vim sequence conflicts remain documented in
[Vim commands](VIM-COMMANDS.md). Re-run affected manual checks if functional
code changes after the prior regression pass.

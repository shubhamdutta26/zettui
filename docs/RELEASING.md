# Releasing

Update Cargo.toml, refresh Cargo.lock with `cargo check`, and update the changelog and roadmap. Run the checks in [Development](DEVELOPMENT.md). Commit, rebase onto main, merge, and push. Create an annotated `vX.Y.Z` tag only after verifying the version and push it. The tag triggers the Linux release workflow; it verifies the manifest version and generates the portable x86-64 archive, Debian and RPM packages, Arch PKGBUILD and checksums. Verify workflow success and download/check assets before announcing the release.

Local packaging requires a static musl binary, binutils and optional dpkg-deb/rpmbuild. Run `bash scripts/package-linux.sh target/x86_64-unknown-linux-musl/release/zettui X.Y.Z`. Outputs are in `dist/`; verify with `(cd dist && sha256sum -c SHA256SUMS)`. The config example is installed as documentation, never as a user's live configuration.

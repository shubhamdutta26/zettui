# Lesson 7 — Linux install and automated GitHub releases

The files in this kit **add** packaging and CI to your existing v0.6.1 repository. They do not replace any Rust source file. Keep your working Omarchy-aware theme behavior.

## A. Install safely into your existing project

1. Commit your tested Lesson 6 changes before starting:

   ```bash
   cd ~/Projects/zettui
   git status
   # If Lesson 6 is working and tests pass, commit it.
   git add Cargo.toml Cargo.lock src/ README.md ROADMAP.md LESSON-6.md
   git commit -m "Finish Lesson 6 auto light/dark themes"
   git switch -c feature/linux-releases
   ```

2. Unzip the Lesson 7 kit and copy only the new files:

   ```bash
   unzip ~/Downloads/zettui-lesson-7-kit.zip -d /tmp
   mkdir -p .github/workflows scripts
   cp /tmp/zettui-lesson-7-kit/.github/workflows/*.yml .github/workflows/
   cp /tmp/zettui-lesson-7-kit/scripts/package-linux.sh scripts/
   cp /tmp/zettui-lesson-7-kit/LESSON-7.md .
   ```

3. Open `Cargo.toml` and change only the `[package]` version to `0.7.0`. Don't replace the dependency list or theme source code. Add `/dist/` to `.gitignore` (the release output should not be committed). Ensure `Cargo.lock` is present and committed:

   ```bash
   test -f Cargo.lock || cargo generate-lockfile
   cargo fmt --all -- --check
   cargo test
   cargo build --release
   ./target/release/zettui --help
   ```

4. Commit and push to your existing GitHub remote:

   ```bash
   printf "\n/dist/\n" >> .gitignore  # Do this just once if /dist/ is not already listed
   git add .github/workflows scripts/package-linux.sh LESSON-7.md Cargo.toml Cargo.lock .gitignore
   git commit -m "Add Linux CI and release packaging"
   git switch main
   git merge --ff-only feature/linux-releases
   git branch -d feature/linux-releases
   git push origin main
   ```

   In the GitHub **Actions** tab, verify the `Rust CI` workflow succeeded on `main` before publishing.

## B. Publish your first release

The release workflow builds the x86-64 Linux executable with the musl target and packages it into a `.tar.gz`, `.deb`, `.rpm`, and checksum-verified Arch `PKGBUILD`. It triggers only when you push a version tag matching `Cargo.toml`.

```bash
git switch main
git status
git tag -a v0.7.0 -m "Zettui v0.7.0 Linux release"
git push origin v0.7.0
```

Go to GitHub → Actions → **Linux Release**. If that workflow passes, GitHub → Releases will contain your files. This process creates assets only after the workflow completes; pushing the tag does not by itself guarantee successful builds.

### Expected release assets

- `zettui_0.7.0_linux_x86_64.tar.gz` — portable executable, license, README
- `zettui_0.7.0_linux_x86_64.deb` — Debian / Ubuntu package
- `zettui_0.7.0_linux_x86_64.rpm` — Fedora / RPM package
- `PKGBUILD` — Arch / Omarchy package recipe (build with `makepkg -si`)
- `SHA256SUMS` — integrity checks for these files

Only x86-64 Linux is targeted in Lesson 7; ARM64, official distribution repositories, and automatic system updates are future work. A `.deb` file is not an APT repository, and an `.rpm` file is not a DNF repository.

## C. Test the installation on each distribution

Download assets from **your own GitHub release page**. To check their integrity, keep them together in one directory and run:

```bash
sha256sum -c SHA256SUMS
```

### Arch Linux / Omarchy

Download `PKGBUILD`, the `.tar.gz` archive, and (optionally) `SHA256SUMS` from the same release. In the download directory:

```bash
less PKGBUILD    # inspect any third-party PKGBUILD before executing
makepkg -si
zettui --help
zettui ~/Documents/Notes
```

The native Arch package will be built from the already-compiled portable binary. This is **not** an AUR publication.

### Debian / Ubuntu

```bash
sudo apt install ./zettui_0.7.0_linux_x86_64.deb
zettui --help
```

### Fedora

```bash
sudo dnf install ./zettui_0.7.0_linux_x86_64.rpm
zettui --help
```

### Any x86-64 Linux (no root required)

```bash
tar -xzf zettui_0.7.0_linux_x86_64.tar.gz
mkdir -p ~/.local/bin
install -m755 zettui ~/.local/bin/zettui
~/.local/bin/zettui --help
```

Add `~/.local/bin` to your PATH if it isn't there already. Existing notes and `~/.config/zettui/config.toml` are not part of the release, and packages do not replace them.

## D. What are the new files?

- `.github/workflows/ci.yml`: runs format checking, unit tests, and a normal release-mode build on GitHub when you push to `main` or open a PR.
- `.github/workflows/release.yml`: uses a pushed `v*` tag to compile a static musl executable, package it, and publish a GitHub Release.
- `scripts/package-linux.sh`: converts that one executable into a generic archive, native Debian/RPM packages, an Arch `PKGBUILD`, and SHA256 checksums.

**Do not upload private notes, Zotero exports, or your local `config.toml`.** GitHub is hosting the application, not your knowledge vault.

### Troubleshooting

- **“Tag ... does not match Cargo package version”**: edit Cargo.toml so it matches `v0.7.0`, and publish a *new* release tag once corrected (avoid force-moving public tags).
- **“Cargo.lock must be committed”**: run `cargo generate-lockfile`, then commit `Cargo.lock` on main before tagging.
- **Action failed with compilation error**: fix it on a feature branch and test before tagging again.
- **No release files**: inspect the failing step in GitHub Actions; do not assume success just because you pushed a tag.
- **Executable works locally but not on another CPU**: this build is for x86-64 Linux only.

## E. A recommended follow-up

Once the basic release works, we can add ARM64 builds, an AUR submission, and a `zettui --version` flag. Afterwards, we can improve global vault search and Markdown syntax highlighting.

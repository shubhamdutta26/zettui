#!/usr/bin/env bash
# Package an already-compiled static Linux x86_64 Zettui binary.
# Usage: bash scripts/package-linux.sh /path/to/zettui 0.7.0
set -euo pipefail

usage() { echo "Usage: $0 /path/to/static/zettui 0.7.0" >&2; exit 2; }
[[ $# -eq 2 ]] || usage
binary="$1"
version="${2#v}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage
[[ -f "$binary" && -x "$binary" ]] || { echo "Executable not found: $binary" >&2; exit 1; }
[[ "$(uname -m)" == "x86_64" ]] || { echo 'This recipe currently supports x86_64 only' >&2; exit 1; }
command -v readelf >/dev/null || { echo 'Install binutils (readelf) first' >&2; exit 1; }
if ! header="$(LC_ALL=C readelf -h "$binary" 2>/dev/null)"; then
  echo "Not a Linux ELF binary: $binary" >&2; exit 1
fi
if ! grep -Eq 'Class:[[:space:]]+ELF64' <<< "$header" ||
   ! grep -Eq 'Machine:[[:space:]]+Advanced Micro Devices X86-64' <<< "$header"; then
  echo 'Binary must be an x86-64 ELF executable.' >&2; exit 1
fi
program_headers="$(LC_ALL=C readelf -l "$binary")"
dynamic_section="$(LC_ALL=C readelf -d "$binary")"
if grep -q INTERP <<< "$program_headers" || grep -q '(NEEDED)' <<< "$dynamic_section"; then
  echo 'Binary is dynamically linked. Build with --target x86_64-unknown-linux-musl first.' >&2
  exit 1
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ -f "$root/LICENSE" && -f "$root/README.md" && -f "$root/config.example.toml" ]] || {
  echo 'Run this script inside the Zettui repository (LICENSE, README.md and config.example.toml required)' >&2; exit 1;
}
mkdir -p "$root/dist"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
name="zettui_${version}_linux_x86_64"
archive="${name}.tar.gz"
# Clear only the outputs for this version; preserve other local builds.
rm -f -- "$root/dist/$archive" "$root/dist/${name}.deb" "$root/dist/${name}.rpm" "$root/dist/PKGBUILD" "$root/dist/SHA256SUMS"

# General-purpose binary archive (for any x86-64 Linux distribution).
mkdir -p "$work/archive"
install -m755 "$binary" "$work/archive/zettui"
install -m644 "$root/LICENSE" "$work/archive/LICENSE"
install -m644 "$root/README.md" "$work/archive/README.md"
install -m644 "$root/config.example.toml" "$work/archive/config.example.toml"
tar -czf "$root/dist/$archive" -C "$work/archive" zettui LICENSE README.md config.example.toml

# Debian/Ubuntu .deb (if dpkg-deb is installed).
if command -v dpkg-deb >/dev/null; then
  mkdir -p "$work/deb/DEBIAN" "$work/deb/usr/bin" "$work/deb/usr/share/doc/zettui"
  install -m755 "$binary" "$work/deb/usr/bin/zettui"
  install -m644 "$root/LICENSE" "$work/deb/usr/share/doc/zettui/copyright"
  install -m644 "$root/README.md" "$work/deb/usr/share/doc/zettui/README.md"
  install -m644 "$root/config.example.toml" "$work/deb/usr/share/doc/zettui/config.example.toml"
  cat > "$work/deb/DEBIAN/control" <<CONTROL
Package: zettui
Version: $version
Section: editors
Priority: optional
Architecture: amd64
Maintainer: Zettui maintainers
Description: Local-first Vim-style terminal notes manager
 Zettui manages Markdown and other text notes in a Linux terminal.
CONTROL
  dpkg-deb --root-owner-group --build "$work/deb" "$root/dist/${name}.deb"
else
  echo 'dpkg-deb unavailable; skipping Debian package' >&2
fi

# Fedora .rpm (if rpmbuild is installed).
if command -v rpmbuild >/dev/null; then
  rpmroot="$work/rpmbuild"
  mkdir -p "$rpmroot"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
  install -m755 "$binary" "$rpmroot/SOURCES/zettui"
  install -m644 "$root/LICENSE" "$rpmroot/SOURCES/LICENSE"
  install -m644 "$root/README.md" "$rpmroot/SOURCES/README.md"
  install -m644 "$root/config.example.toml" "$rpmroot/SOURCES/config.example.toml"
  cat > "$rpmroot/SPECS/zettui.spec" <<SPEC
%global debug_package %{nil}
Name: zettui
Version: ${version}
Release: 1%{?dist}
Summary: Local-first Vim-style terminal notes manager
License: MIT
BuildArch: x86_64

%description
Zettui is a terminal application for editing and connecting text notes.

%install
rm -rf %{buildroot}
install -Dm0755 %{_sourcedir}/zettui %{buildroot}%{_bindir}/zettui
install -Dm0644 %{_sourcedir}/LICENSE %{buildroot}%{_datadir}/licenses/zettui/LICENSE
install -Dm0644 %{_sourcedir}/README.md %{buildroot}%{_datadir}/doc/zettui/README.md

install -Dm0644 %{_sourcedir}/config.example.toml %{buildroot}%{_datadir}/doc/zettui/config.example.toml

%files
%{_bindir}/zettui
%license %{_datadir}/licenses/zettui/LICENSE
%doc %{_datadir}/doc/zettui/README.md
%doc %{_datadir}/doc/zettui/config.example.toml
SPEC
  rpmbuild -bb --define "_topdir $rpmroot" "$rpmroot/SPECS/zettui.spec"
  rpm_file="$(find "$rpmroot/RPMS" -type f -name 'zettui-*.rpm' -print -quit)"
  [[ -n "$rpm_file" ]] || { echo 'rpmbuild did not produce an RPM' >&2; exit 1; }
  cp "$rpm_file" "$root/dist/${name}.rpm"
else
  echo 'rpmbuild unavailable; skipping RPM package' >&2
fi

# Arch/Omarchy can create a native .pkg.tar.zst from this checksum-verified local recipe.
archive_sha="$(sha256sum "$root/dist/$archive" | cut -d' ' -f1)"
cat > "$root/dist/PKGBUILD" <<PKGBUILD
# Copy this file and the matching .tar.gz into the same directory, then run: makepkg -si
pkgname=zettui-bin
pkgver=${version}
pkgrel=1
pkgdesc='Local-first Vim-style terminal notes manager'
arch=('x86_64')
license=('MIT')
source=("zettui_\${pkgver}_linux_x86_64.tar.gz")
sha256sums=('${archive_sha}')

package() {
  install -Dm755 "\${srcdir}/zettui" "\${pkgdir}/usr/bin/zettui"
  install -Dm644 "\${srcdir}/LICENSE" "\${pkgdir}/usr/share/licenses/\${pkgname}/LICENSE"
  install -Dm644 "\${srcdir}/README.md" "\${pkgdir}/usr/share/doc/\${pkgname}/README.md"
  install -Dm644 "\${srcdir}/config.example.toml" "\${pkgdir}/usr/share/doc/\${pkgname}/config.example.toml"
}
PKGBUILD

# Generate hashes only for files we actually created for this version.
(
  cd "$root/dist"
  files=("$archive" "PKGBUILD")
  [[ -f "${name}.deb" ]] && files+=("${name}.deb")
  [[ -f "${name}.rpm" ]] && files+=("${name}.rpm")
  sha256sum "${files[@]}" > SHA256SUMS
)
echo "Packages are in $root/dist/"

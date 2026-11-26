#!/usr/bin/env bash
# Verify locally generated release packages without installing them.
set -euo pipefail
usage() { echo "Usage: $0 VERSION [--available]" >&2; exit 2; }
[[ $# -eq 1 || $# -eq 2 ]] || usage
version="${1#v}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage
[[ $# -eq 1 || "$2" == --available ]] || usage
available="${2:-}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist="$root/dist"
name="zettui_${version}_linux_x86_64"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

for asset in "$name.tar.gz" PKGBUILD SHA256SUMS; do
  test -s "$dist/$asset"
done
for suffix in deb rpm; do
  if [[ -z "$available" || -e "$dist/$name.$suffix" ]]; then
    test -s "$dist/$name.$suffix"
  fi
done
# Ensure each expected asset is covered by the checksum manifest.
assets=("$name.tar.gz" PKGBUILD)
for suffix in deb rpm; do
  [[ ! -f "$dist/$name.$suffix" ]] || assets+=("$name.$suffix")
done
for asset in "${assets[@]}"; do
  grep -Fqx "$(sha256sum "$dist/$asset" | cut -d' ' -f1)  $asset" "$dist/SHA256SUMS"
done
(cd "$dist" && sha256sum --strict -c SHA256SUMS)
bash -n "$dist/PKGBUILD"

audit_binary() {
  local binary="$1" label="$2" example="$3" home="$work/home-$2"
  local program_headers dynamic_section
  test -x "$binary"
  program_headers="$(LC_ALL=C readelf -l "$binary")"
  dynamic_section="$(LC_ALL=C readelf -d "$binary")"
  if grep -q INTERP <<< "$program_headers" || grep -q '(NEEDED)' <<< "$dynamic_section"; then
    echo "Unexpected dynamic linkage in $label" >&2; exit 1
  fi
  mkdir -p "$home/config/zettui"
  cp "$example" "$home/config/zettui/config.toml"
  (
    export HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_STATE_HOME="$home/state"
    cd "$home"
    [[ "$("$binary" --version)" == "zettui $version" ]]
    help="$("$binary" --help)"
    grep -Fq -- '--check-config' <<< "$help"
    grep -Fq -- '--stats-import-zettlr' <<< "$help"
    "$binary" --check-config
    test ! -e "$XDG_STATE_HOME"
  )
  echo "Verified $label executable and example configuration"
}

mkdir -p "$work/tar"
tar -xzf "$dist/$name.tar.gz" -C "$work/tar"
test -s "$work/tar/LICENSE"
test -s "$work/tar/README.md"
audit_binary "$work/tar/zettui" tar "$work/tar/config.example.toml"

if [[ -f "$dist/$name.deb" ]]; then
  [[ "$(dpkg-deb -f "$dist/$name.deb" Package)" == zettui ]]
  [[ "$(dpkg-deb -f "$dist/$name.deb" Version)" == "$version" ]]
  [[ "$(dpkg-deb -f "$dist/$name.deb" Architecture)" == amd64 ]]
  dpkg-deb -x "$dist/$name.deb" "$work/deb"
  test -s "$work/deb/usr/share/doc/zettui/copyright"
  audit_binary "$work/deb/usr/bin/zettui" deb "$work/deb/usr/share/doc/zettui/config.example.toml"
else
  echo 'Debian package unavailable; not tested.'
fi

if [[ -f "$dist/$name.rpm" ]]; then
  [[ "$(rpm -qp --qf '%{NAME} %{VERSION} %{ARCH}' "$dist/$name.rpm")" == "zettui $version x86_64" ]]
  mkdir -p "$work/rpm"
  (cd "$work/rpm" && rpm2cpio "$dist/$name.rpm" | cpio -id --quiet --no-absolute-filenames)
  test -s "$work/rpm/usr/share/licenses/zettui/LICENSE"
  audit_binary "$work/rpm/usr/bin/zettui" rpm "$work/rpm/usr/share/doc/zettui/config.example.toml"
else
  echo 'RPM package unavailable; not tested.'
fi

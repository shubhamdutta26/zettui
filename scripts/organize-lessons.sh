#!/usr/bin/env bash
# Move old LESSON*.md files out of the repository root without discarding them.
# Run from the root of the Zettui Git repository.
set -euo pipefail

if [[ ! -f Cargo.toml || ! -d src ]] || ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo 'Run this script from the Zettui repository root.' >&2
  exit 1
fi

mkdir -p docs/lessons
moved=0
for lesson in LESSON*.md; do
  [[ -f "$lesson" ]] || continue
  target="docs/lessons/$lesson"
  if [[ -e "$target" ]]; then
    echo "Refusing to overwrite $target" >&2
    exit 1
  fi
  if git ls-files --error-unmatch -- "$lesson" >/dev/null 2>&1; then
    git mv -- "$lesson" "$target"
  else
    mv -- "$lesson" "$target"
  fi
  echo "Moved $lesson -> $target"
  moved=$((moved + 1))
done

echo "Finished: moved $moved lesson files. Other project files were left untouched."

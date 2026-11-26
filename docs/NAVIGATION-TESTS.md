# Intelligent Navigation regression checklist

Run from the repository root, before tagging 0.7.12. The fixtures below are disposable; never test Trash or external-change handling on real notes.

## Automated checks

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
cargo doc --no-deps --document-private-items
```

## Disposable vault

```bash
demo=$(mktemp -d)
mkdir -p "$demo/Folder"
cat > "$demo/Navigation.md" <<'NOTE'
---
title: Navigation regression
tags: [navigation]
# ignored YAML comment
---
# Introduction
## Methods
### Expression
Body text with α and β.
```{r}
# ignored code heading
print(1)
```
<!--
# ignored HTML heading
-->
$$
# ignored display math
$$
## Results
### SEC
### SEC
# Conclusion
NOTE
printf '# Linked note\n' > "$demo/Folder/Linked.md"
printf 'Plain text\n' > "$demo/plain.txt"
cargo run --locked -- "$demo"
```

## Manual checks

| Context / input | Expected |
| --- | --- |
| Browser `l` on a note, then Enter | `l` does not open the note; Enter opens it |
| Browser F3; editor F2/F4 | Preview, Files and Outline preferences act independently |
| Alt+Left/Right, Alt+Up/Down | Resize Files and Outline; widths survive toggling |
| F6, j/k, h/l | Outline focus, selection and collapse; no editor movement or modification |
| F6, End, Enter | Jump to Conclusion; return to editor |
| F6, j/k, Esc | Cancel navigation without moving cursor |
| F7, `sec` | Two SEC headings distinguished by line number |
| F7, `mthds`, Enter | Fuzzy-match Methods and jump |
| F7, `ignored` | No heading matches from YAML, code, comments or math |
| Insert an unsaved heading; Esc, F7 | New heading is immediately searchable |
| Collapse a branch; F7 | Collapsed descendant remains searchable |
| F4, F7; Space z, F7 | Search works with hidden Outline and in Zen |
| F8, `search document headings`, Enter | Palette closes before heading picker opens |
| F8, `xyz`, Enter, Esc | No action; Escape cancels |
| `:headings` before Enter, F2/F8 | Ex input remains intact; no panel or picker changes |
| Space then Alt+Right | Leader is canceled; next key is ordinary editor input |
| F8 in Insert/Visual | Palette does not open; editor mode remains intact |
| Ctrl+P | Zotero picker if configured, otherwise its existing notice |
| Space c / :commands | Palette opens in Normal mode |
| F6 then F8 then Esc | Outline focus is restored |
| F6 then shrink terminal | Outline focus released when hidden; preferences restored on widening |
| Space z twice | Restore previous sidebar preferences |
| Ctrl+G; desktop theme changes | Outline, editor and pickers update with theme |
| Modified note, palette Close note | Refused until saved or explicitly discarded with :q! |
| Browser F8, `delete`, Enter, Esc | Trash confirmation appears; file remains |
| Plain text file, F7/F6 | Markdown heading navigation is unavailable |
| Browser F8, `quit zettui`, Enter | Application exits normally |

Repeat terminal-sensitive checks in Foot and COSMIC Terminal (or other terminals you support). Ctrl+Shift+P is deliberately not a palette binding; a terminal may still encode it as Ctrl+P. Confirm existing search, snippets, citations, link navigation, Vim editing and save/close behavior in a disposable copy of the example vault.

For external-change safety, open Navigation.md, edit without saving, change the file from a second terminal, then select **Save and close note** in the palette. The external file must remain unchanged and the editor must retain its unsaved buffer. Use `:q!` only after verifying the fixture and preserving any edits you want.

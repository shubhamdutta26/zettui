# Zettui: phased roadmap

- [x] Lesson 1: Read-only folder browser and text preview (`0.1.0` starter)
- [ ] Lesson 2: Vim-inspired editor integration (EdTUI), Save and unsaved-changes warning
- [ ] Lesson 3: Create, rename, move folders and files safely
- [ ] Lesson 4: Markdown `[[wiki links]]`, note lookup, and backlinks
- [ ] Lesson 5: Zotero Better CSL JSON import, citation search, insert `[@key]`
- [ ] Lesson 6: TOML config, user-configurable root / bibliography / color themes
- [ ] Lesson 7: Search across files and indexing (without changing notes)
- [ ] Lesson 8: Unit/integration tests, GitHub CI, Linux release binaries, .deb, .rpm, AUR/PKGBUILD

## Proposed config (not implemented yet)

```toml
notes_dir = "/home/USER/Documents/Zettlr"
bibliography = "/home/USER/Documents/Zotero/references.json"
theme = "dark"
```

## Design principles

1. Local-first: never store notes in a proprietary format.
2. Preserve original Markdown and links; do not rewrite existing notes automatically.
3. Treat Zotero export as read-only; Zotero remains the source of truth.
4. Start with explicit save, safe file writes, and protection against overwrites.
5. Keep portable CLI build working on Linux.

# Zettui roadmap

Development milestones are listed below; older milestone numbers do not necessarily correspond to published GitHub release tags.

## Completed

| Version | Milestone |
| --- | --- |
| **0.1.0** | Read-only terminal file browser and text previews |
| **0.2.0** | Vim-style editor, save/close commands, unsaved-change warnings, external-change protection |
| **0.3.0** | Create, rename, move, and Trash files/directories safely |
| **0.4.0** | Wiki links, Markdown links, backlinks, Zettelkasten ID resolution, navigation history |
| **0.5.0** | Zotero JSON import, citation search, `[@citekey]` insertion, config file support |
| **0.6.0** | Color themes, custom overrides, and current-folder filename search |
| **0.6.1** | Automatic Omarchy light/dark detection and terminal-background matching |
| **0.7.0** | GitHub CI/release workflows and x86-64 Linux packaging (tar.gz, DEB, RPM, Arch recipe) |
| **0.7.1** | Streamlined README, versioned roadmap, archived lessons, automatic version label in TUI |

## Planned (tentative order)

| Version | Planned work |
| --- | --- |
| **0.8.x** | Full-vault text search, filename search across subdirectories, useful result previews |
| **0.9.x** | Link-aware rename/move, broken-link reporting, stronger link resolution |
| **0.10.x** | Markdown/YAML syntax highlighting, improved status/help interface |
| **0.11.x** | Note templates; automatic Zettelkasten IDs and YAML front matter |
| **Later** | Custom keybindings; citation previews; ARM64 Linux releases; broader package distribution |

## Project principles

- Keep notes as ordinary files; never require a proprietary database or cloud service.
- Preserve user metadata and never silently overwrite unsaved or externally modified content.
- Build new features with tests and retain a useful keyboard-first workflow.

Previous development tutorials live in [`docs/lessons/`](docs/lessons/) after repository cleanup.

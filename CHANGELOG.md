# Changelog

## 0.8.1 — UI and Theme Refinements

### Improved

- Outline heading text uses the Markdown editor’s H1–H6 colors, including custom heading overrides and the syntax-highlighting toggle. The selected entry retains its selection style.
- Month and Year writing-activity heatmaps, legends and compact calendars follow the active theme accent instead of a fixed success-color tint. Activity bands, counts and special markers are unchanged.
- `Ctrl+G` cycles themes directly in Writing Analytics without resetting the active view or selected date; the dashboard footer advertises the shortcut.
- Regression coverage for heading colors, activity palettes, dashboard theme cycling and footer width.

### Compatibility and validation

- No dependency, configuration, note, statistics or session format changes. The buffer tab layout is unchanged.
- See the [v0.8.1 release checklist](docs/RELEASE-CHECKLIST-0.8.1.md) for validation requirements.

## 0.8.0 — Research Toolkit

### Added

- Ranked Zotero citation search, multi-citation marking/insertion and in-picker bibliography refresh.
- Citation previews, unresolved bracketed-citation audit, and local PDF attachment lookup through Better BibTeX (personal library; optional curl and running Zotero for live lookup).
- Standalone Vim-style YAML metadata editor with independent undo/redo, apply/discard protection and adjustable dimensions. Applying updates the live note; saving to disk is separate.
- Optional Pandoc export to PDF, DOCX, standalone HTML, EPUB and LaTeX, including live buffers, batch folder export, citations and local images. PDF is the default; installed engines can be detected or selected explicitly.
- Compact export dialog with adaptive diagnostics, no-overwrite protection and direct output opening. `Space x` opens export in Normal mode; `:export` and the F8 palette remain available. `Space e` still toggles Files.

### Compatibility and validation

- No new Rust dependencies; Pandoc and a compatible PDF engine are optional external dependencies. Live attachment lookup requires curl. Export does not execute Quarto code cells; LaTeX source references external images.
- Metadata validation checks delimiters, not full YAML semantics. Citation audit does not cover all Pandoc citation syntax.
- Final release checks, exact minimum Rust version, COSMIC and physical multi-computer statistics exchange must be recorded according to the [release checklist](docs/RELEASE-CHECKLIST-0.8.0.md).

## 0.7.15 — Codebase Maintenance and Stability

### Added

- Read-only `--check-config` command.
- Dedicated CLI, keyboard and Ex command references.
- Regression tests for editing, search, keyboard routing, configuration and statistics CLI failures.
- Pre-tag Linux packaging and extracted-binary verification in CI.

### Improved

- Reduce redundant allocations in editing and search; retain existing dependencies.
- Separate popup rendering, YAML and fenced-code highlighting; share input handling across five search pickers.
- Prevent modified browser keys from triggering plain-key file actions or quitting.
- Parse help/version before configuration loading, reject missing flag values, report configuration read errors, and support `--` for literal workspace paths.
- Synchronize documentation and record known Vim routing conflicts and validation requirements.
- Verify extracted Linux package binaries and checksums in CI before tagging and again before publishing; add a CLI regression for failed statistics imports and exports.

### Validation and compatibility

- Existing dependencies, configuration defaults and statistics data format retained.
- Maintainer reports successful local manual regression testing before the version bump, except COSMIC and physical multi-computer statistics exchange, which remain manually unverified.
- Some inherited Vim multi-key sequences conflict with application-reserved keys. See [Vim commands](docs/VIM-COMMANDS.md).
- See the [v0.7.15 release checklist](docs/RELEASE-CHECKLIST-0.7.15.md) for pending release-candidate checks.

## 0.7.14 — Writing Analytics & Goals

### Added

- Local-first daily word and character addition/deletion tracking, with automatic JSON persistence.
- Daily, weekly, monthly, yearly and lifetime writing statistics, averages and streaks.
- Configurable daily and weekly word-count goals (`[writing_goals]`).
- Interactive analytics dashboard (`F10`, `:analytics`, `:stats`), monthly writing calendar and yearly contribution heatmap.
- Theme-aware, responsive analytics panels, goal progress bars and weekly activity chart.
- Multi-device statistics snapshot export and duplicate-safe import (`--stats-export FILE`, `--stats-import FILE`, `--stats-device-id`).
- Historical Zettlr word and character count migration (`--stats-import-zettlr FILE SOURCE`).

### Improved

- Writing analytics documentation, CLI guidance and integration tests.
- Statistics remain independent of note workspaces and retain device identities during snapshot exchange.

### Compatibility and known limitations

- No new dependencies; existing note files and configurations remain supported. Goals are disabled by default.
- **Multi-computer statistics exchange has not been tested by the maintainer on multiple physical computers.** Integration tests simulate independent state directories; they do not establish real-world multi-computer compatibility. Keep backups and exchange exported snapshots rather than synchronizing the live statistics file.
- Zettlr migration preserves word and character counts; historical deletions and Pomodoro data are not imported. Use a stable source identifier for each independent legacy history.
- Snapshot exchange is manual; automatic cross-device synchronization is not included.

## 0.7.13 — Buffers

### Added

- Multiple open note buffers with independent editing states.
- Theme-aware buffer tabs with modified-note indicators.
- Fuzzy buffer picker and Vim-style buffer navigation.
- Independent cursor, viewport and undo/redo; the last in-note search query and direction are shared across buffers.
- Workspace-specific session persistence.
- Automatic restoration of open buffers and active notes.
- Restoration of panel visibility, dimensions and browser location.
- Hidden-folder visibility toggle using F9.
- Configurable buffer, session and browser preferences.
- Application-wide save and quit commands.
- Buffer and session commands in the command palette.

### Improved

- Protection against unsaved changes across multiple buffers.
- Protection against external file modifications.
- File operation safety for open notes.
- Compact buffer tab styling and alignment.
- Integration tests and documentation.

### Compatibility

- Existing configuration files remain supported.
- No new dependencies.
- Session metadata is stored outside the notes directory.

## 0.7.12 — Intelligent Navigation

- Centralize application command dispatch across keyboard bindings, Ex commands and the new palette.
- Add a shared H1–H6 ATX heading parser and section hierarchy for Outline, heading search and Focus mode.
- Add independently configurable Files, Preview and Outline visibility and widths, with temporary Zen and narrow-terminal layouts.
- Add interactive Outline selection, collapse/expand, cursor tracking and heading jumps.
- Add fuzzy heading search over the live buffer, including collapsed and unsaved headings.
- Add a context-aware fuzzy command palette (`F8`, `Space c`, `:commands`), preserving Zotero's `Ctrl+P` binding and existing file safety checks.
- Keep the Ex command bar isolated from global shortcuts; add event-routing and external-change regression tests.

## 0.7.11

- Expand Markdown source highlighting with nested emphasis, escaped delimiters, balanced link destinations, autolinks, citations, mathematics and improved table detection.
- Add context-aware YAML frontmatter and fenced YAML highlighting, including typed scalars, nested collections, comments, anchors, aliases and multiline strings.
- Expand language-specific fenced-code highlighting, numeric literals, functions, operators and selected multiline strings and comments; recognize Quarto-style fence identifiers.
- Add optional Markdown, YAML and code color overrides while retaining existing heading overrides and theme inheritance.
- Avoid duplicate automatic heading colors and separate inline-code coloring from bold text.
- Add regression tests for syntax, Unicode offsets, source preservation and configuration overrides.

## 0.7.10

- Native COSMIC v2, Omarchy and terminal theming, contrast correction and system heading colors.
- Repository housekeeping, CLI module extraction, documentation and stricter CI.
- Split application coordination, browser, appearance, dialogs, editing, events and rendering into focused modules.
- Sectioned configuration required; legacy flat keys now produce an error.
- Cache Markdown lexical styling, word counts and unchanged system palette snapshots.
- Configuration template included in release packages.

## 0.7.9

- Markdown syntax highlighting and custom heading colors.
- Zen, Focus and Typewriter modes.
- Structured TOML configuration and active panel borders.
- Version command.

Earlier milestones: [Roadmap](ROADMAP.md).

# Changelog

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

//! Lesson 4: a read-only note link index and link resolver.
//!
//! We deliberately do not rewrite Zettlr YAML metadata, note IDs or existing
//! links. Ambiguous links are shown but never opened arbitrarily.

use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

const MAX_FILES: usize = 12_000;
const MAX_NOTE_BYTES: u64 = 256_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkItem {
    pub label: String,
    pub target: Option<PathBuf>,
}

/// A safe-to-insert link to an already existing text note.
/// `link` is the text between `[[` and `]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSuggestion {
    pub label: String,
    pub link: String,
}

#[derive(Debug, Clone)]
struct Note {
    path: PathBuf,
    stem: String,
    title: Option<String>,
    id: Option<String>,
    // Path relative to the notes root (always with / separators).
    relative: String,
}

pub struct Index {
    root: PathBuf,
    notes: Vec<Note>,
}

#[derive(Clone, Copy)]
enum LinkKind {
    Wiki,
    Markdown,
}

#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    Found(PathBuf),
    Missing,
    Ambiguous,
}

#[derive(Debug)]
struct RawLink {
    text: String,
    kind: LinkKind,
}

// A manually implemented Debug keeps RawLink readable in tests and diagnostics.
impl std::fmt::Debug for LinkKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wiki => f.write_str("Wiki"),
            Self::Markdown => f.write_str("Markdown"),
        }
    }
}

fn supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "txt" | "json" | "yaml" | "yml" | "toml"
            )
        })
}

fn source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown" | "txt"))
}

/// Read a simple one-line YAML scalar; no YAML reserialization is performed.
/// Complex YAML scalars are not supported until a later metadata lesson.
fn frontmatter_field(text: &str, field: &str) -> Option<String> {
    let mut lines = text.trim_start_matches('\u{feff}').lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim() != field {
            continue;
        }
        let value = value.trim();
        if value.is_empty() || value == "|" || value == ">" {
            return None;
        }
        return Some(value.trim_matches(|c| c == '\'' || c == '"').to_owned());
    }
    None
}

/// Walk a notes tree without following directory or file symlinks.
fn visit(root: &Path, dir: &Path, notes: &mut Vec<Note>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            continue;
        }
        let path = entry.path();
        if ty.is_dir() {
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            visit(root, &path, notes)?;
        } else if ty.is_file() && supported_file(&path) {
            if notes.len() >= MAX_FILES {
                return Err(io::Error::other(format!(
                    "Index limit ({MAX_FILES} text files) reached; narrow your notes root"
                )));
            }
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let relative = path
                .strip_prefix(root)
                .map_err(|_| io::Error::other("Note lies outside notes root"))?
                .to_string_lossy()
                .replace('\\', "/");
            let text = if entry.metadata()?.len() <= MAX_NOTE_BYTES && source_file(&path) {
                fs::read_to_string(&path).ok()
            } else {
                None
            };
            notes.push(Note {
                path,
                stem,
                relative,
                title: text.as_deref().and_then(|s| frontmatter_field(s, "title")),
                id: text.as_deref().and_then(|s| frontmatter_field(s, "id")),
            });
        }
    }
    Ok(())
}

/// Extract links outside fenced code blocks and front matter.
/// Supported syntax: [[Note]], [[Note|label]], [[Note#heading]], and
/// [label](relative/path.md). This is deliberately a small Markdown subset.
fn parse_links(text: &str) -> Vec<RawLink> {
    let mut found = Vec::new();
    let mut fence: Option<char> = None;
    let mut in_frontmatter = false;
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if i == 0 && trimmed.trim_start_matches('\u{feff}') == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if trimmed == "---" {
                in_frontmatter = false;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let marker = trimmed.chars().next().unwrap();
            match fence {
                None => fence = Some(marker),
                Some(old) if old == marker => fence = None,
                _ => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let mut cursor = line;
        while let Some(start) = cursor.find("[[") {
            let after = &cursor[start + 2..];
            let Some(end) = after.find("]]") else {
                break;
            };
            let text = after[..end].split('|').next().unwrap_or("").trim();
            if !text.is_empty() {
                found.push(RawLink {
                    text: text.to_owned(),
                    kind: LinkKind::Wiki,
                });
            }
            cursor = &after[end + 2..];
        }
        // Tiny Markdown link parser: [label](path), excluding images ![...].
        let bytes = line.as_bytes();
        let mut pos = 0;
        while pos < bytes.len() {
            if bytes[pos] != b'[' || (pos > 0 && bytes[pos - 1] == b'!') {
                pos += 1;
                continue;
            }
            let Some(rest) = line.get(pos..) else {
                pos += 1;
                continue;
            };
            let Some(close) = rest.find("](") else {
                break;
            };
            if rest[..close].contains(']') {
                pos += 1;
                continue;
            }
            let value = &rest[close + 2..];
            let Some(end) = value.find(')') else {
                break;
            };
            if !value[..end].is_empty() {
                found.push(RawLink {
                    text: value[..end].to_owned(),
                    kind: LinkKind::Markdown,
                });
            }
            pos += close + 2 + end + 1;
        }
    }
    found
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn normalize_relative(path: &Path) -> Option<PathBuf> {
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::Normal(value) => parts.push(value.to_os_string()),
            Component::ParentDir => {
                parts.pop()?;
            }
            _ => return None,
        }
    }
    let mut clean = PathBuf::new();
    for part in parts {
        clean.push(part);
    }
    Some(clean)
}

fn strip_fragment(link: &str) -> &str {
    link.split('#').next().unwrap_or("").trim()
}

impl Index {
    pub fn build(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        let mut notes = Vec::new();
        visit(&root, &root, &mut notes)?;
        notes.sort_by_key(|a| a.relative.to_lowercase());
        Ok(Self { root, notes })
    }

    fn resolve(&self, source: &Path, link: &RawLink) -> Resolution {
        let raw = strip_fragment(&link.text);
        if raw.is_empty() {
            return Resolution::Missing;
        }
        match link.kind {
            LinkKind::Wiki => self.resolve_wiki(source, raw),
            LinkKind::Markdown => self.resolve_markdown(source, raw),
        }
    }

    fn resolve_wiki(&self, source: &Path, raw: &str) -> Resolution {
        let raw = raw.trim();
        if raw.starts_with('/') || raw.contains('\\') {
            return Resolution::Missing;
        }
        let input = Path::new(raw);
        // No traversal in wiki links, even if it stays inside the root.
        if input
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Resolution::Missing;
        }
        let qualified = raw.contains('/');
        let local = source
            .parent()
            .unwrap_or(&self.root)
            .strip_prefix(&self.root)
            .unwrap_or(Path::new(""));
        let local = local.to_string_lossy().replace('\\', "/");
        let key = raw.trim_end_matches(".md");

        if qualified {
            let local_key = if local.is_empty() {
                key.to_owned()
            } else {
                format!("{local}/{key}")
            };
            let local_matches: Vec<&Note> = self
                .notes
                .iter()
                .filter(|n| {
                    same(&format!("{}.{}", local_key, "md"), &n.relative)
                        || same(&local_key, &n.relative)
                        || same(&format!("{}.markdown", local_key), &n.relative)
                        || same(&local_key, &n.relative_without_ext())
                })
                .collect();
            if !local_matches.is_empty() {
                return unique(local_matches);
            }
            let root_matches = self
                .notes
                .iter()
                .filter(|n| same(key, &n.relative_without_ext()) || same(raw, &n.relative))
                .collect();
            return unique(root_matches);
        }

        // Same-folder matches win; globally duplicated names are ambiguous.
        let matches: Vec<&Note> = self
            .notes
            .iter()
            .filter(|n| {
                same(key, &n.stem)
                    || n.title.as_deref().is_some_and(|t| same(key, t))
                    || n.id.as_deref().is_some_and(|id| same(key, id))
            })
            .collect();
        let nearby: Vec<&Note> = matches
            .iter()
            .copied()
            .filter(|n| n.path.parent() == source.parent())
            .collect();
        if !nearby.is_empty() {
            unique(nearby)
        } else {
            unique(matches)
        }
    }

    fn resolve_markdown(&self, source: &Path, raw: &str) -> Resolution {
        // External URLs, absolute paths, email links, and same-page anchors are not local notes.
        let raw = raw.trim_matches(|c| c == '<' || c == '>');
        if raw.starts_with('/') || raw.contains("://") || raw.starts_with("mailto:") {
            return Resolution::Missing;
        }
        let root_relative = match source
            .parent()
            .unwrap_or(&self.root)
            .strip_prefix(&self.root)
        {
            Ok(p) => p.join(raw),
            Err(_) => return Resolution::Missing,
        };
        let Some(clean) = normalize_relative(&root_relative) else {
            return Resolution::Missing;
        };
        let relative = clean.to_string_lossy().replace('\\', "/");
        let found = self
            .notes
            .iter()
            .filter(|n| same(&n.relative, &relative))
            .collect();
        unique(found)
    }

    fn read_source(&self, source: &Path) -> io::Result<Vec<RawLink>> {
        if !self.notes.iter().any(|n| n.path == source) || !source_file(source) {
            return Ok(Vec::new());
        }
        if fs::metadata(source)?.len() > MAX_NOTE_BYTES {
            return Ok(Vec::new());
        }
        Ok(parse_links(&fs::read_to_string(source)?))
    }

    /// Suggest notes for the wiki-link completion picker. Do not suggest a
    /// target unless the existing link resolver will resolve it uniquely.
    /// The result uses the short filename where safe, otherwise a folder path
    /// or a unique Zettelkasten ID. It never creates any files.
    pub fn suggestions(&self, source: &Path) -> Vec<NoteSuggestion> {
        let mut result = Vec::new();
        for note in &self.notes {
            if note.path == source {
                continue;
            }
            let mut candidates = vec![
                note.stem.clone(),
                note.relative_without_ext(),
                note.relative.clone(),
            ];
            if let Some(id) = &note.id {
                candidates.push(id.clone());
            }
            let link = candidates.into_iter().find(|candidate| {
                !candidate.is_empty()
                    && !candidate
                        .chars()
                        .any(|c| matches!(c, '|' | '[' | ']' | '#' | '\n' | '\r'))
                    && self.resolve_wiki(source, candidate) == Resolution::Found(note.path.clone())
            });
            if let Some(link) = link {
                let title = note.title.as_deref().unwrap_or(&note.stem);
                result.push(NoteSuggestion {
                    label: format!(
                        "{}  —  {}  [{}]",
                        title,
                        note.relative,
                        note.id.as_deref().unwrap_or("no id")
                    ),
                    link,
                });
            }
        }
        result
    }

    pub fn outgoing(&self, source: &Path) -> io::Result<Vec<LinkItem>> {
        let mut result = Vec::new();
        for link in self.read_source(source)? {
            let resolution = self.resolve(source, &link);
            let (target, suffix) = match resolution {
                Resolution::Found(path) => (Some(path), ""),
                Resolution::Missing => (None, "  [missing or external]"),
                Resolution::Ambiguous => (None, "  [ambiguous — use folder path or ID]"),
            };
            result.push(LinkItem {
                label: format!("{}{}", link.text, suffix),
                target,
            });
        }
        Ok(result)
    }

    pub fn backlinks(&self, target: &Path) -> io::Result<Vec<LinkItem>> {
        let mut result = Vec::new();
        for note in &self.notes {
            if note.path == target || !source_file(&note.path) {
                continue;
            }
            let mut count = 0;
            for link in self.read_source(&note.path)? {
                if self.resolve(&note.path, &link) == Resolution::Found(target.to_path_buf()) {
                    count += 1;
                }
            }
            if count != 0 {
                result.push(LinkItem {
                    label: format!(
                        "{}  ({count} link{})",
                        note.relative,
                        if count == 1 { "" } else { "s" }
                    ),
                    target: Some(note.path.clone()),
                });
            }
        }
        Ok(result)
    }
}

impl Note {
    fn relative_without_ext(&self) -> String {
        Path::new(&self.relative)
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/")
    }
}

fn unique(matches: Vec<&Note>) -> Resolution {
    match matches.as_slice() {
        [] => Resolution::Missing,
        [note] => Resolution::Found(note.path.clone()),
        _ => Resolution::Ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("Antibody")).unwrap();
        fs::create_dir(dir.path().join("Readings")).unwrap();
        fs::write(dir.path().join("welcome.md"),
            "---\nid: 20261122\ntitle: Welcome Note\ntags: [rust]\n---\n\n# Welcome\n[[Antibody/VHH]]\n[[20260001|VHH via ID]]\n[Readings](Readings/paper.md)\n").unwrap();
        fs::write(
            dir.path().join("Antibody/VHH.md"),
            "---\nid: 20260001\ntitle: VHH Humanization\n---\n[[welcome]]\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("Readings/paper.md"),
            "See [[VHH Humanization]]",
        )
        .unwrap();
        dir
    }

    #[test]
    fn parses_wiki_aliases_and_markdown_but_not_yaml_or_fences() {
        let text = "---\ntags: [[ShouldIgnore]]\n---\n[[A|label]] [[B#head]] [C](Readings/paper.md)\n```md\n[[Nope]]\n```\n";
        let found = parse_links(text);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].text, "A");
        assert_eq!(found[1].text, "B#head");
        assert_eq!(found[2].text, "Readings/paper.md");
    }

    #[test]
    fn finds_outgoing_using_folder_paths_ids_and_markdown_links() {
        let dir = setup();
        let index = Index::build(dir.path()).unwrap();
        let outgoing = index.outgoing(&dir.path().join("welcome.md")).unwrap();
        assert_eq!(outgoing.len(), 3);
        assert_eq!(outgoing[0].target, Some(dir.path().join("Antibody/VHH.md")));
        assert_eq!(outgoing[1].target, Some(dir.path().join("Antibody/VHH.md")));
        assert_eq!(
            outgoing[2].target,
            Some(dir.path().join("Readings/paper.md"))
        );
    }

    #[test]
    fn detects_backlinks_without_changing_notes() {
        let dir = setup();
        let index = Index::build(dir.path()).unwrap();
        let links = index
            .backlinks(&dir.path().join("Antibody/VHH.md"))
            .unwrap();
        assert_eq!(links.len(), 2);
        assert!(
            links
                .iter()
                .any(|l| l.label.starts_with("welcome.md  (2 links)"))
        );
        assert!(
            links
                .iter()
                .any(|l| l.label.starts_with("Readings/paper.md"))
        );
    }

    #[test]
    fn duplicate_titles_are_ambiguous_not_guessed() {
        let dir = setup();
        fs::write(dir.path().join("Readings/VHH.md"), "# Other VHH\n").unwrap();
        let index = Index::build(dir.path()).unwrap();
        let raw = RawLink {
            text: "VHH".to_owned(),
            kind: LinkKind::Wiki,
        };
        assert_eq!(
            index.resolve(&dir.path().join("welcome.md"), &raw),
            Resolution::Ambiguous
        );
        assert_eq!(
            index.resolve(&dir.path().join("Antibody/VHH.md"), &raw),
            Resolution::Found(dir.path().join("Antibody/VHH.md"))
        );
    }

    #[test]
    fn path_traversal_and_symlinks_not_indexed() {
        let dir = setup();
        let index = Index::build(dir.path()).unwrap();
        let raw = RawLink {
            text: "../../etc/passwd".to_owned(),
            kind: LinkKind::Wiki,
        };
        assert_eq!(
            index.resolve(&dir.path().join("welcome.md"), &raw),
            Resolution::Missing
        );
        let raw = RawLink {
            text: "../../../etc/passwd".to_owned(),
            kind: LinkKind::Markdown,
        };
        assert_eq!(
            index.resolve(&dir.path().join("welcome.md"), &raw),
            Resolution::Missing
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside = tempfile::tempdir().unwrap();
            fs::write(outside.path().join("outside.md"), "secret").unwrap();
            symlink(
                outside.path().join("outside.md"),
                dir.path().join("linked.md"),
            )
            .unwrap();
            assert!(
                Index::build(dir.path())
                    .unwrap()
                    .notes
                    .iter()
                    .all(|n| !n.relative.ends_with("linked.md"))
            );
        }
    }

    #[test]
    fn completion_uses_short_name_when_unique_and_path_when_duplicated() {
        let dir = setup();
        let index = Index::build(dir.path()).unwrap();
        let items = index.suggestions(&dir.path().join("welcome.md"));
        let vhh = items
            .iter()
            .find(|item| item.label.contains("Antibody/VHH.md"))
            .unwrap();
        assert_eq!(vhh.link, "VHH");
        fs::write(dir.path().join("Readings/VHH.md"), "# Different VHH\n").unwrap();
        let index = Index::build(dir.path()).unwrap();
        let items = index.suggestions(&dir.path().join("welcome.md"));
        let first = items
            .iter()
            .find(|item| item.label.contains("Antibody/VHH.md"))
            .unwrap();
        let second = items
            .iter()
            .find(|item| item.label.contains("Readings/VHH.md"))
            .unwrap();
        assert_eq!(first.link, "Antibody/VHH");
        assert_eq!(second.link, "Readings/VHH");
        assert!(items.iter().all(|item| !item.label.contains("welcome.md")));
    }

    #[test]
    fn completion_finds_titles_and_ids_without_modifying_files() {
        let dir = setup();
        let path = dir.path().join("Antibody/VHH.md");
        let before = fs::read(&path).unwrap();
        let index = Index::build(dir.path()).unwrap();
        let suggestions = index.suggestions(&dir.path().join("welcome.md"));
        let item = suggestions
            .iter()
            .find(|item| item.label.contains("VHH Humanization"))
            .unwrap();
        assert!(item.label.contains("20260001"));
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn leaves_original_bytes_unchanged() {
        let dir = setup();
        let path = dir.path().join("welcome.md");
        let before = fs::read(&path).unwrap();
        let index = Index::build(dir.path()).unwrap();
        index.outgoing(&path).unwrap();
        index.backlinks(&path).unwrap();
        assert_eq!(before, fs::read(&path).unwrap());
    }
}

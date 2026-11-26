//! Read-only, on-demand vault search. Files are indexed when the picker opens.
//! No background service, SQLite database, or changes to note content.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const MAX_FILE_BYTES: u64 = 1_048_576;
const MAX_FILES: usize = 20_000;
const MAX_TOTAL_BYTES: u64 = 128 * 1_048_576;
const EXTENSIONS: &[&str] = &[
    "md", "markdown", "txt", "json", "yaml", "yml", "toml", "rst",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Text,
    Tags,
}
impl SearchMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "Full text",
            Self::Tags => "Tags",
        }
    }
    pub fn toggle(self) -> Self {
        match self {
            Self::Text => Self::Tags,
            Self::Tags => Self::Text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: PathBuf,
    /// 1-based line number. Tag results jump to the beginning of their note.
    pub line: usize,
    pub excerpt: String,
}

struct IndexedFile {
    path: PathBuf,
    text: String,
    tags: Vec<String>,
}

pub struct VaultIndex {
    pub root: PathBuf,
    files: Vec<IndexedFile>,
    pub skipped: usize,
    pub truncated: bool,
}

impl VaultIndex {
    /// Recursively scan regular UTF-8 note files under root. Do not follow symlinks.
    pub fn load(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        let mut index = Self {
            root: root.clone(),
            files: Vec::new(),
            skipped: 0,
            truncated: false,
        };
        let mut pending = vec![root];
        let mut bytes_indexed: u64 = 0;
        while let Some(dir) = pending.pop() {
            let mut entries = match fs::read_dir(&dir) {
                Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
                Err(error) if dir == index.root => return Err(error),
                Err(_) => {
                    index.skipped += 1;
                    continue;
                }
            };
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
                let kind = match entry.file_type() {
                    Ok(kind) => kind,
                    Err(_) => {
                        index.skipped += 1;
                        continue;
                    }
                };
                if kind.is_symlink() {
                    continue;
                }
                let path = entry.path();
                if kind.is_dir() {
                    pending.push(path);
                    continue;
                }
                if !kind.is_file() || !is_text_note(&path) {
                    continue;
                }
                if index.files.len() >= MAX_FILES {
                    index.truncated = true;
                    break;
                }
                let file_bytes = match fs::metadata(&path) {
                    Ok(metadata) if metadata.len() > MAX_FILE_BYTES => {
                        index.skipped += 1;
                        continue;
                    }
                    Ok(metadata) => metadata.len(),
                    Err(_) => {
                        index.skipped += 1;
                        continue;
                    }
                };
                if bytes_indexed.saturating_add(file_bytes) > MAX_TOTAL_BYTES {
                    index.truncated = true;
                    break;
                }
                match fs::read_to_string(&path) {
                    Ok(text) => {
                        let tags = frontmatter_tags(&text);
                        bytes_indexed += file_bytes;
                        index.files.push(IndexedFile { path, text, tags });
                    }
                    Err(_) => index.skipped += 1,
                }
            }
            if index.truncated {
                break;
            }
        }
        Ok(index)
    }

    pub fn count(&self) -> usize {
        self.files.len()
    }

    /// Case-insensitive substring search, returning at most `limit` matches.
    /// Text search requires every space-delimited term to appear in the same line.
    pub fn search(&self, mode: SearchMode, query: &str, limit: usize) -> Vec<SearchHit> {
        let query = query.trim().trim_start_matches('#').trim().to_lowercase();
        if query.is_empty() || limit == 0 {
            return Vec::new();
        }
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut results = Vec::new();
        for file in &self.files {
            match mode {
                SearchMode::Text => {
                    for (line_no, line) in file.text.lines().enumerate() {
                        let lower = line.to_lowercase();
                        if words.iter().all(|word| lower.contains(*word)) {
                            results.push(SearchHit {
                                path: file.path.clone(),
                                line: line_no + 1,
                                excerpt: truncate_chars(line.trim(), 150),
                            });
                            if results.len() == limit {
                                return results;
                            }
                        }
                    }
                }
                SearchMode::Tags => {
                    let matching: Vec<&str> = file
                        .tags
                        .iter()
                        .filter(|tag| {
                            let lower = tag.to_lowercase();
                            words.iter().all(|word| lower.contains(*word))
                        })
                        .map(String::as_str)
                        .collect();
                    if !matching.is_empty() {
                        results.push(SearchHit {
                            path: file.path.clone(),
                            line: 1,
                            excerpt: format!("tags: {}", matching.join(", ")),
                        });
                        if results.len() == limit {
                            return results;
                        }
                    }
                }
            }
        }
        results
    }
}

fn is_text_note(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

fn truncate_chars(value: &str, max: usize) -> String {
    match value.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", &value[..end]),
        None => value.to_owned(),
    }
}

/// Supports Zettlr/Obsidian's common frontmatter forms:
///
/// ```yaml
/// tags: [antibody, "vhh engineering"]
/// ```
///
/// ```yaml
/// tags:
///   - antibody
///   - vhh
/// ```
///
/// ```yaml
/// tags: antibody
/// ```
///
/// This is intentionally a small parser, not a complete YAML implementation.
fn frontmatter_tags(text: &str) -> Vec<String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Vec::new();
    }
    let mut tags = Vec::new();
    let mut inside_list = false;
    for line in lines.take(512) {
        let stripped = line.trim();
        if stripped == "---" || stripped == "..." {
            break;
        }
        if let Some(rest) = stripped.strip_prefix("tags:") {
            inside_list = rest.trim().is_empty();
            if !inside_list {
                let raw = rest.trim();
                if let Some(inner) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                    tags.extend(inner.split(',').filter_map(clean_tag));
                } else if let Some(tag) = clean_tag(raw) {
                    tags.push(tag);
                }
            }
            continue;
        }
        if inside_list {
            if let Some(tag) = stripped.strip_prefix("- ").and_then(clean_tag) {
                tags.push(tag);
            } else if !stripped.is_empty() && !stripped.starts_with('#') {
                // Another YAML field starts: do not consume unrelated items.
                inside_list = false;
            }
        }
    }
    tags
}

fn clean_tag(raw: &str) -> Option<String> {
    let tag = raw
        .trim()
        .trim_matches(|c| c == '\'' || c == '"')
        .trim_start_matches('#')
        .trim();
    (!tag.is_empty()).then(|| tag.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn common_zettlr_tags_parse() {
        assert_eq!(
            frontmatter_tags(
                "---\ntitle: Test\ntags: [Antibody, 'VHH engineering']\nid: 42\n---\n# Body"
            ),
            vec!["Antibody", "VHH engineering"]
        );
        assert_eq!(
            frontmatter_tags("---\ntags:\n  - binding\n  - CD4\ntitle: Something\n---"),
            vec!["binding", "CD4"]
        );
        assert!(frontmatter_tags("# tags: [not-a-tag]").is_empty());
    }

    #[test]
    fn searches_nested_files_and_tags_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("Antibody")).unwrap();
        let note = dir.path().join("Antibody/VHH.md");
        let original = "---\ntags: [Protein, Humanization]\n---\n# Introduction\nFramework residues are important.\n";
        fs::write(&note, original).unwrap();
        fs::write(dir.path().join("other.json"), "{\"test\": \"affinity\"}").unwrap();
        let index = VaultIndex::load(dir.path()).unwrap();
        assert_eq!(index.count(), 2);
        let hits = index.search(SearchMode::Text, "FRAMEWORK important", 100);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 5);
        assert_eq!(hits[0].path, note);
        assert_eq!(index.search(SearchMode::Tags, "#human", 100).len(), 1);
        assert_eq!(index.search(SearchMode::Tags, "affinity", 100).len(), 0);
        assert_eq!(fs::read_to_string(&note).unwrap(), original);
    }

    #[test]
    fn tag_search_preserves_case_and_matches_all_terms_per_tag() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("note.md"),
            "---\ntags: [Protein, 'VHH Éngineering', 'VHH Binding']\n---\n",
        )
        .unwrap();
        let index = VaultIndex::load(dir.path()).unwrap();
        let hits = index.search(SearchMode::Tags, "vhh ÉNG", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].excerpt, "tags: VHH Éngineering");
        assert!(index.search(SearchMode::Tags, "protein vhh", 10).is_empty());
    }

    #[test]
    fn excerpt_truncation_preserves_unicode_boundaries() {
        assert_eq!(truncate_chars("β café", 6), "β café");
        assert_eq!(truncate_chars("β café", 5), "β caf…");
        assert_eq!(truncate_chars("β", 0), "…");
        assert_eq!(truncate_chars("", 0), "");
    }

    #[test]
    fn skips_hidden_directories_and_symlinks_and_large_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/secrets.md"), "secret text").unwrap();
        fs::write(
            dir.path().join("large.md"),
            "x".repeat((MAX_FILE_BYTES + 1) as usize),
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(".git/secrets.md", dir.path().join("shortcut.md")).unwrap();
        let index = VaultIndex::load(dir.path()).unwrap();
        assert_eq!(index.count(), 0);
        assert_eq!(index.skipped, 1);
    }

    #[test]
    fn empty_queries_and_limits() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("note.md"), "foo\nfoo\nfoo").unwrap();
        let idx = VaultIndex::load(dir.path()).unwrap();
        assert!(idx.search(SearchMode::Text, "", 99).is_empty());
        assert_eq!(idx.search(SearchMode::Text, "foo", 2).len(), 2);
        assert!(idx.search(SearchMode::Text, "foo", 0).is_empty());
    }
}

//! Read-only Zotero bibliography loader and text search.
//! Supports Better CSL JSON (array of CSL items) and BetterBibTeX JSON
//! debug exports (object with an `items` array and `citationKey` fields).

use crate::fuzzy::{Score, score};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

const MAX_BIB_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Reference {
    pub key: String,
    pub title: String,
    pub authors: String,
    pub year: String,
    all_authors: String,
    journal: String,
    doi: String,
    full_authors: String,
    abstract_text: String,
    attachments: Vec<PathBuf>,
}

impl Reference {
    pub fn summary(&self) -> String {
        format!(
            "{} ({}) · {}  [@{}]",
            self.authors, self.year, self.title, self.key
        )
    }

    /// Read-only bibliographic details, with absent fields made explicit.
    pub fn preview(&self) -> String {
        fn present(value: &str) -> &str {
            if value.is_empty() {
                "Not available"
            } else {
                value
            }
        }
        format!(
            "@{}\n\n{}\n\nAuthors: {}\nYear: {}\nJournal: {}\nDOI: {}\n\nAbstract\n{}",
            self.key,
            self.title,
            present(&self.full_authors),
            self.year,
            present(&self.journal),
            present(&self.doi),
            present(&self.abstract_text)
        )
    }

    /// Resolve only exported local PDF paths, not URLs or Zotero storage IDs.
    pub fn pdf_paths(&self, bibliography_path: &Path) -> Vec<PathBuf> {
        let parent = bibliography_path.parent().unwrap_or_else(|| Path::new("."));
        self.attachments
            .iter()
            .map(|path| {
                if path.is_absolute() {
                    path.clone()
                } else {
                    parent.join(path)
                }
            })
            .collect()
    }

    fn search_score(&self, words: &[&str]) -> Option<Vec<Score>> {
        let fields = [
            self.key.as_str(),
            self.title.as_str(),
            self.all_authors.as_str(),
            self.year.as_str(),
            self.journal.as_str(),
            self.doi.as_str(),
        ];
        let mut scores = words
            .iter()
            .map(|word| fields.iter().filter_map(|field| score(field, word)).min())
            .collect::<Option<Vec<_>>>()?;
        // Compare weakest token first, so exact multi-token hits beat fuzzy hits.
        scores.sort_unstable_by(|a, b| b.cmp(a));
        Some(scores)
    }
}

#[derive(Debug)]
pub struct Bibliography {
    pub references: Vec<Reference>,
}

impl Bibliography {
    pub fn load(path: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        if metadata.len() > MAX_BIB_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Bibliography exceeds 32 MiB; use a smaller export",
            ));
        }
        let raw = fs::read_to_string(path)?;
        Self::parse(&raw)
    }

    pub fn parse(raw: &str) -> io::Result<Self> {
        let root: Value = serde_json::from_str(raw).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Bibliography JSON error: {e}"),
            )
        })?;
        let items = match &root {
            Value::Array(entries) => entries,
            Value::Object(map) => map.get("items").and_then(Value::as_array)
                .ok_or_else(|| invalid("Expected a CSL JSON array or a BetterBibTeX JSON object containing an items array"))?,
            _ => return Err(invalid("Bibliography root must be a JSON array or object with items")),
        };
        let mut keys = HashSet::new();
        let mut references = Vec::new();
        for (i, item) in items.iter().enumerate() {
            // Zotero debug exports can include attachments and standalone notes.
            if matches!(
                item.get("itemType").and_then(Value::as_str),
                Some("attachment" | "note")
            ) {
                continue;
            }
            let key = item
                .get("citationKey")
                .or_else(|| item.get("citation-key"))
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if key.is_empty() {
                continue;
            }
            if !valid_key(key) {
                return Err(invalid(&format!(
                    "Invalid citation key at item {}: {key:?}",
                    i + 1
                )));
            }
            if !keys.insert(key.to_owned()) {
                return Err(invalid(&format!("Duplicate citation key: {key}")));
            }
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .unwrap_or("Untitled reference")
                .to_owned();
            let author_list = contributor_names(item);
            let authors = short_author_names(&author_list);
            let all_authors = author_list.join(" ");
            let year = year_of(item);
            references.push(Reference {
                key: key.to_owned(),
                title,
                authors,
                year,
                all_authors,
                journal: first_text(
                    item,
                    &["container-title", "publicationTitle", "journalAbbreviation"],
                ),
                doi: first_text(item, &["DOI", "doi"]),
                full_authors: full_author_names(item),
                abstract_text: first_text(item, &["abstract", "abstractNote"]),
                attachments: pdf_attachments(item),
            });
        }
        if references.is_empty() {
            return Err(invalid(
                "No citation keys found. Export as Better CSL JSON, or ensure BetterBibTeX items have citationKey fields",
            ));
        }
        references.sort_by_key(|a| a.key.to_lowercase());
        Ok(Self { references })
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<usize> {
        let query = query.to_lowercase();
        let words: Vec<&str> = query.split_whitespace().collect();
        if words.is_empty() {
            return (0..self.references.len().min(max_results)).collect();
        }
        if max_results == 0 {
            return Vec::new();
        }
        let mut ranked: Vec<_> = self
            .references
            .iter()
            .enumerate()
            .filter_map(|(index, item)| Some((item.search_score(&words)?, index)))
            .collect();
        ranked.sort_unstable();
        ranked
            .into_iter()
            .take(max_results)
            .map(|(_, index)| index)
            .collect()
    }
}

fn first_text(item: &Value, fields: &[&str]) -> String {
    fields
        .iter()
        .filter_map(|field| item.get(*field).and_then(Value::as_str))
        .map(str::trim)
        .find(|text| !text.is_empty())
        .unwrap_or_default()
        .to_owned()
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_owned())
}

/// Citation commands use `[@key]`. Reject whitespace and delimiters that would break this syntax.
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|c| {
            !c.is_whitespace()
                && !c.is_control()
                && !matches!(c, '[' | ']' | '@' | ',' | ';' | '{' | '}')
        })
}

fn pdf_attachments(item: &Value) -> Vec<PathBuf> {
    fn add_path(paths: &mut Vec<PathBuf>, raw: &str) {
        let raw = raw.trim();
        if raw.is_empty()
            || raw.contains("://")
            || raw.starts_with("storage:")
            || raw.contains('\0')
        {
            return;
        }
        let path = PathBuf::from(raw);
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            && !paths.contains(&path)
        {
            paths.push(path);
        }
    }
    fn add_file(paths: &mut Vec<PathBuf>, file: &str) {
        for raw in file.split(';') {
            add_path(paths, raw);
        }
    }
    let mut paths = Vec::new();
    if let Some(attachments) = item.get("attachments").and_then(Value::as_array) {
        for attachment in attachments {
            if let Some(path) = attachment.as_str() {
                add_path(&mut paths, path);
            } else {
                add_path(&mut paths, &first_text(attachment, &["path", "localPath"]));
            }
        }
    }
    // Some CSL JSON exports add a custom file field containing plain paths.
    match item.get("file") {
        Some(Value::String(file)) => add_file(&mut paths, file),
        Some(Value::Array(files)) => {
            for file in files.iter().filter_map(Value::as_str) {
                add_file(&mut paths, file);
            }
        }
        _ => {}
    }
    paths
}

fn full_author_names(item: &Value) -> String {
    let authors = item
        .get("author")
        .and_then(Value::as_array)
        .or_else(|| item.get("creators").and_then(Value::as_array));
    let Some(authors) = authors else {
        return String::new();
    };
    authors
        .iter()
        .filter(|creator| {
            creator
                .get("creatorType")
                .and_then(Value::as_str)
                .is_none_or(|role| role == "author")
        })
        .map(|creator| {
            let literal = first_text(creator, &["literal", "name"]);
            if !literal.is_empty() {
                return literal;
            }
            let given = first_text(creator, &["given", "firstName"]);
            let family = first_text(creator, &["family", "lastName"]);
            format!("{given} {family}").trim().to_owned()
        })
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

fn contributor_names(item: &Value) -> Vec<String> {
    let authors = item
        .get("author")
        .and_then(Value::as_array)
        .or_else(|| item.get("creators").and_then(Value::as_array));
    let Some(authors) = authors else {
        return Vec::new();
    };
    authors
        .iter()
        .filter(|creator| {
            creator
                .get("creatorType")
                .and_then(Value::as_str)
                .is_none_or(|role| role == "author")
        })
        .filter_map(|creator| {
            let name = creator
                .get("family")
                .or_else(|| creator.get("lastName"))
                .or_else(|| creator.get("literal"))
                .or_else(|| creator.get("name"))
                .and_then(Value::as_str)?
                .trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_owned())
            }
        })
        .collect()
}

fn short_author_names(names: &[String]) -> String {
    match names {
        [] => "Unknown author".to_owned(),
        [one] => one.to_owned(),
        [one, two] => format!("{one} & {two}"),
        [first, ..] => format!("{first} et al."),
    }
}

fn year_of(item: &Value) -> String {
    if let Some(year) = item
        .get("issued")
        .and_then(|issued| issued.get("date-parts"))
        .and_then(Value::as_array)
        .and_then(|date_parts| date_parts.first())
        .and_then(Value::as_array)
        .and_then(|parts| parts.first())
        .and_then(Value::as_i64)
    {
        return year.to_string();
    }
    if let Some(year) = item.get("year").and_then(Value::as_i64) {
        return year.to_string();
    }
    for field in ["year", "date"] {
        if let Some(date) = item.get(field).and_then(Value::as_str)
            && let Some(year) = date
                .as_bytes()
                .windows(4)
                .find(|w| w.iter().all(|byte| byte.is_ascii_digit()))
        {
            return String::from_utf8_lossy(year).into_owned();
        }
    }
    "n.d.".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSL: &str = r#"[
      {"id":"Jain2017", "title":"Biophysical properties of clinical antibodies",
       "author":[{"family":"Jain","given":"T"},{"family":"Sun"}],
       "issued":{"date-parts":[[2017]]}},
      {"id":"Smith2020", "title":"Yeast display", "author":[{"family":"Smith"}],
       "issued":{"date-parts":[[2020,5,1]]}}
    ]"#;

    #[test]
    fn reads_csl_and_searches_multiple_fields() {
        let bibliography = Bibliography::parse(CSL).unwrap();
        assert_eq!(bibliography.references.len(), 2);
        assert_eq!(bibliography.search("jain 2017", 20), vec![0]);
        assert_eq!(bibliography.references[0].authors, "Jain & Sun");
        assert_eq!(bibliography.references[0].year, "2017");
        assert_eq!(bibliography.search("sun", 20), vec![0]);
    }

    #[test]
    fn supports_better_bibtex_json_debug_export() {
        let raw = r#"{"config":{},"items":[{
            "citationKey":"Forness2018", "title":"Preaching Christology", "date":"2018",
            "creators":[{"firstName":"Philip", "lastName":"Forness", "creatorType":"author"}]
        }]}"#;
        let bibliography = Bibliography::parse(raw).unwrap();
        assert_eq!(bibliography.references[0].authors, "Forness");
        assert_eq!(bibliography.references[0].year, "2018");
    }

    #[test]
    fn fuzzy_search_ranks_exact_matches_before_subsequences() {
        let bibliography = Bibliography::parse(
            r#"[{"id":"Alpha","title":"Size exclusion chromatography"},
                {"id":"Zulu","title":"SEC"}]"#,
        )
        .unwrap();
        assert_eq!(bibliography.search("sec", 10), vec![1, 0]);
        assert_eq!(bibliography.search("sec", 1), vec![1]);
        assert_eq!(bibliography.search("szex", 10), vec![0]);
    }

    #[test]
    fn searches_journal_and_doi_in_both_export_formats() {
        let csl = Bibliography::parse(
            r#"[{"id":"A","container-title":"Nature Methods","DOI":"10.1234/test"}]"#,
        )
        .unwrap();
        assert_eq!(csl.search("nature 10.1234/test", 10), vec![0]);
        let bbt = Bibliography::parse(
            r#"{"items":[{"citationKey":"B","publicationTitle":"Science","DOI":"10.1234/other"}]}"#,
        )
        .unwrap();
        assert_eq!(bbt.search("science 10.1234/other", 10), vec![0]);
    }

    #[test]
    fn preview_metadata_and_pdf_paths_from_both_formats() {
        let csl = Bibliography::parse(r#"[{"id":"A","title":"Study","author":[{"given":"Ada","family":"Lovelace"}],"abstract":"Results","DOI":"10.1/a"}]"#).unwrap();
        let reference = &csl.references[0];
        assert!(reference.preview().contains("Ada Lovelace"));
        assert!(reference.preview().contains("Results"));
        assert!(
            reference
                .pdf_paths(Path::new("/bib/references.json"))
                .is_empty()
        );
        let bbt = Bibliography::parse(r#"{"items":[{"citationKey":"B","abstractNote":"Findings","attachments":[{"path":"papers/a.PDF"},{"path":"/tmp/b.pdf"},{"path":"https://example.org/x.pdf"},{"path":"storage:paper.pdf"},{"path":"papers/a.PDF"},{"path":"notes.txt"}]}]}"#).unwrap();
        let reference = &bbt.references[0];
        assert!(reference.preview().contains("Findings"));
        assert_eq!(
            reference.pdf_paths(Path::new("/bib/references.json")),
            vec![
                PathBuf::from("/bib/papers/a.PDF"),
                PathBuf::from("/tmp/b.pdf")
            ]
        );
    }

    #[test]
    fn csl_file_field_accepts_plain_pdf_paths() {
        let bibliography = Bibliography::parse(r#"[{"id":"A","file":"papers/one.pdf; /tmp/two.PDF; https://example.org/x.pdf"},{"id":"B","file":["papers/three.pdf","notes.txt"]}]"#).unwrap();
        assert_eq!(
            bibliography.references[0].pdf_paths(Path::new("/bib/refs.json")),
            vec![
                PathBuf::from("/bib/papers/one.pdf"),
                PathBuf::from("/tmp/two.PDF")
            ]
        );
        assert_eq!(
            bibliography.references[1].pdf_paths(Path::new("/bib/refs.json")),
            vec![PathBuf::from("/bib/papers/three.pdf")]
        );
    }

    #[test]
    fn refuses_duplicate_keys() {
        let raw = r#"[{"id":"Same"},{"id":"Same"}]"#;
        assert!(Bibliography::parse(raw).is_err());
    }

    #[test]
    fn rejects_malformed_and_missing_keys() {
        assert!(Bibliography::parse("not-json").is_err());
        assert!(Bibliography::parse(r#"[{"title":"Missing key"}]"#).is_err());
        assert!(Bibliography::parse(r#"[{"id":"Bad key"}]"#).is_err());
    }

    #[test]
    fn search_is_case_insensitive_and_limited() {
        let bibliography = Bibliography::parse(CSL).unwrap();
        assert_eq!(bibliography.search("YEAST", 10), vec![1]);
        assert_eq!(bibliography.search("  JAIN  2017 ", 10), vec![0]);
        assert!(bibliography.search("YEAST JAIN", 10).is_empty());
        assert!(bibliography.search("YEAST", 0).is_empty());
        assert_eq!(bibliography.search("", 1).len(), 1);
        assert!(bibliography.search("nonexistent", 10).is_empty());
    }
}

//! Read-only Zotero bibliography loader and text search.
//! Supports Better CSL JSON (array of CSL items) and BetterBibTeX JSON
//! debug exports (object with an `items` array and `citationKey` fields).

use serde_json::Value;
use std::{collections::HashSet, fs, io, path::Path};

const MAX_BIB_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Reference {
    pub key: String,
    pub title: String,
    pub authors: String,
    pub year: String,
    all_authors: String,
}

impl Reference {
    pub fn summary(&self) -> String {
        format!(
            "{} ({}) · {}  [@{}]",
            self.authors, self.year, self.title, self.key
        )
    }

    fn matches(&self, query: &str) -> bool {
        let haystack = format!(
            "{} {} {} {}",
            self.key, self.title, self.all_authors, self.year
        )
        .to_lowercase();
        query
            .to_lowercase()
            .split_whitespace()
            .all(|word| haystack.contains(word))
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
        self.references
            .iter()
            .enumerate()
            .filter(|(_, item)| item.matches(query))
            .take(max_results)
            .map(|(i, _)| i)
            .collect()
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_owned())
}

/// Citation commands use [@key]. Reject whitespace and delimiters that would break this syntax.
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|c| {
            !c.is_whitespace()
                && !c.is_control()
                && !matches!(c, '[' | ']' | '@' | ',' | ';' | '{' | '}')
        })
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
        assert_eq!(bibliography.search("", 1).len(), 1);
        assert!(bibliography.search("nonexistent", 10).is_empty());
    }
}

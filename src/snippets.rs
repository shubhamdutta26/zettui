//! Read-only snippet files and Zettlr-style template expansion.
//! The editor owns all note changes; this module never writes notes or templates.

use chrono::{DateTime, Local};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
};

const MAX_SNIPPET_BYTES: u64 = 32 * 1024;

#[derive(Clone, Debug)]
pub struct Snippet {
    pub name: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub index: u16,
    pub default: String,
    pub value: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Expansion {
    pub text: String,
    /// Zero-based cursor position for EdTUI's Index2(row, col).
    pub cursor_row: usize,
    pub cursor_col: usize,
}

pub fn default_snippets_dir() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()) {
        return PathBuf::from(xdg).join("zettui/snippets");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".config/zettui/snippets");
    }
    PathBuf::from(".config/zettui/snippets")
}

/// Read files at picker-open time so edits to templates need no app restart.
/// Only the top level of the directory is read; symlinks are ignored.
pub fn load_snippets(dir: &Path) -> io::Result<Vec<Snippet>> {
    if !dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Snippet directory missing: {} (create it and add new-note.md)",
                dir.display()
            ),
        ));
    }
    let mut snippets = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path())?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.starts_with('.') {
            continue;
        }
        // Extensionless Zettlr snippet names are also supported.
        let extension = entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        if extension
            .as_deref()
            .is_some_and(|ext| !matches!(ext, "md" | "txt" | "snippet"))
        {
            continue;
        }
        if meta.len() > MAX_SNIPPET_BYTES {
            continue;
        }
        let source = fs::read_to_string(entry.path())?.replace("\r\n", "\n");
        let name = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&filename)
            .to_string();
        snippets.push(Snippet { name, source });
    }
    snippets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(snippets)
}

/// Find ordered, uniquely numbered `${1:default}` etc. Fields with a repeated
/// number share the same value, as in Zettlr. `$0` is not an editable field.
pub fn fields(source: &str) -> io::Result<Vec<Field>> {
    let mut values = BTreeMap::<u16, String>::new();
    let mut rest = source;
    while let Some(pos) = rest.find("${") {
        rest = &rest[pos + 2..];
        let end = rest.find('}').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Unclosed ${number:default} placeholder",
            )
        })?;
        let body = &rest[..end];
        let (number, default) = body.split_once(':').ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid placeholder: ${{{body}}}"),
            )
        })?;
        let index: u16 = number.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid tab stop: {number}"),
            )
        })?;
        if index == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Use $0 for the final cursor position",
            ));
        }
        if let Some(previous) = values.insert(index, default.to_owned()) {
            if previous != default {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Conflicting defaults for tab stop {index}"),
                ));
            }
        }
        rest = &rest[end + 1..];
    }
    Ok(values
        .into_iter()
        .map(|(index, default)| Field {
            index,
            value: default.clone(),
            default,
        })
        .collect())
}

pub fn expand(source: &str, fields: &[Field], now: DateTime<Local>) -> io::Result<Expansion> {
    let values: BTreeMap<u16, &str> = fields.iter().map(|f| (f.index, f.value.as_str())).collect();
    let year = now.format("%Y").to_string();
    let month = now.format("%m").to_string();
    let day = now.format("%d").to_string();
    // Zettlr's default ZKN ID format: YYYYMMDDHHMMSS.
    let zkn_id = now.format("%Y%m%d%H%M%S").to_string();
    let macros = [
        ("$CURRENT_YEAR", year.as_str()),
        ("$CURRENT_MONTH", month.as_str()),
        ("$CURRENT_DATE", day.as_str()),
        ("$ZKN_ID", zkn_id.as_str()),
    ];
    let mut output = String::new();
    let mut end_cursor = None;
    let mut rest = source;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("${") {
            let end = after.find('}').ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Unclosed snippet placeholder")
            })?;
            let (number, _) = after[..end].split_once(':').ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid numbered placeholder")
            })?;
            let index: u16 = number.parse().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "Invalid placeholder number")
            })?;
            let value = values.get(&index).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Missing value for placeholder {index}"),
                )
            })?;
            output.push_str(value);
            rest = &after[end + 1..];
            continue;
        }
        if let Some(next) = rest.strip_prefix("$0") {
            if end_cursor.replace(output.len()).is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Multiple $0 cursor markers",
                ));
            }
            rest = next;
            continue;
        }
        if let Some((token, replacement)) = macros.iter().find(|(token, _)| rest.starts_with(token))
        {
            output.push_str(replacement);
            rest = &rest[token.len()..];
            continue;
        }
        let ch = rest.chars().next().expect("nonempty input");
        output.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    let position = end_cursor.unwrap_or(output.len());
    let before_cursor = &output[..position];
    Ok(Expansion {
        cursor_row: before_cursor.bytes().filter(|&byte| byte == b'\n').count(),
        cursor_col: before_cursor
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .chars()
            .count(),
        text: output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const SAMPLE: &str = "---\ntitle: \"${1:Untitled}\"\nauthor: Shubham Dutta\ndate: $CURRENT_YEAR-$CURRENT_MONTH-$CURRENT_DATE\nid: $ZKN_ID\ntags: [${2:tag}]\n---\n\n## ${3:Main Topic}\n\n$0";

    #[test]
    fn expands_zettlr_example_and_positions_cursor() {
        let mut fields = fields(SAMPLE).unwrap();
        assert_eq!(
            fields.iter().map(|f| f.index).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        fields[0].value = "Antibody Humanization".into();
        fields[1].value = "vhh".into();
        fields[2].value = "Frameworks".into();
        let now = Local
            .with_ymd_and_hms(2026, 11, 23, 8, 30, 42)
            .single()
            .unwrap();
        let result = expand(SAMPLE, &fields, now).unwrap();
        assert!(result.text.contains("date: 2026-11-23\n"));
        assert!(result.text.contains("id: 20261123083042\n"));
        assert!(result.text.contains("title: \"Antibody Humanization\""));
        assert!(result.text.contains("tags: [vhh]"));
        assert!(result.text.ends_with("## Frameworks\n\n"));
        assert_eq!(result.cursor_col, 0);
        assert_eq!(result.cursor_row, 10);
    }

    #[test]
    fn repeated_placeholders_share_values() {
        let t = "${1:Topic} and ${1:Topic} $0";
        let mut f = fields(t).unwrap();
        f[0].value = "Science".into();
        let now = Local::now();
        assert_eq!(expand(t, &f, now).unwrap().text, "Science and Science ");
    }

    #[test]
    fn malformed_placeholders_are_rejected() {
        assert!(fields("${1:no end").is_err());
        assert!(fields("${0:bad}").is_err());
        assert!(fields("${x:bad}").is_err());
    }

    #[test]
    fn loads_extensionless_and_markdown_snippets_but_not_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("new-note.md"), SAMPLE).unwrap();
        fs::write(dir.path().join("daily"), "# Today").unwrap();
        fs::write(dir.path().join("ignore.json"), "{}").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("new-note.md", dir.path().join("fake.md")).unwrap();
        let items = load_snippets(dir.path()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "daily");
    }
}

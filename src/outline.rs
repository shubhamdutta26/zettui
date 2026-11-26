//! Source-line navigation for top-level Markdown ATX headings.
//! Independent of rendering, syntax colors and editor state.

/// A heading in source order. Rows and parent indices are zero-based.
#[derive(Debug, PartialEq, Eq)]
pub struct Heading<'a> {
    pub level: usize,
    pub row: usize,
    // Consumed by the outline UI and heading picker in subsequent phases.
    #[allow(dead_code)]
    pub title: &'a str,
    /// Nearest preceding heading with a smaller level (if any).
    pub parent: Option<usize>,
    /// Exclusive end of this section, including all its subsections.
    pub end_row: usize,
}

/// Markdown block markers can be indented by up to three spaces.
fn block_content(line: &str) -> Option<&str> {
    let content = line.trim_start_matches(' ');
    (line.len() - content.len() <= 3 && !content.starts_with('\t')).then_some(content)
}

fn fence_marker(content: &str) -> Option<(u8, usize)> {
    let marker = *content.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let count = content.bytes().take_while(|&b| b == marker).count();
    (count >= 3).then_some((marker, count))
}

fn atx_heading(content: &str) -> Option<(usize, &str)> {
    let level = content.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &content[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_end_matches([' ', '\t']);
    // A closing hash run is optional, and must be separated by whitespace.
    let before_hashes = rest.trim_end_matches('#');
    let title = if before_hashes.len() < rest.len() && before_hashes.ends_with([' ', '\t']) {
        before_hashes
    } else {
        rest
    };
    Some((level, title.trim_matches([' ', '\t'])))
}

/// Track multiline HTML comments; same-line inline code and escapes are opaque.
/// This deliberately does not attempt to parse every HTML or inline Markdown rule.
fn update_comment(line: &str, comment: &mut bool) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if *comment {
            let Some(end) = bytes[i..].windows(3).position(|w| w == b"-->") else {
                return;
            };
            i += end + 3;
            *comment = false;
        } else if bytes[i..].starts_with(b"<!--") {
            *comment = true;
            i += 4;
        } else if bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i] == b'`' {
            let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            i += run;
            // Only skip a code span when a closing run of exactly the same
            // length exists on this line. An unmatched opener is literal text.
            let mut j = i;
            while j < bytes.len() {
                if bytes[j] == b'`' {
                    let end = bytes[j..].iter().take_while(|&&b| b == b'`').count();
                    j += end;
                    if end == run {
                        i = j;
                        break;
                    }
                } else {
                    j += 1;
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Extract ATX H1–H6 headings from a source snapshot (including unsaved text).
/// Frontmatter, fenced/indented code, multiline comments and $$ blocks are opaque.
/// Setext headings and headings nested in lists/quotes are not parsed.
/// Titles borrow the source and retain inline Markdown; no text is rewritten.
pub fn extract(text: &str) -> Vec<Heading<'_>> {
    let mut headings: Vec<Heading<'_>> = Vec::new();
    let mut parent: Option<usize> = None;
    let mut yaml = false;
    let mut fence: Option<(u8, usize)> = None;
    let mut comment = false;
    let mut math = false;
    let mut line_count = 0;

    // Unlike str::lines(), split retains the editor's final empty logical row.
    for (row, raw) in text.split('\n').enumerate() {
        line_count = row + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let line = if row == 0 {
            line.strip_prefix('\u{feff}').unwrap_or(line)
        } else {
            line
        };
        if row == 0 && line.trim_end() == "---" {
            yaml = true;
            continue;
        }
        if yaml {
            if matches!(line.trim_end(), "---" | "...") {
                yaml = false;
            }
            continue;
        }
        let content = block_content(line);
        if let Some((marker, len)) = fence {
            if let Some(content) = content
                && let Some((closing, count)) = fence_marker(content)
                && marker == closing
                && count >= len
                && content[count..].trim().is_empty()
            {
                fence = None;
            }
            continue;
        }
        if comment {
            update_comment(line, &mut comment);
            continue;
        }
        if math {
            if content.is_some_and(|s| s.trim_end() == "$$") {
                math = false;
            }
            continue;
        }
        let Some(content) = content else {
            continue;
        };
        if let Some((marker, len)) = fence_marker(content)
            && (marker != b'`' || !content[len..].contains('`'))
        {
            fence = Some((marker, len));
            continue;
        }
        if content.trim_end() == "$$" {
            math = true;
            continue;
        }
        if let Some((level, title)) = atx_heading(content) {
            // Close completed siblings/ancestors; skipped heading levels do not
            // create synthetic parents. Each heading is closed exactly once.
            while let Some(index) = parent {
                if headings[index].level < level {
                    break;
                }
                headings[index].end_row = row;
                parent = headings[index].parent;
            }
            headings.push(Heading {
                level,
                row,
                title,
                parent,
                end_row: 0,
            });
            parent = Some(headings.len() - 1);
        }
        update_comment(content, &mut comment);
    }
    while let Some(index) = parent {
        headings[index].end_row = line_count;
        parent = headings[index].parent;
    }
    headings
}

/// Index of the last heading at or before the logical cursor row, if any.
pub fn active_heading(headings: &[Heading<'_>], row: usize) -> Option<usize> {
    headings.partition_point(|h| h.row <= row).checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles<'a>(headings: &[Heading<'a>]) -> Vec<&'a str> {
        headings.iter().map(|h| h.title).collect()
    }

    #[test]
    fn all_six_levels_and_nested_sections() {
        let text = "# A\n## B\n### C\n#### D\n##### E\n###### F\ntext\n## G\n# H\n";
        let h = extract(text);
        assert_eq!(
            h.iter().map(|h| h.level).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6, 2, 1]
        );
        assert_eq!(
            h.iter().map(|h| h.parent).collect::<Vec<_>>(),
            [
                None,
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(0),
                None
            ]
        );
        assert_eq!(
            h.iter().map(|h| h.end_row).collect::<Vec<_>>(),
            [8, 7, 7, 7, 7, 7, 8, 10]
        );
    }

    #[test]
    fn skipped_levels_and_duplicate_titles() {
        let h = extract("### Same\n##### Same\n#### Same\n## Same\n###### Same");
        assert_eq!(titles(&h), ["Same"; 5]);
        assert_eq!(
            h.iter().map(|h| h.parent).collect::<Vec<_>>(),
            [None, Some(0), Some(0), None, Some(3)]
        );
        assert_eq!(
            h.iter().map(|h| h.end_row).collect::<Vec<_>>(),
            [3, 2, 3, 5, 5]
        );
    }

    #[test]
    fn atx_spacing_closing_hashes_and_unicode() {
        let text = "#hashtag\n####### invalid\n    # code\n\t# code\n   ## β antibody ###  \n# C#\n# literal###\n# escaped \\###\n#\n## ###\n###\t日本語";
        let h = extract(text);
        assert_eq!(
            titles(&h),
            [
                "β antibody",
                "C#",
                "literal###",
                "escaped \\###",
                "",
                "",
                "日本語"
            ]
        );
        assert_eq!(
            h.iter().map(|h| h.row).collect::<Vec<_>>(),
            [4, 5, 6, 7, 8, 9, 10]
        );
    }

    #[test]
    fn frontmatter_bom_crlf_and_indented_delimiters() {
        let text =
            "\u{feff}---\r\nnotes: |\r\n  ---\r\n  # YAML\r\n# comment\r\n...\r\n# Actual\r\n";
        let h = extract(text);
        assert_eq!(titles(&h), ["Actual"]);
        assert_eq!((h[0].row, h[0].end_row), (6, 8));
        assert!(extract("---\n# unclosed frontmatter").is_empty());
    }

    #[test]
    fn fences_respect_marker_length_and_closing_syntax() {
        let text = "# A\n````{r, echo=false}\n# Fake\n```\n# Fake\n~~~~\n# Fake\n```` not a closer\n# Fake\n   `````\n## B\n~~~python\n# Fake\n~~~\n# C\n```\n# Unclosed";
        assert_eq!(titles(&extract(text)), ["A", "B", "C"]);
        assert_eq!(titles(&extract("```bad`info\n# Actual")), ["Actual"]);
    }

    #[test]
    fn comments_math_and_inline_code() {
        let text = "<!--\n# Fake\n```\n-->\n# A\n`<!--`\n## B\n\\<!--\n## C\n$$\n# Fake\n<!--\n$$\n# D\ntext <!--\n## Fake\n-->\n# E";
        assert_eq!(titles(&extract(text)), ["A", "B", "C", "D", "E"]);
        assert_eq!(titles(&extract("    <!--\n# A")), ["A"]);
    }

    #[test]
    fn cursor_lookup_before_between_and_after_headings() {
        let h = extract("intro\n\n# A\nbody\n## B\nbody\n");
        assert_eq!(active_heading(&h, 0), None);
        assert_eq!(active_heading(&h, 2), Some(0));
        assert_eq!(active_heading(&h, 3), Some(0));
        assert_eq!(active_heading(&h, 4), Some(1));
        assert_eq!(active_heading(&h, 6), Some(1));
        assert_eq!(active_heading(&[], 42), None);
    }

    #[test]
    fn empty_plain_and_unsupported_constructs() {
        for text in ["", "plain\ntext", "> # Quote\n- # List", "Title\n====="] {
            assert!(extract(text).is_empty());
        }
    }
}

//! Theme-aware, source-preserving Markdown highlighting. Ranges are character
//! offsets, not UTF-8 byte offsets, matching EdTUI's Index2 coordinates.
use crate::theme::Theme;
use edtui::{Highlight, Index2};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::path::Path;

pub fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        ["md", "markdown", "mdown", "mkd", "qmd"]
            .iter()
            .any(|e| s.eq_ignore_ascii_case(e))
    })
}

fn starts(c: &[char], i: usize, s: &str) -> bool {
    c.get(i..).is_some_and(|rest| {
        rest.iter().copied().zip(s.chars()).all(|(a, b)| a == b) && rest.len() >= s.chars().count()
    })
}
fn find(c: &[char], from: usize, s: &str) -> Option<usize> {
    (from..c.len()).find(|&i| starts(c, i, s) && (i == 0 || c[i - 1] != '\\'))
}
fn paint(styles: &mut [Style], a: usize, b: usize, style: Style) {
    if a <= b && b <= styles.len() {
        styles[a..b].fill(style);
    }
}
fn paired(
    c: &[char],
    styles: &mut [Style],
    i: usize,
    open: &str,
    close: &str,
    body: Style,
    dim: Style,
) -> Option<usize> {
    if !starts(c, i, open) {
        return None;
    }
    let a = i + open.chars().count();
    let b = find(c, a, close)?;
    if b == a {
        return None;
    }
    let end = b + close.chars().count();
    paint(styles, i, a, dim);
    paint(styles, a, b, body);
    paint(styles, b, end, dim);
    Some(end)
}

#[derive(Default)]
struct Context {
    fence: Option<(char, usize, String)>,
    yaml: bool,
    math: bool,
    comment: bool,
}

fn code(c: &[char], styles: &mut [Style], lang: &str, t: Theme) {
    // Small lexical highlighter, deliberately not a language parser.
    let keywords = "let mut fn pub impl struct enum use mod match if else for while loop return const static async await move trait type where as in break continue def class import from pass yield lambda try except finally with raise True False None function var export new null true false SELECT FROM WHERE JOIN GROUP ORDER INSERT UPDATE DELETE library function TRUE FALSE NULL NA next repeat in";
    let known = [
        "rust",
        "rs",
        "python",
        "py",
        "r",
        "bash",
        "sh",
        "shell",
        "zsh",
        "javascript",
        "js",
        "typescript",
        "ts",
        "json",
        "yaml",
        "yml",
        "toml",
        "sql",
        "html",
        "css",
    ]
    .contains(&lang);
    if !known {
        return;
    }
    let mut i = 0;
    while i < c.len() {
        if (matches!(
            lang,
            "python" | "py" | "r" | "bash" | "sh" | "shell" | "zsh" | "yaml" | "yml" | "toml"
        ) && c[i] == '#')
            || (matches!(
                lang,
                "rust" | "rs" | "javascript" | "js" | "typescript" | "ts"
            ) && starts(c, i, "//"))
            || (lang == "sql" && starts(c, i, "--"))
        {
            paint(
                styles,
                i,
                c.len(),
                Style::default().fg(t.muted).add_modifier(Modifier::ITALIC),
            );
            break;
        }
        if c[i] == '\'' || c[i] == '"' {
            let q = c[i];
            let a = i;
            i += 1;
            while i < c.len() {
                if c[i] == '\\' {
                    i = (i + 2).min(c.len());
                    continue;
                }
                let done = c[i] == q;
                i += 1;
                if done {
                    break;
                }
            }
            paint(styles, a, i, Style::default().fg(t.success));
            continue;
        }
        if c[i].is_ascii_digit() {
            let a = i;
            i += 1;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            paint(styles, a, i, Style::default().fg(t.warning));
            continue;
        }
        if c[i].is_alphabetic() || c[i] == '_' {
            let a = i;
            i += 1;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            let word: String = c[a..i].iter().collect();
            if keywords
                .split_whitespace()
                .any(|k| k == word || (lang == "sql" && k.eq_ignore_ascii_case(&word)))
            {
                paint(
                    styles,
                    a,
                    i,
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                );
            }
            continue;
        }
        i += 1;
    }
}

fn line_styles(line: &str, row: usize, ctx: &mut Context, t: Theme) -> (Vec<char>, Vec<Style>) {
    let c: Vec<char> = line.chars().collect();
    let n = c.len();
    let mut s = vec![Style::default(); n];
    let dim = Style::default().fg(t.muted);
    let accent = Style::default().fg(t.accent);
    let green = Style::default().fg(t.success);
    let warn = Style::default().fg(t.warning);
    let link = Style::default()
        .fg(t.directory)
        .add_modifier(Modifier::UNDERLINED);
    let trim = line.trim();
    let start = c.iter().position(|x| !x.is_whitespace()).unwrap_or(n);
    if row == 0 && trim.trim_start_matches('\u{feff}') == "---" {
        ctx.yaml = true;
        s.fill(dim);
        return (c, s);
    }
    if ctx.yaml {
        if trim == "---" || trim == "..." {
            ctx.yaml = false;
            s.fill(dim);
        } else {
            s.fill(green);
            if let Some(k) = c.iter().position(|&x| x == ':') {
                paint(&mut s, 0, k, accent);
                paint(&mut s, k, k + 1, dim);
            }
            code(&c, &mut s, "yaml", t);
        }
        return (c, s);
    }
    if let Some((marker, len, lang)) = ctx.fence.as_ref() {
        let count = c[start..].iter().take_while(|&&x| x == *marker).count();
        if count >= *len && c[start + count..].iter().all(|x| x.is_whitespace()) {
            ctx.fence = None;
            s.fill(dim);
        } else {
            code(&c, &mut s, lang, t);
        }
        return (c, s);
    }
    if start < n && matches!(c[start], '`' | '~') && !ctx.comment && !ctx.math {
        let marker = c[start];
        let len = c[start..].iter().take_while(|&&x| x == marker).count();
        if len >= 3 {
            let lang: String = c[start + len..].iter().collect();
            let lang = lang
                .trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();
            ctx.fence = Some((marker, len, lang));
            s.fill(dim);
            return (c, s);
        }
    }
    if trim == "$$" && !ctx.comment {
        ctx.math = !ctx.math;
        s.fill(dim);
        return (c, s);
    }
    if ctx.math {
        s.fill(warn);
        return (c, s);
    }
    if start < n && !ctx.comment {
        let h = c[start..].iter().take_while(|&&x| x == '#').count();
        if (1..=6).contains(&h) && (start + h == n || c[start + h].is_whitespace()) {
            let color = match h {
                1 => t.accent,
                2 => t.directory,
                3 => t.success,
                _ => t.foreground,
            };
            s.fill(Style::default().fg(color).add_modifier(Modifier::BOLD));
            paint(&mut s, start, start + h, dim);
        } else if trim.chars().filter(|x| !x.is_whitespace()).count() >= 3
            && ['-', '*', '_']
                .iter()
                .any(|&m| trim.chars().all(|x| x == m || x.is_whitespace()))
        {
            s.fill(dim);
            return (c, s);
        } else if c[start] == '>' {
            s.fill(dim.add_modifier(Modifier::ITALIC));
        } else {
            let marker = if matches!(c[start], '-' | '+' | '*')
                && c.get(start + 1).is_some_and(|x| x.is_whitespace())
            {
                Some(start + 1)
            } else {
                let digits = c[start..].iter().take_while(|x| x.is_ascii_digit()).count();
                if digits > 0
                    && c.get(start + digits)
                        .is_some_and(|x| matches!(*x, '.' | ')'))
                    && c.get(start + digits + 1).is_some_and(|x| x.is_whitespace())
                {
                    Some(start + digits + 1)
                } else {
                    None
                }
            };
            if let Some(end) = marker {
                paint(&mut s, start, end, accent);
                let a = end + c[end..].iter().take_while(|x| x.is_whitespace()).count();
                for (m, style) in [("[x]", green), ("[X]", green), ("[ ]", warn)] {
                    if starts(&c, a, m) {
                        paint(&mut s, a, a + 3, style);
                    }
                }
            }
        }
    }
    let mut i = 0;
    while i < n {
        if ctx.comment || starts(&c, i, "<!--") {
            let from = if ctx.comment { i } else { i + 4 };
            if let Some(end) = find(&c, from, "-->") {
                paint(&mut s, i, end + 3, dim);
                ctx.comment = false;
                i = end + 3;
                continue;
            }
            paint(&mut s, i, n, dim);
            ctx.comment = true;
            break;
        }
        if c[i] == '\\' {
            i = (i + 2).min(n);
            continue;
        }
        if c[i] == '`' {
            let len = c[i..].iter().take_while(|&&x| x == '`').count();
            let delim = "`".repeat(len);
            if let Some(end) = paired(&c, &mut s, i, &delim, &delim, warn, dim) {
                i = end;
                continue;
            }
            i += len;
            continue;
        }
        let mut matched = false;
        for (a, b, style) in [
            ("[[", "]]", link),
            ("[@", "]", accent),
            ("[^", "]", accent),
            (
                "***",
                "***",
                warn.add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ),
            (
                "___",
                "___",
                warn.add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ),
            ("**", "**", warn.add_modifier(Modifier::BOLD)),
            ("__", "__", warn.add_modifier(Modifier::BOLD)),
            ("~~", "~~", dim.add_modifier(Modifier::CROSSED_OUT)),
            ("==", "==", warn.add_modifier(Modifier::BOLD)),
            ("$$", "$$", warn),
            ("$", "$", warn),
            ("*", "*", green.add_modifier(Modifier::ITALIC)),
            ("_", "_", green.add_modifier(Modifier::ITALIC)),
        ] {
            if a == "_" && i > 0 && c[i - 1].is_alphanumeric() {
                continue;
            }
            if let Some(end) = paired(&c, &mut s, i, a, b, style, dim) {
                i = end;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        let a = if starts(&c, i, "![") { i + 1 } else { i };
        if c[a] == '[' {
            if let Some(mid) = find(&c, a + 1, "](") {
                if let Some(end) = find(&c, mid + 2, ")") {
                    paint(&mut s, i, a + 1, dim);
                    paint(&mut s, a + 1, mid, link);
                    paint(&mut s, mid, mid + 2, dim);
                    paint(&mut s, mid + 2, end, link);
                    paint(&mut s, end, end + 1, dim);
                    i = end + 1;
                    continue;
                }
            }
        }
        if c[i] == '#'
            && (i == 0 || c[i - 1].is_whitespace())
            && c.get(i + 1).is_some_and(|x| x.is_alphanumeric())
        {
            let a = i;
            i += 1;
            while i < n && (c[i].is_alphanumeric() || matches!(c[i], '_' | '-' | '/')) {
                i += 1;
            }
            paint(&mut s, a, i, green);
            continue;
        }
        if c[i] == '|' && !ctx.comment {
            s[i] = dim;
        }
        i += 1;
    }
    (c, s)
}

fn runs(styles: &[Style]) -> impl Iterator<Item = (usize, usize, Style)> + '_ {
    let mut i = 0;
    std::iter::from_fn(move || {
        if i == styles.len() {
            return None;
        }
        let a = i;
        let style = styles[i];
        i += 1;
        while i < styles.len() && styles[i] == style {
            i += 1;
        }
        Some((a, i, style))
    })
}

pub fn preview(text: &str, t: Theme) -> Vec<Line<'static>> {
    let mut ctx = Context::default();
    text.split('\n')
        .enumerate()
        .map(|(row, line)| {
            let (c, s) = line_styles(line.trim_end_matches('\r'), row, &mut ctx, t);
            Line::from(
                runs(&s)
                    .map(|(a, b, style)| Span::styled(c[a..b].iter().collect::<String>(), style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

pub fn editor_highlights(
    text: &str,
    t: Theme,
    markdown: bool,
    search: &[(Index2, Index2)],
    active: usize,
) -> Vec<Highlight> {
    let mut ctx = Context::default();
    let mut result = Vec::new();
    let mut hit = 0;
    for (row, line) in text.split('\n').enumerate() {
        let (_, mut s) = if markdown {
            line_styles(line, row, &mut ctx, t)
        } else {
            (Vec::new(), vec![Style::default(); line.chars().count()])
        };
        let first_hit = hit;
        while hit < search.len() && search[hit].0.row == row {
            let (a, b) = &search[hit];
            paint(
                &mut s,
                a.col,
                b.col + 1,
                Style::default().fg(t.selected_fg).bg(t.selected_bg),
            );
            hit += 1;
        }
        if active > first_hit && active <= hit {
            let (a, b) = &search[active - 1];
            paint(
                &mut s,
                a.col,
                b.col + 1,
                Style::default().fg(t.background).bg(t.accent),
            );
        }
        for (a, b, style) in runs(&s) {
            if style != Style::default() {
                result.push(Highlight::new(
                    Index2::new(row, a),
                    Index2::new(row, b - 1),
                    style,
                ));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_preserved() {
        let t = Theme::named("dark").unwrap();
        let text = "# β heading\n**bold** and `x`\n```r\n# comment\n```\n";
        let reconstructed = preview(text, t)
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(reconstructed, text);
    }
    #[test]
    fn unicode_offsets_and_fences() {
        let t = Theme::named("dark").unwrap();
        let mut ctx = Context::default();
        let (_, s) = line_styles("β **x**", 0, &mut ctx, t);
        assert_eq!(s[4].fg, Some(t.warning));
        line_styles("```r", 1, &mut ctx, t);
        let (_, s) = line_styles("# heading", 2, &mut ctx, t);
        assert_eq!(s[0].fg, Some(t.muted));
        line_styles("```", 3, &mut ctx, t);
        assert!(ctx.fence.is_none());
    }
}

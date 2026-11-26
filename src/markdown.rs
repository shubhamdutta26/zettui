//! Theme-aware, source-preserving Markdown highlighting. Ranges are character
//! offsets, not UTF-8 byte offsets, matching EdTUI's Index2 coordinates.
mod code;
mod yaml;

use self::code::{CodeContext, code};
use self::yaml::{YamlContext, yaml_line};
use crate::theme::Theme;
use edtui::{Highlight, Index2};
use ratatui::style::Color;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use serde::{Deserialize, Deserializer};
use std::path::Path;

fn optional_color<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Color>, D::Error> {
    let value = Option::<String>::deserialize(d)?;
    value
        .as_deref()
        .map(crate::theme::parse_hex)
        .transpose()
        .map_err(serde::de::Error::custom)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct HeadingColors {
    #[serde(deserialize_with = "optional_color")]
    pub h1: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub h2: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub h3: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub h4: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub h5: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub h6: Option<Color>,
}
impl HeadingColors {
    fn get(&self, level: usize) -> Option<Color> {
        [self.h1, self.h2, self.h3, self.h4, self.h5, self.h6][level.clamp(1, 6) - 1]
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SyntaxColors {
    #[serde(deserialize_with = "optional_color")]
    pub bold: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub italic: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub strikethrough: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub highlight: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub inline_code: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub link: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub link_destination: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub wiki_link: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub tag: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub citation: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub footnote: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub math: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub comment: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub blockquote: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub list_marker: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub task_done: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub task_pending: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub delimiter: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub horizontal_rule: Option<Color>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct YamlColors {
    #[serde(deserialize_with = "optional_color")]
    pub key: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub string: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub number: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub boolean: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub null: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub comment: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub marker: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub reference: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub delimiter: Option<Color>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct CodeColors {
    #[serde(deserialize_with = "optional_color")]
    pub keyword: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub constant: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub function: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub string: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub number: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub comment: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub operator: Option<Color>,
    #[serde(deserialize_with = "optional_color")]
    pub delimiter: Option<Color>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct MarkdownSettings {
    pub syntax_highlighting: bool,
    pub highlight_code_blocks: bool,
    pub highlight_frontmatter: bool,
    pub dim_delimiters: bool,
    pub heading_colors: HeadingColors,
    pub code_colors: CodeColors,
    pub yaml_colors: YamlColors,
    pub syntax_colors: SyntaxColors,
}
impl Default for MarkdownSettings {
    fn default() -> Self {
        Self {
            syntax_highlighting: true,
            highlight_code_blocks: true,
            highlight_frontmatter: true,
            dim_delimiters: true,
            heading_colors: HeadingColors::default(),
            code_colors: CodeColors::default(),
            yaml_colors: YamlColors::default(),
            syntax_colors: SyntaxColors::default(),
        }
    }
}

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
fn escaped(c: &[char], i: usize) -> bool {
    !c[..i]
        .iter()
        .rev()
        .take_while(|&&ch| ch == '\\')
        .count()
        .is_multiple_of(2)
}
fn find(c: &[char], from: usize, s: &str) -> Option<usize> {
    (from..c.len()).find(|&i| starts(c, i, s) && !escaped(c, i))
}
fn link_end(c: &[char], from: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, &ch) in c.iter().enumerate().skip(from) {
        if escaped(c, i) {
            continue;
        }
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
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
    let b = if open.starts_with('`') {
        (a..c.len()).find(|&j| {
            c[j] == '`'
                && c[j - 1] != '`'
                && c[j..].iter().take_while(|&&ch| ch == '`').count() == close.len()
        })?
    } else {
        find(c, a, close)?
    };
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
    code_context: CodeContext,
    yaml: bool,
    yaml_context: YamlContext,
    math: bool,
    comment: bool,
}

// Distinct hue families with light/dark contrast, independent of duplicate theme accents.
fn heading_color(level: usize, t: Theme, settings: &MarkdownSettings) -> ratatui::style::Color {
    let light = matches!(t.background, Color::Rgb(r,g,b) if (u32::from(r)*299+u32::from(g)*587+u32::from(b)*114)>150_000);
    let colors = if light {
        [
            (112, 48, 160),
            (28, 82, 168),
            (0, 105, 113),
            (36, 112, 54),
            (133, 91, 0),
            (177, 65, 35),
        ]
    } else {
        [
            (203, 166, 247),
            (137, 180, 250),
            (148, 226, 213),
            (166, 227, 161),
            (249, 226, 175),
            (250, 179, 135),
        ]
    };
    let fallback = colors.map(|(r, g, b)| Color::Rgb(r, g, b));
    let mut resolved = [None; 6];
    for (i, slot) in resolved.iter_mut().enumerate() {
        *slot = settings.heading_colors.get(i + 1);
    }
    for i in 0..6 {
        if resolved[i].is_some() {
            continue;
        }
        let preferred = t.heading_colors[i].unwrap_or(fallback[i]);
        resolved[i] = Some(if !resolved.contains(&Some(preferred)) {
            preferred
        } else {
            fallback
                .iter()
                .copied()
                .find(|c| !resolved.contains(&Some(*c)))
                .expect("six fallback colors for six headings")
        });
    }
    resolved[level.clamp(1, 6) - 1].expect("resolved heading")
}

fn delimiter_style(t: Theme, dim: bool, color: Option<Color>) -> Style {
    Style::default().fg(if dim {
        color.unwrap_or(t.muted)
    } else {
        t.foreground
    })
}

fn line_styles(
    line: &str,
    row: usize,
    ctx: &mut Context,
    t: Theme,
    settings: &MarkdownSettings,
) -> (Vec<char>, Vec<Style>) {
    let c: Vec<char> = line.chars().collect();
    let n = c.len();
    let mut s = vec![Style::default(); n];
    if !settings.syntax_highlighting {
        return (c, s);
    }
    let colors = &settings.syntax_colors;
    let delimiter = delimiter_style(t, settings.dim_delimiters, colors.delimiter);
    let yaml_delimiter =
        delimiter_style(t, settings.dim_delimiters, settings.yaml_colors.delimiter);
    let code_delimiter =
        delimiter_style(t, settings.dim_delimiters, settings.code_colors.delimiter);
    let list_marker = Style::default().fg(colors.list_marker.unwrap_or(t.accent));
    let math = Style::default().fg(colors.math.unwrap_or(t.warning));
    let inline_code = Style::default().fg(colors.inline_code.unwrap_or_else(|| {
        if t.accent == colors.bold.unwrap_or(t.warning) {
            t.foreground
        } else {
            t.accent
        }
    }));
    let comment = Style::default()
        .fg(colors.comment.unwrap_or(t.muted))
        .add_modifier(Modifier::ITALIC);
    let link = Style::default()
        .fg(colors.link.unwrap_or(t.directory))
        .add_modifier(Modifier::UNDERLINED);
    let trim = line.trim();
    let start = c.iter().position(|x| !x.is_whitespace()).unwrap_or(n);
    if row == 0 && line.trim_start_matches('\u{feff}').trim_end() == "---" {
        ctx.yaml = true;
        if settings.highlight_frontmatter {
            s.fill(yaml_delimiter);
        }
        return (c, s);
    }
    if ctx.yaml {
        if start == 0 && matches!(trim, "---" | "...") {
            ctx.yaml = false;
            ctx.yaml_context = YamlContext::default();
            if settings.highlight_frontmatter {
                s.fill(yaml_delimiter);
            }
        } else if settings.highlight_frontmatter {
            yaml_line(&c, &mut s, &mut ctx.yaml_context, t, settings);
        }
        return (c, s);
    }
    if let Some((marker, len, lang)) = ctx.fence.as_ref() {
        let count = c[start..].iter().take_while(|&&x| x == *marker).count();
        if count >= *len && c[start + count..].iter().all(|x| x.is_whitespace()) {
            ctx.fence = None;
            ctx.code_context = CodeContext::default();
            if settings.highlight_code_blocks {
                s.fill(code_delimiter);
            }
        } else if settings.highlight_code_blocks {
            code(&c, &mut s, lang, t, &mut ctx.code_context, settings);
        }
        return (c, s);
    }
    if start < n && matches!(c[start], '`' | '~') && !ctx.comment && !ctx.math {
        let marker = c[start];
        let len = c[start..].iter().take_while(|&&x| x == marker).count();
        if len >= 3 && (marker != '`' || !c[start + len..].contains(&'`')) {
            let lang: String = c[start + len..].iter().collect();
            let lang = lang
                .trim()
                .trim_start_matches('{')
                .trim_start()
                .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == '}')
                .next()
                .unwrap_or("")
                .to_lowercase();
            ctx.fence = Some((marker, len, lang));
            ctx.code_context = CodeContext::default();
            if settings.highlight_code_blocks {
                s.fill(code_delimiter);
            }
            return (c, s);
        }
    }
    if trim == "$$" && !ctx.comment {
        ctx.math = !ctx.math;
        s.fill(delimiter);
        return (c, s);
    }
    if ctx.math {
        s.fill(math);
        return (c, s);
    }
    if start < n && !ctx.comment {
        let h = c[start..].iter().take_while(|&&x| x == '#').count();
        if (1..=6).contains(&h) && (start + h == n || c[start + h].is_whitespace()) {
            let color = heading_color(h, t, settings);
            s.fill(Style::default().fg(color).add_modifier(Modifier::BOLD));
        } else if trim.chars().filter(|x| !x.is_whitespace()).count() >= 3
            && ['-', '*', '_']
                .iter()
                .any(|&m| trim.chars().all(|x| x == m || x.is_whitespace()))
        {
            s.fill(Style::default().fg(colors.horizontal_rule.unwrap_or(t.muted)));
            return (c, s);
        } else if c[start] == '>' {
            s.fill(
                Style::default()
                    .fg(colors.blockquote.unwrap_or(t.muted))
                    .add_modifier(Modifier::ITALIC),
            );
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
                paint(&mut s, start, end, list_marker);
                let a = end + c[end..].iter().take_while(|x| x.is_whitespace()).count();
                for (m, style) in [
                    (
                        "[x]",
                        Style::default().fg(colors.task_done.unwrap_or(t.success)),
                    ),
                    (
                        "[X]",
                        Style::default().fg(colors.task_done.unwrap_or(t.success)),
                    ),
                    (
                        "[ ]",
                        Style::default().fg(colors.task_pending.unwrap_or(t.warning)),
                    ),
                ] {
                    if starts(&c, a, m) {
                        paint(&mut s, a, a + 3, style);
                    }
                }
            }
        }
    }
    // Delimiter stack: apply formatting only when a matching closer is found.
    // Character offsets are preserved, and opaque spans are skipped by the scanner.
    let mut emphasis: Vec<(char, usize, usize)> = Vec::new();
    let table_like = trim.starts_with('|')
        || trim.ends_with('|')
        || (trim.contains(" | ") && trim.matches('|').count() >= 2);
    let mut i = 0;
    while i < n {
        if i == start
            && c[i] == '*'
            && c.get(i + 1).is_some_and(|ch| ch.is_whitespace())
            && !ctx.comment
        {
            i += 1;
            continue;
        }
        if ctx.comment || starts(&c, i, "<!--") {
            let from = if ctx.comment { i } else { i + 4 };
            if let Some(end) = find(&c, from, "-->") {
                paint(&mut s, i, end + 3, comment);
                ctx.comment = false;
                i = end + 3;
                continue;
            }
            paint(&mut s, i, n, comment);
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
            if let Some(end) = paired(&c, &mut s, i, &delim, &delim, inline_code, delimiter) {
                i = end;
                continue;
            }
            i += len;
            continue;
        }
        if matches!(c[i], '*' | '_' | '~') {
            let marker = c[i];
            let len = c[i..].iter().take_while(|&&ch| ch == marker).count();
            let prev = i.checked_sub(1).map(|j| c[j]);
            let next = c.get(i + len).copied();
            let intraword = marker == '_'
                && prev.is_some_and(char::is_alphanumeric)
                && next.is_some_and(char::is_alphanumeric);
            let can_open = !intraword && next.is_some_and(|ch| !ch.is_whitespace());
            let can_close = !intraword && prev.is_some_and(|ch| !ch.is_whitespace());
            let mut used = 0;
            if can_close {
                while let Some(&(m, width, begin)) = emphasis.last() {
                    if m != marker || width > len - used {
                        break;
                    }
                    emphasis.pop();
                    let close = i + used;
                    let (color, modifier) = match (marker, width) {
                        ('~', _) => (
                            colors.strikethrough.unwrap_or(t.muted),
                            Modifier::CROSSED_OUT,
                        ),
                        (_, 2) => (colors.bold.unwrap_or(t.warning), Modifier::BOLD),
                        _ => (colors.italic.unwrap_or(t.success), Modifier::ITALIC),
                    };
                    for style in &mut s[begin + width..close] {
                        if style.fg.is_none() {
                            *style = style.fg(color);
                        }
                        *style = style.add_modifier(modifier);
                    }
                    paint(&mut s, begin, begin + width, delimiter);
                    paint(&mut s, close, close + width, delimiter);
                    used += width;
                }
            }
            if can_open {
                while used < len && emphasis.len() < 64 {
                    let width = if len - used >= 2 { 2 } else { 1 };
                    if marker == '~' && width == 1 {
                        break;
                    }
                    emphasis.push((marker, width, i + used));
                    used += width;
                }
            }
            i += len;
            continue;
        }
        let mut matched = false;
        for (a, b, style) in [
            (
                "[[",
                "]]",
                Style::default()
                    .fg(colors.wiki_link.unwrap_or(t.directory))
                    .add_modifier(Modifier::UNDERLINED),
            ),
            (
                "[@",
                "]",
                Style::default().fg(colors.citation.unwrap_or(t.accent)),
            ),
            (
                "[-@",
                "]",
                Style::default().fg(colors.citation.unwrap_or(t.accent)),
            ),
            (
                "[^",
                "]",
                Style::default().fg(colors.footnote.unwrap_or(t.accent)),
            ),
            (
                "==",
                "==",
                Style::default()
                    .fg(colors.highlight.unwrap_or(t.warning))
                    .add_modifier(Modifier::BOLD),
            ),
            ("$$", "$$", math),
            ("$", "$", math),
        ] {
            if a == "$"
                && (c.get(i + 1).is_none_or(|ch| ch.is_whitespace()) || c.get(i + 1) == Some(&'$'))
            {
                continue;
            }
            if a == "$"
                && !find(&c, i + 1, "$").is_some_and(|end| {
                    end > i + 1
                        && !c[end - 1].is_whitespace()
                        && !c.get(end + 1).is_some_and(char::is_ascii_digit)
                })
            {
                continue;
            }
            if let Some(end) = paired(&c, &mut s, i, a, b, style, delimiter) {
                i = end;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        let a = if starts(&c, i, "![") { i + 1 } else { i };
        if c[a] == '['
            && let Some(mid) = find(&c, a + 1, "](")
            && let Some(end) = link_end(&c, mid + 2)
        {
            paint(&mut s, i, a + 1, delimiter);
            paint(&mut s, a + 1, mid, link);
            paint(&mut s, mid, mid + 2, delimiter);
            paint(
                &mut s,
                mid + 2,
                end,
                Style::default()
                    .fg(colors.link_destination.unwrap_or(t.accent))
                    .add_modifier(Modifier::UNDERLINED),
            );
            paint(&mut s, end, end + 1, delimiter);
            i = end + 1;
            continue;
        }
        if c[i] == '<'
            && let Some(end) = find(&c, i + 1, ">")
        {
            let body: String = c[i + 1..end].iter().collect();
            if !body.chars().any(char::is_whitespace)
                && (body.starts_with("https://")
                    || body.starts_with("http://")
                    || body.starts_with("mailto:")
                    || body.contains('@'))
            {
                paint(&mut s, i, i + 1, delimiter);
                paint(&mut s, i + 1, end, link);
                paint(&mut s, end, end + 1, delimiter);
                i = end + 1;
                continue;
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
            paint(
                &mut s,
                a,
                i,
                Style::default().fg(colors.tag.unwrap_or(t.success)),
            );
            continue;
        }
        if c[i] == '|' && table_like && !ctx.comment {
            s[i] = delimiter;
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

pub fn preview(text: &str, t: Theme, settings: &MarkdownSettings) -> Vec<Line<'static>> {
    let mut ctx = Context::default();
    text.split('\n')
        .enumerate()
        .map(|(row, line)| {
            let (c, s) = line_styles(line.trim_end_matches('\r'), row, &mut ctx, t, settings);
            Line::from(
                runs(&s)
                    .map(|(a, b, style)| Span::styled(c[a..b].iter().collect::<String>(), style))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

/// Cache lexical styling independently of cursor, search and focus overlays.
/// Exact text comparison avoids stale results after undo, paste or note switches.
#[derive(Default)]
pub struct HighlightCache {
    entry: Option<CachedStyles>,
}

struct CachedStyles {
    text: String,
    theme: Theme,
    markdown: bool,
    settings: MarkdownSettings,
    lines: Vec<Vec<Style>>,
    words: usize,
}

impl HighlightCache {
    pub fn prepare(
        &mut self,
        text: &str,
        theme: Theme,
        markdown: bool,
        settings: &MarkdownSettings,
    ) {
        if self.entry.as_ref().is_some_and(|entry| {
            entry.text == text
                && entry.theme == theme
                && entry.markdown == markdown
                && entry.settings == *settings
        }) {
            return;
        }
        let mut ctx = Context::default();
        let lines = text
            .split('\n')
            .enumerate()
            .map(|(row, line)| {
                if markdown {
                    line_styles(line, row, &mut ctx, theme, settings).1
                } else {
                    vec![Style::default(); line.chars().count()]
                }
            })
            .collect();
        self.entry = Some(CachedStyles {
            text: text.to_owned(),
            theme,
            markdown,
            settings: settings.clone(),
            lines,
            words: text.split_whitespace().count(),
        });
    }

    pub fn word_count(&self) -> usize {
        self.entry.as_ref().map_or(0, |entry| entry.words)
    }

    pub fn highlights(
        &self,
        search: &[(Index2, Index2)],
        active: usize,
        focus: Option<(std::ops::Range<usize>, f32)>,
    ) -> Vec<Highlight> {
        let Some(entry) = &self.entry else {
            return Vec::new();
        };
        let t = entry.theme;
        let mut result = Vec::new();
        let mut hit = 0;
        for (row, base) in entry.lines.iter().enumerate() {
            // Overlay edits must never mutate cached lexical styles.
            let mut s = base.clone();
            if let Some((range, strength)) = &focus
                && !range.contains(&row)
            {
                for style in &mut s {
                    *style = crate::focus::dim(*style, t, *strength);
                }
            }
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
}

#[cfg(test)]
fn editor_highlights(
    text: &str,
    t: Theme,
    markdown: bool,
    search: &[(Index2, Index2)],
    active: usize,
    focus: Option<(std::ops::Range<usize>, f32)>,
    settings: &MarkdownSettings,
) -> Vec<Highlight> {
    let mut cache = HighlightCache::default();
    cache.prepare(text, t, markdown, settings);
    cache.highlights(search, active, focus)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_invalidates_and_overlays_do_not_modify_base() {
        let mut cache = HighlightCache::default();
        let dark = Theme::named("dark").unwrap();
        let light = Theme::named("light").unwrap();
        let settings = MarkdownSettings::default();
        cache.prepare("# Hello", dark, true, &settings);
        assert_eq!(cache.word_count(), 2);
        let base = cache.entry.as_ref().unwrap().lines.clone();
        let hit = [(Index2::new(0, 2), Index2::new(0, 6))];
        assert!(!cache.highlights(&hit, 1, Some((1..2, 0.5))).is_empty());
        assert_eq!(cache.entry.as_ref().unwrap().lines, base);
        cache.prepare("# Hello", light, true, &settings);
        assert_eq!(cache.entry.as_ref().unwrap().theme, light);
        cache.prepare("new text here", light, false, &settings);
        assert_eq!(cache.word_count(), 3);
        assert!(cache.highlights(&[], 0, None).is_empty());
        let disabled = MarkdownSettings {
            syntax_highlighting: false,
            ..settings
        };
        cache.prepare("# Hello", dark, true, &disabled);
        assert!(cache.highlights(&[], 0, None).is_empty());
    }

    #[test]
    fn heading_levels_are_distinct() {
        for name in ["dark", "light", "midnight"] {
            let t = Theme::named(name).unwrap();
            let colors = (1..=6)
                .map(|n| heading_color(n, t, &MarkdownSettings::default()))
                .collect::<Vec<_>>();
            for i in 0..6 {
                assert!(!colors[..i].contains(&colors[i]));
                let (_, s) = line_styles(
                    &format!("{} Heading", "#".repeat(i + 1)),
                    0,
                    &mut Context::default(),
                    t,
                    &MarkdownSettings::default(),
                );
                assert_eq!(s[0].fg, Some(colors[i]));
            }
        }
    }
    #[test]
    fn source_preserved() {
        let t = Theme::named("dark").unwrap();
        let text = "# β heading\n**bold** and `x`\n```r\n# comment\n```\n";
        let reconstructed = preview(text, t, &MarkdownSettings::default())
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
        let (_, s) = line_styles("β **x**", 0, &mut ctx, t, &MarkdownSettings::default());
        assert_eq!(s[4].fg, Some(t.warning));
        line_styles("```r", 1, &mut ctx, t, &MarkdownSettings::default());
        let (_, s) = line_styles("# heading", 2, &mut ctx, t, &MarkdownSettings::default());
        assert_eq!(s[0].fg, Some(t.muted));
        line_styles("```", 3, &mut ctx, t, &MarkdownSettings::default());
        assert!(ctx.fence.is_none());
    }
}

#[cfg(test)]
mod settings_tests {
    use super::*;
    #[test]
    fn overrides_are_partial_and_apply_to_preview() {
        let settings: MarkdownSettings =
            toml::from_str("[heading_colors]\nh2 = '#123456'").unwrap();
        for name in ["dark", "light"] {
            let t = Theme::named(name).unwrap();
            let lines = preview("# One\n## Two", t, &settings);
            assert_eq!(lines[1].spans[0].style.fg, Some(Color::Rgb(18, 52, 86)));
            assert_eq!(
                lines[0].spans[0].style.fg,
                Some(heading_color(1, t, &MarkdownSettings::default()))
            );
        }
    }
    #[test]
    fn disabled_blocks_stay_plain_and_do_not_leak_context() {
        let t = Theme::named("dark").unwrap();
        let settings = MarkdownSettings {
            highlight_code_blocks: false,
            highlight_frontmatter: false,
            ..Default::default()
        };
        let lines = preview(
            "---\n# yaml comment\n---\n```r\n# code comment\n```\n# Heading",
            t,
            &settings,
        );
        for line in &lines[..6] {
            assert!(line.spans.iter().all(|s| s.style == Style::default()));
        }
        assert_eq!(
            lines[6].spans[0].style.fg,
            Some(heading_color(1, t, &settings))
        );
    }
    #[test]
    fn master_switch_and_delimiters() {
        let t = Theme::named("dark").unwrap();
        let disabled = MarkdownSettings {
            syntax_highlighting: false,
            ..Default::default()
        };
        assert!(editor_highlights("# Heading", t, true, &[], 0, None, &disabled).is_empty());
        let hit = [(Index2::new(0, 0), Index2::new(0, 0))];
        assert!(!editor_highlights("# Heading", t, true, &hit, 1, None, &disabled).is_empty());
        let settings = MarkdownSettings {
            dim_delimiters: false,
            ..Default::default()
        };
        let (_, styles) = line_styles("**bold**", 0, &mut Context::default(), t, &settings);
        assert_eq!(styles[0].fg, Some(t.foreground));
        assert_eq!(styles[2].fg, Some(t.warning));
    }
}

#[cfg(test)]
mod expanded_markdown_tests {
    use super::*;

    fn styles(line: &str) -> Vec<Style> {
        line_styles(
            line,
            1,
            &mut Context::default(),
            Theme::named("dark").unwrap(),
            &MarkdownSettings::default(),
        )
        .1
    }

    #[test]
    fn nested_emphasis_and_unicode() {
        let s = styles("β **outer *inner***");
        assert!(s[4].add_modifier.contains(Modifier::BOLD));
        assert!(
            s[11]
                .add_modifier
                .contains(Modifier::BOLD | Modifier::ITALIC)
        );
        assert_eq!(s.len(), "β **outer *inner***".chars().count());
    }

    #[test]
    fn opaque_code_and_intraword_underscores() {
        let s = styles("`**x**` foo_bar_baz");
        assert!(!s[3].add_modifier.contains(Modifier::BOLD));
        assert_eq!(s[12], Style::default());
        assert!(
            styles("unclosed **text")
                .iter()
                .all(|s| *s == Style::default())
        );
    }

    #[test]
    fn balanced_link_and_plain_pipe() {
        let s = styles("[x](a(b)c)");
        assert!(s[8].add_modifier.contains(Modifier::UNDERLINED));
        assert_eq!(styles("a | b")[2], Style::default());
    }

    #[test]
    fn comments_and_math_are_opaque() {
        let s = styles("<!-- **x** -->");
        assert!(s[6].add_modifier.contains(Modifier::ITALIC));
        assert!(!s[6].add_modifier.contains(Modifier::BOLD));
        assert_eq!(styles("$5 and $10")[0], Style::default());
    }
}

#[cfg(test)]
mod expanded_yaml_tests {
    use super::*;

    fn check(doc: &str, row: usize, token: &str, color: Color, modifier: Modifier) {
        let theme = Theme::named("dark").unwrap();
        let settings = MarkdownSettings::default();
        let mut ctx = Context::default();
        for (index, line) in doc.split('\n').enumerate() {
            let (_, styles) = line_styles(line, index, &mut ctx, theme, &settings);
            if index == row {
                let offset = line.find(token).expect("token exists");
                let start = line[..offset].chars().count();
                let end = start + token.chars().count();
                for style in &styles[start..end] {
                    assert_eq!(style.fg, Some(color), "{token}: {line}");
                    assert!(style.add_modifier.contains(modifier));
                }
                return;
            }
        }
        panic!("row missing");
    }

    #[test]
    fn scalar_types_and_unicode_keys() {
        let t = Theme::named("dark").unwrap();
        let doc = "---\nβ title: sample 123\nid: 202611251730\nkd: 1e-8\nactive: false\nmissing: null\ndate: 2026-11-25\nquoted: \"false # text\"\n---";
        for (row, token, color) in [
            (1, "β title", t.directory),
            (1, "sample 123", t.success),
            (2, "202611251730", t.warning),
            (3, "1e-8", t.warning),
            (4, "false", t.accent),
            (5, "null", t.accent),
            (6, "2026-11-25", t.warning),
            (7, "false # text", t.success),
        ] {
            check(doc, row, token, color, Modifier::empty());
        }
    }

    #[test]
    fn screenshot_regressions() {
        let t = Theme::named("dark").unwrap();
        let doc = "---\nupdated: 2026-11-25T17:30:00-05:00\nempty: ~\nnegative: -12\nhex: 0xFF\n\"special:key\": value\nmetadata: {species: rhesus, active: true, count: 3}\ntags:\n  - antibody\n---";
        for (row, token, color) in [
            (1, "2026-11-25T17:30:00-05:00", t.warning),
            (2, "~", t.accent),
            (3, "-12", t.warning),
            (4, "0xFF", t.warning),
            (5, "\"special:key\"", t.directory),
            (6, "species", t.directory),
            (6, "active", t.directory),
            (6, "count", t.directory),
            (8, "-", t.warning),
        ] {
            check(doc, row, token, color, Modifier::empty());
        }
    }

    #[test]
    fn flow_collections_urls_and_comments() {
        let t = Theme::named("dark").unwrap();
        let doc = "---\nurl: https://example.org/a#b # comment\nitems: [true, 1.25, 'it''s fine', {name: β}]\nanchor: &base value\ncopy: *base\n---";
        check(
            doc,
            1,
            "https://example.org/a#b",
            t.success,
            Modifier::empty(),
        );
        check(doc, 1, "# comment", t.muted, Modifier::ITALIC);
        for (row, token, color) in [
            (2, "true", t.accent),
            (2, "1.25", t.warning),
            (2, "it''s fine", t.success),
            (2, "name", t.directory),
            (2, "β", t.success),
            (3, "&base", t.accent),
            (4, "*base", t.accent),
        ] {
            check(doc, row, token, color, Modifier::empty());
        }
    }

    #[test]
    fn multiline_scalars_and_frontmatter_boundary() {
        let t = Theme::named("dark").unwrap();
        let doc = "---\nnotes: |-\n  # literal comment\n  ---\n  **literal**\nnext: >\n  text: literal\nquoted: \"first\n  # second\"\n...\n# Heading\n---";
        for (row, token) in [
            (2, "# literal comment"),
            (3, "---"),
            (4, "**literal**"),
            (6, "text: literal"),
            (8, "# second"),
        ] {
            check(doc, row, token, t.success, Modifier::empty());
        }
        check(doc, 5, "next", t.directory, Modifier::empty());
        check(
            doc,
            10,
            "Heading",
            heading_color(1, t, &MarkdownSettings::default()),
            Modifier::BOLD,
        );
        check(doc, 11, "---", t.muted, Modifier::empty());
        let restored = preview(doc, t, &MarkdownSettings::default())
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(restored, doc);
    }
}

#[cfg(test)]
mod expanded_code_tests {
    use super::*;
    #[test]
    fn numbers_and_language_keywords() {
        let t = Theme::named("dark").unwrap();
        let mut ctx = Context::default();
        let settings = MarkdownSettings::default();
        line_styles("```{r, echo=false}", 0, &mut ctx, t, &settings);
        let (_, s) = line_styles("let <- 1e-8", 1, &mut ctx, t, &settings);
        assert_eq!(s[0], Style::default());
        assert!(s[7..].iter().all(|style| style.fg == Some(t.warning)));
        let (_, s) = line_styles("print(TRUE)", 2, &mut ctx, t, &settings);
        assert_eq!(s[0].fg, Some(t.directory));
        assert!(s[6].add_modifier.contains(Modifier::BOLD));
    }
    #[test]
    fn multiline_state_and_fence_reset() {
        let t = Theme::named("dark").unwrap();
        let mut ctx = Context::default();
        let settings = MarkdownSettings::default();
        let doc = "```python\nx = \"\"\"hello\n# string\nend\"\"\"\n```\n```rust\n/* comment\nlet x = 1;\n*/ let x = 2;\n```\n```unknown\nlet x = 1\n```";
        let lines: Vec<_> = doc
            .lines()
            .enumerate()
            .map(|(i, l)| line_styles(l, i, &mut ctx, t, &settings).1)
            .collect();
        assert_eq!(lines[2][0].fg, Some(t.success));
        assert!(lines[7][0].add_modifier.contains(Modifier::ITALIC));
        assert_eq!(lines[8][3].fg, Some(t.accent));
        assert!(lines[11].iter().all(|s| *s == Style::default()));
        assert!(ctx.fence.is_none());
    }
}

#[cfg(test)]
mod fenced_integration_tests {
    use super::*;
    #[test]
    fn r_fence_variants() {
        let t = Theme::named("dark").unwrap();
        let settings = MarkdownSettings::default();
        for fence in ["```r", "```{r}", "```{r, echo=false}"] {
            let mut ctx = Context::default();
            line_styles(fence, 0, &mut ctx, t, &settings);
            let (_, s) = line_styles("print(1e-8)", 1, &mut ctx, t, &settings);
            assert_eq!(s[0].fg, Some(t.directory));
            assert!(s[6..10].iter().all(|s| s.fg == Some(t.warning)));
        }
    }
    #[test]
    fn yaml_and_c_fences() {
        let t = Theme::named("dark").unwrap();
        let settings = MarkdownSettings::default();
        let mut ctx = Context::default();
        line_styles("```yaml", 0, &mut ctx, t, &settings);
        let (_, s) = line_styles("url: https://example.org/a#b", 1, &mut ctx, t, &settings);
        assert_eq!(s[0].fg, Some(t.directory));
        assert!(s[5..].iter().all(|s| s.fg == Some(t.success)));
        line_styles("```", 2, &mut ctx, t, &settings);
        for fence in ["```c", "```cpp", "```c++"] {
            line_styles(fence, 3, &mut ctx, t, &settings);
            let (_, s) = line_styles("int main() { return 0; }", 4, &mut ctx, t, &settings);
            assert!(s[0].add_modifier.contains(Modifier::BOLD));
            assert_eq!(s[4].fg, Some(t.directory));
            assert!(s[13].add_modifier.contains(Modifier::BOLD));
            line_styles("```", 5, &mut ctx, t, &settings);
        }
    }
}

#[cfg(test)]
mod syntax_color_tests {
    use super::*;

    #[test]
    fn partial_overrides_and_validation() {
        let settings: MarkdownSettings = toml::from_str(
            "[syntax_colors]\ninline_code = '#123456'\n[yaml_colors]\nnumber = '#654321'\n[code_colors]\nkeyword = '#112233'",
        ).unwrap();
        let t = Theme::named("dark").unwrap();
        let doc = "---\ncount: 123\n---\n`code` **bold**\n```rust\nlet x = 123;\n```";
        let mut ctx = Context::default();
        let rows: Vec<_> = doc
            .lines()
            .enumerate()
            .map(|(i, l)| line_styles(l, i, &mut ctx, t, &settings).1)
            .collect();
        assert_eq!(rows[1][7].fg, Some(Color::Rgb(101, 67, 33)));
        assert_eq!(rows[3][1].fg, Some(Color::Rgb(18, 52, 86)));
        assert_eq!(rows[3][9].fg, Some(t.warning));
        assert_eq!(rows[5][0].fg, Some(Color::Rgb(17, 34, 51)));
        assert_eq!(rows[5][8].fg, Some(t.warning));
        assert!(toml::from_str::<MarkdownSettings>("[code_colors]\nnumber = 'red'").is_err());
        assert!(toml::from_str::<MarkdownSettings>("[yaml_colors]\nunknown = '#123456'").is_err());
    }

    #[test]
    fn duplicate_native_headings_and_inline_code() {
        let mut t = Theme::named("dark").unwrap();
        t.heading_colors = [Some(t.accent); 6];
        let settings = MarkdownSettings::default();
        let colors: Vec<_> = (1..=6).map(|n| heading_color(n, t, &settings)).collect();
        for (i, color) in colors.iter().enumerate() {
            assert!(!colors[..i].contains(color));
        }
        let (_, s) = line_styles("**bold** `code`", 1, &mut Context::default(), t, &settings);
        assert_ne!(s[2].fg, s[10].fg);
    }

    #[test]
    fn yaml_fences_inherit_yaml_colors_and_cache_invalidates() {
        let t = Theme::named("dark").unwrap();
        let mut settings = MarkdownSettings::default();
        let text = "```yaml\nactive: true\n```";
        let mut cache = HighlightCache::default();
        cache.prepare(text, t, true, &settings);
        settings.yaml_colors.boolean = Some(Color::Rgb(1, 2, 3));
        cache.prepare(text, t, true, &settings);
        assert_eq!(
            cache.entry.as_ref().unwrap().lines[1][8].fg,
            settings.yaml_colors.boolean
        );
        let lines = preview(text, t, &settings);
        assert!(
            lines[1]
                .spans
                .iter()
                .any(|s| s.style.fg == settings.yaml_colors.boolean)
        );
    }
}

//! Stateful YAML highlighting for frontmatter and fenced blocks.

use super::{MarkdownSettings, YamlColors, delimiter_style, paint};
use crate::theme::Theme;
use ratatui::style::{Modifier, Style};

/// State for YAML block scalars and flow collections; independent of Markdown.
#[derive(Default)]
pub(super) struct YamlContext {
    block_indent: Option<usize>,
    flow_depth: usize,
    quote: Option<char>,
}

fn yaml_colon(c: &[char], i: usize, flow: bool) -> bool {
    c[i] == ':'
        && (c.get(i + 1).is_none_or(|ch| {
            ch.is_whitespace() || (flow && matches!(*ch, ',' | ']' | '}' | '[' | '{'))
        }) || (flow && i > 0 && matches!(c[i - 1], '\'' | '"')))
}

fn yaml_scalar_style(value: &str, t: Theme, colors: &YamlColors) -> Style {
    let lower = value.to_ascii_lowercase();
    let numeric = value.replace('_', "");
    let unsigned = numeric.trim_start_matches(['+', '-']);
    let radix_number = [("0x", 16), ("0o", 8), ("0b", 2)]
        .iter()
        .any(|(prefix, radix)| {
            unsigned.strip_prefix(*prefix).is_some_and(|digits| {
                !digits.is_empty() && digits.chars().all(|ch| ch.is_digit(*radix))
            })
        });
    let bytes = value.as_bytes();
    let date = bytes.len() >= 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && (bytes.len() == 10 || matches!(bytes[10], b'T' | b't' | b' '));
    if matches!(lower.as_str(), "true" | "false" | "null" | "~") {
        Style::default()
            .fg(if matches!(lower.as_str(), "null" | "~") {
                colors.null.unwrap_or(t.accent)
            } else {
                colors.boolean.unwrap_or(t.accent)
            })
            .add_modifier(Modifier::BOLD)
    } else if numeric.parse::<f64>().is_ok() && unsigned.chars().any(|ch| ch.is_ascii_digit())
        || radix_number
        || date
        || matches!(lower.as_str(), ".inf" | "+.inf" | "-.inf" | ".nan")
    {
        Style::default().fg(colors.number.unwrap_or(t.warning))
    } else {
        Style::default().fg(colors.string.unwrap_or(t.success))
    }
}

fn yaml_quote_end(c: &[char], mut i: usize, quote: char) -> Option<usize> {
    while i < c.len() {
        if quote == '"' && c[i] == '\\' {
            i += 2;
        } else if c[i] == quote {
            if quote == '\'' && c.get(i + 1) == Some(&quote) {
                i += 2;
            } else {
                return Some(i + 1);
            }
        } else {
            i += 1;
        }
    }
    None
}

pub(super) fn yaml_line(
    c: &[char],
    s: &mut [Style],
    ctx: &mut YamlContext,
    t: Theme,
    settings: &MarkdownSettings,
) {
    let colors = &settings.yaml_colors;
    let delimiter = delimiter_style(t, settings.dim_delimiters, colors.delimiter);
    let n = c.len();
    let start = c.iter().position(|ch| !ch.is_whitespace()).unwrap_or(n);
    let string = Style::default().fg(colors.string.unwrap_or(t.success));
    let key = Style::default().fg(colors.key.unwrap_or(t.directory));
    let marker = Style::default().fg(colors.marker.unwrap_or(t.warning));
    let reference = Style::default().fg(colors.reference.unwrap_or(t.accent));
    let comment = Style::default()
        .fg(colors.comment.unwrap_or(t.muted))
        .add_modifier(Modifier::ITALIC);
    if let Some(indent) = ctx.block_indent {
        if start == n || start > indent {
            paint(s, start, n, string);
            return;
        }
        ctx.block_indent = None;
    }
    let mut i = start;
    let mut parent_indent = start;
    if let Some(quote) = ctx.quote {
        if let Some(end) = yaml_quote_end(c, i, quote) {
            paint(s, i, end, string);
            ctx.quote = None;
            i = end;
        } else {
            paint(s, i, n, string);
            return;
        }
    }
    while i < n {
        if c[i].is_whitespace() {
            i += 1;
            continue;
        }
        if c[i] == '#' {
            paint(s, i, n, comment);
            break;
        }
        let flow = ctx.flow_depth > 0;
        if !flow && matches!(c[i], '-' | '?') && c.get(i + 1).is_none_or(|ch| ch.is_whitespace()) {
            s[i] = marker;
            i += 1;
            parent_indent = c[i..]
                .iter()
                .position(|ch| !ch.is_whitespace())
                .map_or(parent_indent, |offset| i + offset);
            // A direct sequence scalar is indented relative to the dash, not its content.
            if c.get(parent_indent)
                .is_some_and(|ch| matches!(*ch, '|' | '>'))
            {
                parent_indent = start;
            }
            continue;
        }
        if matches!(c[i], '[' | '{') {
            ctx.flow_depth += 1;
            s[i] = delimiter;
            i += 1;
            continue;
        }
        if flow && matches!(c[i], ']' | '}' | ',') {
            if c[i] != ',' {
                ctx.flow_depth -= 1;
            }
            s[i] = delimiter;
            i += 1;
            continue;
        }
        if yaml_colon(c, i, flow) {
            s[i] = delimiter;
            i += 1;
            continue;
        }
        if matches!(c[i], '\'' | '"') {
            let quote = c[i];
            let end = yaml_quote_end(c, i + 1, quote);
            let stop = end.unwrap_or(n);
            let next = stop + c[stop..].iter().take_while(|ch| ch.is_whitespace()).count();
            let is_key = next < n && yaml_colon(c, next, flow);
            paint(s, i, stop, if is_key { key } else { string });
            if end.is_none() {
                ctx.quote = Some(quote);
            }
            i = stop;
            continue;
        }
        if matches!(c[i], '&' | '*' | '!') {
            let a = i;
            i += 1;
            while i < n && !c[i].is_whitespace() && !matches!(c[i], ',' | '[' | ']' | '{' | '}') {
                i += 1;
            }
            paint(s, a, i, reference);
            continue;
        }
        if !flow && matches!(c[i], '|' | '>') {
            let end = i
                + 1
                + c[i + 1..]
                    .iter()
                    .take_while(|&&ch| matches!(ch, '+' | '-' | '1'..='9'))
                    .count();
            if c.get(end).is_none_or(|ch| ch.is_whitespace()) {
                paint(s, i, end, marker);
                ctx.block_indent = Some(parent_indent);
                i = end;
                continue;
            }
        }
        let a = i;
        i += 1;
        while i < n {
            if yaml_colon(c, i, flow)
                || (flow && matches!(c[i], ',' | ']' | '}'))
                || (c[i] == '#' && c[i - 1].is_whitespace())
            {
                break;
            }
            i += 1;
        }
        let end = a + c[a..i]
            .iter()
            .rposition(|ch| !ch.is_whitespace())
            .map_or(0, |p| p + 1);
        let value: String = c[a..end].iter().collect();
        let style = if i < n && yaml_colon(c, i, flow) {
            key
        } else {
            yaml_scalar_style(&value, t, colors)
        };
        paint(s, a, end, style);
    }
}

//! Stateful syntax highlighting for fenced code blocks.

use super::yaml::{YamlContext, yaml_line};
use super::{MarkdownSettings, escaped, paint, starts};
use crate::theme::Theme;
use ratatui::style::{Modifier, Style};

#[derive(Default)]
pub(super) struct CodeContext {
    block: Option<&'static str>,
    quote: Option<(&'static str, bool)>,
    yaml: YamlContext,
}

fn code_number(c: &[char], start: usize) -> usize {
    let mut i = start;
    if c.get(i) == Some(&'0') {
        let radix = match c.get(i + 1) {
            Some('x' | 'X') => 16,
            Some('b' | 'B') => 2,
            Some('o' | 'O') => 8,
            _ => 0,
        };
        if radix != 0 && c.get(i + 2).is_some_and(|ch| ch.is_digit(radix)) {
            i += 2;
            while c.get(i).is_some_and(|ch| ch.is_digit(radix) || *ch == '_') {
                i += 1;
            }
            return i;
        }
    }
    while c.get(i).is_some_and(|ch| ch.is_ascii_digit() || *ch == '_') {
        i += 1;
    }
    if c.get(i) == Some(&'.') && c.get(i + 1) != Some(&'.') {
        i += 1;
        while c.get(i).is_some_and(|ch| ch.is_ascii_digit() || *ch == '_') {
            i += 1;
        }
    }
    if matches!(c.get(i), Some('e' | 'E')) {
        let mut j = i + 1;
        if matches!(c.get(j), Some('+' | '-')) {
            j += 1;
        }
        let digits = j;
        while c.get(j).is_some_and(|ch| ch.is_ascii_digit() || *ch == '_') {
            j += 1;
        }
        if j > digits {
            i = j;
        }
    }
    i
}

pub(super) fn code(
    c: &[char],
    styles: &mut [Style],
    lang: &str,
    t: Theme,
    ctx: &mut CodeContext,
    settings: &MarkdownSettings,
) {
    let lang = match lang {
        "rs" => "rust",
        "py" => "python",
        "sh" | "shell" | "zsh" => "bash",
        "js" => "javascript",
        "ts" => "typescript",
        "yml" => "yaml",
        "tex" => "latex",
        "c++" | "cc" | "cxx" => "cpp",
        other => other,
    };
    if lang == "yaml" {
        yaml_line(c, styles, &mut ctx.yaml, t, settings);
        return;
    }
    let keywords = match lang {
        "rust" => {
            "let mut fn pub impl struct enum use mod match if else for while loop return const static async await move trait type where as in break continue crate self Self super ref unsafe dyn extern union"
        }
        "python" => {
            "def class import from pass yield lambda try except finally with raise if elif else for while return break continue as in is not and or assert del global nonlocal async await match case"
        }
        "r" => "function if else for while repeat in next break return",
        "bash" => {
            "if then elif else fi for while until do done case esac function in select export local readonly return break continue declare set unset"
        }
        "javascript" | "typescript" => {
            "function let const var class extends import export from new return if else for while switch case break continue try catch finally throw async await typeof instanceof interface type implements public private readonly enum"
        }
        "sql" => {
            "select from where join inner left right outer on group by order having insert into values update set delete create table alter drop distinct as limit offset union all and or not is in case when then else end with asc desc"
        }
        "c" | "cpp" => {
            "int char float double void bool long short unsigned signed const static struct enum union typedef if else for while do switch case break continue return sizeof class namespace template typename public private protected using auto new delete virtual override constexpr include define"
        }
        "typst" => {
            "let set show import include if else for while in return break continue context as"
        }
        "latex" | "json" | "toml" | "html" | "css" => "",
        _ => return,
    };
    let constants = match lang {
        "python" => "True False None",
        "r" => "TRUE FALSE NULL NA NaN Inf NA_integer_ NA_real_ NA_character_",
        "rust" => "true false Some None Ok Err",
        "javascript" | "typescript" => "true false null undefined NaN Infinity",
        "json" | "toml" | "c" | "cpp" => "true false null nullptr NULL",
        "sql" => "true false null",
        "typst" => "true false none auto",
        _ => "",
    };
    let colors = &settings.code_colors;
    let comment = Style::default()
        .fg(colors.comment.unwrap_or(t.muted))
        .add_modifier(Modifier::ITALIC);
    let string = Style::default().fg(colors.string.unwrap_or(t.success));
    let number = Style::default().fg(colors.number.unwrap_or(t.warning));
    let keyword = Style::default()
        .fg(colors.keyword.unwrap_or(t.accent))
        .add_modifier(Modifier::BOLD);
    let constant = Style::default()
        .fg(colors.constant.unwrap_or(t.accent))
        .add_modifier(Modifier::BOLD);
    let function = Style::default().fg(colors.function.unwrap_or(t.directory));
    let operator = Style::default().fg(colors.operator.unwrap_or(t.accent));
    let slash = matches!(
        lang,
        "rust" | "javascript" | "typescript" | "c" | "cpp" | "typst"
    );
    let mut i = 0;
    while i < c.len() {
        if let Some(end) = ctx.block {
            let stop = (i..c.len())
                .find(|&j| starts(c, j, end))
                .map(|j| j + end.len());
            paint(styles, i, stop.unwrap_or(c.len()), comment);
            if let Some(stop) = stop {
                ctx.block = None;
                i = stop;
                continue;
            }
            break;
        }
        if let Some((end, multiline)) = ctx.quote {
            let a = i;
            let mut closed = false;
            while i < c.len() {
                if c[i] == '\\' {
                    i = (i + 2).min(c.len());
                    continue;
                }
                if starts(c, i, end) {
                    i += end.len();
                    closed = true;
                    break;
                }
                i += 1;
            }
            paint(styles, a, i, string);
            if closed || !multiline {
                ctx.quote = None;
            }
            continue;
        }
        if c[i].is_whitespace() {
            i += 1;
            continue;
        }
        let block = if (slash || matches!(lang, "sql" | "css")) && starts(c, i, "/*") {
            Some(("/*", "*/"))
        } else if lang == "html" && starts(c, i, "<!--") {
            Some(("<!--", "-->"))
        } else {
            None
        };
        if let Some((open, end)) = block {
            paint(styles, i, i + open.len(), comment);
            i += open.len();
            ctx.block = Some(end);
            continue;
        }
        if (matches!(lang, "python" | "r" | "toml") && c[i] == '#')
            || (lang == "bash"
                && c[i] == '#'
                && (i == 0 || c[i - 1].is_whitespace() || matches!(c[i - 1], ';' | '|')))
            || (slash && starts(c, i, "//"))
            || (lang == "sql" && starts(c, i, "--"))
            || (lang == "latex" && c[i] == '%' && !escaped(c, i))
        {
            paint(styles, i, c.len(), comment);
            break;
        }
        let quote = if matches!(lang, "python" | "toml") && starts(c, i, "\"\"\"") {
            Some(("\"\"\"", true))
        } else if matches!(lang, "python" | "toml") && starts(c, i, "'''") {
            Some(("'''", true))
        } else if c[i] == '"' {
            Some(("\"", matches!(lang, "rust" | "bash" | "r")))
        } else if c[i] == '\''
            && !matches!(lang, "typst" | "latex" | "json")
            && (lang != "rust"
                || c.get(i + 2) == Some(&'\'')
                || (c.get(i + 1) == Some(&'\\') && c.get(i + 3) == Some(&'\'')))
        {
            Some(("'", lang == "bash" || lang == "r"))
        } else if c[i] == '`' && matches!(lang, "javascript" | "typescript" | "r" | "bash") {
            Some(("`", true))
        } else {
            None
        };
        if let Some((end, multiline)) = quote {
            paint(styles, i, i + end.len(), string);
            i += end.len();
            ctx.quote = Some((end, multiline));
            continue;
        }
        if c[i].is_ascii_digit() || (c[i] == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let end = code_number(c, i);
            paint(styles, i, end, number);
            i = end;
            continue;
        }
        if lang == "latex" && c[i] == '\\' {
            let a = i;
            i += 1;
            while c.get(i).is_some_and(|ch| ch.is_alphabetic()) {
                i += 1;
            }
            if i == a + 1 && i < c.len() {
                i += 1;
            }
            paint(styles, a, i, keyword);
            continue;
        }
        if c[i].is_alphabetic() || c[i] == '_' {
            let a = i;
            i += 1;
            while c
                .get(i)
                .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_' || (lang == "r" && *ch == '.'))
            {
                i += 1;
            }
            let word: String = c[a..i].iter().collect();
            let lookup = if lang == "sql" {
                word.to_ascii_lowercase()
            } else {
                word
            };
            if keywords.split_whitespace().any(|k| k == lookup) {
                paint(styles, a, i, keyword);
            } else if constants.split_whitespace().any(|k| k == lookup) {
                paint(styles, a, i, constant);
            } else if c[i..].iter().find(|ch| !ch.is_whitespace()) == Some(&'(') {
                paint(styles, a, i, function);
            }
            continue;
        }
        if "+-*/%=!<>|&^~:$#".contains(c[i]) {
            styles[i] = operator;
        }
        i += 1;
    }
    if ctx.quote.is_some_and(|(_, multiline)| !multiline) {
        ctx.quote = None;
    }
}

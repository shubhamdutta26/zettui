//! Shared popup chrome and rendering for searchable pickers and link dialogs.

use super::super::{App, display_component, tail_label};
use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap},
};

/// Shared popup chrome. All dimensions are terminal cells; small screens retain
/// a visible list row and a compact help line whenever possible.
fn popup_inner(
    frame: &mut Frame,
    theme: Theme,
    title: &str,
    max_width: u16,
    max_height: u16,
) -> Option<Rect> {
    let screen = frame.area();
    let width = screen.width.saturating_sub(2).min(max_width);
    let height = screen.height.saturating_sub(2).min(max_height);
    if width < 12 || height < 5 {
        return None;
    }
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(format!(" {title} "))
        .padding(Padding::horizontal(1))
        .style(Style::default().fg(theme.foreground).bg(theme.surface))
        .border_style(Style::default().fg(theme.accent))
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    Some(inner)
}

fn popup_help(frame: &mut Frame, theme: Theme, area: Rect, controls: &[(&str, &str)]) {
    let mut spans = Vec::new();
    for (i, (key, description)) in controls.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {description}"),
            Style::default().fg(theme.muted),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true }),
        area,
    );
}

fn popup_list(
    frame: &mut Frame,
    theme: Theme,
    area: Rect,
    items: Vec<ListItem<'_>>,
    selected: usize,
    empty: &str,
) {
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(empty)
                .style(Style::default().fg(theme.muted))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let mut state = ListState::default();
    state.select(Some(selected.min(items.len() - 1)));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_symbol("▏ ")
            .highlight_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg)),
        area,
        &mut state,
    );
}

/// Draw a query, result count, and controls; return the remaining list area.
fn picker_body(
    frame: &mut Frame,
    theme: Theme,
    inner: Rect,
    label: &str,
    query: &str,
    metadata: &str,
    controls: &[(&str, &str)],
) -> Rect {
    let rich = inner.height >= 7;
    let header_height = if rich {
        4
    } else if inner.height >= 4 {
        2
    } else {
        1
    };
    let footer_height = if rich { 2 } else { 1 };
    let areas = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(1),
        Constraint::Length(footer_height),
    ])
    .split(inner);
    let field_height = if rich { 3 } else { 1 };
    let header =
        Layout::vertical([Constraint::Length(field_height), Constraint::Min(0)]).split(areas[0]);
    let value = tail_label(
        &format!("{query}▏"),
        usize::from(header[0].width.saturating_sub(if rich { 2 } else { 0 })),
    );
    let field =
        Paragraph::new(value).style(Style::default().fg(theme.foreground).bg(theme.background));
    if rich {
        frame.render_widget(
            field.block(
                Block::bordered()
                    .title(format!(" {label} "))
                    .border_style(Style::default().fg(theme.border))
                    .title_style(Style::default().fg(theme.accent)),
            ),
            header[0],
        );
    } else {
        frame.render_widget(field, header[0]);
    }
    frame.render_widget(
        Paragraph::new(metadata).style(Style::default().fg(theme.muted)),
        header[1],
    );
    popup_help(frame, theme, areas[2], controls);
    areas[1]
}

impl App {
    /// Display links on demand. Only this dialog captures j/k/Enter/Esc.
    pub(super) fn draw_link_dialog(&mut self, frame: &mut Frame) {
        let Some(dialog) = &self.links else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, &dialog.title, 94, 18) else {
            return;
        };
        let areas = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);
        frame.render_widget(
            Paragraph::new(format!(
                "{} links · ! missing or ambiguous",
                dialog.items.len()
            ))
            .style(Style::default().fg(theme.muted)),
            areas[0],
        );
        let items = dialog
            .items
            .iter()
            .map(|item| {
                let (prefix, color) = if item.target.is_some() {
                    ("  ", theme.foreground)
                } else {
                    ("! ", theme.warning)
                };
                ListItem::new(Line::styled(
                    format!("{prefix}{}", item.label),
                    Style::default().fg(color),
                ))
            })
            .collect();
        popup_list(
            frame,
            theme,
            areas[1],
            items,
            dialog.selected,
            "No links found.",
        );
        popup_help(
            frame,
            theme,
            areas[2],
            &[("Esc", "Close"), ("j/k", "Select"), ("Enter", "Open")],
        );
    }

    pub(super) fn draw_citation_picker(&self, frame: &mut Frame) {
        let Some(dialog) = &self.citation_picker else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, "Zotero citations", 110, 24) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            "Search references",
            &dialog.query,
            &format!("{} results · max 200", dialog.matches.len()),
            &[
                ("Esc", "Cancel"),
                ("↑↓", "Select"),
                ("Enter", "Insert"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = dialog
            .matches
            .iter()
            .map(|&index| {
                let reference = &dialog.bibliography.references[index];
                let title =
                    Line::styled(reference.summary(), Style::default().fg(theme.foreground));
                if area.height >= 2 {
                    ListItem::new(vec![
                        title,
                        Line::styled(
                            format!("  @{}", reference.key),
                            Style::default().fg(theme.muted),
                        ),
                    ])
                } else {
                    ListItem::new(title)
                }
            })
            .collect();
        popup_list(
            frame,
            theme,
            area,
            items,
            dialog.selected,
            "No matching citations.",
        );
    }

    pub(super) fn draw_wiki_completion(&self, frame: &mut Frame) {
        let Some(dialog) = &self.wiki_completion else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, "Link a note — [[", 100, 20) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            "Search notes",
            &dialog.query,
            &format!("{} matches · max 200", dialog.matches.len()),
            &[
                ("Esc", "Cancel (keeps [[)"),
                ("↑↓", "Select"),
                ("Enter", "Insert"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = dialog
            .matches
            .iter()
            .map(|&index| {
                let item = &dialog.suggestions[index];
                ListItem::new(Line::styled(
                    item.label.clone(),
                    Style::default().fg(theme.foreground),
                ))
            })
            .collect();
        popup_list(
            frame,
            theme,
            area,
            items,
            dialog.selected,
            "No matching notes.",
        );
    }

    pub(super) fn draw_snippet_picker(&self, frame: &mut Frame) {
        let Some(dialog) = &self.snippet_picker else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, "Apply snippet", 100, 22) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            "Search snippets",
            &dialog.query,
            &format!("{} snippets", dialog.matches.len()),
            &[
                ("Esc", "Cancel"),
                ("↑↓", "Select"),
                ("Enter", "Choose"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = dialog
            .matches
            .iter()
            .map(|&index| ListItem::new(dialog.items[index].name.clone()))
            .collect();
        if area.width >= 60 && area.height >= 5 {
            let columns =
                Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(area);
            popup_list(
                frame,
                theme,
                columns[0],
                items,
                dialog.selected,
                "No matching snippets.",
            );
            let preview = dialog
                .matches
                .get(dialog.selected)
                .and_then(|&index| dialog.items.get(index))
                .map(|item| item.source.as_str())
                .unwrap_or("");
            frame.render_widget(
                Paragraph::new(preview)
                    .wrap(Wrap { trim: false })
                    .style(Style::default().fg(theme.foreground))
                    .block(
                        Block::bordered()
                            .title(" Template preview ")
                            .border_style(Style::default().fg(theme.border)),
                    ),
                columns[1],
            );
        } else {
            popup_list(
                frame,
                theme,
                area,
                items,
                dialog.selected,
                "No matching snippets.",
            );
        }
    }

    pub(super) fn draw_snippet_form(&self, frame: &mut Frame) {
        let Some(form) = &self.snippet_form else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(
            frame,
            theme,
            &format!("Snippet: {}", form.snippet.name),
            88,
            18,
        ) else {
            return;
        };
        let areas = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(if inner.height >= 6 { 2 } else { 1 }),
        ])
        .split(inner);
        frame.render_widget(
            Paragraph::new(format!(
                "Field {} of {} · Type to replace default",
                (form.active + 1).min(form.fields.len()),
                form.fields.len()
            ))
            .style(Style::default().fg(theme.muted)),
            areas[0],
        );
        let items = form
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let label = format!("${}: ", field.index);
                let value = if i == form.active {
                    tail_label(
                        &format!("{}▏", field.value),
                        usize::from(areas[1].width)
                            .saturating_sub(2 + Span::raw(label.as_str()).width()),
                    )
                } else {
                    field.value.clone()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(label, Style::default().fg(theme.accent)),
                    Span::raw(value),
                ]))
            })
            .collect();
        popup_list(frame, theme, areas[1], items, form.active, "No fields.");
        popup_help(
            frame,
            theme,
            areas[2],
            &[
                ("Esc", "Cancel"),
                ("Tab/Enter", "Next/apply"),
                ("Shift+Tab", "Previous"),
                ("Ctrl+U", "Clear"),
            ],
        );
    }

    pub(super) fn draw_buffer_picker(&self, frame: &mut Frame) {
        let Some(picker) = &self.buffer_picker else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, "Open buffers", 92, 25) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            "Search buffers",
            &picker.query,
            &format!(
                "{} matches · {} open",
                picker.matches.len(),
                self.editor.len()
            ),
            &[
                ("Esc/F5", "Cancel"),
                ("↑↓", "Select"),
                ("Enter", "Switch"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = picker
            .matches
            .iter()
            .map(|&i| {
                let note = &self.editor.notes()[i];
                let path = note
                    .path
                    .strip_prefix(&self.root)
                    .unwrap_or(&note.path)
                    .to_string_lossy();
                ListItem::new(format!(
                    "{} {}{} {}",
                    if self.editor.active_index() == Some(i) {
                        "•"
                    } else {
                        " "
                    },
                    i + 1,
                    if note.is_dirty() { " [+]" } else { "" },
                    display_component(&path)
                ))
            })
            .collect();
        popup_list(
            frame,
            theme,
            area,
            items,
            picker.selected,
            "No matching buffers.",
        );
    }

    pub(super) fn draw_command_palette(&self, frame: &mut Frame) {
        let Some(palette) = &self.command_palette else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, "Command palette", 92, 25) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            "Search commands",
            &palette.query,
            &format!(
                "{} matches · {} available",
                palette.matches.len(),
                palette.items.len()
            ),
            &[
                ("Esc/F8", "Cancel"),
                ("↑↓", "Select"),
                ("Enter", "Run"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = palette
            .matches
            .iter()
            .map(|&index| {
                let item = palette.items[index];
                // Reserve space for the label first; hints are secondary on small screens.
                let title = Span::raw(item.title);
                let hint = format!("  {}", item.shortcut);
                let width = title.width() + Span::raw(hint.as_str()).width();
                let mut spans = vec![title];
                if width + 2 <= usize::from(area.width) {
                    let gap = usize::from(area.width).saturating_sub(width + 2);
                    spans.push(Span::raw(" ".repeat(gap)));
                    spans.push(Span::styled(hint, Style::default().fg(theme.muted)));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        popup_list(
            frame,
            theme,
            area,
            items,
            palette.selected,
            "No matching available commands.",
        );
    }

    pub(super) fn draw_heading_search(&self, frame: &mut Frame) {
        let Some(search) = &self.heading_search else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, "Search headings", 84, 23) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            "Heading title",
            &search.query,
            &format!(
                "{} matches · {} headings",
                search.matches.len(),
                search.items.len()
            ),
            &[
                ("Esc", "Cancel"),
                ("↑↓", "Select"),
                ("Enter", "Jump"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = search
            .matches
            .iter()
            .map(|&index| {
                let hit = &search.items[index];
                let title = if hit.title.is_empty() {
                    "(untitled)".to_owned()
                } else {
                    display_component(&hit.title)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("H{} · Ln {}  ", hit.level, hit.row + 1),
                        Style::default().fg(theme.muted),
                    ),
                    Span::raw(title),
                ]))
            })
            .collect();
        popup_list(
            frame,
            theme,
            area,
            items,
            search.selected,
            if search.items.is_empty() {
                "No headings in this note."
            } else {
                "No matching headings."
            },
        );
    }

    pub(super) fn draw_vault_search(&self, frame: &mut Frame) {
        let Some(dialog) = &self.vault_search else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(
            frame,
            theme,
            &format!("Search vault — {}", dialog.mode.label()),
            106,
            25,
        ) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            dialog.mode.label(),
            &dialog.query,
            &format!(
                "{} matches · {} indexed · {} skipped{}",
                dialog.matches.len(),
                dialog.index.count(),
                dialog.index.skipped,
                if dialog.index.truncated {
                    " · file limit reached"
                } else {
                    ""
                }
            ),
            &[
                ("Esc", "Cancel"),
                ("↑↓", "Select"),
                ("Enter", "Open"),
                ("Tab", "Text/Tags"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = dialog
            .matches
            .iter()
            .map(|hit| {
                let relative = hit
                    .path
                    .strip_prefix(&dialog.index.root)
                    .unwrap_or(&hit.path);
                let name = relative
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_else(|| relative.to_string_lossy());
                let location = format!("{}:{}", relative.display(), hit.line);
                if area.height >= 3 && area.width >= 32 {
                    ListItem::new(vec![
                        Line::styled(
                            format!("{} · {}", name, hit.excerpt),
                            Style::default().fg(theme.foreground),
                        ),
                        Line::styled(format!("  {location}"), Style::default().fg(theme.muted)),
                    ])
                } else {
                    ListItem::new(format!("{location}  {}", hit.excerpt))
                }
            })
            .collect();
        popup_list(
            frame,
            theme,
            area,
            items,
            dialog.selected,
            if dialog.query.is_empty() {
                "Type a query to search your notes."
            } else {
                "No matching notes."
            },
        );
    }

    pub(super) fn draw_find_dialog(&self, frame: &mut Frame) {
        let Some(dialog) = &self.find_dialog else {
            return;
        };
        let theme = self.theme;
        let Some(inner) = popup_inner(frame, theme, "Find in current folder", 84, 18) else {
            return;
        };
        let area = picker_body(
            frame,
            theme,
            inner,
            "Filename",
            &dialog.query,
            &format!("{} matches", dialog.matches.len()),
            &[
                ("Esc", "Cancel"),
                ("↑↓", "Select"),
                ("Enter", "Jump to file"),
                ("Ctrl+U", "Clear"),
            ],
        );
        let items = dialog
            .matches
            .iter()
            .map(|&index| {
                let entry = &self.entries[index];
                let (prefix, color) = if entry.is_dir {
                    ("▸ ", theme.directory)
                } else {
                    ("  ", theme.foreground)
                };
                ListItem::new(Line::styled(
                    format!("{prefix}{}", entry.name),
                    Style::default().fg(color),
                ))
            })
            .collect();
        popup_list(
            frame,
            theme,
            area,
            items,
            dialog.selected,
            "No matching files or folders.",
        );
    }
}

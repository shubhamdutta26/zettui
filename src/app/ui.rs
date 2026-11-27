//! Render the main interface without changing note contents.

mod popups;

use super::{App, PromptKind, breadcrumb_parts, display_component, fit_breadcrumb, tail_label};
use crate::theme::Theme;
use crate::{focus, markdown, outline};
use edtui::{EditorMode, EditorTheme, EditorView, LineNumbers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap},
};

/// Use a distinct tab surface even when the theme surface equals the background.
fn buffer_tab_style(theme: Theme, active: bool) -> Style {
    use crate::system_theme::{blend, readable};
    use ratatui::style::Color;

    let rgb_base =
        matches!(theme.background, Color::Rgb(..)) && matches!(theme.foreground, Color::Rgb(..));
    let bg = if active {
        theme.accent
    } else if rgb_base {
        blend(theme.background, theme.foreground, 0.18)
    } else {
        theme.selected_bg
    };
    let preferred = if active {
        theme.background
    } else {
        theme.foreground
    };
    let fg = if matches!(bg, Color::Rgb(..)) && rgb_base {
        readable(preferred, bg, theme.foreground, true)
    } else {
        preferred
    };
    let style = Style::default().fg(fg).bg(bg);
    if active {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

/// Render live headings; selection is independent of the cursor while focused.
fn draw_outline(
    frame: &mut Frame,
    (theme, settings): (Theme, &markdown::MarkdownSettings),
    area: Rect,
    headings: Option<&[outline::Heading<'_>]>,
    view: &outline::OutlineView,
    row: usize,
    focused: bool,
) {
    let block = Block::bordered()
        .title(" Outline ")
        .padding(Padding::horizontal(1))
        .border_type(if focused {
            BorderType::Thick
        } else {
            BorderType::Plain
        })
        .border_style(Style::default().fg(if focused { theme.accent } else { theme.border }))
        .title_style(Style::default().fg(if focused { theme.accent } else { theme.muted }));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let Some(headings) = headings else {
        frame.render_widget(
            Paragraph::new("Markdown notes only")
                .style(Style::default().fg(theme.muted))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    };
    if headings.is_empty() {
        let message = "No headings";
        frame.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(theme.muted))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
    let active = view.active_visible(headings, row);
    let visible = view.visible(headings);
    let items: Vec<ListItem<'_>> = visible
        .iter()
        .map(|&i| {
            let h = &headings[i];
            let title = if h.title.is_empty() {
                "(untitled)".to_owned()
            } else {
                display_component(h.title)
            };
            let branch = if !outline::OutlineView::has_children(headings, i) {
                "  "
            } else if view.is_collapsed(i) {
                "▸ "
            } else {
                "▾ "
            };
            let marker = if active == Some(i) { "• " } else { "  " };
            ListItem::new(format!(
                "{marker}{}{branch}{title}",
                "  ".repeat(h.level - 1)
            ))
            .style(Style::default().fg(if settings.syntax_highlighting {
                markdown::heading_color(h.level, theme, settings)
            } else {
                theme.foreground
            }))
        })
        .collect();
    let mut state = ListState::default();
    let selected = view
        .selected
        .and_then(|i| visible.iter().position(|&v| v == i));
    state.select(selected);
    frame.render_stateful_widget(
        List::new(items)
            .highlight_symbol("▏ ")
            .highlight_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg)),
        inner,
        &mut state,
    );
}

impl App {
    pub(super) fn draw(&mut self, frame: &mut Frame) {
        if self.analytics_dashboard.is_some() {
            self.draw_analytics_dashboard(frame);
            return;
        }
        let theme = self.theme;
        let statistics_warning = self
            .statistics
            .as_ref()
            .and_then(zettui::writing_activity::Tracker::warning)
            .map(str::to_owned);
        frame.render_widget(Block::default().style(theme.base_style()), frame.area());
        let zen = self.editor.is_some() && self.writing.zen.enabled;
        let interactive = self.command.is_some()
            || self.note_search.input.is_some()
            || self.writing.leader
            || self.buffer_prefix.is_some();
        let footer_height = if !zen {
            3
        } else if interactive {
            2
        } else {
            u16::from(self.writing.zen.show_status_bar)
                + u16::from(!self.message.is_empty() || statistics_warning.is_some())
        };
        let areas = Layout::vertical([
            Constraint::Length(u16::from(!zen || self.writing.zen.show_title)),
            Constraint::Min(1),
            Constraint::Length(footer_height),
        ])
        .split(frame.area());
        let mut panes = self.panels.areas(
            areas[1],
            self.browser_width_percent,
            self.editor.is_some(),
            zen,
        );
        if zen {
            panes[1] = focus::centered(areas[1], self.writing.zen.width.clamp(20, 240));
        }

        // A hidden or unusably short outline must never retain input ownership.
        if self.outline_view.focused && (panes[2].width == 0 || panes[2].height < 3) {
            self.outline_view.focused = false;
            self.message = "Outline unavailable at this terminal size or layout.".into();
        }
        let show_editor_cursor = !self.outline_view.focused
            && !self.has_active_popup()
            && self.note_search.input.is_none();
        let browser_active = self.editor.is_none() && !self.has_active_popup();
        let editor_active =
            self.editor.is_some() && !self.outline_view.focused && !self.has_active_popup();
        // Show the actual directory of the open note, or the current browser folder.
        let directory = self
            .editor
            .as_ref()
            .and_then(|note| note.path.parent())
            .unwrap_or(&self.current_dir);
        let parts = breadcrumb_parts(&self.root, directory);
        let mut header = vec![
            Span::styled(
                " Zettui",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" v{}  /  ", env!("CARGO_PKG_VERSION")),
                Style::default().fg(theme.muted),
            ),
        ];
        let prefix_width = header.iter().map(Span::width).sum::<usize>();
        let available = usize::from(areas[0].width).saturating_sub(prefix_width);
        let visible = fit_breadcrumb(&parts, available);
        for (index, part) in visible.iter().enumerate() {
            if index > 0 {
                header.push(Span::styled(" › ", Style::default().fg(theme.muted)));
            }
            let style = if part == "…" {
                Style::default().fg(theme.muted)
            } else if index + 1 == visible.len() {
                Style::default()
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            };
            header.push(Span::styled(part.as_str(), style));
        }
        if zen {
            if self.writing.zen.show_title {
                let title = self
                    .editor
                    .as_ref()
                    .and_then(|n| n.path.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                frame.render_widget(
                    Paragraph::new(title)
                        .centered()
                        .style(Style::default().fg(theme.muted)),
                    areas[0],
                );
            }
        } else {
            frame.render_widget(Paragraph::new(Line::from(header)), areas[0]);
        }

        if panes[0].width > 0 && panes[0].height > 0 {
            let items: Vec<ListItem> = self
                .entries
                .iter()
                .map(|entry| {
                    let prefix = if entry.is_dir { "▸ " } else { "  " };
                    let color = if entry.is_dir {
                        theme.directory
                    } else {
                        theme.foreground
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(prefix.to_owned(), Style::default().fg(color)),
                        Span::styled(entry.name.clone(), Style::default().fg(color)),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::bordered()
                        .title(" Files ")
                        .border_type(if browser_active {
                            BorderType::Thick
                        } else {
                            BorderType::Plain
                        })
                        .padding(Padding::horizontal(1))
                        .border_style(Style::default().fg(if browser_active {
                            theme.accent
                        } else {
                            theme.border
                        }))
                        .title_style(Style::default().fg(if browser_active {
                            theme.accent
                        } else {
                            theme.muted
                        })),
                )
                .highlight_style(Style::default().bg(theme.selected_bg).fg(theme.selected_fg))
                .highlight_symbol("▏ ");
            let mut state = ListState::default();
            if !self.entries.is_empty() {
                state.select(Some(self.selected));
            }
            frame.render_stateful_widget(list, panes[0], &mut state);
        }

        let tabs_visible = !zen
            && self.show_tab_bar
            && self.editor.is_some()
            && panes[1].height >= 4
            && panes[1].width > 1;
        if tabs_visible {
            let area = Rect::new(panes[1].x, panes[1].y, panes[1].width, 1);
            self.draw_buffer_tabs(frame, area);
            panes[1].y += 1;
            panes[1].height -= 1;
        }
        let popup_active = self.has_active_popup();
        let (mode, location, help, stats, dirty_state) = if let Some(note) = self.editor.as_mut() {
            let filename = note
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("note")
                .to_owned();
            let source = note.state.lines.to_string();
            if panes[2].width > 0 && panes[2].height > 0 {
                let is_markdown = markdown::is_markdown(&note.path);
                let headings = if is_markdown {
                    outline::extract(&source)
                } else {
                    Vec::new()
                };
                self.outline_view.sync(&headings, note.state.cursor.row);
                draw_outline(
                    frame,
                    (theme, &self.markdown_settings),
                    panes[2],
                    is_markdown.then_some(headings.as_slice()),
                    &self.outline_view,
                    note.state.cursor.row,
                    self.outline_view.focused && !popup_active,
                );
            }
            let modified = note.is_dirty_text(&source);
            let dirty = if modified { " [+]" } else { "" };
            let title = format!(" {filename}{dirty} ");
            let border = Block::bordered()
                .border_type(if editor_active {
                    BorderType::Thick
                } else {
                    BorderType::Plain
                })
                .padding(Padding::horizontal(1))
                .border_style(Style::default().fg(if editor_active {
                    theme.accent
                } else {
                    theme.border
                }))
                .title_style(Style::default().fg(if modified {
                    theme.warning
                } else if editor_active {
                    theme.accent
                } else {
                    theme.muted
                }));
            let border = if tabs_visible {
                border
            } else {
                border.title(title)
            };
            let mut inner = if zen {
                panes[1]
            } else {
                border.inner(panes[1])
            };
            if !zen {
                frame.render_widget(border, panes[1]);
            }
            let editor_theme = EditorTheme::default()
                .base(theme.base_style())
                .line_numbers_style(
                    Style::default().fg(theme.muted).bg(theme
                        .base_style()
                        .bg
                        .unwrap_or(ratatui::style::Color::Reset)),
                )
                .selection_style(Style::default().fg(theme.selected_fg).bg(theme.selected_bg))
                .cursor_style(
                    if note.state.mode == EditorMode::Insert
                        || self.outline_view.focused
                        || self.metadata.is_some()
                        || self.export_dialog.is_some()
                        || self.heading_search.is_some()
                        || self.command_palette.is_some()
                        || self.buffer_picker.is_some()
                    {
                        // Use the hardware bar in Insert mode; hide the painted
                        // editor cursor while a navigation surface owns keyboard focus.
                        theme.base_style()
                    } else {
                        Style::default().fg(theme.background).bg(theme.accent)
                    },
                )
                .hide_status_line();
            let focus_range = if self.writing.focus.enabled && note.state.mode != EditorMode::Visual
            {
                Some((
                    focus::active_range(&source, note.state.cursor.row, self.writing.focus.mode),
                    self.writing.focus.dim_strength,
                ))
            } else {
                None
            };
            if self.writing.focus.typewriter && inner.height > 2 {
                let half = usize::from(inner.height / 2);
                let row = note.state.cursor.row;
                let (x, _) = note.state.viewport_offset();
                note.state.set_viewport_offset(x, row.saturating_sub(half));
                // Display-only blank space above the start of the document.
                let pad = half.saturating_sub(row) as u16;
                inner.y = inner.y.saturating_add(pad);
                inner.height = inner.height.saturating_sub(pad);
            }
            self.highlight_cache.prepare(
                &source,
                theme,
                markdown::is_markdown(&note.path),
                &self.markdown_settings,
            );
            note.state.set_highlights(self.highlight_cache.highlights(
                &self.note_search.painted,
                self.note_search.active,
                focus_range,
            ));
            frame.render_widget(
                EditorView::new(&mut note.state)
                    .wrap(self.writing.editor.word_wrap)
                    .line_numbers(
                        if if zen {
                            self.writing.zen.show_line_numbers
                        } else {
                            self.writing.editor.line_numbers
                        } {
                            LineNumbers::Absolute
                        } else {
                            LineNumbers::None
                        },
                    )
                    .theme(editor_theme),
                inner,
            );
            // Only show the hardware cursor when the editor has input focus.
            if show_editor_cursor && let Some(cursor) = note.state.cursor_screen_position() {
                frame.set_cursor_position((cursor.x, cursor.y));
            }
            let mode = if self.outline_view.focused {
                "OUTLINE".to_owned()
            } else if self.note_search.input.is_some() {
                "SEARCH".to_owned()
            } else if self.command.is_some() {
                "COMMAND".to_owned()
            } else {
                note.state.mode.name().to_uppercase()
            };
            let help = if self.outline_view.focused {
                "j/k Move | h/l Collapse/Expand | Enter Jump | F7 Search | F8 Commands | Esc/F6 Editor".to_owned()
            } else if self.buffer_prefix == Some('b') {
                "Space b: b Switch | d Close | o Close others | p Alternate | t Tabs | s Save all"
                    .to_owned()
            } else if self.buffer_prefix.is_some() {
                "b Switch buffer | Esc Cancel".to_owned()
            } else if self.writing.leader {
                "Space: b Buffers | c Commands | z Zen | f Focus | t Typewriter | e Files | p Preview | o Outline | l Navigate outline | m Metadata | x Export | h Headings | 1/2/3 Focus level | n Numbers | s Status"
                    .to_owned()
            } else if let Some(input) = &self.note_search.input {
                format!(
                    "{}{}▏  [{}/{}] | Enter confirm | Esc cancel",
                    input.direction.prefix(),
                    input.query,
                    self.note_search.active,
                    self.note_search.count
                )
            } else if let Some(command) = &self.command {
                format!(":{command}▏   Enter run | Esc cancel")
            } else if mode == "NORMAL" {
                "i Insert | :w Save | F5 Buffers | F8 Commands | / Find".to_owned()
            } else if mode == "INSERT" {
                "Esc Normal | Ctrl+S Save | Ctrl+P Cite | Ctrl+E Snippet | Ctrl+F Search".to_owned()
            } else {
                "Esc Normal | Ctrl+S Save | Ctrl+F Search | Alt+T Tags".to_owned()
            };
            let stats = format!(
                "{} words  │  Ln {}, Col {}",
                self.highlight_cache.word_count(),
                note.state.cursor.row + 1,
                note.state.cursor.col + 1
            );
            (mode, filename, help, stats, Some(modified))
        } else {
            if panes[1].width > 0 && panes[1].height > 0 {
                let preview = if self
                    .entries
                    .get(self.selected)
                    .is_some_and(|e| !e.is_dir && markdown::is_markdown(&e.path))
                {
                    Paragraph::new(markdown::preview(
                        &self.preview,
                        theme,
                        &self.markdown_settings,
                    ))
                } else {
                    Paragraph::new(self.preview.as_str())
                }
                .block(
                    Block::bordered()
                        .title(" Preview ")
                        .padding(Padding::horizontal(1))
                        .border_style(Style::default().fg(theme.border))
                        .title_style(Style::default().fg(theme.muted)),
                )
                .style(theme.base_style())
                .wrap(Wrap { trim: false });
                frame.render_widget(preview, panes[1]);
            }
            (
                "BROWSER".to_owned(),
                {
                    let relative = self
                        .current_dir
                        .strip_prefix(&self.root)
                        .unwrap_or(&self.current_dir);
                    if relative.as_os_str().is_empty() {
                        ".".to_owned()
                    } else {
                        display_component(&relative.to_string_lossy())
                    }
                },
                "Enter Open | n New note | F8 Commands | / Find | Ctrl+F Search | F3 Preview | q Quit"
                    .to_owned(),
                String::new(),
                None,
            )
        };
        let mode = if popup_active {
            "DIALOG"
        } else {
            mode.as_str()
        };
        let mode_color = match mode {
            "INSERT" => theme.directory,
            "VISUAL" => theme.foreground,
            "COMMAND" | "SEARCH" | "DIALOG" => theme.warning,
            "NORMAL" => theme.success,
            _ => theme.accent,
        };
        let mut status = vec![Span::styled(
            format!(" {mode} "),
            Style::default()
                .fg(theme.background)
                .bg(mode_color)
                .add_modifier(Modifier::BOLD),
        )];
        if let Some(modified) = dirty_state {
            status.push(Span::styled(
                if modified {
                    "  ● Modified"
                } else {
                    "  ✓ Saved"
                },
                Style::default().fg(if modified {
                    theme.warning
                } else {
                    theme.success
                }),
            ));
        }
        let writing_label = if self.editor.is_some() {
            self.writing.label()
        } else {
            String::new()
        };
        if !writing_label.is_empty() {
            status.push(Span::styled(
                format!("  {writing_label}"),
                Style::default().fg(theme.accent),
            ));
        }
        if statistics_warning.is_some() {
            status.push(Span::styled(
                "  Stats unsaved",
                Style::default().fg(theme.warning),
            ));
        }
        let location_width = usize::from(areas[2].width)
            .saturating_sub(status.iter().map(Span::width).sum::<usize>() + 2);
        if location_width > 0 && !zen && !tabs_visible {
            status.push(Span::styled(
                format!("  {}", tail_label(&location, location_width)),
                Style::default().fg(theme.muted),
            ));
        }
        let footer_rows = Layout::vertical([
            Constraint::Length(u16::from(
                !zen || interactive || self.writing.zen.show_status_bar,
            )),
            Constraint::Length(u16::from(
                !zen || interactive || !self.message.is_empty() || statistics_warning.is_some(),
            )),
            Constraint::Length(u16::from(footer_height > 2)),
        ])
        .split(areas[2]);
        let status_line = Line::from(status);
        // Give the mode and save state priority on narrow terminals.
        let stats_line = Line::from(Span::styled(stats, Style::default().fg(theme.muted)));
        let stats_width = stats_line.width();
        if stats_width > 0
            && status_line.width() + stats_width + 2 <= usize::from(footer_rows[0].width)
        {
            let columns =
                Layout::horizontal([Constraint::Min(0), Constraint::Length(stats_width as u16)])
                    .split(footer_rows[0]);
            frame.render_widget(
                Paragraph::new(status_line).style(theme.base_style()),
                columns[0],
            );
            frame.render_widget(
                Paragraph::new(stats_line).style(theme.base_style()),
                columns[1],
            );
        } else {
            frame.render_widget(
                Paragraph::new(status_line).style(theme.base_style()),
                footer_rows[0],
            );
        }
        let mut shortcuts = Vec::new();
        if popup_active {
            shortcuts.push(Span::styled(
                "See dialog for controls",
                Style::default().fg(theme.muted),
            ));
        } else if self.command.is_some()
            || self.note_search.input.is_some()
            || self.writing.leader
            || self.buffer_prefix.is_some()
        {
            shortcuts.push(Span::styled(
                help.as_str(),
                Style::default().fg(theme.accent),
            ));
        } else {
            for (i, item) in help.split(" | ").enumerate() {
                if i > 0 {
                    shortcuts.push(Span::raw("   "));
                }
                let (key, description) = item.split_once(' ').unwrap_or((item, ""));
                shortcuts.push(Span::styled(
                    key,
                    Style::default()
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD),
                ));
                shortcuts.push(Span::styled(
                    format!(" {description}"),
                    Style::default().fg(theme.muted),
                ));
            }
        }
        if !zen || interactive {
            frame.render_widget(
                Paragraph::new(Line::from(shortcuts)).style(theme.base_style()),
                footer_rows[1],
            );
        }
        if !zen || (!interactive && (!self.message.is_empty() || statistics_warning.is_some())) {
            let message = match statistics_warning {
                Some(error) if self.message.is_empty() => format!("Writing stats unsaved: {error}"),
                Some(error) => format!("{} | Writing stats unsaved: {error}", self.message),
                None => self.message.clone(),
            };
            frame.render_widget(
                Paragraph::new(message).style(theme.base_style()),
                footer_rows[if zen { 1 } else { 2 }],
            );
        }

        if let Some(prompt) = &self.prompt {
            let screen = frame.area();
            let width = screen.width.saturating_sub(2).min(74);
            let height = screen.height.saturating_sub(2).min(8);
            if width >= 12 && height >= 5 {
                let popup = Rect::new(
                    screen.x + (screen.width - width) / 2,
                    screen.y + (screen.height - height) / 2,
                    width,
                    height,
                );
                let (title, help, input) = match &prompt.kind {
                    PromptKind::NewNote => (
                        "Create note",
                        "Single filename. Add .json, .txt, etc. or leave blank for .md",
                        prompt.input.clone(),
                    ),
                    PromptKind::NewFolder => (
                        "Create folder",
                        "Enter a folder name without / or ..",
                        prompt.input.clone(),
                    ),
                    PromptKind::Rename { .. } => (
                        "Rename selected",
                        "Enter a new filename (including its extension)",
                        prompt.input.clone(),
                    ),
                    PromptKind::Move { .. } => (
                        "Move selected",
                        "Existing folder RELATIVE TO NOTES ROOT (e.g. Readings or .)",
                        prompt.input.clone(),
                    ),
                    PromptKind::Trash { source } => (
                        "Move to Trash?",
                        "Press y to confirm, n or Esc to cancel",
                        source
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "item".to_owned()),
                    ),
                };
                let contents = format!(
                    "{help}\n\n> {input}{}\n\n{}",
                    if matches!(&prompt.kind, PromptKind::Trash { .. }) {
                        ""
                    } else {
                        "▏"
                    },
                    if matches!(&prompt.kind, PromptKind::Trash { .. }) {
                        "Trash is recoverable using your desktop file manager"
                    } else {
                        "Enter: confirm   Esc: cancel   Ctrl+U: clear"
                    }
                );
                frame.render_widget(Clear, popup);
                frame.render_widget(
                    Paragraph::new(contents)
                        .block(
                            Block::bordered()
                                .title(format!(" {title} "))
                                .padding(Padding::horizontal(1))
                                .title_style(
                                    Style::default()
                                        .fg(theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                )
                                .border_style(Style::default().fg(theme.accent)),
                        )
                        .style(Style::default().fg(theme.foreground).bg(theme.surface))
                        .wrap(Wrap { trim: false }),
                    popup,
                );
            }
        }
        if self.links.is_some() {
            self.draw_link_dialog(frame);
        }
        if self.citation_picker.is_some() {
            self.draw_citation_picker(frame);
        }
        if self.research_dialog.is_some() {
            self.draw_research_dialog(frame);
        }
        if self.find_dialog.is_some() {
            self.draw_find_dialog(frame);
        }
        if self.wiki_completion.is_some() {
            self.draw_wiki_completion(frame);
        }
        if self.snippet_picker.is_some() {
            self.draw_snippet_picker(frame);
        }
        if self.snippet_form.is_some() {
            self.draw_snippet_form(frame);
        }
        if self.vault_search.is_some() {
            self.draw_vault_search(frame);
        }
        if self.heading_search.is_some() {
            self.draw_heading_search(frame);
        }
        if self.command_palette.is_some() {
            self.draw_command_palette(frame);
        }
        if self.buffer_picker.is_some() {
            self.draw_buffer_picker(frame);
        }
        if self.metadata.is_some() {
            self.draw_metadata_editor(frame);
        }
        if self.export_dialog.is_some() {
            self.draw_export(frame);
        }
    }

    fn draw_buffer_tabs(&self, frame: &mut Frame, area: Rect) {
        let Some(active) = self.editor.active_index() else {
            return;
        };
        let labels: Vec<_> = self
            .editor
            .notes()
            .iter()
            .enumerate()
            .map(|(i, note)| {
                let name = note.path.file_name().unwrap_or_default();
                let duplicate = self
                    .editor
                    .notes()
                    .iter()
                    .filter(|n| n.path.file_name() == Some(name))
                    .count()
                    > 1;
                let label = if duplicate {
                    note.path
                        .strip_prefix(&self.root)
                        .unwrap_or(&note.path)
                        .to_string_lossy()
                } else {
                    name.to_string_lossy()
                };
                format!(
                    " {} {}{} ",
                    i + 1,
                    display_component(&label),
                    if note.is_dirty() { " [+]" } else { "" }
                )
            })
            .collect();
        // Reserve the first cell for a half-cell tab edge aligned with the border.
        let width = usize::from(area.width.saturating_sub(1));
        if width == 0 {
            return;
        }
        let mut start = 0;
        let mut used = labels[..=active]
            .iter()
            .map(|s| Span::raw(s.as_str()).width())
            .sum::<usize>()
            + active;
        while start < active && used > width {
            used -= Span::raw(labels[start].as_str()).width() + 1;
            start += 1;
        }
        let mut spans = Vec::new();
        let mut remaining = width;
        for (i, label) in labels.iter().enumerate().skip(start) {
            let gap = usize::from(i > start);
            if remaining <= gap {
                break;
            }
            let available = remaining - gap;
            let label_width = Span::raw(label.as_str()).width();
            if label_width > available && i != active {
                break;
            }
            if gap > 0 {
                spans.push(Span::styled(" ", self.theme.base_style()));
            }
            let label = if label_width > available {
                tail_label(label, available)
            } else {
                label.clone()
            };
            let cells = Span::raw(label.as_str()).width();
            remaining -= gap + cells;
            let style = buffer_tab_style(self.theme, i == active);
            if i == start {
                // A box-drawing vertical border is centered in its cell.
                // Begin the first tab at that same half-cell position.
                spans.push(Span::styled(
                    "▐",
                    Style::default()
                        .fg(style.bg.unwrap_or(self.theme.background))
                        .bg(self.theme.background),
                ));
            }
            spans.push(Span::styled(label, style));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }
}

#[cfg(test)]
mod tab_style_tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn tabs_have_distinct_backgrounds_even_when_surface_matches_background() {
        for (name, bg, fg) in [
            ("dark", Color::Rgb(45, 53, 59), Color::Rgb(211, 198, 170)),
            ("light", Color::Rgb(250, 248, 242), Color::Rgb(45, 53, 59)),
        ] {
            let mut theme = Theme::named(name).unwrap();
            theme.background = bg;
            theme.foreground = fg;
            theme.accent = Color::Rgb(127, 187, 179);
            theme.surface = theme.background;
            let active = buffer_tab_style(theme, true);
            let inactive = buffer_tab_style(theme, false);
            assert_eq!(active.bg, Some(theme.accent));
            assert_ne!(inactive.bg, Some(theme.background));
            assert_ne!(active.bg, inactive.bg);
            assert!(active.add_modifier.contains(Modifier::BOLD));
        }
    }
}

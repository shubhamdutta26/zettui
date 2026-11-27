//! Modal document export controls and background job coordination.
use super::{App, display_component, tail_label};
use crate::{
    document_export::{self, Format, Job, Update},
    markdown,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Clear, Padding, Paragraph, Wrap},
};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
};

pub(super) struct ExportDialog {
    source: PathBuf,
    directory: bool,
    format: Format,
    destination: String,
    custom_destination: bool,
    citations: bool,
    csl: String,
    field: usize,
    engines: Vec<String>,
    engine_index: usize,
    status: String,
    status_scroll: u16,
    scroll_limit: Cell<u16>,
    details_override: Option<bool>,
    outcome: Option<bool>, // Some(true) success; Some(false) failure
    warnings: bool,
    completed: Option<PathBuf>,
    receiver: Option<Receiver<Update>>,
}

impl ExportDialog {
    fn details_visible(&self) -> bool {
        !self.status.is_empty()
            && self
                .details_override
                .unwrap_or(self.warnings || self.outcome == Some(false))
    }
}

fn export_area(screen: Rect, pdf: bool, diagnostic_rows: usize) -> Rect {
    let width = screen.width.saturating_sub(4).min(76);
    // The compact layout already reserves two rows for status messages.
    // Only allocate additional rows when the wrapped diagnostics need them.
    let extra = diagnostic_rows.saturating_sub(2).min(8) as u16;
    let preferred_height = (if pdf { 16 } else { 15 }) + extra;
    let height = screen.height.saturating_sub(2).min(preferred_height);
    Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    )
}

// Shorten only the displayed path, never the value sent to Pandoc.
fn compact_path_with_home(value: &str, home: Option<&Path>) -> String {
    if let Some(relative) = home
        .filter(|path| path.is_absolute())
        .and_then(|home| Path::new(value).strip_prefix(home).ok())
    {
        return if relative.as_os_str().is_empty() {
            "~".into()
        } else {
            format!("~/{}", relative.display())
        };
    }
    value.to_owned()
}

fn compact_path(value: &str) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    compact_path_with_home(value, home.as_deref())
}

// Wrap by terminal cell width, preserving diagnostic line breaks and whitespace.
fn diagnostic_lines(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for source in text.split('\n') {
        let mut line = String::new();
        let mut cells = 0;
        for ch in display_component(source).chars() {
            let mut encoded = [0; 4];
            let encoded_char: &str = ch.encode_utf8(&mut encoded);
            let ch_width = Span::raw(encoded_char).width();
            if cells > 0 && cells + ch_width > width {
                lines.push(std::mem::take(&mut line));
                cells = 0;
            }
            line.push(ch);
            cells += ch_width;
        }
        lines.push(line);
    }
    lines
}

impl App {
    pub(super) fn open_export(&mut self) {
        if self.has_active_popup() || self.command.is_some() {
            return;
        }
        let (source, directory) = if let Some(note) = self.editor.as_ref() {
            (note.path.clone(), false)
        } else if let Some(entry) = self.entries.get(self.selected) {
            (entry.path.clone(), entry.is_dir)
        } else {
            (self.current_dir.clone(), true)
        };
        if !directory && !markdown::is_markdown(&source) {
            self.message = "Select a Markdown note or directory for export.".into();
            return;
        }
        if directory
            && !fs::symlink_metadata(&source)
                .is_ok_and(|meta| meta.is_dir() && !meta.file_type().is_symlink())
        {
            self.message = "Export requires a real directory, not a symlink.".into();
            return;
        }
        let settings = &self.export_settings;
        let destination =
            document_export::suggested_path(&source, directory, settings.format, settings)
                .to_string_lossy()
                .into_owned();
        let engines = document_export::engine_choices(settings.pdf_engine.as_deref());
        let engine_index = settings
            .pdf_engine
            .as_ref()
            .and_then(|engine| engines.iter().position(|choice| choice == engine))
            .unwrap_or(0);
        self.export_dialog = Some(ExportDialog {
            source,
            directory,
            format: settings.format,
            destination,
            custom_destination: false,
            citations: self.bibliography_path.is_some(),
            csl: settings
                .csl
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            field: 0,
            engines,
            engine_index,
            status: String::new(),
            status_scroll: 0,
            scroll_limit: Cell::new(0),
            details_override: None,
            outcome: None,
            warnings: false,
            completed: None,
            receiver: None,
        });
        self.writing.leader = false;
        self.outline_view.focused = false;
        self.message.clear();
    }

    fn export_job(&self, dialog: &ExportDialog) -> Result<Job, String> {
        let destination = document_export::resolve_path(&dialog.destination, &self.root)
            .map_err(|e| e.to_string())?;
        let bibliography = if dialog.citations {
            Some(
                self.bibliography_path
                    .as_ref()
                    .ok_or("No bibliography configured.")?
                    .canonicalize()
                    .map_err(|e| format!("Bibliography: {e}"))?,
            )
        } else {
            None
        };
        let csl = if dialog.citations && !dialog.csl.trim().is_empty() {
            Some(
                document_export::resolve_path(&dialog.csl, &self.root)
                    .and_then(|p| p.canonicalize())
                    .map_err(|e| format!("CSL style: {e}"))?,
            )
        } else {
            None
        };
        let snapshots = self
            .editor
            .notes()
            .iter()
            .filter(|note| {
                if dialog.directory {
                    note.path.starts_with(&dialog.source)
                } else {
                    note.path == dialog.source
                }
            })
            .map(|note| (note.path.clone(), note.state.lines.to_string()))
            .collect();
        Ok(Job {
            source: dialog.source.clone(),
            directory: dialog.directory,
            destination,
            format: dialog.format,
            root: self.root.clone(),
            bibliography,
            csl,
            resource_dirs: self.export_settings.resource_dirs.clone(),
            pdf_engine: Some(dialog.engines[dialog.engine_index].clone()),
            snapshots,
        })
    }

    pub(super) fn handle_export_key(&mut self, key: KeyEvent) {
        let Some(mut dialog) = self.export_dialog.take() else {
            return;
        };
        if dialog.receiver.is_some() {
            self.export_dialog = Some(dialog);
            return;
        }
        let ctrl = key.modifiers == KeyModifiers::CONTROL;
        match key.code {
            KeyCode::Esc => return,
            KeyCode::Char('d') if ctrl && !dialog.status.is_empty() => {
                dialog.details_override = Some(!dialog.details_visible());
                dialog.status_scroll = 0;
            }
            KeyCode::PageUp if dialog.details_visible() => {
                dialog.status_scroll = dialog
                    .status_scroll
                    .min(dialog.scroll_limit.get())
                    .saturating_sub(3);
            }
            KeyCode::PageDown if !dialog.status.is_empty() => {
                if dialog.details_visible() {
                    dialog.status_scroll = dialog
                        .status_scroll
                        .saturating_add(3)
                        .min(dialog.scroll_limit.get());
                } else {
                    dialog.details_override = Some(true);
                    dialog.status_scroll = 0;
                }
            }
            KeyCode::Char('o') if ctrl => {
                if let Some(path) = &dialog.completed {
                    let result = std::process::Command::new("xdg-open")
                        .arg(path)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                    match result {
                        Ok(mut child) => {
                            std::thread::spawn(move || {
                                let _ = child.wait();
                            });
                            self.message = "Requested opening exported output.".into();
                        }
                        Err(error) => {
                            self.message = format!("Cannot launch xdg-open: {error}");
                            dialog.status.push_str(&format!("\n{}", self.message));
                            dialog.details_override = Some(true);
                            dialog.status_scroll = 0;
                        }
                    }
                } else {
                    dialog.status = "Export a document first.".into();
                    dialog.status_scroll = 0;
                    dialog.details_override = None;
                    dialog.outcome = None;
                    dialog.warnings = false;
                }
            }
            KeyCode::Tab | KeyCode::Down if !ctrl => {
                dialog.field =
                    (dialog.field + 1) % (if dialog.format == Format::Pdf { 5 } else { 4 })
            }
            KeyCode::BackTab | KeyCode::Up if !ctrl => {
                let count = if dialog.format == Format::Pdf { 5 } else { 4 };
                dialog.field = (dialog.field + count - 1) % count;
            }
            KeyCode::Left | KeyCode::Right if dialog.field == 4 && dialog.format == Format::Pdf => {
                let count = dialog.engines.len();
                dialog.engine_index = (dialog.engine_index
                    + if key.code == KeyCode::Right {
                        1
                    } else {
                        count - 1
                    })
                    % count;
                dialog.status.clear();
                dialog.status_scroll = 0;
                dialog.details_override = None;
                dialog.outcome = None;
                dialog.warnings = false;
            }
            KeyCode::Left | KeyCode::Right if dialog.field == 0 => {
                dialog.format = dialog.format.cycle(key.code == KeyCode::Right);
                if !dialog.custom_destination {
                    dialog.destination = document_export::suggested_path(
                        &dialog.source,
                        dialog.directory,
                        dialog.format,
                        &self.export_settings,
                    )
                    .to_string_lossy()
                    .into_owned();
                }
                dialog.status.clear();
                dialog.status_scroll = 0;
                dialog.details_override = None;
                dialog.outcome = None;
                dialog.warnings = false;
            }
            KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right if dialog.field == 2 => {
                if self.bibliography_path.is_some() {
                    dialog.citations = !dialog.citations;
                } else {
                    dialog.status = "Configure a Zotero bibliography first.".into();
                }
            }
            KeyCode::Enter if !ctrl => {
                dialog.status_scroll = 0;
                dialog.details_override = None;
                dialog.outcome = None;
                dialog.warnings = false;
                dialog.completed = None;
                match self.export_job(&dialog).and_then(|job| {
                    document_export::preflight(&job).map_err(|e| e.to_string())?;
                    Ok(job)
                }) {
                    Ok(job) => {
                        let (tx, rx) = mpsc::channel();
                        match std::thread::Builder::new()
                            .name("zettui-document-export".into())
                            .spawn(move || {
                                let result =
                                    document_export::run(job, &tx).map_err(|e| e.to_string());
                                let _ = tx.send(Update::Finished(result));
                            }) {
                            Ok(_handle) => {
                                dialog.receiver = Some(rx);
                                dialog.status = "Starting Pandoc…".into();
                            }
                            Err(error) => {
                                dialog.status = format!("Cannot start export: {error}");
                                dialog.outcome = Some(false);
                            }
                        }
                    }
                    Err(error) => {
                        dialog.status = format!("Export failed: {error}");
                        dialog.outcome = Some(false);
                    }
                }
            }
            _ => {
                let input = match dialog.field {
                    1 => Some(&mut dialog.destination),
                    3 => Some(&mut dialog.csl),
                    _ => None,
                };
                if let Some(input) = input {
                    let changed = match key.code {
                        KeyCode::Backspace => input.pop().is_some(),
                        KeyCode::Char('u') if ctrl => {
                            input.clear();
                            true
                        }
                        KeyCode::Char(ch)
                            if !key
                                .modifiers
                                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                                && !ch.is_control() =>
                        {
                            input.push(ch);
                            true
                        }
                        _ => false,
                    };
                    if changed {
                        if dialog.field == 1 {
                            dialog.custom_destination = true;
                        }
                        dialog.status.clear();
                        dialog.status_scroll = 0;
                        dialog.details_override = None;
                        dialog.outcome = None;
                        dialog.warnings = false;
                    }
                }
            }
        }
        self.export_dialog = Some(dialog);
    }

    pub(super) fn poll_export(&mut self) {
        let Some(dialog) = &mut self.export_dialog else {
            return;
        };
        let Some(receiver) = dialog.receiver.as_ref() else {
            return;
        };
        loop {
            match receiver.try_recv() {
                Ok(Update::Progress(status)) => {
                    dialog.status = status;
                    dialog.status_scroll = 0;
                }
                Ok(Update::Finished(result)) => {
                    dialog.status_scroll = 0;
                    dialog.details_override = None;
                    dialog.status = match result {
                        Ok(result) => {
                            dialog.completed = Some(result.destination.clone());
                            dialog.outcome = Some(true);
                            dialog.warnings = !result.warnings.is_empty();
                            let mut message = format!(
                                "Exported {} document(s): {}",
                                result.count,
                                result.destination.display()
                            );
                            if dialog.warnings {
                                message.push_str(&format!("\n{}", result.warnings));
                            }
                            message
                        }
                        Err(error) => {
                            dialog.outcome = Some(false);
                            format!("Export failed: {error}")
                        }
                    };
                    self.message = dialog.status.lines().next().unwrap_or_default().to_owned();
                    dialog.receiver = None;
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    dialog.status_scroll = 0;
                    dialog.details_override = None;
                    dialog.status = "Export worker stopped unexpectedly.".into();
                    dialog.outcome = Some(false);
                    dialog.receiver = None;
                    break;
                }
            }
        }
    }

    pub(super) fn draw_export(&self, frame: &mut Frame) {
        let Some(dialog) = &self.export_dialog else {
            return;
        };
        let screen = frame.area();
        let pdf = dialog.format == Format::Pdf;
        let expanded = dialog.details_visible();
        // The border and horizontal padding consume four columns in total.
        let status_width = usize::from(screen.width.saturating_sub(4).min(76).saturating_sub(4));
        let diagnostic_rows = if expanded {
            diagnostic_lines(&dialog.status, status_width).len()
        } else {
            0
        };
        let area = export_area(screen, pdf, diagnostic_rows);
        if area.width < 12 || area.height < 4 {
            return;
        }
        let theme = self.theme;
        frame.render_widget(Clear, area);
        let block = Block::bordered()
            .title(" Document export ")
            .style(theme.base_style())
            .border_style(Style::default().fg(theme.accent))
            .padding(Padding::horizontal(1));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let count = if pdf { 5 } else { 4 };
        if inner.width < 32 || inner.height < count as u16 + 6 {
            frame.render_widget(
                Paragraph::new("Resize terminal for export dialog. Esc closes when idle.")
                    .style(Style::default().fg(theme.muted))
                    .wrap(Wrap { trim: true }),
                inner,
            );
            return;
        }
        // Drop decorative spacing first on smaller terminals. The footer and fields stay visible.
        let gap = if inner.height >= count as u16 + 9 {
            1
        } else {
            0
        };
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(gap),
            Constraint::Length(count as u16),
            Constraint::Length(gap),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(inner);
        let source = dialog
            .source
            .file_name()
            .unwrap_or(dialog.source.as_os_str())
            .to_string_lossy();
        let description = format!(
            "{}: {}",
            if dialog.directory {
                "Folder (batch)"
            } else {
                "Note"
            },
            display_component(&source)
        );
        frame.render_widget(
            Paragraph::new(tail_label(&description, usize::from(rows[0].width)))
                .style(Style::default().fg(theme.muted)),
            rows[0],
        );

        let format = format!("← {} →", dialog.format.label());
        let destination = compact_path(&dialog.destination);
        let csl = if dialog.csl.is_empty() {
            "(Pandoc default)".into()
        } else if dialog.field == 3 {
            compact_path(&dialog.csl)
        } else {
            Path::new(&dialog.csl).file_name().map_or_else(
                || dialog.csl.clone(),
                |name| name.to_string_lossy().into_owned(),
            )
        };
        let citations = if dialog.citations {
            "Enabled"
        } else {
            "Disabled"
        };
        let choice = &dialog.engines[dialog.engine_index];
        let selected_engine = if pdf {
            document_export::select_pdf_engine(Some(choice.as_str())).ok()
        } else {
            None
        };
        let engine = if choice.is_empty() {
            format!(
                "← Auto ({}) →",
                selected_engine.as_deref().unwrap_or("none installed")
            )
        } else if selected_engine.is_some() {
            format!("← {choice} →")
        } else {
            format!("← {choice} (missing) →")
        };
        let fields = [
            ("Format", format.as_str()),
            ("Destination", destination.as_str()),
            ("Citations", citations),
            ("CSL style", csl.as_str()),
            ("PDF engine", engine.as_str()),
        ];
        let field_rows = Layout::vertical(vec![Constraint::Length(1); count]).split(rows[2]);
        for (index, (label, value)) in fields.iter().take(count).enumerate() {
            let active = dialog.field == index;
            let prefix = format!("{} {:<12}", if active { ">" } else { " " }, label);
            let available = usize::from(field_rows[index].width)
                .saturating_sub(Span::raw(prefix.as_str()).width());
            let value = tail_label(&display_component(value), available);
            let style = if active {
                Style::default()
                    .fg(theme.selected_fg)
                    .bg(theme.selected_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.foreground)
            };
            frame.render_widget(
                Paragraph::new(format!("{prefix}{value}")).style(style),
                field_rows[index],
            );
        }
        let pandoc = document_export::executable_exists("pandoc");
        let ready = pandoc && (!pdf || selected_engine.is_some());
        let readiness: String = if !pandoc {
            "Pandoc not found".into()
        } else if !pdf {
            "✓ Pandoc available".into()
        } else if let Some(engine) = &selected_engine {
            format!(
                "✓ Pandoc · {} available",
                display_component(
                    Path::new(engine)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(engine)
                )
            )
        } else {
            "PDF engine missing".into()
        };
        frame.render_widget(
            Paragraph::new(readiness).style(Style::default().fg(if ready {
                theme.success
            } else {
                theme.warning
            })),
            rows[4],
        );

        let color = match dialog.outcome {
            Some(false) => theme.warning,
            Some(true) if dialog.warnings => theme.warning,
            Some(true) => theme.success,
            None => theme.muted,
        };
        let status: String = if dialog.status.is_empty() {
            "Choose a new output path; no overwrite.".into()
        } else if expanded {
            dialog.status.clone()
        } else if dialog.warnings {
            format!(
                "{} (warnings; Ctrl+d)",
                dialog.status.lines().next().unwrap_or_default()
            )
        } else {
            dialog.status.lines().next().unwrap_or_default().to_owned()
        };
        let lines = diagnostic_lines(&status, usize::from(rows[5].width));
        let limit = if expanded {
            u16::try_from(lines.len().saturating_sub(usize::from(rows[5].height)))
                .unwrap_or(u16::MAX)
        } else {
            0
        };
        dialog.scroll_limit.set(limit);
        let offset = if expanded {
            dialog.status_scroll.min(limit)
        } else {
            0
        };
        let visible = lines
            .into_iter()
            .skip(usize::from(offset))
            .take(usize::from(rows[5].height))
            .collect::<Vec<_>>()
            .join("\n");
        frame.render_widget(
            Paragraph::new(visible).style(Style::default().fg(color)),
            rows[5],
        );
        let help = if dialog.receiver.is_some() {
            "Export running — please wait.".to_owned()
        } else if inner.width >= 54 {
            format!(
                "Tab/↑↓ field · ←→ select · Space toggle\nEnter export · Esc close · Ctrl+o open\n{}",
                if expanded {
                    "PgUp/PgDn scroll · Ctrl+d collapse"
                } else {
                    "Ctrl+u clear path · Ctrl+d details"
                }
            )
        } else {
            format!(
                "Tab/↑↓ field · ←→ · Space\nEnter export · Esc · Ctrl+o open\n{}",
                if expanded {
                    "PgUp/PgDn · Ctrl+d collapse"
                } else {
                    "Ctrl+u clear · Ctrl+d details"
                }
            )
        };
        frame.render_widget(
            Paragraph::new(help).style(Style::default().fg(theme.muted)),
            rows[6],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Appearance, AutoThemes, ColorOverrides, ThemeChoice};

    #[test]
    fn export_layout_is_compact_and_expands_for_diagnostics() {
        let screen = Rect::new(0, 0, 104, 30);
        assert_eq!(export_area(screen, true, 0), Rect::new(14, 7, 76, 16));
        let short_error = "Export failed: Destination exists. Choose another name.";
        let rows = diagnostic_lines(short_error, 72).len();
        assert_eq!(rows, 1);
        assert_eq!(
            export_area(screen, true, rows),
            export_area(screen, true, 0)
        );
        assert_eq!(export_area(screen, true, 2).height, 16);
        assert_eq!(export_area(screen, true, 3).height, 17);
        assert_eq!(export_area(screen, true, 6).height, 20);
        assert_eq!(export_area(screen, true, 30), Rect::new(14, 3, 76, 24));
        assert_eq!(export_area(screen, false, 0).height, 15);
        assert_eq!(export_area(screen, false, 1).height, 15);
        assert_eq!(export_area(screen, false, 3).height, 16);
        assert_eq!(
            export_area(Rect::new(3, 4, 40, 18), true, 30),
            Rect::new(5, 5, 36, 16)
        );
    }

    #[test]
    fn export_display_paths_preserve_original_and_component_boundaries() {
        let home = Some(Path::new("/home/sam"));
        assert_eq!(
            compact_path_with_home("/home/sam/Documents/note.pdf", home),
            "~/Documents/note.pdf"
        );
        assert_eq!(compact_path_with_home("/home/sam", home), "~");
        assert_eq!(
            compact_path_with_home("/home/samuel/note.pdf", home),
            "/home/samuel/note.pdf"
        );
        assert_eq!(
            compact_path_with_home("assets/figure.png", home),
            "assets/figure.png"
        );
    }

    #[test]
    fn export_diagnostics_wrap_without_losing_text() {
        assert_eq!(
            diagnostic_lines("abcdefgh\nxyz", 4),
            ["abcd", "efgh", "xyz"].map(str::to_owned)
        );
        assert_eq!(diagnostic_lines("a界b", 3), ["a界", "b"].map(str::to_owned));
    }

    #[test]
    fn export_modal_snapshots_live_note_and_cancels_without_saving() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Note.md");
        fs::write(&path, "Original\n").unwrap();
        let mut app = App::new(
            dir.path().to_path_buf(),
            None,
            ThemeChoice::Dark,
            ColorOverrides::default(),
            Appearance::Dark,
            AutoThemes::default(),
        )
        .unwrap();
        app.editor.open(&path).unwrap();
        app.open_export();
        assert!(app.has_active_popup());
        let dialog = app.export_dialog.as_ref().unwrap();
        assert!(!dialog.directory);
        assert!(!dialog.citations);
        let job = app.export_job(dialog).unwrap();
        assert_eq!(
            job.snapshots.get(&path.canonicalize().unwrap()).unwrap(),
            &app.editor.as_ref().unwrap().state.lines.to_string()
        );
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let dialog = app.export_dialog.as_mut().unwrap();
        dialog.status = "Export failed: sample\n".repeat(30);
        dialog.outcome = Some(false);
        assert!(dialog.details_visible());
        terminal.draw(|frame| app.draw(frame)).unwrap();
        app.handle_export_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert!(app.export_dialog.as_ref().unwrap().status_scroll > 0);
        app.handle_export_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(!app.export_dialog.as_ref().unwrap().details_visible());
        app.handle_export_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert!(app.export_dialog.as_ref().unwrap().details_visible());
        terminal.draw(|frame| app.draw(frame)).unwrap();
        app.handle_export_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.has_active_popup());
        assert_eq!(fs::read_to_string(path).unwrap(), "Original\n");
    }
}

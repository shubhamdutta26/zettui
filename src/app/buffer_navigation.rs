//! Buffer actions and modal input, independent of edtui text editing.
use super::App;
use crate::{buffer_switcher::BufferSwitcher, commands::Command};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use edtui::EditorMode;

impl App {
    fn buffer_changed(&mut self, changed: bool) {
        if changed {
            self.reset_buffer_context();
            self.message = format!("{} open buffer(s).", self.editor.len());
        }
    }

    fn activate_buffer(&mut self, index: usize) {
        let changed = self.editor.activate(index);
        self.buffer_changed(changed);
    }

    pub(super) fn cycle_buffer(&mut self, forward: bool) {
        let changed = self.editor.cycle(forward);
        self.buffer_changed(changed);
    }

    pub(super) fn alternate_buffer(&mut self) {
        let changed = self.editor.activate_previous();
        self.buffer_changed(changed);
        if !changed {
            self.message = "No previous buffer available.".into();
        }
    }

    pub(super) fn close_buffer(&mut self) {
        let Some(index) = self.editor.active_index() else {
            return;
        };
        self.close_editor(false);
        if self.editor.is_none() && self.editor.len() > 0 {
            self.activate_buffer(index.min(self.editor.len() - 1));
        }
    }

    pub(super) fn close_other_buffers(&mut self) {
        self.message = if self.editor.close_others() {
            "Closed other buffers.".into()
        } else {
            "Cannot close other buffers: unsaved changes or no active buffer.".into()
        };
    }

    pub(super) fn save_all_buffers(&mut self) {
        let (saved, failures) = self.editor.save_all();
        self.update_preview();
        self.message = if failures.is_empty() {
            format!("Saved {saved} modified buffer(s).")
        } else {
            format!(
                "Saved {saved}; {} failed. {}",
                failures.len(),
                failures.join("; ")
            )
        };
    }

    pub(super) fn open_buffer_picker(&mut self) {
        if self
            .editor
            .as_ref()
            .is_some_and(|n| n.state.mode != EditorMode::Normal)
        {
            self.message = "Switch to Normal mode to select buffers.".into();
            return;
        }
        if self.editor.len() == 0 {
            self.message = "No open buffers.".into();
            return;
        }
        self.buffer_picker = Some(BufferSwitcher::new(&self.editor));
        self.writing.leader = false;
        self.buffer_prefix = None;
    }

    pub(super) fn handle_buffer_picker_key(&mut self, key: KeyEvent) {
        let Some(mut picker) = self.buffer_picker.take() else {
            return;
        };
        let ctrl = key.modifiers == KeyModifiers::CONTROL;
        match key.code {
            KeyCode::Esc | KeyCode::F(5) => return,
            KeyCode::Enter => {
                if let Some(&index) = picker.matches.get(picker.selected) {
                    self.activate_buffer(index);
                    self.outline_view.focused = false;
                    return;
                }
            }
            KeyCode::Up => picker.selected = picker.selected.saturating_sub(1),
            KeyCode::Down => {
                picker.selected = (picker.selected + 1).min(picker.matches.len().saturating_sub(1))
            }
            KeyCode::Char('p') if ctrl => picker.selected = picker.selected.saturating_sub(1),
            KeyCode::Char('n') if ctrl => {
                picker.selected = (picker.selected + 1).min(picker.matches.len().saturating_sub(1))
            }
            KeyCode::Backspace => {
                picker.query.pop();
                picker.filter(&self.editor);
            }
            KeyCode::Char('u') if ctrl => {
                picker.query.clear();
                picker.filter(&self.editor);
            }
            KeyCode::Char(ch)
                if !ch.is_control()
                    && !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                picker.query.push(ch);
                picker.filter(&self.editor);
            }
            _ => {}
        }
        self.buffer_picker = Some(picker);
    }

    pub(super) fn handle_buffer_prefix(&mut self, key: KeyEvent) -> bool {
        if !self
            .editor
            .as_ref()
            .is_some_and(|n| n.state.mode == EditorMode::Normal)
        {
            self.buffer_prefix = None;
            return false;
        }
        if let Some(prefix) = self.buffer_prefix.take() {
            if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                return false;
            }
            let command = match (prefix, key.code) {
                ('[', KeyCode::Char('b')) => Command::PreviousBuffer,
                (']', KeyCode::Char('b')) => Command::NextBuffer,
                ('b', KeyCode::Char('b')) => Command::BufferPicker,
                ('b', KeyCode::Char('d')) => Command::CloseBuffer,
                ('b', KeyCode::Char('o')) => Command::CloseOtherBuffers,
                ('b', KeyCode::Char('p')) => Command::AlternateBuffer,
                ('b', KeyCode::Char('t')) => Command::ToggleTabs,
                ('b', KeyCode::Char('s')) => Command::SaveAll,
                _ => return key.code == KeyCode::Esc,
            };
            self.execute_editor_command(command);
            return true;
        }
        if self.writing.leader && key.code == KeyCode::Char('b') && key.modifiers.is_empty() {
            self.writing.leader = false;
            self.buffer_prefix = Some('b');
            return true;
        }
        if !self.writing.leader
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && let KeyCode::Char(ch @ ('[' | ']')) = key.code
        {
            self.buffer_prefix = Some(ch);
            return true;
        }
        false
    }
}

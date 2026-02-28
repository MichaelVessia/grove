use super::*;

impl GroveApp {
    pub(super) fn handle_settings_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.settings_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        enum PostAction {
            None,
            Save,
            Cancel,
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Char(_) if ctrl_n => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(_) if ctrl_p => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left | KeyCode::Char('h') => {}
            KeyCode::Right | KeyCode::Char('l') => {}
            KeyCode::Char(' ') => {}
            KeyCode::Enter => match dialog.focused_field {
                SettingsDialogField::SaveButton => post_action = PostAction::Save,
                SettingsDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_settings_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("settings", "dialog_cancelled");
                self.close_active_dialog();
            }
        }
    }

    pub(super) fn open_settings_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        self.set_settings_dialog(SettingsDialogState {
            focused_field: SettingsDialogField::SaveButton,
        });
    }

    pub(super) fn apply_settings_dialog_save(&mut self) {
        if self.settings_dialog().is_none() {
            return;
        }

        self.close_active_dialog();
        self.show_info_toast("settings are read-only, edit ~/.config/grove/config.toml");
    }
}

use super::*;

impl GroveApp {
    fn cycle_settings_theme(&mut self, next: bool) {
        let Some(next_theme) = ({
            let Some(dialog) = self.settings_dialog_mut() else {
                return;
            };
            if dialog.focused_field != SettingsDialogField::Theme {
                return;
            }

            let next_theme = if next {
                next_theme_name(dialog.theme)
            } else {
                previous_theme_name(dialog.theme)
            };
            dialog.theme = next_theme;
            Some(next_theme)
        }) else {
            return;
        };
        self.theme_name = next_theme;
    }

    pub(super) fn cancel_settings_dialog(&mut self) {
        let Some(initial_theme) = self.settings_dialog().map(|dialog| dialog.initial_theme) else {
            return;
        };
        self.theme_name = initial_theme;
        self.close_active_dialog();
    }

    fn save_theme_to_global_settings(&self, theme: ThemeName) -> Result<(), String> {
        let mut global = crate::infrastructure::config::load_global_from_path(&self.config_path)?;
        global.theme = theme;
        crate::infrastructure::config::save_global_to_path(&self.config_path, &global)
    }

    pub(super) fn handle_settings_dialog_key(&mut self, key_event: KeyEvent) {
        if self.settings_dialog().is_none() {
            return;
        }
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
                if let Some(dialog) = self.settings_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                if let Some(dialog) = self.settings_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Char(_) if ctrl_n => {
                if let Some(dialog) = self.settings_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::Char(_) if ctrl_p => {
                if let Some(dialog) = self.settings_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Left | KeyCode::Char('h') => self.cycle_settings_theme(false),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                self.cycle_settings_theme(true);
            }
            KeyCode::Enter => match self.settings_dialog().map(|dialog| dialog.focused_field) {
                Some(SettingsDialogField::Theme) => self.cycle_settings_theme(true),
                Some(SettingsDialogField::SaveButton) => post_action = PostAction::Save,
                Some(SettingsDialogField::CancelButton) => post_action = PostAction::Cancel,
                None => {}
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_settings_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("settings", "dialog_cancelled");
                self.cancel_settings_dialog();
            }
        }
    }

    pub(super) fn open_settings_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        self.set_settings_dialog(SettingsDialogState {
            focused_field: SettingsDialogField::Theme,
            initial_theme: self.theme_name,
            theme: self.theme_name,
        });
    }

    pub(super) fn apply_settings_dialog_save(&mut self) {
        let Some(theme) = self.settings_dialog().map(|dialog| dialog.theme) else {
            return;
        };

        if let Err(error) = self.save_theme_to_global_settings(theme) {
            self.show_error_toast(format!("settings save failed: {error}"));
            return;
        }

        self.theme_name = theme;
        self.close_active_dialog();
        self.show_success_toast(format!("theme saved: {}", theme.config_key()));
    }
}

use super::*;

impl GroveApp {
    fn retheme_grove_tmux_sessions(&mut self, theme: ThemeName) -> Result<(), String> {
        let rows = self
            .tmux_input
            .list_sessions_with_tab_metadata()
            .map_err(|error| format!("session query failed: {error}"))?;
        let sessions = crate::application::agent_runtime::grove_managed_tmux_sessions(&rows);
        for session_name in sessions {
            for command in
                crate::application::agent_runtime::tmux_theme_commands(session_name.as_str(), theme)
            {
                self.execute_tmux_command(command.as_slice())
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    fn cycle_settings_theme(&mut self, next: bool) {
        let focused_theme =
            self.current_settings_dialog_focus_field() == Some(SettingsDialogField::Theme);
        let Some(next_theme) = ({
            let Some(dialog) = self.settings_dialog_mut() else {
                return;
            };
            if !focused_theme {
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
        self.sync_active_dialog_focus_field();
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
                self.focus_next_dialog_field();
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                self.focus_prev_dialog_field();
            }
            KeyCode::Char(_) if ctrl_n => {
                self.focus_next_dialog_field();
            }
            KeyCode::Char(_) if ctrl_p => {
                self.focus_prev_dialog_field();
            }
            KeyCode::Left | KeyCode::Char('h') => self.cycle_settings_theme(false),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                self.cycle_settings_theme(true);
            }
            KeyCode::Enter => match self.current_settings_dialog_focus_field() {
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
        match self.retheme_grove_tmux_sessions(theme) {
            Ok(()) => self.show_success_toast(format!("theme saved: {}", theme.config_key())),
            Err(error) => {
                self.show_warning_toast(format!("theme saved, tmux retheme failed: {error}"));
            }
        }
    }
}

use super::*;

impl GroveApp {
    pub(super) fn handle_settings_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.settings_dialog.as_mut() else {
            return;
        };

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
            KeyCode::Left | KeyCode::Char('h') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.previous();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                SettingsDialogField::Multiplexer => {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
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
                self.settings_dialog = None;
            }
        }
    }

    pub(super) fn open_settings_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        self.settings_dialog = Some(SettingsDialogState {
            multiplexer: self.multiplexer,
            focused_field: SettingsDialogField::Multiplexer,
        });
    }

    fn has_running_workspace_sessions(&self) -> bool {
        self.state
            .workspaces
            .iter()
            .any(|workspace| workspace.status.has_session())
    }

    pub(super) fn apply_settings_dialog_save(&mut self) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };

        if dialog.multiplexer != self.multiplexer && self.has_running_workspace_sessions() {
            self.show_toast(
                "restart running workspaces before switching multiplexer",
                true,
            );
            return;
        }

        let selected = dialog.multiplexer;
        self.multiplexer = selected;
        self.tmux_input = input_for_multiplexer(selected);
        let config = GroveConfig {
            multiplexer: selected,
            projects: self.projects.clone(),
        };
        if let Err(error) = crate::infrastructure::config::save_to_path(&self.config_path, &config)
        {
            self.show_toast(format!("settings save failed: {error}"), true);
            return;
        }

        self.settings_dialog = None;
        self.interactive = None;
        self.lazygit_ready_sessions.clear();
        self.lazygit_failed_sessions.clear();
        self.refresh_workspaces(None);
        self.poll_preview();
        self.show_toast(format!("multiplexer set to {}", selected.label()), false);
    }
}

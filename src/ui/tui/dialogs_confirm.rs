use super::*;

impl GroveApp {
    pub(super) fn open_restart_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.start_in_flight || self.stop_in_flight {
            self.show_info_toast("agent lifecycle already in progress");
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no agent running");
            return;
        };
        if !workspace_can_stop_agent(Some(&workspace)) {
            self.show_info_toast("no agent running");
            return;
        }

        self.set_confirm_dialog(ConfirmDialogState {
            action: ConfirmDialogAction::RestartAgent {
                workspace_name: workspace.name.clone(),
                workspace_path: workspace.path.clone(),
                agent: workspace.agent,
            },
            focused_field: ConfirmDialogField::ConfirmButton,
        });
        self.log_dialog_event_with_fields(
            "confirm",
            "dialog_opened",
            [
                (
                    "target".to_string(),
                    Value::from("restart_agent".to_string()),
                ),
                ("workspace".to_string(), Value::from(workspace.name)),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
            ],
        );
    }

    pub(super) fn open_quit_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        self.set_confirm_dialog(ConfirmDialogState {
            action: ConfirmDialogAction::QuitApp,
            focused_field: ConfirmDialogField::ConfirmButton,
        });
        self.log_dialog_event_with_fields(
            "confirm",
            "dialog_opened",
            [("target".to_string(), Value::from("quit_app".to_string()))],
        );
    }

    fn cancel_confirm_dialog(&mut self, target: &'static str) {
        self.log_dialog_event_with_fields(
            "confirm",
            "dialog_cancelled",
            [("target".to_string(), Value::from(target.to_string()))],
        );
        self.close_active_dialog();
    }

    fn confirm_dialog_target(action: &ConfirmDialogAction) -> &'static str {
        match action {
            ConfirmDialogAction::RestartAgent { .. } => "restart_agent",
            ConfirmDialogAction::QuitApp => "quit_app",
        }
    }

    fn confirm_confirm_dialog(&mut self) {
        let Some(dialog) = self.take_confirm_dialog() else {
            return;
        };
        let target = Self::confirm_dialog_target(&dialog.action);
        self.log_dialog_event_with_fields(
            "confirm",
            "dialog_confirmed",
            [("target".to_string(), Value::from(target.to_string()))],
        );

        match dialog.action {
            ConfirmDialogAction::RestartAgent { workspace_path, .. } => {
                self.restart_workspace_agent_for_path(&workspace_path);
            }
            ConfirmDialogAction::QuitApp => {
                self.queue_cmd(Cmd::Quit);
            }
        }
    }

    pub(super) fn handle_confirm_dialog_key(&mut self, key_event: KeyEvent) {
        let no_modifiers = key_event.modifiers.is_empty();
        let target = self
            .confirm_dialog()
            .map(|dialog| Self::confirm_dialog_target(&dialog.action))
            .unwrap_or("confirm");
        match key_event.code {
            KeyCode::Escape => {
                self.cancel_confirm_dialog(target);
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.cancel_confirm_dialog(target);
                return;
            }
            KeyCode::Char('y') if no_modifiers => {
                self.confirm_confirm_dialog();
                return;
            }
            KeyCode::Char('n') if no_modifiers => {
                self.cancel_confirm_dialog(target);
                return;
            }
            _ => {}
        }

        let mut should_confirm = false;
        let mut should_cancel = false;
        let Some(dialog) = self.confirm_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match dialog.focused_field {
                ConfirmDialogField::ConfirmButton => should_confirm = true,
                ConfirmDialogField::CancelButton => should_cancel = true,
            },
            KeyCode::Tab => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Char(_) if ctrl_n => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(_) if ctrl_p => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Up | KeyCode::Char('k') if no_modifiers => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Down | KeyCode::Char('j') if no_modifiers => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(character) if no_modifiers => {
                if (dialog.focused_field == ConfirmDialogField::ConfirmButton
                    || dialog.focused_field == ConfirmDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    dialog.focused_field =
                        if dialog.focused_field == ConfirmDialogField::ConfirmButton {
                            ConfirmDialogField::CancelButton
                        } else {
                            ConfirmDialogField::ConfirmButton
                        };
                }
            }
            _ => {}
        }

        if should_cancel {
            self.cancel_confirm_dialog(target);
            return;
        }
        if should_confirm {
            self.confirm_confirm_dialog();
        }
    }
}

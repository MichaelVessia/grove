use super::*;

impl GroveApp {
    pub(super) fn open_quit_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        self.set_confirm_dialog(ConfirmDialogState {
            action: ConfirmDialogAction::QuitApp,
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
            ConfirmDialogAction::CloseActiveTab { .. } => "close_active_tab",
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
            ConfirmDialogAction::CloseActiveTab {
                workspace_path,
                tab_id,
                session_name,
            } => {
                self.force_close_active_tab_and_session(&workspace_path, tab_id, &session_name);
            }
            ConfirmDialogAction::QuitApp => {
                self.queue_cmd(Cmd::Quit);
            }
        }
    }

    pub(super) fn handle_confirm_dialog_key(&mut self, key_event: KeyEvent) {
        self.sync_active_dialog_focus_field();
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
        let Some(focused_field) = self.current_confirm_dialog_focus_field() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match focused_field {
                ConfirmDialogField::ConfirmButton => should_confirm = true,
                ConfirmDialogField::CancelButton => should_cancel = true,
            },
            KeyCode::Tab => {
                self.focus_next_dialog_field();
            }
            KeyCode::BackTab => {
                self.focus_prev_dialog_field();
            }
            KeyCode::Char(_) if ctrl_n => {
                self.focus_next_dialog_field();
            }
            KeyCode::Char(_) if ctrl_p => {
                self.focus_prev_dialog_field();
            }
            KeyCode::Up | KeyCode::Char('k') if no_modifiers => {
                self.focus_prev_dialog_field();
            }
            KeyCode::Down | KeyCode::Char('j') if no_modifiers => {
                self.focus_next_dialog_field();
            }
            KeyCode::Char(character) if no_modifiers => {
                if (focused_field == ConfirmDialogField::ConfirmButton
                    || focused_field == ConfirmDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    self.focus_dialog_field(
                        if focused_field == ConfirmDialogField::ConfirmButton {
                            FOCUS_ID_CONFIRM_CANCEL_BUTTON
                        } else {
                            FOCUS_ID_CONFIRM_CONFIRM_BUTTON
                        },
                    );
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

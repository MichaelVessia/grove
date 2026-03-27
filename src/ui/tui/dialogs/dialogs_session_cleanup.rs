use super::*;

impl GroveApp {
    fn plan_session_cleanup_dialog(
        &self,
        options: SessionCleanupOptions,
    ) -> Result<SessionCleanupPlan, String> {
        let session_rows = self
            .tmux_input
            .list_sessions_for_cleanup()
            .map_err(|error| error.to_string())?;
        Ok(
            crate::application::session_cleanup::plan_session_cleanup_from_session_rows(
                &self.state.tasks,
                session_rows.as_str(),
                options,
            ),
        )
    }

    pub(super) fn open_session_cleanup_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let options = SessionCleanupOptions {
            include_stale: false,
            include_attached: false,
        };
        let plan = match self.plan_session_cleanup_dialog(options) {
            Ok(plan) => plan,
            Err(error) => {
                self.session.last_tmux_error = Some(error.clone());
                self.show_error_toast("session cleanup scan failed");
                return;
            }
        };

        self.set_session_cleanup_dialog(SessionCleanupDialogState {
            options,
            plan,
            last_error: None,
        });
        self.log_dialog_event("session_cleanup", "dialog_opened");
    }

    fn refresh_session_cleanup_dialog_plan_with_options(&mut self, options: SessionCleanupOptions) {
        let plan_result = self.plan_session_cleanup_dialog(options);
        let mut error_toast = None;
        if let Some(dialog) = self.session_cleanup_dialog_mut() {
            dialog.options = options;
            match plan_result {
                Ok(plan) => {
                    dialog.plan = plan;
                    dialog.last_error = None;
                }
                Err(error) => {
                    dialog.last_error = Some(error.clone());
                    error_toast = Some(error);
                }
            }
        }

        if let Some(error) = error_toast {
            self.session.last_tmux_error = Some(error);
            self.show_error_toast("session cleanup scan failed");
        }
    }

    fn confirm_session_cleanup_dialog(&mut self) {
        let Some(dialog) = self.session_cleanup_dialog().cloned() else {
            return;
        };
        if dialog.plan.candidates.is_empty() {
            self.show_info_toast("no sessions to clean");
            return;
        }

        self.log_dialog_event_with_fields(
            "session_cleanup",
            "dialog_confirmed",
            [
                (
                    "candidates".to_string(),
                    Value::from(usize_to_u64(dialog.plan.candidates.len())),
                ),
                (
                    "include_stale".to_string(),
                    Value::from(dialog.options.include_stale),
                ),
                (
                    "include_attached".to_string(),
                    Value::from(dialog.options.include_attached),
                ),
            ],
        );

        let result = apply_session_cleanup(&dialog.plan);
        let killed_count = result.killed.len();
        let gone_count = result.already_gone.len();
        let failure_count = result.failures.len();

        if failure_count > 0 {
            let summary = format!("{} session cleanup failure(s)", failure_count);
            if let Some((_, first_error)) = result.failures.first() {
                self.session.last_tmux_error = Some(first_error.clone());
            }
            if let Some(state) = self.session_cleanup_dialog_mut() {
                state.last_error = Some(summary);
            }
            self.show_error_toast("session cleanup had failures");
        } else {
            if killed_count > 0 {
                self.show_success_toast(format!("killed {} session(s)", killed_count));
            } else if gone_count > 0 {
                self.show_info_toast(format!("{} session(s) already gone", gone_count));
            } else {
                self.show_info_toast("no sessions cleaned");
            }
            if let Some(state) = self.session_cleanup_dialog_mut() {
                state.last_error = None;
            }
        }

        self.refresh_session_cleanup_dialog_plan_with_options(dialog.options);
    }

    pub(super) fn handle_session_cleanup_dialog_key(&mut self, key_event: KeyEvent) {
        self.sync_active_dialog_focus_field();
        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("session_cleanup", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("session_cleanup", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('D') if no_modifiers => {
                self.confirm_session_cleanup_dialog();
                return;
            }
            _ => {}
        }

        let mut refresh_options = None;
        let mut confirm_cleanup = false;
        let mut cancel_dialog = false;
        let Some(focused_field) = self.current_session_cleanup_dialog_focus_field() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match focused_field {
                SessionCleanupDialogField::IncludeStale => {
                    if let Some(dialog) = self.session_cleanup_dialog_mut() {
                        dialog.options.include_stale = !dialog.options.include_stale;
                        refresh_options = Some(dialog.options);
                    }
                }
                SessionCleanupDialogField::IncludeAttached => {
                    if let Some(dialog) = self.session_cleanup_dialog_mut() {
                        dialog.options.include_attached = !dialog.options.include_attached;
                        refresh_options = Some(dialog.options);
                    }
                }
                SessionCleanupDialogField::ApplyButton => {
                    confirm_cleanup = true;
                }
                SessionCleanupDialogField::CancelButton => {
                    cancel_dialog = true;
                }
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
            KeyCode::Char(' ') if no_modifiers => {
                if focused_field == SessionCleanupDialogField::IncludeStale {
                    if let Some(dialog) = self.session_cleanup_dialog_mut() {
                        dialog.options.include_stale = !dialog.options.include_stale;
                        refresh_options = Some(dialog.options);
                    }
                } else if focused_field == SessionCleanupDialogField::IncludeAttached
                    && let Some(dialog) = self.session_cleanup_dialog_mut()
                {
                    dialog.options.include_attached = !dialog.options.include_attached;
                    refresh_options = Some(dialog.options);
                }
            }
            KeyCode::Char(character) if no_modifiers => {
                if (focused_field == SessionCleanupDialogField::ApplyButton
                    || focused_field == SessionCleanupDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    self.focus_dialog_field(
                        if focused_field == SessionCleanupDialogField::ApplyButton {
                            FOCUS_ID_SESSION_CLEANUP_CANCEL_BUTTON
                        } else {
                            FOCUS_ID_SESSION_CLEANUP_APPLY_BUTTON
                        },
                    );
                }
            }
            _ => {}
        }

        if let Some(options) = refresh_options {
            self.refresh_session_cleanup_dialog_plan_with_options(options);
        }

        if cancel_dialog {
            self.log_dialog_event("session_cleanup", "dialog_cancelled");
            self.close_active_dialog();
            return;
        }
        if confirm_cleanup {
            self.confirm_session_cleanup_dialog();
        }
    }
}

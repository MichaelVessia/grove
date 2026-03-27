use super::*;

impl GroveApp {
    pub(super) fn handle_stop_dialog_key(&mut self, key_event: KeyEvent) {
        if self.dialogs.stop_in_flight || self.dialogs.restart_in_flight {
            return;
        }
        self.sync_active_dialog_focus_field();

        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("stop", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("stop", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('x') if no_modifiers => {
                self.confirm_stop_dialog();
                return;
            }
            _ => {}
        }

        let mut confirm_stop = false;
        let mut cancel_dialog = false;
        let Some(focused_field) = self.stop_dialog().map(|dialog| dialog.focused_field) else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match focused_field {
                StopDialogField::StopButton => {
                    confirm_stop = true;
                }
                StopDialogField::CancelButton => {
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
            KeyCode::Char(character) if no_modifiers => {
                if (focused_field == StopDialogField::StopButton
                    || focused_field == StopDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    self.focus_dialog_field(if focused_field == StopDialogField::StopButton {
                        FOCUS_ID_STOP_CANCEL_BUTTON
                    } else {
                        FOCUS_ID_STOP_CONFIRM_BUTTON
                    });
                }
            }
            _ => {}
        }

        if cancel_dialog {
            self.log_dialog_event("stop", "dialog_cancelled");
            self.close_active_dialog();
            return;
        }
        if confirm_stop {
            self.confirm_stop_dialog();
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn open_stop_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.session.interactive.is_some() {
            self.exit_interactive_to_list();
        }
        if self.dialogs.stop_in_flight || self.dialogs.restart_in_flight {
            self.show_info_toast("agent lifecycle already in progress");
            return;
        }

        if self.selected_home_tab_targets_task_root() {
            let Some(task) = self.state.selected_task().cloned() else {
                self.show_info_toast("no agent running");
                return;
            };
            let session_name = session_name_for_task(&task.slug);
            if !self.session.agent_sessions.is_ready(&session_name) {
                self.show_info_toast("no agent running");
                return;
            }
            let workspace = Workspace::try_new(
                task.name.clone(),
                task.root_path.clone(),
                task.branch.clone(),
                None,
                self.task_agent_for_selected_task(),
                WorkspaceStatus::Active,
                false,
            )
            .expect("task root workspace should be valid");

            self.set_stop_dialog(StopDialogState {
                workspace: workspace.clone(),
                session_name: session_name.clone(),
                focused_field: StopDialogField::StopButton,
            });
            self.log_dialog_event_with_fields(
                "stop",
                "dialog_opened",
                [
                    ("workspace".to_string(), Value::from(workspace.name.clone())),
                    ("branch".to_string(), Value::from(workspace.branch.clone())),
                    (
                        "path".to_string(),
                        Value::from(workspace.path.display().to_string()),
                    ),
                    ("session".to_string(), Value::from(session_name)),
                ],
            );
            self.session.last_tmux_error = None;
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
        let session_name = session_name_for_workspace_ref(&workspace);

        self.set_stop_dialog(StopDialogState {
            workspace: workspace.clone(),
            session_name: session_name.clone(),
            focused_field: StopDialogField::StopButton,
        });
        self.log_dialog_event_with_fields(
            "stop",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("branch".to_string(), Value::from(workspace.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
                ("session".to_string(), Value::from(session_name)),
            ],
        );
        self.session.last_tmux_error = None;
    }

    fn confirm_stop_dialog(&mut self) {
        if self.dialogs.stop_in_flight || self.dialogs.restart_in_flight {
            return;
        }

        let Some(dialog) = self.take_stop_dialog() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "stop",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace.name.clone()),
                ),
                (
                    "branch".to_string(),
                    Value::from(dialog.workspace.branch.clone()),
                ),
                (
                    "path".to_string(),
                    Value::from(dialog.workspace.path.display().to_string()),
                ),
                ("session".to_string(), Value::from(dialog.session_name)),
            ],
        );

        if self
            .state
            .tasks
            .iter()
            .any(|task| task.root_path == dialog.workspace.path)
        {
            if let Some(task) = self
                .state
                .tasks
                .iter()
                .find(|task| task.root_path == dialog.workspace.path)
                .cloned()
            {
                self.stop_task_agent(task);
            }
            return;
        }

        self.stop_workspace_agent(dialog.workspace);
    }
}

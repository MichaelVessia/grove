use super::*;

impl GroveApp {
    pub(super) fn handle_update_from_base_dialog_key(&mut self, key_event: KeyEvent) {
        if self.update_from_base_in_flight {
            return;
        }
        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("update_from_base", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("update_from_base", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('u') if no_modifiers => {
                self.confirm_update_from_base_dialog();
                return;
            }
            _ => {}
        }

        let mut confirm_update = false;
        let mut cancel_dialog = false;
        let Some(dialog) = self.update_from_base_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match dialog.focused_field {
                UpdateFromBaseDialogField::UpdateButton => {
                    confirm_update = true;
                }
                UpdateFromBaseDialogField::CancelButton => {
                    cancel_dialog = true;
                }
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
                if (dialog.focused_field == UpdateFromBaseDialogField::UpdateButton
                    || dialog.focused_field == UpdateFromBaseDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    dialog.focused_field =
                        if dialog.focused_field == UpdateFromBaseDialogField::UpdateButton {
                            UpdateFromBaseDialogField::CancelButton
                        } else {
                            UpdateFromBaseDialogField::UpdateButton
                        };
                }
            }
            _ => {}
        }

        if cancel_dialog {
            self.log_dialog_event("update_from_base", "dialog_cancelled");
            self.close_active_dialog();
            return;
        }
        if confirm_update {
            self.confirm_update_from_base_dialog();
        }
    }
    pub(super) fn open_update_from_base_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.update_from_base_in_flight {
            self.show_info_toast("workspace update already in progress");
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let base_branch = if workspace.is_main {
            workspace.branch.clone()
        } else {
            let Some(base_branch) = workspace
                .base_branch
                .clone()
                .filter(|value| !value.trim().is_empty())
            else {
                self.show_info_toast("workspace base branch marker is missing");
                return;
            };
            if base_branch == workspace.branch {
                self.show_info_toast("workspace branch already matches base branch");
                return;
            }
            base_branch
        };

        self.set_update_from_base_dialog(UpdateFromBaseDialogState {
            project_name: workspace.project_name.clone(),
            project_path: workspace.project_path.clone(),
            is_main_workspace: workspace.is_main,
            workspace_name: workspace.name.clone(),
            workspace_branch: workspace.branch.clone(),
            workspace_path: workspace.path.clone(),
            base_branch: base_branch.clone(),
            focused_field: UpdateFromBaseDialogField::UpdateButton,
        });
        self.log_dialog_event_with_fields(
            "update_from_base",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("branch".to_string(), Value::from(workspace.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
                ("base_branch".to_string(), Value::from(base_branch)),
                (
                    "is_main_workspace".to_string(),
                    Value::from(workspace.is_main),
                ),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }
    fn confirm_update_from_base_dialog(&mut self) {
        if self.update_from_base_in_flight {
            return;
        }

        let Some(dialog) = self.take_update_from_base_dialog() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "update_from_base",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                (
                    "workspace_branch".to_string(),
                    Value::from(dialog.workspace_branch.clone()),
                ),
                (
                    "workspace_path".to_string(),
                    Value::from(dialog.workspace_path.display().to_string()),
                ),
                (
                    "base_branch".to_string(),
                    Value::from(dialog.base_branch.clone()),
                ),
                (
                    "is_main_workspace".to_string(),
                    Value::from(dialog.is_main_workspace),
                ),
            ],
        );

        let workspace_name = dialog.workspace_name.clone();
        let workspace_path = dialog.workspace_path.clone();
        let workspace_branch = dialog.workspace_branch.clone();
        let base_branch = dialog.base_branch.clone();
        let request = UpdateWorkspaceFromBaseRequest {
            project_name: dialog.project_name,
            project_path: dialog.project_path,
            workspace_name: dialog.workspace_name,
            workspace_branch: dialog.workspace_branch,
            workspace_path: dialog.workspace_path,
            base_branch: dialog.base_branch,
        };

        if !self.tmux_input.supports_background_launch() {
            let (result, warnings) = update_workspace_from_base(request);
            self.apply_update_from_base_completion(UpdateWorkspaceFromBaseCompletion {
                workspace_name,
                workspace_path,
                workspace_branch,
                base_branch,
                result,
                warnings,
            });
            return;
        }

        self.update_from_base_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let (result, warnings) = update_workspace_from_base(request);
            Msg::UpdateWorkspaceFromBaseCompleted(UpdateWorkspaceFromBaseCompletion {
                workspace_name,
                workspace_path,
                workspace_branch,
                base_branch,
                result,
                warnings,
            })
        }));
    }
}

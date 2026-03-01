use super::*;

impl GroveApp {
    pub(super) fn handle_merge_dialog_key(&mut self, key_event: KeyEvent) {
        if self.dialogs.merge_in_flight {
            return;
        }
        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("merge", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("merge", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('m') if no_modifiers => {
                self.confirm_merge_dialog();
                return;
            }
            _ => {}
        }

        let mut confirm_merge = false;
        let mut cancel_dialog = false;
        let Some(dialog) = self.merge_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match dialog.focused_field {
                MergeDialogField::CleanupWorkspace => {
                    dialog.cleanup_workspace = !dialog.cleanup_workspace;
                }
                MergeDialogField::CleanupLocalBranch => {
                    dialog.cleanup_local_branch = !dialog.cleanup_local_branch;
                }
                MergeDialogField::MergeButton => {
                    confirm_merge = true;
                }
                MergeDialogField::CancelButton => {
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
            KeyCode::Char(' ') if no_modifiers => match dialog.focused_field {
                MergeDialogField::CleanupWorkspace => {
                    dialog.cleanup_workspace = !dialog.cleanup_workspace;
                }
                MergeDialogField::CleanupLocalBranch => {
                    dialog.cleanup_local_branch = !dialog.cleanup_local_branch;
                }
                MergeDialogField::MergeButton | MergeDialogField::CancelButton => {}
            },
            KeyCode::Char(character) if no_modifiers => {
                if (dialog.focused_field == MergeDialogField::MergeButton
                    || dialog.focused_field == MergeDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    dialog.focused_field = if dialog.focused_field == MergeDialogField::MergeButton
                    {
                        MergeDialogField::CancelButton
                    } else {
                        MergeDialogField::MergeButton
                    };
                }
            }
            _ => {}
        }

        if cancel_dialog {
            self.log_dialog_event("merge", "dialog_cancelled");
            self.close_active_dialog();
            return;
        }
        if confirm_merge {
            self.confirm_merge_dialog();
        }
    }
    pub(super) fn open_merge_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.dialogs.merge_in_flight {
            self.show_info_toast("workspace merge already in progress");
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        if workspace.is_main {
            self.show_info_toast("cannot merge base workspace");
            return;
        }
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

        self.set_merge_dialog(MergeDialogState {
            project_name: workspace.project_name.clone(),
            project_path: workspace.project_path.clone(),
            workspace_name: workspace.name.clone(),
            workspace_branch: workspace.branch.clone(),
            workspace_path: workspace.path.clone(),
            base_branch,
            cleanup_workspace: true,
            cleanup_local_branch: true,
            focused_field: MergeDialogField::CleanupWorkspace,
        });
        self.log_dialog_event_with_fields(
            "merge",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("branch".to_string(), Value::from(workspace.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
                (
                    "base_branch".to_string(),
                    Value::from(
                        workspace
                            .base_branch
                            .as_ref()
                            .map_or(String::new(), ToOwned::to_owned),
                    ),
                ),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.session.last_tmux_error = None;
    }
    fn confirm_merge_dialog(&mut self) {
        if self.dialogs.merge_in_flight {
            return;
        }

        let Some(dialog) = self.take_merge_dialog() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "merge",
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
                    "cleanup_workspace".to_string(),
                    Value::from(dialog.cleanup_workspace),
                ),
                (
                    "cleanup_local_branch".to_string(),
                    Value::from(dialog.cleanup_local_branch),
                ),
            ],
        );

        let workspace_name = dialog.workspace_name.clone();
        let workspace_path = dialog.workspace_path.clone();
        let workspace_branch = dialog.workspace_branch.clone();
        let base_branch = dialog.base_branch.clone();
        let request = MergeWorkspaceRequest {
            project_name: dialog.project_name,
            project_path: dialog.project_path,
            workspace_name: dialog.workspace_name,
            workspace_branch: dialog.workspace_branch,
            workspace_path: dialog.workspace_path,
            base_branch: dialog.base_branch,
            cleanup_workspace: dialog.cleanup_workspace,
            cleanup_local_branch: dialog.cleanup_local_branch,
        };

        if !self.tmux_input.supports_background_launch() {
            let (result, warnings) = merge_workspace(request);
            self.apply_merge_workspace_completion(MergeWorkspaceCompletion {
                workspace_name,
                workspace_path,
                workspace_branch,
                base_branch,
                result,
                warnings,
            });
            return;
        }

        self.dialogs.merge_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let (result, warnings) = merge_workspace(request);
            Msg::MergeWorkspaceCompleted(MergeWorkspaceCompletion {
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

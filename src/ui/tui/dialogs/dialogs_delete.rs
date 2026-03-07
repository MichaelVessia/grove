use super::*;

impl GroveApp {
    pub(super) fn workspace_delete_requested(&self, workspace_path: &Path) -> bool {
        self.dialogs
            .delete_requested_workspaces
            .contains(workspace_path)
    }

    fn task_delete_requested(&self, task: &Task) -> bool {
        task.worktrees
            .iter()
            .any(|worktree| self.workspace_delete_requested(worktree.path.as_path()))
    }

    fn queue_or_start_delete_workspace(&mut self, queued_delete: QueuedDeleteWorkspace) {
        let already_requested = queued_delete
            .requested_workspace_paths
            .iter()
            .any(|path| self.dialogs.delete_requested_workspaces.contains(path));
        if already_requested {
            self.show_info_toast(format!(
                "task '{}' delete already requested",
                queued_delete.workspace_name
            ));
            return;
        }
        for path in &queued_delete.requested_workspace_paths {
            self.dialogs
                .delete_requested_workspaces
                .insert(path.clone());
        }

        if self.dialogs.delete_in_flight {
            let queued_workspace_name = queued_delete.workspace_name.clone();
            self.dialogs
                .pending_delete_workspaces
                .push_back(queued_delete);
            self.show_info_toast(format!("task '{}' delete queued", queued_workspace_name));
            return;
        }

        self.launch_delete_workspace_task(queued_delete);
    }

    fn launch_delete_workspace_task(&mut self, queued_delete: QueuedDeleteWorkspace) {
        let request = queued_delete.request;
        let workspace_name = queued_delete.workspace_name;
        let workspace_path = queued_delete.workspace_path;
        let requested_workspace_paths = queued_delete.requested_workspace_paths;
        self.dialogs.delete_in_flight = true;
        self.dialogs.delete_in_flight_workspace = Some(workspace_path.clone());
        self.queue_cmd(Cmd::task(move || {
            let (result, warnings) = delete_task(request);
            Msg::DeleteWorkspaceCompleted(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                requested_workspace_paths,
                result,
                warnings,
            })
        }));
    }

    pub(super) fn start_next_queued_delete_workspace(&mut self) {
        if let Some(queued_delete) = self.dialogs.pending_delete_workspaces.pop_front() {
            self.launch_delete_workspace_task(queued_delete);
            return;
        }

        self.dialogs.delete_in_flight = false;
        self.dialogs.delete_in_flight_workspace = None;
    }

    pub(super) fn handle_delete_dialog_key(&mut self, key_event: KeyEvent) {
        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("delete", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("delete", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('D') if no_modifiers => {
                self.confirm_delete_dialog();
                return;
            }
            _ => {}
        }

        let mut confirm_delete = false;
        let mut cancel_dialog = false;
        let Some(dialog) = self.delete_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match dialog.focused_field {
                DeleteDialogField::DeleteLocalBranch => {
                    dialog.delete_local_branch = !dialog.delete_local_branch;
                }
                DeleteDialogField::KillTmuxSessions => {
                    dialog.kill_tmux_sessions = !dialog.kill_tmux_sessions;
                }
                DeleteDialogField::DeleteButton => {
                    confirm_delete = true;
                }
                DeleteDialogField::CancelButton => {
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
            KeyCode::Char(' ') if no_modifiers => {
                if dialog.focused_field == DeleteDialogField::DeleteLocalBranch {
                    dialog.delete_local_branch = !dialog.delete_local_branch;
                } else if dialog.focused_field == DeleteDialogField::KillTmuxSessions {
                    dialog.kill_tmux_sessions = !dialog.kill_tmux_sessions;
                }
            }
            KeyCode::Char(character) if no_modifiers => {
                if (dialog.focused_field == DeleteDialogField::DeleteButton
                    || dialog.focused_field == DeleteDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    dialog.focused_field =
                        if dialog.focused_field == DeleteDialogField::DeleteButton {
                            DeleteDialogField::CancelButton
                        } else {
                            DeleteDialogField::DeleteButton
                        };
                }
            }
            _ => {}
        }

        if cancel_dialog {
            self.log_dialog_event("delete", "dialog_cancelled");
            self.close_active_dialog();
            return;
        }
        if confirm_delete {
            self.confirm_delete_dialog();
        }
    }
    pub(super) fn open_delete_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let Some(task) = self.state.selected_task().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        if self
            .state
            .selected_workspace()
            .is_some_and(|workspace| workspace.is_main)
        {
            self.show_info_toast("cannot delete base workspace");
            return;
        }
        if self.task_delete_requested(&task) {
            self.show_info_toast(format!("task '{}' delete already requested", task.name));
            return;
        }

        let is_missing = !task.root_path.exists();
        self.set_delete_dialog(DeleteDialogState {
            task: task.clone(),
            is_missing,
            delete_local_branch: is_missing,
            kill_tmux_sessions: true,
            focused_field: DeleteDialogField::DeleteLocalBranch,
        });
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_opened",
            [
                ("task".to_string(), Value::from(task.name.clone())),
                ("branch".to_string(), Value::from(task.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(task.root_path.display().to_string()),
                ),
                (
                    "worktree_count".to_string(),
                    Value::from(usize_to_u64(task.worktrees.len())),
                ),
                ("is_missing".to_string(), Value::from(is_missing)),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.session.last_tmux_error = None;
    }
    fn confirm_delete_dialog(&mut self) {
        let Some(dialog) = self.take_delete_dialog() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_confirmed",
            [
                ("task".to_string(), Value::from(dialog.task.name.clone())),
                (
                    "branch".to_string(),
                    Value::from(dialog.task.branch.clone()),
                ),
                (
                    "path".to_string(),
                    Value::from(dialog.task.root_path.display().to_string()),
                ),
                (
                    "delete_local_branch".to_string(),
                    Value::from(dialog.delete_local_branch),
                ),
                (
                    "kill_tmux_sessions".to_string(),
                    Value::from(dialog.kill_tmux_sessions),
                ),
                (
                    "worktree_count".to_string(),
                    Value::from(usize_to_u64(dialog.task.worktrees.len())),
                ),
                ("is_missing".to_string(), Value::from(dialog.is_missing)),
            ],
        );

        let workspace_name = dialog.task.name.clone();
        let workspace_path = dialog.task.root_path.clone();
        let requested_workspace_paths = dialog
            .task
            .worktrees
            .iter()
            .map(|worktree| worktree.path.clone())
            .collect::<Vec<PathBuf>>();
        let request = DeleteTaskRequest {
            task: dialog.task,
            delete_local_branch: dialog.delete_local_branch,
            kill_tmux_sessions: dialog.kill_tmux_sessions,
        };
        if !self.tmux_input.supports_background_launch() {
            let (result, warnings) = delete_task(request);
            self.apply_delete_workspace_completion(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                requested_workspace_paths,
                result,
                warnings,
            });
            return;
        }

        self.queue_or_start_delete_workspace(QueuedDeleteWorkspace {
            request,
            workspace_name,
            workspace_path,
            requested_workspace_paths,
        });
    }
}

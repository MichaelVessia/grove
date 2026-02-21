use super::*;

impl GroveApp {
    pub(super) fn workspace_delete_requested(&self, workspace_path: &Path) -> bool {
        self.delete_requested_workspaces.contains(workspace_path)
    }

    fn queue_or_start_delete_workspace(&mut self, queued_delete: QueuedDeleteWorkspace) {
        if !self
            .delete_requested_workspaces
            .insert(queued_delete.workspace_path.clone())
        {
            self.show_info_toast(format!(
                "workspace '{}' delete already requested",
                queued_delete.workspace_name
            ));
            return;
        }

        if self.delete_in_flight {
            let queued_workspace_name = queued_delete.workspace_name.clone();
            self.pending_delete_workspaces.push_back(queued_delete);
            self.show_info_toast(format!(
                "workspace '{}' delete queued",
                queued_workspace_name
            ));
            return;
        }

        self.launch_delete_workspace_task(queued_delete);
    }

    fn launch_delete_workspace_task(&mut self, queued_delete: QueuedDeleteWorkspace) {
        let request = queued_delete.request;
        let workspace_name = queued_delete.workspace_name;
        let workspace_path = queued_delete.workspace_path;
        let daemon_socket_path = self.daemon_socket_path.clone();
        self.delete_in_flight = true;
        self.delete_in_flight_workspace = Some(workspace_path.clone());
        self.queue_cmd(Cmd::task(move || {
            let (result, warnings) = execute_workspace_delete(request, daemon_socket_path);
            Msg::DeleteWorkspaceCompleted(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result,
                warnings,
            })
        }));
    }

    pub(super) fn start_next_queued_delete_workspace(&mut self) {
        if let Some(queued_delete) = self.pending_delete_workspaces.pop_front() {
            self.launch_delete_workspace_task(queued_delete);
            return;
        }

        self.delete_in_flight = false;
        self.delete_in_flight_workspace = None;
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

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        if !self.ensure_workspace_backend_available(&workspace, "workspace delete") {
            return;
        }
        if workspace.is_main {
            self.show_info_toast("cannot delete base workspace");
            return;
        }
        if self.workspace_delete_requested(&workspace.path) {
            self.show_info_toast(format!(
                "workspace '{}' delete already requested",
                workspace.name
            ));
            return;
        }

        let is_missing = !workspace.path.exists();
        self.set_delete_dialog(DeleteDialogState {
            project_name: workspace.project_name.clone(),
            project_path: workspace.project_path.clone(),
            workspace_name: workspace.name.clone(),
            branch: workspace.branch.clone(),
            path: workspace.path.clone(),
            is_missing,
            delete_local_branch: is_missing,
            kill_tmux_sessions: true,
            focused_field: DeleteDialogField::DeleteLocalBranch,
        });
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("branch".to_string(), Value::from(workspace.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
                ("is_missing".to_string(), Value::from(is_missing)),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }
    fn confirm_delete_dialog(&mut self) {
        let Some(dialog) = self.take_delete_dialog() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "delete",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                ("branch".to_string(), Value::from(dialog.branch.clone())),
                (
                    "path".to_string(),
                    Value::from(dialog.path.display().to_string()),
                ),
                (
                    "delete_local_branch".to_string(),
                    Value::from(dialog.delete_local_branch),
                ),
                (
                    "kill_tmux_sessions".to_string(),
                    Value::from(dialog.kill_tmux_sessions),
                ),
                ("is_missing".to_string(), Value::from(dialog.is_missing)),
            ],
        );

        let workspace_name = dialog.workspace_name.clone();
        let workspace_path = dialog.path.clone();
        let Some(repo_root) = dialog.project_path else {
            self.apply_delete_workspace_completion(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result: Err("workspace project root unavailable".to_string()),
                warnings: Vec::new(),
            });
            return;
        };
        let request = WorkspaceDeleteRequest {
            context: RepoContext { repo_root },
            selector: WorkspaceSelector::NameAndPath {
                name: dialog.workspace_name,
                path: dialog.path,
            },
            delete_branch: dialog.delete_local_branch,
            force_stop: dialog.kill_tmux_sessions,
            dry_run: false,
        };
        if !self.tmux_input.supports_background_launch() {
            let (result, warnings) =
                execute_workspace_delete(request, self.daemon_socket_path.clone());
            self.apply_delete_workspace_completion(DeleteWorkspaceCompletion {
                workspace_name,
                workspace_path,
                result,
                warnings,
            });
            return;
        }

        self.queue_or_start_delete_workspace(QueuedDeleteWorkspace {
            request,
            workspace_name,
            workspace_path,
        });
    }
}

fn execute_workspace_delete(
    request: WorkspaceDeleteRequest,
    daemon_socket_path: Option<PathBuf>,
) -> (Result<(), String>, Vec<String>) {
    if let Some(socket_path) = daemon_socket_path.as_deref() {
        let (workspace, workspace_path) = daemon_selector_parts(&request.selector);
        let payload = DaemonWorkspaceDeletePayload {
            repo_root: request.context.repo_root.display().to_string(),
            workspace,
            workspace_path,
            delete_branch: request.delete_branch,
            force_stop: request.force_stop,
            dry_run: request.dry_run,
        };
        return match workspace_delete_via_socket(socket_path, payload) {
            Ok(Ok(response)) => (Ok(()), response.warnings),
            Ok(Err(error)) => (Err(error.message), Vec::new()),
            Err(error) => (Err(format!("daemon request failed: {error}")), Vec::new()),
        };
    }

    let service = InProcessLifecycleCommandService::new();
    match service.workspace_delete(request) {
        Ok(response) => (Ok(()), response.warnings),
        Err(error) => (Err(error.message), Vec::new()),
    }
}

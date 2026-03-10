use super::*;

impl GroveApp {
    pub(super) fn handle_pull_upstream_dialog_key(&mut self, key_event: KeyEvent) {
        if self.dialogs.pull_upstream_in_flight {
            return;
        }
        let no_modifiers = key_event.modifiers.is_empty();
        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("pull_upstream", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('q') if no_modifiers => {
                self.log_dialog_event("pull_upstream", "dialog_cancelled");
                self.close_active_dialog();
                return;
            }
            KeyCode::Char('p') if no_modifiers => {
                self.confirm_pull_upstream_dialog();
                return;
            }
            _ => {}
        }

        let mut confirm_pull = false;
        let mut cancel_dialog = false;
        let Some(dialog) = self.pull_upstream_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Enter => match dialog.focused_field {
                PullUpstreamDialogField::PullButton => {
                    confirm_pull = true;
                }
                PullUpstreamDialogField::CancelButton => {
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
                if (dialog.focused_field == PullUpstreamDialogField::PullButton
                    || dialog.focused_field == PullUpstreamDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    dialog.focused_field =
                        if dialog.focused_field == PullUpstreamDialogField::PullButton {
                            PullUpstreamDialogField::CancelButton
                        } else {
                            PullUpstreamDialogField::PullButton
                        };
                }
            }
            _ => {}
        }

        if cancel_dialog {
            self.log_dialog_event("pull_upstream", "dialog_cancelled");
            self.close_active_dialog();
            return;
        }
        if confirm_pull {
            self.confirm_pull_upstream_dialog();
        }
    }

    pub(super) fn open_pull_upstream_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.dialogs.pull_upstream_in_flight {
            self.show_info_toast("pull upstream already in progress");
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        if !workspace.is_main {
            self.show_info_toast("pull upstream is only available on base workspaces");
            return;
        }

        let base_branch = workspace.branch.clone();
        let project_path = workspace.project_path.clone().unwrap_or_default();
        let project_name = workspace.project_name.clone().unwrap_or_default();

        let propagate_target_count = self
            .state
            .workspaces
            .iter()
            .filter(|ws| {
                !ws.is_main
                    && ws
                        .project_path
                        .as_ref()
                        .is_some_and(|pp| refer_to_same_location(pp, &project_path))
                    && ws.base_branch.as_ref().is_some_and(|bb| bb == &base_branch)
            })
            .count();

        self.set_pull_upstream_dialog(PullUpstreamDialogState {
            task_slug: workspace.task_slug.clone(),
            project_name,
            project_path: project_path.clone(),
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
            base_branch: base_branch.clone(),
            propagate_target_count,
            focused_field: PullUpstreamDialogField::PullButton,
        });
        self.log_dialog_event_with_fields(
            "pull_upstream",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("branch".to_string(), Value::from(base_branch.clone())),
                (
                    "path".to_string(),
                    Value::from(workspace.path.display().to_string()),
                ),
                ("base_branch".to_string(), Value::from(base_branch)),
                (
                    "propagate_target_count".to_string(),
                    Value::from(propagate_target_count as u64),
                ),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.session.last_tmux_error = None;
    }

    fn confirm_pull_upstream_dialog(&mut self) {
        if self.dialogs.pull_upstream_in_flight {
            return;
        }

        let Some(dialog) = self.take_pull_upstream_dialog() else {
            return;
        };
        self.log_dialog_event_with_fields(
            "pull_upstream",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace_name.clone()),
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
                    "propagate_target_count".to_string(),
                    Value::from(dialog.propagate_target_count as u64),
                ),
            ],
        );

        let workspace_name = dialog.workspace_name.clone();
        let workspace_path = dialog.workspace_path.clone();
        let base_branch = dialog.base_branch.clone();
        let propagate_target_count = dialog.propagate_target_count;
        let request = UpdateWorkspaceFromBaseRequest {
            task_slug: dialog.task_slug,
            project_name: Some(dialog.project_name),
            project_path: Some(dialog.project_path),
            workspace_name: dialog.workspace_name,
            workspace_branch: dialog.base_branch.clone(),
            workspace_path: dialog.workspace_path,
            base_branch: dialog.base_branch,
        };

        if !self.tmux_input.supports_background_launch() {
            let (result, _warnings) =
                update_workspace_from_base_with_terminator(request, &RuntimeSessionTerminator);
            self.apply_pull_upstream_completion(PullUpstreamCompletion {
                workspace_name,
                workspace_path,
                base_branch,
                result,
                propagate_target_count,
            });
            return;
        }

        self.dialogs.pull_upstream_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let (result, _warnings) =
                update_workspace_from_base_with_terminator(request, &RuntimeSessionTerminator);
            Msg::PullUpstreamCompleted(PullUpstreamCompletion {
                workspace_name,
                workspace_path,
                base_branch,
                result,
                propagate_target_count,
            })
        }));
    }

    pub(super) fn apply_pull_upstream_completion(&mut self, completion: PullUpstreamCompletion) {
        self.dialogs.pull_upstream_in_flight = false;
        match completion.result {
            Ok(()) => {
                self.telemetry.event_log.log(
                    LogEvent::new("workspace_lifecycle", "pull_upstream_completed")
                        .with_data("workspace", Value::from(completion.workspace_name.clone()))
                        .with_data("base_branch", Value::from(completion.base_branch.clone()))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data(
                            "propagate_target_count",
                            Value::from(completion.propagate_target_count as u64),
                        ),
                );
                self.session.last_tmux_error = None;
                self.refresh_workspaces(Some(completion.workspace_path));
                if completion.propagate_target_count > 0 {
                    self.show_info_toast(format!(
                        "pulled {}, {} workspace(s) can be updated",
                        completion.base_branch, completion.propagate_target_count
                    ));
                } else {
                    self.show_success_toast(format!("pulled {}", completion.base_branch));
                }
            }
            Err(error) => {
                self.telemetry.event_log.log(
                    LogEvent::new("workspace_lifecycle", "pull_upstream_failed")
                        .with_data("workspace", Value::from(completion.workspace_name))
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.display().to_string()),
                        )
                        .with_data("error", Value::from(error.clone())),
                );
                self.session.last_tmux_error = Some(error.clone());
                self.show_error_toast(format!("pull upstream failed: {error}"));
            }
        }
    }
}

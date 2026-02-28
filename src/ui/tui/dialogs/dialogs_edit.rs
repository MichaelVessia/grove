use crate::infrastructure::process::stderr_trimmed;

use super::*;

impl GroveApp {
    fn switch_workspace_branch(workspace_path: &Path, branch: &str) -> Result<(), String> {
        let output = Command::new("git")
            .current_dir(workspace_path)
            .args(["switch", branch])
            .output()
            .map_err(|error| format!("git switch {branch}: {error}"))?;
        if output.status.success() {
            return Ok(());
        }

        let stderr = stderr_trimmed(&output);
        if stderr.is_empty() {
            return Err(format!(
                "git switch {branch}: exit status {}",
                output.status
            ));
        }
        Err(format!("git switch {branch}: {stderr}"))
    }

    pub(super) fn handle_edit_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.edit_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        enum PostAction {
            None,
            Save,
            Cancel,
        }

        if dialog.focused_field == EditDialogField::BaseBranch
            && Self::allows_text_input_modifiers(key_event.modifiers)
        {
            match key_event.code {
                KeyCode::Backspace => {
                    dialog.base_branch.pop();
                    return;
                }
                KeyCode::Char(character) if !character.is_control() => {
                    dialog.base_branch.push(character);
                    return;
                }
                _ => {}
            }
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Char(_) if ctrl_n => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(_) if ctrl_p => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Char('j')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != EditDialogField::BaseBranch =>
            {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char('k')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != EditDialogField::BaseBranch =>
            {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::select_previous_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::CancelButton {
                    dialog.focused_field = EditDialogField::SaveButton;
                }
            }
            KeyCode::Right => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::select_next_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::SaveButton {
                    dialog.focused_field = EditDialogField::CancelButton;
                }
            }
            KeyCode::Char('h')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != EditDialogField::BaseBranch =>
            {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::select_previous_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::CancelButton {
                    dialog.focused_field = EditDialogField::SaveButton;
                }
            }
            KeyCode::Char('l')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != EditDialogField::BaseBranch =>
            {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::select_next_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::SaveButton {
                    dialog.focused_field = EditDialogField::CancelButton;
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::select_next_edit_dialog_agent(dialog);
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                EditDialogField::BaseBranch => dialog.focused_field = dialog.focused_field.next(),
                EditDialogField::Agent => Self::select_next_edit_dialog_agent(dialog),
                EditDialogField::SaveButton => post_action = PostAction::Save,
                EditDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_edit_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("edit", "dialog_cancelled");
                self.close_active_dialog();
            }
        }
    }
    pub(super) fn open_edit_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let base_branch = if workspace.is_main {
            workspace.branch.clone()
        } else {
            workspace
                .base_branch
                .as_ref()
                .map(|branch| branch.trim())
                .filter(|branch| !branch.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| workspace.branch.clone())
        };

        self.set_edit_dialog(EditDialogState {
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
            is_main: workspace.is_main,
            branch: workspace.branch.clone(),
            base_branch: base_branch.clone(),
            agent: workspace.agent,
            was_running: workspace.status.has_session(),
            focused_field: EditDialogField::BaseBranch,
        });
        self.log_dialog_event_with_fields(
            "edit",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("base_branch".to_string(), Value::from(base_branch)),
                ("agent".to_string(), Value::from(workspace.agent.label())),
                (
                    "running".to_string(),
                    Value::from(workspace.status.has_session()),
                ),
            ],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    fn select_next_edit_dialog_agent(dialog: &mut EditDialogState) {
        dialog.agent = Self::next_agent(dialog.agent);
    }

    fn select_previous_edit_dialog_agent(dialog: &mut EditDialogState) {
        dialog.agent = Self::previous_agent(dialog.agent);
    }

    fn apply_edit_dialog_save(&mut self) {
        let Some(dialog) = self.edit_dialog().cloned() else {
            return;
        };
        let target_branch = dialog.base_branch.trim().to_string();
        if target_branch.is_empty() {
            self.show_info_toast(workspace_lifecycle_error_message(
                &WorkspaceLifecycleError::EmptyBaseBranch,
            ));
            return;
        }
        if dialog.is_main
            && let Err(error) = Self::switch_workspace_branch(
                dialog.workspace_path.as_path(),
                target_branch.as_str(),
            )
        {
            self.show_error_toast(format!("base workspace switch failed: {error}"));
            return;
        }

        self.log_dialog_event_with_fields(
            "edit",
            "dialog_confirmed",
            [
                (
                    "workspace".to_string(),
                    Value::from(dialog.workspace_name.clone()),
                ),
                (
                    "base_branch".to_string(),
                    Value::from(target_branch.clone()),
                ),
                ("agent".to_string(), Value::from(dialog.agent.label())),
                ("was_running".to_string(), Value::from(dialog.was_running)),
                ("is_main".to_string(), Value::from(dialog.is_main)),
            ],
        );

        if let Err(error) =
            write_workspace_base_marker(&dialog.workspace_path, target_branch.as_str())
        {
            self.show_error_toast(format!(
                "workspace edit failed: {}",
                workspace_lifecycle_error_message(&error)
            ));
            return;
        }

        if let Err(error) = write_workspace_agent_marker(&dialog.workspace_path, dialog.agent) {
            self.show_error_toast(format!(
                "workspace edit failed: {}",
                workspace_lifecycle_error_message(&error)
            ));
            return;
        }

        if let Some(workspace) = self
            .state
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.path == dialog.workspace_path)
        {
            workspace.agent = dialog.agent;
            workspace.base_branch = Some(target_branch.clone());
            if dialog.is_main {
                workspace.branch = target_branch.clone();
            }
            workspace.supported_agent = true;
        }

        self.close_active_dialog();
        self.last_tmux_error = None;
        if dialog.is_main && dialog.was_running {
            self.show_info_toast("base workspace switched, restart agent to apply agent change");
        } else if dialog.is_main {
            self.show_success_toast(format!("base workspace switched to '{target_branch}'"));
        } else if dialog.was_running {
            self.show_info_toast("workspace updated, restart agent to apply change");
        } else {
            self.show_success_toast("workspace updated");
        }
    }
}

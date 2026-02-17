use super::*;

impl GroveApp {
    pub(super) fn handle_edit_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.edit_dialog.as_mut() else {
            return;
        };

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
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::CancelButton {
                    dialog.focused_field = EditDialogField::SaveButton;
                }
            }
            KeyCode::Right => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::SaveButton {
                    dialog.focused_field = EditDialogField::CancelButton;
                }
            }
            KeyCode::Char('h')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != EditDialogField::BaseBranch =>
            {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::CancelButton {
                    dialog.focused_field = EditDialogField::SaveButton;
                }
            }
            KeyCode::Char('l')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != EditDialogField::BaseBranch =>
            {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                } else if dialog.focused_field == EditDialogField::SaveButton {
                    dialog.focused_field = EditDialogField::CancelButton;
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == EditDialogField::Agent {
                    Self::toggle_edit_dialog_agent(dialog);
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                EditDialogField::BaseBranch => dialog.focused_field = dialog.focused_field.next(),
                EditDialogField::Agent => Self::toggle_edit_dialog_agent(dialog),
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
                self.edit_dialog = None;
            }
        }
    }
    pub(super) fn open_edit_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        let base_branch = workspace
            .base_branch
            .as_ref()
            .map(|branch| branch.trim())
            .filter(|branch| !branch.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| workspace.branch.clone());

        self.edit_dialog = Some(EditDialogState {
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
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

    fn toggle_edit_dialog_agent(dialog: &mut EditDialogState) {
        dialog.agent = Self::toggle_agent(dialog.agent);
    }

    fn apply_edit_dialog_save(&mut self) {
        let Some(dialog) = self.edit_dialog.as_ref().cloned() else {
            return;
        };
        let base_branch = dialog.base_branch.trim().to_string();
        if base_branch.is_empty() {
            self.show_toast(
                workspace_lifecycle_error_message(&WorkspaceLifecycleError::EmptyBaseBranch),
                true,
            );
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
                ("base_branch".to_string(), Value::from(base_branch.clone())),
                ("agent".to_string(), Value::from(dialog.agent.label())),
                ("was_running".to_string(), Value::from(dialog.was_running)),
            ],
        );

        if let Err(error) =
            write_workspace_base_marker(&dialog.workspace_path, base_branch.as_str())
        {
            self.show_toast(
                format!(
                    "workspace edit failed: {}",
                    workspace_lifecycle_error_message(&error)
                ),
                true,
            );
            return;
        }

        if let Err(error) = write_workspace_agent_marker(&dialog.workspace_path, dialog.agent) {
            self.show_toast(
                format!(
                    "workspace edit failed: {}",
                    workspace_lifecycle_error_message(&error)
                ),
                true,
            );
            return;
        }

        if let Some(workspace) = self
            .state
            .workspaces
            .iter_mut()
            .find(|workspace| workspace.path == dialog.workspace_path)
        {
            workspace.agent = dialog.agent;
            workspace.base_branch = Some(base_branch);
            workspace.supported_agent = true;
        }

        self.edit_dialog = None;
        self.last_tmux_error = None;
        if dialog.was_running {
            self.show_toast("workspace updated, restart agent to apply change", false);
        } else {
            self.show_toast("workspace updated", false);
        }
    }
}

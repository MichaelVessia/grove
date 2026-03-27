use super::*;

impl GroveApp {
    pub(super) fn open_start_parent_agent_dialog(&mut self) {
        if self.dialogs.start_in_flight || self.dialogs.restart_in_flight {
            self.show_info_toast("agent lifecycle already in progress");
            return;
        }

        let Some(task) = self.state.selected_task().cloned() else {
            self.show_info_toast("no task selected");
            return;
        };

        let prompt = read_workspace_launch_prompt(&task.root_path).unwrap_or_default();
        let init_command = self.task_init_command_for_task(&task);
        let permission_mode = self.task_permission_mode_for_task(&task);
        let agent = self.task_agent_for_selected_task();
        self.set_launch_dialog(LaunchDialogState {
            target: LaunchDialogTarget::ParentTask(task.clone()),
            agent,
            start_config: StartAgentConfigState::new(
                String::new(),
                prompt.clone(),
                init_command.clone().unwrap_or_default(),
                permission_mode,
            ),
        });
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(task.name.clone())),
                ("agent".to_string(), Value::from(agent.label())),
                ("name_len".to_string(), Value::from(0u64)),
                (
                    "prompt_len".to_string(),
                    Value::from(usize_to_u64(prompt.len())),
                ),
                (
                    "permission_mode".to_string(),
                    Value::from(permission_mode.label()),
                ),
                (
                    "init_len".to_string(),
                    Value::from(usize_to_u64(init_command.unwrap_or_default().len())),
                ),
            ],
        );
        self.session.last_tmux_error = None;
    }

    pub(super) fn handle_launch_dialog_key(&mut self, key_event: KeyEvent) {
        if self.dialogs.start_in_flight || self.dialogs.restart_in_flight {
            return;
        }
        self.sync_active_dialog_focus_field();
        let Some(focused_field) = self.current_launch_dialog_focus_field() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("launch", "dialog_cancelled");
                self.close_active_dialog();
            }
            KeyCode::Enter => {
                enum EnterAction {
                    ConfirmStart,
                    CancelDialog,
                }

                let action = match focused_field {
                    LaunchDialogField::Agent => EnterAction::ConfirmStart,
                    LaunchDialogField::StartButton => EnterAction::ConfirmStart,
                    LaunchDialogField::CancelButton => EnterAction::CancelDialog,
                    LaunchDialogField::StartConfig(_) => EnterAction::ConfirmStart,
                };

                match action {
                    EnterAction::ConfirmStart => self.confirm_start_dialog(),
                    EnterAction::CancelDialog => {
                        self.log_dialog_event("launch", "dialog_cancelled");
                        self.close_active_dialog();
                    }
                }
            }
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
            KeyCode::Backspace => {
                if let Some(dialog) = self.launch_dialog_mut()
                    && let LaunchDialogField::StartConfig(field) = focused_field
                {
                    dialog.start_config.backspace(field);
                }
            }
            KeyCode::Left => {
                if let Some(dialog) = self.launch_dialog_mut()
                    && focused_field == LaunchDialogField::Agent
                {
                    dialog.agent = dialog.agent.previous();
                }
            }
            KeyCode::Right => {
                if let Some(dialog) = self.launch_dialog_mut()
                    && focused_field == LaunchDialogField::Agent
                {
                    dialog.agent = dialog.agent.next();
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if (focused_field == LaunchDialogField::StartButton
                    || focused_field == LaunchDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    self.focus_dialog_field(if focused_field == LaunchDialogField::StartButton {
                        FOCUS_ID_LAUNCH_CANCEL_BUTTON
                    } else {
                        FOCUS_ID_LAUNCH_START_BUTTON
                    });
                    return;
                }

                if let Some(dialog) = self.launch_dialog_mut() {
                    match focused_field {
                        LaunchDialogField::Agent => {
                            if character == 'j' || character == 'l' {
                                dialog.agent = dialog.agent.next();
                            } else if character == 'k' || character == 'h' {
                                dialog.agent = dialog.agent.previous();
                            }
                        }
                        LaunchDialogField::StartConfig(field) => match field {
                            StartAgentConfigField::Name
                            | StartAgentConfigField::Prompt
                            | StartAgentConfigField::InitCommand => {
                                if !character.is_control() {
                                    dialog.start_config.push_char(field, character);
                                }
                            }
                            StartAgentConfigField::Unsafe => {
                                if character == ' ' || character == 'j' || character == 'k' {
                                    dialog.start_config.cycle_permission_mode(dialog.agent);
                                }
                            }
                        },
                        LaunchDialogField::StartButton | LaunchDialogField::CancelButton => {}
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn open_start_dialog(&mut self) {
        if self.dialogs.start_in_flight || self.dialogs.restart_in_flight {
            self.show_info_toast("agent lifecycle already in progress");
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let prompt = read_workspace_launch_prompt(&workspace.path).unwrap_or_default();
        let init_command = self.workspace_init_command_for_workspace(&workspace);
        let permission_mode = self.workspace_permission_mode_for_workspace(&workspace);
        let agent = self
            .last_agent_selection
            .get(workspace.path.as_path())
            .copied()
            .unwrap_or(workspace.agent);
        self.set_launch_dialog(LaunchDialogState {
            target: LaunchDialogTarget::WorkspaceTab,
            agent,
            start_config: StartAgentConfigState::new(
                String::new(),
                prompt.clone(),
                init_command.clone().unwrap_or_default(),
                permission_mode,
            ),
        });
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                ("agent".to_string(), Value::from(agent.label())),
                ("name_len".to_string(), Value::from(0u64)),
                (
                    "prompt_len".to_string(),
                    Value::from(usize_to_u64(prompt.len())),
                ),
                (
                    "permission_mode".to_string(),
                    Value::from(permission_mode.label()),
                ),
                (
                    "init_len".to_string(),
                    Value::from(usize_to_u64(init_command.unwrap_or_default().len())),
                ),
            ],
        );
        self.session.last_tmux_error = None;
    }
}

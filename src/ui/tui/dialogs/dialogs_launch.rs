use super::*;

impl GroveApp {
    pub(super) fn handle_launch_dialog_key(&mut self, key_event: KeyEvent) {
        if self.dialogs.start_in_flight || self.dialogs.restart_in_flight {
            return;
        }
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

                let action = self
                    .launch_dialog()
                    .map(|dialog| match dialog.focused_field {
                        LaunchDialogField::Agent => EnterAction::ConfirmStart,
                        LaunchDialogField::StartButton => EnterAction::ConfirmStart,
                        LaunchDialogField::CancelButton => EnterAction::CancelDialog,
                        LaunchDialogField::StartConfig(_) => EnterAction::ConfirmStart,
                    });

                match action {
                    Some(EnterAction::ConfirmStart) => self.confirm_start_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("launch", "dialog_cancelled");
                        self.close_active_dialog();
                    }
                    None => {}
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.launch_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.launch_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Char(_) if ctrl_n => {
                if let Some(dialog) = self.launch_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::Char(_) if ctrl_p => {
                if let Some(dialog) = self.launch_dialog_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.launch_dialog_mut()
                    && let LaunchDialogField::StartConfig(field) = dialog.focused_field
                {
                    dialog.start_config.backspace(field);
                }
            }
            KeyCode::Left => {
                if let Some(dialog) = self.launch_dialog_mut()
                    && dialog.focused_field == LaunchDialogField::Agent
                {
                    dialog.agent = dialog.agent.previous();
                }
            }
            KeyCode::Right => {
                if let Some(dialog) = self.launch_dialog_mut()
                    && dialog.focused_field == LaunchDialogField::Agent
                {
                    dialog.agent = dialog.agent.next();
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if let Some(dialog) = self.launch_dialog_mut() {
                    if (dialog.focused_field == LaunchDialogField::StartButton
                        || dialog.focused_field == LaunchDialogField::CancelButton)
                        && (character == 'h' || character == 'l')
                    {
                        dialog.focused_field =
                            if dialog.focused_field == LaunchDialogField::StartButton {
                                LaunchDialogField::CancelButton
                            } else {
                                LaunchDialogField::StartButton
                            };
                        return;
                    }
                    match dialog.focused_field {
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
                                    dialog.start_config.toggle_unsafe();
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

        if self.selected_home_tab_targets_task_root() {
            let Some(task) = self.state.selected_task().cloned() else {
                self.show_info_toast("no task selected");
                return;
            };
            let prompt = read_workspace_launch_prompt(&task.root_path).unwrap_or_default();
            let init_command = self.task_init_command_for_task(&task);
            let skip_permissions = self.task_skip_permissions_for_task(&task);
            let agent = self.task_agent_for_selected_task();
            self.set_launch_dialog(LaunchDialogState {
                agent,
                start_config: StartAgentConfigState::new(
                    String::new(),
                    prompt.clone(),
                    init_command.clone().unwrap_or_default(),
                    skip_permissions,
                ),
                focused_field: LaunchDialogField::Agent,
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
                        "skip_permissions".to_string(),
                        Value::from(skip_permissions),
                    ),
                    (
                        "init_len".to_string(),
                        Value::from(usize_to_u64(init_command.unwrap_or_default().len())),
                    ),
                ],
            );
            self.session.last_tmux_error = None;
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let prompt = read_workspace_launch_prompt(&workspace.path).unwrap_or_default();
        let init_command = self.workspace_init_command_for_workspace(&workspace);
        let skip_permissions = self.workspace_skip_permissions_for_workspace(&workspace);
        let agent = self
            .last_agent_selection
            .get(workspace.path.as_path())
            .copied()
            .unwrap_or(workspace.agent);
        self.set_launch_dialog(LaunchDialogState {
            agent,
            start_config: StartAgentConfigState::new(
                String::new(),
                prompt.clone(),
                init_command.clone().unwrap_or_default(),
                skip_permissions,
            ),
            focused_field: LaunchDialogField::Agent,
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
                    "skip_permissions".to_string(),
                    Value::from(skip_permissions),
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

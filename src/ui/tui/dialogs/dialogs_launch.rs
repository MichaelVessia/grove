use super::*;

impl GroveApp {
    pub(super) fn handle_launch_dialog_key(&mut self, key_event: KeyEvent) {
        if self.start_in_flight || self.restart_in_flight {
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
            KeyCode::Left | KeyCode::Right => {}
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
                        LaunchDialogField::StartConfig(field) => match field {
                            StartAgentConfigField::Prompt | StartAgentConfigField::InitCommand => {
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
        if self.start_in_flight || self.restart_in_flight {
            self.show_info_toast("agent lifecycle already in progress");
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        if !workspace.supported_agent {
            self.show_info_toast("unsupported workspace agent marker");
            return;
        }
        if workspace.status.is_running() {
            self.show_info_toast("agent already running");
            return;
        }
        if !workspace_can_start_agent(Some(&workspace)) {
            self.show_info_toast("workspace cannot be started");
            return;
        }

        let prompt = read_workspace_launch_prompt(&workspace.path).unwrap_or_default();
        let init_command = self.workspace_init_command_for_workspace(&workspace);
        let skip_permissions = self.workspace_skip_permissions_for_workspace(&workspace);
        self.set_launch_dialog(LaunchDialogState {
            start_config: StartAgentConfigState::new(
                prompt.clone(),
                init_command.clone().unwrap_or_default(),
                skip_permissions,
            ),
            focused_field: LaunchDialogField::StartConfig(StartAgentConfigField::Prompt),
        });
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
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
        self.last_tmux_error = None;
    }
}

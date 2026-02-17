use super::*;

impl GroveApp {
    pub(super) fn handle_launch_dialog_key(&mut self, key_event: KeyEvent) {
        if self.start_in_flight {
            return;
        }
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("launch", "dialog_cancelled");
                self.launch_dialog = None;
            }
            KeyCode::Enter => {
                enum EnterAction {
                    ConfirmStart,
                    CancelDialog,
                }

                let action = self
                    .launch_dialog
                    .as_ref()
                    .map(|dialog| match dialog.focused_field {
                        LaunchDialogField::StartButton => EnterAction::ConfirmStart,
                        LaunchDialogField::CancelButton => EnterAction::CancelDialog,
                        LaunchDialogField::Prompt
                        | LaunchDialogField::PreLaunchCommand
                        | LaunchDialogField::Unsafe => EnterAction::ConfirmStart,
                    });

                match action {
                    Some(EnterAction::ConfirmStart) => self.confirm_start_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("launch", "dialog_cancelled");
                        self.launch_dialog = None;
                    }
                    None => {}
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Char(_) if ctrl_n => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::Char(_) if ctrl_p => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => {
                            dialog.prompt.pop();
                        }
                        LaunchDialogField::PreLaunchCommand => {
                            dialog.pre_launch_command.pop();
                        }
                        LaunchDialogField::Unsafe
                        | LaunchDialogField::StartButton
                        | LaunchDialogField::CancelButton => {}
                    }
                }
            }
            KeyCode::Left | KeyCode::Right => {}
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
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
                        LaunchDialogField::Prompt => dialog.prompt.push(character),
                        LaunchDialogField::PreLaunchCommand => {
                            dialog.pre_launch_command.push(character)
                        }
                        LaunchDialogField::Unsafe => {
                            if character == ' ' || character == 'j' || character == 'k' {
                                dialog.skip_permissions = !dialog.skip_permissions;
                            }
                        }
                        LaunchDialogField::StartButton | LaunchDialogField::CancelButton => {}
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn open_start_dialog(&mut self) {
        if self.start_in_flight {
            self.show_toast("agent start already in progress", true);
            return;
        }

        let Some(workspace) = self.state.selected_workspace() else {
            self.show_toast("no workspace selected", true);
            return;
        };
        if !workspace.supported_agent {
            self.show_toast("unsupported workspace agent marker", true);
            return;
        }
        if workspace.status.is_running() {
            self.show_toast("agent already running", true);
            return;
        }
        if !workspace_can_start_agent(Some(workspace)) {
            self.show_toast("workspace cannot be started", true);
            return;
        }

        let prompt = read_workspace_launch_prompt(&workspace.path).unwrap_or_default();
        let skip_permissions = self.launch_skip_permissions;
        self.launch_dialog = Some(LaunchDialogState {
            prompt: prompt.clone(),
            pre_launch_command: String::new(),
            skip_permissions,
            focused_field: LaunchDialogField::Prompt,
        });
        self.log_dialog_event_with_fields(
            "launch",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name.clone())),
                (
                    "prompt_len".to_string(),
                    Value::from(u64::try_from(prompt.len()).unwrap_or(u64::MAX)),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(skip_permissions),
                ),
                ("pre_launch_len".to_string(), Value::from(0_u64)),
            ],
        );
        self.last_tmux_error = None;
    }
}

use super::*;

impl GroveApp {
    fn keybinding_task_running(&self) -> bool {
        self.refresh_in_flight
            || self.project_delete_in_flight
            || self.delete_in_flight
            || self.merge_in_flight
            || self.update_from_base_in_flight
            || self.create_in_flight
            || self.start_in_flight
            || self.stop_in_flight
    }

    fn keybinding_input_nonempty(&self) -> bool {
        if let Some(dialog) = self.launch_dialog.as_ref() {
            return !dialog.prompt.is_empty() || !dialog.pre_launch_command.is_empty();
        }
        if let Some(dialog) = self.create_dialog.as_ref() {
            return !dialog.workspace_name.is_empty() || !dialog.base_branch.is_empty();
        }
        if let Some(project_dialog) = self.project_dialog.as_ref() {
            if !project_dialog.filter.is_empty() {
                return true;
            }
            if let Some(add_dialog) = project_dialog.add_dialog.as_ref() {
                return !add_dialog.name.is_empty() || !add_dialog.path.is_empty();
            }
        }

        false
    }

    pub(super) fn keybinding_state(&self) -> KeybindingAppState {
        KeybindingAppState::new()
            .with_input(self.keybinding_input_nonempty())
            .with_task(self.keybinding_task_running())
            .with_modal(self.modal_open())
    }
    pub(super) fn apply_keybinding_action(&mut self, action: KeybindingAction) -> bool {
        match action {
            KeybindingAction::DismissModal => {
                if self.create_dialog.is_some() {
                    self.log_dialog_event("create", "dialog_cancelled");
                    self.create_dialog = None;
                    self.clear_create_branch_picker();
                } else if self.edit_dialog.is_some() {
                    self.log_dialog_event("edit", "dialog_cancelled");
                    self.edit_dialog = None;
                } else if self.launch_dialog.is_some() {
                    self.log_dialog_event("launch", "dialog_cancelled");
                    self.launch_dialog = None;
                } else if self.delete_dialog.is_some() {
                    self.log_dialog_event("delete", "dialog_cancelled");
                    self.delete_dialog = None;
                } else if self.merge_dialog.is_some() {
                    self.log_dialog_event("merge", "dialog_cancelled");
                    self.merge_dialog = None;
                } else if self.update_from_base_dialog.is_some() {
                    self.log_dialog_event("update_from_base", "dialog_cancelled");
                    self.update_from_base_dialog = None;
                } else if self.settings_dialog.is_some() {
                    self.log_dialog_event("settings", "dialog_cancelled");
                    self.settings_dialog = None;
                } else if self.project_dialog.is_some() {
                    self.project_dialog = None;
                } else if self.keybind_help_open {
                    self.keybind_help_open = false;
                }
                false
            }
            KeybindingAction::ClearInput => {
                if let Some(dialog) = self.launch_dialog.as_mut() {
                    match dialog.focused_field {
                        LaunchDialogField::Prompt => dialog.prompt.clear(),
                        LaunchDialogField::PreLaunchCommand => dialog.pre_launch_command.clear(),
                        LaunchDialogField::Unsafe
                        | LaunchDialogField::StartButton
                        | LaunchDialogField::CancelButton => {}
                    }
                    return false;
                }
                if let Some(dialog) = self.create_dialog.as_mut() {
                    let mut refresh_base_branch = false;
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => dialog.workspace_name.clear(),
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.clear();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::Project
                        | CreateDialogField::Agent
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                    if refresh_base_branch {
                        self.refresh_create_branch_filtered();
                    }
                }
                false
            }
            KeybindingAction::CancelTask => {
                self.show_toast("cannot cancel running lifecycle task", true);
                false
            }
            KeybindingAction::Quit | KeybindingAction::HardQuit => true,
            KeybindingAction::SoftQuit => !self.keybinding_task_running(),
            KeybindingAction::CloseOverlay
            | KeybindingAction::ToggleTreeView
            | KeybindingAction::Bell
            | KeybindingAction::PassThrough => false,
        }
    }
}

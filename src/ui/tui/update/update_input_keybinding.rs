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
            || self.restart_in_flight
    }

    fn keybinding_input_nonempty(&self) -> bool {
        if let Some(dialog) = self.launch_dialog() {
            return dialog.start_config.is_input_nonempty();
        }
        if let Some(dialog) = self.create_dialog() {
            return !dialog.workspace_name.is_empty()
                || !dialog.base_branch.is_empty()
                || dialog.start_config.is_input_nonempty();
        }
        if let Some(project_dialog) = self.project_dialog() {
            if !project_dialog.filter.is_empty() {
                return true;
            }
            if let Some(add_dialog) = project_dialog.add_dialog.as_ref() {
                return !add_dialog.name.is_empty() || !add_dialog.path.is_empty();
            }
            if let Some(defaults_dialog) = project_dialog.defaults_dialog.as_ref() {
                return !defaults_dialog.base_branch.is_empty()
                    || !defaults_dialog.workspace_init_command.is_empty()
                    || !defaults_dialog.claude_env.is_empty()
                    || !defaults_dialog.codex_env.is_empty()
                    || !defaults_dialog.opencode_env.is_empty();
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
                if self.project_reorder_active() {
                    self.cancel_project_reorder_from_dialog();
                    return false;
                }
                if let Some(kind) = self.active_dialog_kind() {
                    self.log_dialog_event(kind, "dialog_cancelled");
                    if kind == "create" {
                        self.clear_create_branch_picker();
                    }
                    self.close_active_dialog();
                } else if self.keybind_help_open {
                    self.keybind_help_open = false;
                }
                false
            }
            KeybindingAction::ClearInput => {
                if let Some(dialog) = self.launch_dialog_mut() {
                    match dialog.focused_field {
                        LaunchDialogField::StartConfig(field) => dialog.start_config.clear(field),
                        LaunchDialogField::StartButton | LaunchDialogField::CancelButton => {}
                    }
                    return false;
                }
                if let Some(dialog) = self.create_dialog_mut() {
                    let mut refresh_base_branch = false;
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => dialog.workspace_name.clear(),
                        CreateDialogField::PullRequestUrl => dialog.pr_url.clear(),
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.clear();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::StartConfig(field) => {
                            dialog.start_config.clear(field);
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
                self.show_info_toast("cannot cancel running lifecycle task");
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

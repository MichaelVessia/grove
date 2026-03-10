use super::*;

impl GroveApp {
    pub(super) fn open_selected_project_defaults_dialog(&mut self) {
        let Some(project_index) = self.selected_project_dialog_project_index() else {
            self.show_info_toast("no project selected");
            return;
        };
        let Some(project) = self.projects.get(project_index) else {
            self.show_info_toast("project not found");
            return;
        };
        let base_branch = project.defaults.base_branch.clone();
        let workspace_init_command = project.defaults.workspace_init_command.clone();
        let claude_env = format_agent_env_vars(&project.defaults.agent_env.claude);
        let codex_env = format_agent_env_vars(&project.defaults.agent_env.codex);
        let opencode_env = format_agent_env_vars(&project.defaults.agent_env.opencode);

        if let Some(project_dialog) = self.project_dialog_mut() {
            let mut defaults_dialog = ProjectDefaultsDialogState {
                project_index,
                base_branch_input: TextInput::new().with_value(base_branch).with_focused(true),
                workspace_init_command_input: TextInput::new().with_value(workspace_init_command),
                claude_env_input: TextInput::new().with_value(claude_env),
                codex_env_input: TextInput::new().with_value(codex_env),
                opencode_env_input: TextInput::new().with_value(opencode_env),
                focused_field: ProjectDefaultsDialogField::BaseBranch,
            };
            defaults_dialog.sync_focus();
            project_dialog.defaults_dialog = Some(defaults_dialog);
        }
    }

    pub(super) fn save_project_defaults_from_dialog(&mut self) {
        let Some(dialog_state) = self
            .project_dialog()
            .and_then(|dialog| dialog.defaults_dialog.clone())
        else {
            return;
        };
        let claude_env = match encode_agent_env_vars(dialog_state.claude_env_input.value()) {
            Ok(env) => env,
            Err(error) => {
                self.show_info_toast(format!("invalid Claude env: {error}"));
                return;
            }
        };
        let codex_env = match encode_agent_env_vars(dialog_state.codex_env_input.value()) {
            Ok(env) => env,
            Err(error) => {
                self.show_info_toast(format!("invalid Codex env: {error}"));
                return;
            }
        };
        let opencode_env = match encode_agent_env_vars(dialog_state.opencode_env_input.value()) {
            Ok(env) => env,
            Err(error) => {
                self.show_info_toast(format!("invalid OpenCode env: {error}"));
                return;
            }
        };
        let project_name = {
            let Some(project) = self.projects.get_mut(dialog_state.project_index) else {
                self.show_info_toast("project not found");
                return;
            };

            project.defaults.base_branch =
                dialog_state.base_branch_input.value().trim().to_string();
            project.defaults.workspace_init_command = dialog_state
                .workspace_init_command_input
                .value()
                .trim()
                .to_string();
            project.defaults.agent_env = AgentEnvDefaults {
                claude: claude_env,
                codex: codex_env,
                opencode: opencode_env,
            };
            project.name.clone()
        };

        if let Err(error) = self.save_projects_config() {
            self.show_error_toast(format!("project defaults save failed: {error}"));
            return;
        }

        if let Some(project_dialog) = self.project_dialog_mut() {
            project_dialog.defaults_dialog = None;
        }
        self.show_success_toast(format!("project '{}' defaults saved", project_name));
    }
}

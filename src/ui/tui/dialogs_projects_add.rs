use crate::infrastructure::process::stderr_trimmed;

use super::*;

impl GroveApp {
    fn normalized_project_path(raw: &str) -> PathBuf {
        if let Some(stripped) = raw.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(stripped);
        }
        PathBuf::from(raw)
    }

    fn save_projects_config_to_path(
        config_path: &Path,
        sidebar_width_pct: u16,
        launch_skip_permissions: bool,
        projects: &[ProjectConfig],
        attention_acks: &[WorkspaceAttentionAckConfig],
    ) -> Result<(), String> {
        let config = GroveConfig {
            sidebar_width_pct,
            projects: projects.to_vec(),
            attention_acks: attention_acks.to_vec(),
            launch_skip_permissions,
        };
        crate::infrastructure::config::save_to_path(config_path, &config)
    }

    fn save_projects_config(&self) -> Result<(), String> {
        Self::save_projects_config_to_path(
            &self.config_path,
            self.sidebar_width_pct,
            self.launch_skip_permissions,
            &self.projects,
            &self.workspace_attention_acks_for_config(),
        )
    }

    pub(super) fn project_reorder_active(&self) -> bool {
        self.project_dialog()
            .and_then(|dialog| dialog.reorder.as_ref())
            .is_some()
    }

    pub(super) fn open_project_reorder_mode(&mut self) {
        if self.project_reorder_active() {
            return;
        }
        let Some(dialog) = self.project_dialog() else {
            return;
        };
        if !dialog.filter.trim().is_empty() {
            self.show_info_toast("clear filter before reordering projects");
            return;
        }
        let Some(selected_project_index) = self.selected_project_dialog_project_index() else {
            self.show_info_toast("no project selected");
            return;
        };
        let Some(selected_project) = self.projects.get(selected_project_index) else {
            self.show_info_toast("project not found");
            return;
        };

        let original_projects = self.projects.clone();
        let moving_project_path = selected_project.path.clone();
        if let Some(dialog) = self.project_dialog_mut() {
            dialog.reorder = Some(ProjectReorderState {
                original_projects,
                moving_project_path,
            });
        }
        self.show_info_toast("reorder mode, j/k or Up/Down move, Enter save, Esc cancel");
    }

    pub(super) fn move_selected_project_in_dialog(&mut self, direction: i8) {
        if !self.project_reorder_active() {
            return;
        }

        let Some(selected_project_index) = self.selected_project_dialog_project_index() else {
            self.show_info_toast("no project selected");
            return;
        };
        if self.projects.is_empty() {
            return;
        }

        let next_project_index = if direction.is_negative() {
            selected_project_index.saturating_sub(1)
        } else {
            selected_project_index
                .saturating_add(1)
                .min(self.projects.len().saturating_sub(1))
        };
        if next_project_index == selected_project_index {
            return;
        }

        self.projects
            .swap(selected_project_index, next_project_index);
        let moving_project_path = self
            .projects
            .get(next_project_index)
            .map(|project| project.path.clone());
        self.refresh_project_dialog_filtered();

        if let Some(path) = moving_project_path.as_ref() {
            self.select_project_dialog_project_by_path(path.as_path());
            if let Some(dialog) = self.project_dialog_mut()
                && let Some(reorder) = dialog.reorder.as_mut()
            {
                reorder.moving_project_path = path.clone();
            }
        }
    }

    fn reorder_workspaces_for_project_order(&mut self) {
        if self.state.workspaces.is_empty() {
            return;
        }

        let selected_workspace_path = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.clone());
        let projects = &self.projects;
        self.state.workspaces.sort_by_key(|workspace| {
            workspace
                .project_path
                .as_ref()
                .and_then(|workspace_project_path| {
                    projects.iter().position(|project| {
                        refer_to_same_location(
                            project.path.as_path(),
                            workspace_project_path.as_path(),
                        )
                    })
                })
                .unwrap_or(usize::MAX)
        });

        if let Some(selected_workspace_path) = selected_workspace_path
            && let Some(index) = self.state.workspaces.iter().position(|workspace| {
                refer_to_same_location(workspace.path.as_path(), selected_workspace_path.as_path())
            })
        {
            self.state.selected_index = index;
        }
    }

    pub(super) fn save_project_reorder_from_dialog(&mut self) {
        if !self.project_reorder_active() {
            return;
        }
        if let Err(error) = self.save_projects_config() {
            self.show_error_toast(format!("project order save failed: {error}"));
            return;
        }

        self.reorder_workspaces_for_project_order();
        if let Some(dialog) = self.project_dialog_mut() {
            dialog.reorder = None;
        }
        self.show_success_toast("project order saved");
    }

    pub(super) fn cancel_project_reorder_from_dialog(&mut self) {
        let Some(reorder) = self
            .project_dialog()
            .and_then(|dialog| dialog.reorder.as_ref().cloned())
        else {
            return;
        };

        self.projects = reorder.original_projects;
        if let Some(dialog) = self.project_dialog_mut() {
            dialog.reorder = None;
        }
        self.refresh_project_dialog_filtered();
        self.select_project_dialog_project_by_path(reorder.moving_project_path.as_path());
        self.show_info_toast("project reorder cancelled");
    }

    pub(super) fn delete_selected_project_from_dialog(&mut self) {
        if self.project_delete_in_flight {
            self.show_info_toast("project delete already in progress");
            return;
        }
        let Some(project_index) = self.selected_project_dialog_project_index() else {
            self.show_info_toast("no project selected");
            return;
        };
        self.delete_project_by_index(project_index);
    }

    pub(super) fn delete_selected_workspace_project(&mut self) {
        if self.project_delete_in_flight {
            self.show_info_toast("project delete already in progress");
            return;
        }
        let Some(workspace) = self.state.selected_workspace() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let Some(project_path) = workspace.project_path.as_ref() else {
            self.show_info_toast("selected workspace has no project");
            return;
        };
        let Some(project_index) = self
            .projects
            .iter()
            .position(|project| refer_to_same_location(&project.path, project_path))
        else {
            self.show_info_toast("selected project not found");
            return;
        };
        self.delete_project_by_index(project_index);
    }

    fn delete_project_by_index(&mut self, project_index: usize) {
        let Some(project) = self.projects.get(project_index).cloned() else {
            self.show_info_toast("project not found");
            return;
        };
        let mut updated_projects = self.projects.clone();
        updated_projects.remove(project_index);

        self.log_dialog_event_with_fields(
            "projects",
            "dialog_confirmed",
            [
                ("project".to_string(), Value::from(project.name.clone())),
                (
                    "path".to_string(),
                    Value::from(project.path.display().to_string()),
                ),
            ],
        );

        if !self.tmux_input.supports_background_launch() {
            let result = Self::save_projects_config_to_path(
                &self.config_path,
                self.sidebar_width_pct,
                self.launch_skip_permissions,
                &updated_projects,
                &self.workspace_attention_acks_for_config(),
            );
            self.apply_delete_project_completion(DeleteProjectCompletion {
                project_name: project.name,
                project_path: project.path,
                projects: updated_projects,
                result,
            });
            return;
        }

        let config_path = self.config_path.clone();
        let sidebar_width_pct = self.sidebar_width_pct;
        let launch_skip_permissions = self.launch_skip_permissions;
        let attention_acks = self.workspace_attention_acks_for_config();
        self.project_delete_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = Self::save_projects_config_to_path(
                &config_path,
                sidebar_width_pct,
                launch_skip_permissions,
                &updated_projects,
                &attention_acks,
            );
            Msg::DeleteProjectCompleted(DeleteProjectCompletion {
                project_name: project.name,
                project_path: project.path,
                projects: updated_projects,
                result,
            })
        }));
    }

    pub(super) fn add_project_from_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_ref() else {
            return;
        };

        let path_input = add_dialog.path.trim();
        if path_input.is_empty() {
            self.show_info_toast("project path is required");
            return;
        }
        let normalized = Self::normalized_project_path(path_input);
        let canonical = match normalized.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                self.show_info_toast(format!("invalid project path: {error}"));
                return;
            }
        };

        let repo_root_output = Command::new("git")
            .current_dir(&canonical)
            .args(["rev-parse", "--show-toplevel"])
            .output();
        let repo_root = match repo_root_output {
            Ok(output) if output.status.success() => {
                let raw = String::from_utf8(output.stdout).unwrap_or_default();
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    canonical.clone()
                } else {
                    PathBuf::from(trimmed)
                }
            }
            Ok(output) => {
                let stderr = stderr_trimmed(&output);
                self.show_info_toast(format!("not a git repository: {stderr}"));
                return;
            }
            Err(error) => {
                self.show_error_toast(format!("git check failed: {error}"));
                return;
            }
        };
        let repo_root = repo_root.canonicalize().unwrap_or(repo_root);

        if self
            .projects
            .iter()
            .any(|project| refer_to_same_location(&project.path, &repo_root))
        {
            self.show_info_toast("project already exists");
            return;
        }

        let project_name = if add_dialog.name.trim().is_empty() {
            project_display_name(&repo_root)
        } else {
            add_dialog.name.trim().to_string()
        };
        self.projects.push(ProjectConfig {
            name: project_name.clone(),
            path: repo_root.clone(),
            defaults: Default::default(),
        });
        if let Err(error) = self.save_projects_config() {
            self.show_error_toast(format!("project save failed: {error}"));
            return;
        }

        if let Some(dialog) = self.project_dialog_mut() {
            dialog.add_dialog = None;
        }
        self.refresh_project_dialog_filtered();
        self.refresh_workspaces(None);
        self.show_success_toast(format!("project '{}' added", project_name));
    }

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
        let setup_commands = format_setup_commands(&project.defaults.setup_commands);
        let auto_run_setup_commands = project.defaults.auto_run_setup_commands;
        let claude_env = format_agent_env_vars(&project.defaults.agent_env.claude);
        let codex_env = format_agent_env_vars(&project.defaults.agent_env.codex);
        let opencode_env = format_agent_env_vars(&project.defaults.agent_env.opencode);

        if let Some(project_dialog) = self.project_dialog_mut() {
            project_dialog.defaults_dialog = Some(ProjectDefaultsDialogState {
                project_index,
                base_branch,
                setup_commands,
                auto_run_setup_commands,
                claude_env,
                codex_env,
                opencode_env,
                focused_field: ProjectDefaultsDialogField::BaseBranch,
            });
        }
    }

    pub(super) fn save_project_defaults_from_dialog(&mut self) {
        let Some(dialog_state) = self
            .project_dialog()
            .and_then(|dialog| dialog.defaults_dialog.clone())
        else {
            return;
        };
        let claude_env = match encode_agent_env_vars(&dialog_state.claude_env) {
            Ok(env) => env,
            Err(error) => {
                self.show_info_toast(format!("invalid Claude env: {error}"));
                return;
            }
        };
        let codex_env = match encode_agent_env_vars(&dialog_state.codex_env) {
            Ok(env) => env,
            Err(error) => {
                self.show_info_toast(format!("invalid Codex env: {error}"));
                return;
            }
        };
        let opencode_env = match encode_agent_env_vars(&dialog_state.opencode_env) {
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

            project.defaults = ProjectDefaults {
                base_branch: dialog_state.base_branch.trim().to_string(),
                setup_commands: parse_setup_commands(&dialog_state.setup_commands),
                auto_run_setup_commands: dialog_state.auto_run_setup_commands,
                agent_env: AgentEnvDefaults {
                    claude: claude_env,
                    codex: codex_env,
                    opencode: opencode_env,
                },
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

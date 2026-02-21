use crate::infrastructure::config::RemoteProfileConfig;
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
        projects: &[ProjectConfig],
        remote_profiles: &[RemoteProfileConfig],
        active_remote_profile: &Option<String>,
        attention_acks: &[WorkspaceAttentionAckConfig],
    ) -> Result<(), String> {
        let config = GroveConfig {
            sidebar_width_pct,
            projects: projects.to_vec(),
            remote_profiles: remote_profiles.to_vec(),
            active_remote_profile: active_remote_profile.clone(),
            attention_acks: attention_acks.to_vec(),
        };
        crate::infrastructure::config::save_to_path(config_path, &config)
    }

    fn save_projects_config(&self) -> Result<(), String> {
        Self::save_projects_config_to_path(
            &self.config_path,
            self.sidebar_width_pct,
            &self.projects,
            &self.remote_profiles,
            &self.active_remote_profile,
            &self.workspace_attention_acks_for_config(),
        )
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
                &updated_projects,
                &self.remote_profiles,
                &self.active_remote_profile,
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
        let remote_profiles = self.remote_profiles.clone();
        let active_remote_profile = self.active_remote_profile.clone();
        let attention_acks = self.workspace_attention_acks_for_config();
        self.project_delete_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = Self::save_projects_config_to_path(
                &config_path,
                sidebar_width_pct,
                &updated_projects,
                &remote_profiles,
                &active_remote_profile,
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
            target: crate::infrastructure::config::ProjectTarget::Local,
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

        if let Some(project_dialog) = self.project_dialog_mut() {
            project_dialog.defaults_dialog = Some(ProjectDefaultsDialogState {
                project_index,
                base_branch,
                setup_commands,
                auto_run_setup_commands,
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
        let project_name = {
            let Some(project) = self.projects.get_mut(dialog_state.project_index) else {
                self.show_info_toast("project not found");
                return;
            };

            project.defaults = ProjectDefaults {
                base_branch: dialog_state.base_branch.trim().to_string(),
                setup_commands: parse_setup_commands(&dialog_state.setup_commands),
                auto_run_setup_commands: dialog_state.auto_run_setup_commands,
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

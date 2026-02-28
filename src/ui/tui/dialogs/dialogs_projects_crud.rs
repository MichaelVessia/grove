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
        projects: &[ProjectConfig],
        attention_acks: &[WorkspaceAttentionAckConfig],
    ) -> Result<(), String> {
        let projects_path = crate::infrastructure::config::projects_path_for(config_path);
        crate::infrastructure::config::save_projects_to_path(
            &projects_path,
            projects,
            attention_acks,
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
        let attention_acks = self.workspace_attention_acks_for_config();
        self.project_delete_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = Self::save_projects_config_to_path(
                &config_path,
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
}

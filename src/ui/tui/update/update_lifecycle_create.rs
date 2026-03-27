use super::update_prelude::*;
use crate::application::task_lifecycle::{
    AddWorktreeToTaskRequest, AddWorktreeToTaskResult, CreateBaseTaskRequest, TaskBranchSource,
    add_worktree_to_task, add_worktree_to_task_in_root, create_base_task, create_base_task_in_root,
};
use crate::infrastructure::paths::refer_to_same_location;
use crate::infrastructure::process::stderr_trimmed;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedGitHubPullRequest {
    owner: String,
    repo: String,
    number: u64,
}

impl GroveApp {
    fn create_task_root_override(&self) -> Option<PathBuf> {
        #[cfg(test)]
        {
            self.task_root_override.clone()
        }

        #[cfg(not(test))]
        {
            None
        }
    }

    fn resolve_pull_request_branch_name(
        &self,
        pull_request: &ParsedGitHubPullRequest,
    ) -> Result<String, String> {
        #[cfg(test)]
        {
            if let Some(branch_name) = self.pull_request_branch_name_override.clone() {
                return Ok(branch_name);
            }
        }

        resolve_pull_request_branch_name_with_gh(pull_request)
    }

    pub(super) fn confirm_create_dialog(&mut self) {
        if self.dialogs.create_in_flight {
            return;
        }

        let Some(dialog) = self.create_dialog().cloned() else {
            return;
        };
        let Some(project) = self.projects.get(dialog.project_index).cloned() else {
            self.show_info_toast("project is required");
            return;
        };

        if dialog.is_add_worktree_mode() {
            self.confirm_add_worktree_dialog(dialog, project);
            return;
        }

        let repositories = if dialog.tab == CreateDialogTab::PullRequest || dialog.register_as_base
        {
            vec![project.clone()]
        } else {
            self.selected_create_dialog_projects()
        };

        if dialog.register_as_base {
            let base_branch = {
                let configured = project.defaults.base_branch.trim();
                if configured.is_empty() {
                    "main".to_string()
                } else {
                    configured.to_string()
                }
            };
            let repo_name = project
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("base")
                .to_string();
            self.log_dialog_event_with_fields(
                "create",
                "dialog_confirmed",
                [
                    ("task_name".to_string(), Value::from(repo_name.clone())),
                    ("branch_mode".to_string(), Value::from("base")),
                    ("branch_value".to_string(), Value::from("repo_root")),
                    (
                        "project_index".to_string(),
                        Value::from(usize_to_u64(dialog.project_index)),
                    ),
                    (
                        "repository_count".to_string(),
                        Value::from(usize_to_u64(repositories.len())),
                    ),
                ],
            );
            let base_request = CreateBaseTaskRequest {
                repository: project.clone(),
                agent: self
                    .state
                    .selected_workspace()
                    .map(|workspace| workspace.agent)
                    .unwrap_or(AgentType::Codex),
                base_branch,
            };
            let task_root_override = self.create_task_root_override();
            if !self.tmux_input.supports_background_launch() {
                let result =
                    execute_create_base_task_request(&base_request, task_root_override.as_deref());
                let request = shim_create_task_request(repo_name, project, base_request);
                self.apply_create_workspace_completion(CreateWorkspaceCompletion {
                    request: CreateWorkspaceRequest::CreateTask(request),
                    result: CreateWorkspaceResult::CreateTask(result),
                });
                return;
            }
            self.dialogs.create_in_flight = true;
            self.queue_cmd(Cmd::task(move || {
                let result =
                    execute_create_base_task_request(&base_request, task_root_override.as_deref());
                let request = shim_create_task_request(repo_name, project, base_request);
                Msg::CreateWorkspaceCompleted(Box::new(CreateWorkspaceCompletion {
                    request: CreateWorkspaceRequest::CreateTask(request),
                    result: CreateWorkspaceResult::CreateTask(result),
                }))
            }));
            return;
        }

        let (task_name, branch_mode_label, branch_value, branch_source): (
            String,
            String,
            String,
            TaskBranchSource,
        ) = match dialog.tab {
            CreateDialogTab::Manual => (
                dialog.task_name.trim().to_string(),
                "implicit".to_string(),
                "project_defaults_or_git".to_string(),
                TaskBranchSource::BaseBranch,
            ),
            CreateDialogTab::PullRequest => {
                let parsed = match parse_github_pull_request_url(dialog.pr_url.as_str()) {
                    Ok(parsed) => parsed,
                    Err(message) => {
                        self.show_info_toast(message);
                        return;
                    }
                };
                if let Err(message) = ensure_project_matches_pull_request(&project.path, &parsed) {
                    self.show_info_toast(message);
                    return;
                }
                let branch_name = match self.resolve_pull_request_branch_name(&parsed) {
                    Ok(branch_name) => branch_name,
                    Err(message) => {
                        self.show_info_toast(message);
                        return;
                    }
                };

                (
                    format!("pr-{}", parsed.number),
                    "pull_request".to_string(),
                    dialog.pr_url.clone(),
                    TaskBranchSource::PullRequest {
                        number: parsed.number,
                        branch_name,
                    },
                )
            }
        };
        self.log_dialog_event_with_fields(
            "create",
            "dialog_confirmed",
            [
                ("task_name".to_string(), Value::from(task_name.clone())),
                ("branch_mode".to_string(), Value::from(branch_mode_label)),
                ("branch_value".to_string(), Value::from(branch_value)),
                (
                    "project_index".to_string(),
                    Value::from(usize_to_u64(dialog.project_index)),
                ),
                (
                    "repository_count".to_string(),
                    Value::from(usize_to_u64(repositories.len())),
                ),
            ],
        );
        let request = CreateTaskRequest {
            task_name: task_name.clone(),
            repositories,
            agent: self
                .state
                .selected_workspace()
                .map(|workspace| workspace.agent)
                .unwrap_or(AgentType::Codex),
            branch_source,
        };

        if let Err(error) = request.validate() {
            self.show_info_toast(task_lifecycle_error_message(&error));
            return;
        }

        let task_root_override = self.create_task_root_override();
        if !self.tmux_input.supports_background_launch() {
            let result = execute_create_task_request(&request, task_root_override.as_deref());
            self.apply_create_workspace_completion(CreateWorkspaceCompletion {
                request: CreateWorkspaceRequest::CreateTask(request),
                result: CreateWorkspaceResult::CreateTask(result),
            });
            return;
        }

        self.dialogs.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = execute_create_task_request(&request, task_root_override.as_deref());
            Msg::CreateWorkspaceCompleted(Box::new(CreateWorkspaceCompletion {
                request: CreateWorkspaceRequest::CreateTask(request),
                result: CreateWorkspaceResult::CreateTask(result),
            }))
        }));
    }

    fn confirm_add_worktree_dialog(&mut self, dialog: CreateDialogState, project: ProjectConfig) {
        let Some(task) = dialog.target_task().cloned() else {
            self.show_info_toast("task is required");
            return;
        };
        self.log_dialog_event_with_fields(
            "create",
            "dialog_confirmed",
            [
                ("task_name".to_string(), Value::from(task.name.clone())),
                ("branch_mode".to_string(), Value::from("add_worktree")),
                ("branch_value".to_string(), Value::from(task.branch.clone())),
                (
                    "project_index".to_string(),
                    Value::from(usize_to_u64(dialog.project_index)),
                ),
                ("repository_count".to_string(), Value::from(1_u64)),
            ],
        );
        let request = AddWorktreeToTaskRequest {
            task,
            repository: project,
            agent: self
                .state
                .selected_workspace()
                .map(|workspace| workspace.agent)
                .unwrap_or(AgentType::Codex),
        };
        let task_root_override = self.create_task_root_override();
        if !self.tmux_input.supports_background_launch() {
            let result = execute_add_worktree_request(&request, task_root_override.as_deref());
            self.apply_create_workspace_completion(CreateWorkspaceCompletion {
                request: CreateWorkspaceRequest::AddWorktree(Box::new(request)),
                result: CreateWorkspaceResult::AddWorktree(result),
            });
            return;
        }

        self.dialogs.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let result = execute_add_worktree_request(&request, task_root_override.as_deref());
            Msg::CreateWorkspaceCompleted(Box::new(CreateWorkspaceCompletion {
                request: CreateWorkspaceRequest::AddWorktree(Box::new(request)),
                result: CreateWorkspaceResult::AddWorktree(result),
            }))
        }));
    }

    pub(super) fn apply_create_workspace_completion(
        &mut self,
        completion: CreateWorkspaceCompletion,
    ) {
        self.dialogs.create_in_flight = false;
        match (completion.request, completion.result) {
            (
                CreateWorkspaceRequest::CreateTask(request),
                CreateWorkspaceResult::CreateTask(result),
            ) => match result {
                Ok(result) => {
                    if result.task.has_base_worktree() {
                        let hidden_before = self.hidden_base_project_paths.len();
                        for repository in &request.repositories {
                            self.hidden_base_project_paths.retain(|path| {
                                !refer_to_same_location(path.as_path(), repository.path.as_path())
                            });
                        }
                        if self.hidden_base_project_paths.len() != hidden_before
                            && let Err(error) = self.save_projects_config()
                        {
                            self.show_error_toast(format!(
                                "task created, but hidden-base state save failed: {error}"
                            ));
                        }
                    }
                    self.close_active_dialog();
                    let preferred_workspace_path = result
                        .task
                        .worktrees
                        .first()
                        .map(|worktree| worktree.path.clone());
                    self.refresh_workspaces(preferred_workspace_path);
                    let _ = self.focus_main_pane(FOCUS_ID_WORKSPACE_LIST);
                    if result.warnings.is_empty() {
                        self.show_success_toast(format!("task '{}' created", request.task_name));
                    } else if let Some(first_warning) = result.warnings.first() {
                        self.show_info_toast(format!(
                            "task '{}' created, warning: {}",
                            request.task_name, first_warning
                        ));
                    }
                }
                Err(error) => {
                    self.show_error_toast(format!(
                        "task create failed: {}",
                        task_lifecycle_error_message(&error)
                    ));
                }
            },
            (
                CreateWorkspaceRequest::AddWorktree(request),
                CreateWorkspaceResult::AddWorktree(result),
            ) => match result {
                Ok(result) => {
                    self.close_active_dialog();
                    self.refresh_workspaces(Some(result.added_worktree_path));
                    let _ = self.focus_main_pane(FOCUS_ID_WORKSPACE_LIST);
                    if result.warnings.is_empty() {
                        self.show_success_toast(format!(
                            "worktree added to task '{}'",
                            request.task.name
                        ));
                    } else if let Some(first_warning) = result.warnings.first() {
                        self.show_info_toast(format!(
                            "worktree added to task '{}', warning: {}",
                            request.task.name, first_warning
                        ));
                    }
                }
                Err(error) => {
                    self.show_error_toast(format!(
                        "worktree add failed: {}",
                        task_lifecycle_error_message(&error)
                    ));
                }
            },
            (CreateWorkspaceRequest::CreateTask(_), CreateWorkspaceResult::AddWorktree(_))
            | (CreateWorkspaceRequest::AddWorktree(_), CreateWorkspaceResult::CreateTask(_)) => {
                self.show_error_toast("create dialog completion mismatched request/result");
            }
        }
    }
}

fn execute_create_base_task_request(
    request: &CreateBaseTaskRequest,
    tasks_root_override: Option<&Path>,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    if let Some(tasks_root) = tasks_root_override {
        return create_base_task_in_root(tasks_root, request);
    }
    create_base_task(request)
}

fn execute_add_worktree_request(
    request: &AddWorktreeToTaskRequest,
    tasks_root_override: Option<&Path>,
) -> Result<AddWorktreeToTaskResult, TaskLifecycleError> {
    let git = CommandGitRunner;
    let setup = CommandSetupScriptRunner;
    let setup_command = CommandSetupCommandRunner;
    if let Some(tasks_root) = tasks_root_override {
        return add_worktree_to_task_in_root(tasks_root, request, &git, &setup, &setup_command);
    }

    add_worktree_to_task(request, &git, &setup, &setup_command)
}

fn shim_create_task_request(
    task_name: String,
    project: ProjectConfig,
    base_request: CreateBaseTaskRequest,
) -> CreateTaskRequest {
    CreateTaskRequest {
        task_name,
        repositories: vec![project],
        agent: base_request.agent,
        branch_source: TaskBranchSource::BaseBranch,
    }
}

fn execute_create_task_request(
    request: &CreateTaskRequest,
    tasks_root_override: Option<&Path>,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    let git = CommandGitRunner;
    let setup = CommandSetupScriptRunner;
    let setup_command = CommandSetupCommandRunner;
    if let Some(tasks_root) = tasks_root_override {
        return create_task_in_root(tasks_root, request, &git, &setup, &setup_command);
    }

    create_task(request, &git, &setup, &setup_command)
}

fn resolve_pull_request_branch_name_with_gh(
    pull_request: &ParsedGitHubPullRequest,
) -> Result<String, String> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!(
                "repos/{}/{}/pulls/{}",
                pull_request.owner, pull_request.repo, pull_request.number
            ),
        ])
        .output()
        .map_err(|error| format!("gh api failed: {error}"))?;
    if !output.status.success() {
        return Err(stderr_trimmed(&output));
    }

    parse_pull_request_head_branch_name(&output.stdout)
}

fn parse_pull_request_head_branch_name(stdout: &[u8]) -> Result<String, String> {
    let payload: Value =
        serde_json::from_slice(stdout).map_err(|error| format!("invalid gh response: {error}"))?;
    let Some(branch_name) = payload
        .get("head")
        .and_then(|head| head.get("ref"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err("pull request head branch is missing".to_string());
    };

    Ok(branch_name.to_string())
}

fn parse_github_pull_request_url(url: &str) -> Result<ParsedGitHubPullRequest, String> {
    let trimmed = url.trim();
    let trimmed = trimmed.trim_end_matches('/');
    let parts = trimmed.split('/').collect::<Vec<&str>>();
    if parts.len() < 7 {
        return Err("GitHub pull request URL is invalid".to_string());
    }

    let owner = parts[3].trim();
    let repo = parts[4].trim();
    let kind = parts[5].trim();
    let number = parts[6].trim();
    if owner.is_empty() || repo.is_empty() || kind != "pull" {
        return Err("GitHub pull request URL is invalid".to_string());
    }
    let number = number
        .parse::<u64>()
        .map_err(|_| "GitHub pull request number is invalid".to_string())?;

    Ok(ParsedGitHubPullRequest {
        owner: owner.to_string(),
        repo: repo.to_string(),
        number,
    })
}

fn ensure_project_matches_pull_request(
    project_root: &Path,
    pull_request: &ParsedGitHubPullRequest,
) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(project_root)
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|error| format!("git remote get-url origin failed: {error}"))?;
    if !output.status.success() {
        return Err(stderr_trimmed(&output));
    }

    let origin = String::from_utf8(output.stdout)
        .map_err(|error| format!("origin URL was invalid UTF-8: {error}"))?;
    let normalized = origin.trim().replace(':', "/");
    let expected_fragment = format!("{}/{}", pull_request.owner, pull_request.repo);
    if normalized.contains(expected_fragment.as_str()) {
        return Ok(());
    }

    Err("selected project does not match pull request repository".to_string())
}

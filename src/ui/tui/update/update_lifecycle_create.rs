use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedGitHubPullRequest {
    owner: String,
    repo: String,
    number: u64,
}

impl GroveApp {
    pub(super) fn confirm_create_dialog(&mut self) {
        if self.create_in_flight {
            return;
        }

        let Some(dialog) = self.create_dialog().cloned() else {
            return;
        };
        let Some(project) = self.projects.get(dialog.project_index).cloned() else {
            self.show_info_toast("project is required");
            return;
        };

        let (workspace_name, branch_mode, branch_mode_label, branch_value) = match dialog.tab {
            CreateDialogTab::Manual => (
                dialog.workspace_name.trim().to_string(),
                BranchMode::NewBranch {
                    base_branch: dialog.base_branch.trim().to_string(),
                },
                "new".to_string(),
                dialog.base_branch.clone(),
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

                (
                    format!("pr-{}", parsed.number),
                    BranchMode::PullRequest {
                        number: parsed.number,
                        base_branch: dialog.base_branch.trim().to_string(),
                    },
                    "pull_request".to_string(),
                    dialog.pr_url.clone(),
                )
            }
        };
        self.log_dialog_event_with_fields(
            "create",
            "dialog_confirmed",
            [
                (
                    "workspace_name".to_string(),
                    Value::from(workspace_name.clone()),
                ),
                ("agent".to_string(), Value::from(dialog.agent.label())),
                ("branch_mode".to_string(), Value::from(branch_mode_label)),
                ("branch_value".to_string(), Value::from(branch_value)),
                (
                    "project_index".to_string(),
                    Value::from(usize_to_u64(dialog.project_index)),
                ),
                (
                    "prompt_len".to_string(),
                    Value::from(usize_to_u64(dialog.start_config.prompt.len())),
                ),
                (
                    "skip_permissions".to_string(),
                    Value::from(dialog.start_config.skip_permissions),
                ),
                (
                    "init_len".to_string(),
                    Value::from(usize_to_u64(dialog.start_config.init_command.len())),
                ),
            ],
        );
        let request = CreateWorkspaceRequest {
            workspace_name: workspace_name.clone(),
            branch_mode,
            agent: dialog.agent,
        };

        if let Err(error) = request.validate() {
            self.show_info_toast(workspace_lifecycle_error_message(&error));
            return;
        }

        self.pending_create_start_config = Some(dialog.start_config.clone());
        let repo_root = project.path;
        if !self.tmux_input.supports_background_launch() {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let setup_command = CommandSetupCommandRunner;
            let result = create_workspace_with_template(
                &repo_root,
                &request,
                None,
                &git,
                &setup,
                &setup_command,
            );
            self.apply_create_workspace_completion(CreateWorkspaceCompletion { request, result });
            return;
        }

        self.create_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let git = CommandGitRunner;
            let setup = CommandSetupScriptRunner;
            let setup_command = CommandSetupCommandRunner;
            let result = create_workspace_with_template(
                &repo_root,
                &request,
                None,
                &git,
                &setup,
                &setup_command,
            );
            Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion { request, result })
        }));
    }

    pub(super) fn apply_create_workspace_completion(
        &mut self,
        completion: CreateWorkspaceCompletion,
    ) {
        self.create_in_flight = false;
        let fallback_skip_permissions = self.launch_skip_permissions;
        let start_config = self.pending_create_start_config.take().unwrap_or_else(|| {
            StartAgentConfigState::new(String::new(), String::new(), fallback_skip_permissions)
        });
        let workspace_name = completion.request.workspace_name;
        match completion.result {
            Ok(result) => {
                self.close_active_dialog();
                self.clear_create_branch_picker();
                if let Err(error) = write_workspace_init_command(
                    &result.workspace_path,
                    trimmed_nonempty(&start_config.init_command).as_deref(),
                ) {
                    self.last_tmux_error =
                        Some(format!("init command marker persist failed: {error}"));
                }
                self.pending_auto_start_workspace = Some(PendingAutoStartWorkspace {
                    workspace_path: result.workspace_path.clone(),
                    start_config: start_config.clone(),
                });
                self.launch_skip_permissions = start_config.skip_permissions;
                self.pending_auto_launch_shell_workspace_path = Some(result.workspace_path.clone());
                self.refresh_workspaces(Some(result.workspace_path));
                self.state.mode = UiMode::List;
                self.state.focus = PaneFocus::WorkspaceList;
                if result.warnings.is_empty() {
                    self.show_success_toast(format!("workspace '{}' created", workspace_name));
                } else if let Some(first_warning) = result.warnings.first() {
                    self.show_info_toast(format!(
                        "workspace '{}' created, warning: {}",
                        workspace_name, first_warning
                    ));
                }
            }
            Err(error) => {
                self.show_error_toast(format!(
                    "workspace create failed: {}",
                    workspace_lifecycle_error_message(&error)
                ));
            }
        }
    }
}

fn parse_github_pull_request_url(url: &str) -> Result<ParsedGitHubPullRequest, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("github pr url is required".to_string());
    }
    let url_without_fragment = trimmed
        .split('#')
        .next()
        .unwrap_or(trimmed)
        .split('?')
        .next()
        .unwrap_or(trimmed);
    let normalized = url_without_fragment.trim_end_matches('/');
    let marker = "github.com/";
    let Some(index) = normalized.find(marker) else {
        return Err("url must be a github pull request link".to_string());
    };

    let path = &normalized[index + marker.len()..];
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    let Some(owner) = segments.next() else {
        return Err("url must include owner".to_string());
    };
    let Some(repo) = segments.next() else {
        return Err("url must include repository".to_string());
    };
    if segments.next() != Some("pull") {
        return Err("url must target a pull request".to_string());
    }
    let Some(number_raw) = segments.next() else {
        return Err("url must include pull request number".to_string());
    };
    let number = number_raw
        .parse::<u64>()
        .map_err(|_| "pull request number is invalid".to_string())?;
    if number == 0 {
        return Err("pull request number is invalid".to_string());
    }

    Ok(ParsedGitHubPullRequest {
        owner: owner.to_string(),
        repo: repo.trim_end_matches(".git").to_string(),
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
        return Err("project origin remote is required".to_string());
    }
    let remote_url = String::from_utf8(output.stdout)
        .map_err(|error| format!("origin url decode failed: {error}"))?;
    let Some((owner, repo)) = parse_github_repo_slug_from_remote(remote_url.trim()) else {
        return Err("project origin must be a github repository".to_string());
    };
    if owner == pull_request.owner && repo == pull_request.repo {
        return Ok(());
    }

    Err(format!(
        "pr repo {} / {} does not match selected project origin {} / {}",
        pull_request.owner, pull_request.repo, owner, repo
    ))
}

fn parse_github_repo_slug_from_remote(remote_url: &str) -> Option<(String, String)> {
    let remote = remote_url.trim();
    if remote.is_empty() {
        return None;
    }

    if let Some(path) = remote.strip_prefix("git@github.com:") {
        return parse_github_repo_path(path);
    }
    if let Some(path) = remote.strip_prefix("ssh://git@github.com/") {
        return parse_github_repo_path(path);
    }
    if let Some(index) = remote.find("github.com/") {
        return parse_github_repo_path(&remote[index + "github.com/".len()..]);
    }
    None
}

fn parse_github_repo_path(path: &str) -> Option<(String, String)> {
    let mut segments = path
        .trim_end_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty());
    let owner = segments.next()?;
    let repo = segments.next()?.trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pull_request_url_accepts_standard_github_links() {
        let parsed = parse_github_pull_request_url(
            "https://github.com/flocasts/web-monorepo/pull/3484/files?foo=bar",
        )
        .expect("url should parse");
        assert_eq!(parsed.owner, "flocasts");
        assert_eq!(parsed.repo, "web-monorepo");
        assert_eq!(parsed.number, 3484);
    }

    #[test]
    fn parse_pull_request_url_rejects_non_pull_links() {
        let result = parse_github_pull_request_url("https://github.com/flocasts/web-monorepo");
        assert_eq!(result, Err("url must target a pull request".to_string()));
    }

    #[test]
    fn parse_remote_slug_supports_ssh_and_https() {
        assert_eq!(
            parse_github_repo_slug_from_remote("git@github.com:flocasts/web-monorepo.git"),
            Some(("flocasts".to_string(), "web-monorepo".to_string()))
        );
        assert_eq!(
            parse_github_repo_slug_from_remote("https://github.com/flocasts/web-monorepo.git"),
            Some(("flocasts".to_string(), "web-monorepo".to_string()))
        );
    }
}

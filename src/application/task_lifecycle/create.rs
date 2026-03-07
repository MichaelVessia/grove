use std::fs;
use std::path::Path;

use crate::domain::{Task, WorkspaceStatus, Worktree};

use super::{
    CreateTaskRequest, CreateTaskResult, GROVE_SETUP_SCRIPT_FILE, TaskBranchSource,
    TaskLifecycleError, create_task_domain, repo_directory_name, resolve_repository_base_branch,
    write_task_manifest,
};
use crate::application::workspace_lifecycle::{
    GitCommandRunner, SetupCommandContext, SetupCommandRunner, SetupScriptContext,
    SetupScriptRunner, copy_env_files, ensure_grove_git_exclude_entries,
    write_workspace_base_marker,
};

pub(super) fn create_task_in_root(
    tasks_root: &Path,
    request: &CreateTaskRequest,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
    setup_command_runner: &impl SetupCommandRunner,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    request.validate()?;

    let task_root = tasks_root.join(&request.task_name);
    fs::create_dir_all(&task_root).map_err(|error| TaskLifecycleError::Io(error.to_string()))?;

    let mut warnings = Vec::new();
    let mut worktrees = Vec::new();

    for repository in &request.repositories {
        let base_branch = resolve_repository_base_branch(repository)?;
        let repository_dir = repo_directory_name(repository)?;
        let worktree_path = task_root.join(repository_dir);
        match &request.branch_source {
            TaskBranchSource::BaseBranch => {
                let args = vec![
                    "worktree".to_string(),
                    "add".to_string(),
                    "-b".to_string(),
                    request.task_name.clone(),
                    worktree_path.to_string_lossy().to_string(),
                    base_branch.clone(),
                ];
                git_runner
                    .run(repository.path.as_path(), &args)
                    .map_err(TaskLifecycleError::GitCommandFailed)?;
            }
            TaskBranchSource::PullRequest { number } => {
                let fetch_args = vec![
                    "fetch".to_string(),
                    "origin".to_string(),
                    format!("pull/{number}/head"),
                ];
                git_runner
                    .run(repository.path.as_path(), &fetch_args)
                    .map_err(TaskLifecycleError::GitCommandFailed)?;
                let args = vec![
                    "worktree".to_string(),
                    "add".to_string(),
                    "-b".to_string(),
                    request.task_name.clone(),
                    worktree_path.to_string_lossy().to_string(),
                    "FETCH_HEAD".to_string(),
                ];
                git_runner
                    .run(repository.path.as_path(), &args)
                    .map_err(TaskLifecycleError::GitCommandFailed)?;
            }
        }

        fs::create_dir_all(&worktree_path)
            .map_err(|error| TaskLifecycleError::Io(error.to_string()))?;
        write_workspace_base_marker(&worktree_path, base_branch.as_str())
            .map_err(|error| TaskLifecycleError::Io(format!("{error:?}")))?;
        ensure_grove_git_exclude_entries(repository.path.as_path())
            .map_err(|error| TaskLifecycleError::Io(format!("{error:?}")))?;
        copy_env_files(repository.path.as_path(), &worktree_path)
            .map_err(|error| TaskLifecycleError::Io(format!("{error:?}")))?;

        let setup_script_path = repository.path.join(GROVE_SETUP_SCRIPT_FILE);
        if setup_script_path.exists() {
            let context = SetupScriptContext {
                script_path: setup_script_path,
                main_worktree_path: repository.path.clone(),
                workspace_path: worktree_path.clone(),
                worktree_branch: request.task_name.clone(),
            };
            if let Err(error) = setup_script_runner.run(&context) {
                warnings.push(format!(
                    "setup script failed for {}: {error}",
                    repository.name
                ));
            }
        }

        let setup_command = repository.defaults.workspace_init_command.trim();
        if !setup_command.is_empty() {
            let context = SetupCommandContext {
                main_worktree_path: repository.path.clone(),
                workspace_path: worktree_path.clone(),
                worktree_branch: request.task_name.clone(),
            };
            if let Err(error) = setup_command_runner.run(&context, setup_command) {
                warnings.push(format!(
                    "setup command failed for {}: {error}",
                    repository.name
                ));
            }
        }

        let worktree = Worktree::try_new(
            repository.name.clone(),
            repository.path.clone(),
            worktree_path,
            request.task_name.clone(),
            request.agent,
            WorkspaceStatus::Idle,
        )
        .map_err(|error| TaskLifecycleError::TaskInvalid(format!("{error:?}")))?
        .with_base_branch(Some(base_branch));
        worktrees.push(worktree);
    }

    let task: Task =
        create_task_domain(request.task_name.as_str(), task_root.as_path(), worktrees)?;
    write_task_manifest(&task_root, &task)?;

    Ok(CreateTaskResult {
        task_root,
        task,
        warnings,
    })
}

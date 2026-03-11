use std::fs;
use std::path::Path;
use std::process::Command;

use crate::domain::{Task, WorkspaceStatus, Worktree};

use super::{
    CreateBaseTaskRequest, CreateTaskRequest, CreateTaskResult, GROVE_SETUP_SCRIPT_FILE,
    TaskBranchSource, TaskLifecycleError, create_task_domain, repo_directory_name,
    resolve_repository_base_branch, write_task_manifest,
};
use crate::application::workspace_lifecycle::{
    GitCommandRunner, SetupCommandContext, SetupCommandRunner, SetupScriptContext,
    SetupScriptRunner, copy_env_files, ensure_grove_git_exclude_entries,
    write_workspace_base_marker,
};

pub(super) fn create_task_worktree(
    task_root: &Path,
    task_branch: &str,
    repository: &crate::infrastructure::config::RepositoryConfig,
    agent: crate::domain::AgentType,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
    setup_command_runner: &impl SetupCommandRunner,
) -> Result<(Worktree, Vec<String>), TaskLifecycleError> {
    let base_branch = resolve_repository_base_branch(repository)?;
    let repository_dir = repo_directory_name(repository)?;
    let worktree_path = task_root.join(repository_dir);
    let args = vec![
        "worktree".to_string(),
        "add".to_string(),
        "-b".to_string(),
        task_branch.to_string(),
        worktree_path.to_string_lossy().to_string(),
        base_branch.clone(),
    ];
    git_runner
        .run(repository.path.as_path(), &args)
        .map_err(TaskLifecycleError::GitCommandFailed)?;

    fs::create_dir_all(&worktree_path)
        .map_err(|error| TaskLifecycleError::Io(error.to_string()))?;
    write_workspace_base_marker(&worktree_path, base_branch.as_str())
        .map_err(|error| TaskLifecycleError::Io(format!("{error:?}")))?;
    ensure_grove_git_exclude_entries(repository.path.as_path())
        .map_err(|error| TaskLifecycleError::Io(format!("{error:?}")))?;
    copy_env_files(repository.path.as_path(), &worktree_path)
        .map_err(|error| TaskLifecycleError::Io(format!("{error:?}")))?;

    let mut warnings = Vec::new();
    let setup_script_path = repository.path.join(GROVE_SETUP_SCRIPT_FILE);
    if setup_script_path.exists() {
        let context = SetupScriptContext {
            script_path: setup_script_path,
            main_worktree_path: repository.path.clone(),
            workspace_path: worktree_path.clone(),
            worktree_branch: task_branch.to_string(),
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
            worktree_branch: task_branch.to_string(),
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
        task_branch.to_string(),
        agent,
        WorkspaceStatus::Idle,
    )
    .map_err(|error| TaskLifecycleError::TaskInvalid(format!("{error:?}")))?
    .with_base_branch(Some(base_branch));

    Ok((worktree, warnings))
}

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
    let task_branch = match &request.branch_source {
        TaskBranchSource::BaseBranch => request.task_name.clone(),
        TaskBranchSource::PullRequest { branch_name, .. } => branch_name.clone(),
    };

    for repository in &request.repositories {
        match &request.branch_source {
            TaskBranchSource::BaseBranch => {
                let (worktree, mut repository_warnings) = create_task_worktree(
                    task_root.as_path(),
                    request.task_name.as_str(),
                    repository,
                    request.agent,
                    git_runner,
                    setup_script_runner,
                    setup_command_runner,
                )?;
                warnings.append(&mut repository_warnings);
                worktrees.push(worktree);
            }
            TaskBranchSource::PullRequest {
                number,
                branch_name,
            } => {
                let base_branch = resolve_repository_base_branch(repository)?;
                let repository_dir = repo_directory_name(repository)?;
                let worktree_path = task_root.join(repository_dir);
                let worktree_branch = branch_name.clone();
                let fetch_args = vec![
                    "fetch".to_string(),
                    "origin".to_string(),
                    format!("pull/{number}/head"),
                ];
                git_runner
                    .run(repository.path.as_path(), &fetch_args)
                    .map_err(TaskLifecycleError::GitCommandFailed)?;
                if local_branch_exists(repository.path.as_path(), branch_name)? {
                    let move_branch_args = vec![
                        "branch".to_string(),
                        "-f".to_string(),
                        branch_name.clone(),
                        "FETCH_HEAD".to_string(),
                    ];
                    git_runner
                        .run(repository.path.as_path(), &move_branch_args)
                        .map_err(TaskLifecycleError::GitCommandFailed)?;
                    let add_existing_args = vec![
                        "worktree".to_string(),
                        "add".to_string(),
                        worktree_path.to_string_lossy().to_string(),
                        branch_name.clone(),
                    ];
                    git_runner
                        .run(repository.path.as_path(), &add_existing_args)
                        .map_err(TaskLifecycleError::GitCommandFailed)?;
                } else {
                    let add_new_args = vec![
                        "worktree".to_string(),
                        "add".to_string(),
                        "-b".to_string(),
                        branch_name.clone(),
                        worktree_path.to_string_lossy().to_string(),
                        "FETCH_HEAD".to_string(),
                    ];
                    git_runner
                        .run(repository.path.as_path(), &add_new_args)
                        .map_err(TaskLifecycleError::GitCommandFailed)?;
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
                        worktree_branch: worktree_branch.clone(),
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
                        worktree_branch: worktree_branch.clone(),
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
                    worktree_branch,
                    request.agent,
                    WorkspaceStatus::Idle,
                )
                .map_err(|error| TaskLifecycleError::TaskInvalid(format!("{error:?}")))?
                .with_base_branch(Some(base_branch));
                worktrees.push(worktree);
            }
        }
    }

    let task: Task = create_task_domain(
        request.task_name.as_str(),
        task_branch.as_str(),
        task_root.as_path(),
        worktrees,
    )?;
    write_task_manifest(&task_root, &task)?;

    Ok(CreateTaskResult {
        task_root,
        task,
        warnings,
    })
}

pub(super) fn create_base_task_in_root(
    tasks_root: &Path,
    request: &CreateBaseTaskRequest,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    let repo_name = repo_directory_name(&request.repository)?;
    if !request.repository.path.exists() {
        return Err(TaskLifecycleError::Io(format!(
            "repository path does not exist: {}",
            request.repository.path.display()
        )));
    }

    let task_root = tasks_root.join(&repo_name);
    fs::create_dir_all(&task_root).map_err(|error| TaskLifecycleError::Io(error.to_string()))?;

    let worktree = Worktree::try_new(
        request.repository.name.clone(),
        request.repository.path.clone(),
        request.repository.path.clone(),
        request.base_branch.clone(),
        request.agent,
        WorkspaceStatus::Main,
    )
    .map_err(|error| TaskLifecycleError::TaskInvalid(format!("{error:?}")))?
    .with_base_branch(Some(request.base_branch.clone()));

    let task = create_task_domain(&repo_name, &request.base_branch, &task_root, vec![worktree])?;
    write_task_manifest(&task_root, &task)?;

    Ok(CreateTaskResult {
        task_root,
        task,
        warnings: Vec::new(),
    })
}

fn local_branch_exists(repo_root: &Path, branch_name: &str) -> Result<bool, TaskLifecycleError> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch_name}"),
        ])
        .output()
        .map_err(|error| TaskLifecycleError::GitCommandFailed(error.to_string()))?;
    if output.status.success() {
        return Ok(true);
    }

    Ok(false)
}

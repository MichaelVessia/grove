use std::fs;
use std::path::Path;

use crate::domain::AgentType;

use super::{
    BranchMode, CommandSetupCommandRunner, CreateWorkspaceRequest, CreateWorkspaceResult,
    GitCommandRunner, SetupCommandContext, SetupCommandRunner, SetupScriptContext,
    SetupScriptRunner, WorkspaceLifecycleError, WorkspaceSetupTemplate,
};

pub(super) fn create_workspace(
    repo_root: &Path,
    request: &CreateWorkspaceRequest,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
    let setup_command_runner = CommandSetupCommandRunner;
    create_workspace_with_template(
        repo_root,
        request,
        None,
        git_runner,
        setup_script_runner,
        &setup_command_runner,
    )
}

pub(super) fn create_workspace_with_template(
    repo_root: &Path,
    request: &CreateWorkspaceRequest,
    setup_template: Option<&WorkspaceSetupTemplate>,
    git_runner: &impl GitCommandRunner,
    setup_script_runner: &impl SetupScriptRunner,
    setup_command_runner: &impl SetupCommandRunner,
) -> Result<CreateWorkspaceResult, WorkspaceLifecycleError> {
    request.validate()?;

    let workspace_path = super::workspace_directory_path(repo_root, &request.workspace_name)?;
    let workspace_parent = workspace_path
        .parent()
        .ok_or(WorkspaceLifecycleError::RepoNameUnavailable)?;
    fs::create_dir_all(workspace_parent)
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;
    let branch = request.branch_name();

    run_create_worktree_command(repo_root, &workspace_path, request, git_runner)?;

    fs::create_dir_all(&workspace_path)
        .map_err(|error| WorkspaceLifecycleError::Io(error.to_string()))?;
    write_workspace_markers(
        &workspace_path,
        request.agent,
        &request.marker_base_branch(),
    )?;
    super::ensure_grove_git_exclude_entries(repo_root)?;
    super::copy_env_files(repo_root, &workspace_path)?;

    let mut warnings = Vec::new();
    let setup_script_path = repo_root.join(super::GROVE_SETUP_SCRIPT_FILE);
    if setup_script_path.exists() {
        let context = SetupScriptContext {
            script_path: setup_script_path,
            main_worktree_path: repo_root.to_path_buf(),
            workspace_path: workspace_path.clone(),
            worktree_branch: branch.clone(),
        };
        if let Err(error) = setup_script_runner.run(&context) {
            warnings.push(format!("setup script failed: {error}"));
        }
    }

    if let Some(template) = setup_template
        && template.auto_run_setup_commands
    {
        let context = SetupCommandContext {
            main_worktree_path: repo_root.to_path_buf(),
            workspace_path: workspace_path.clone(),
            worktree_branch: branch.clone(),
        };
        for command in template.commands.iter().map(|value| value.trim()) {
            if command.is_empty() {
                continue;
            }
            if let Err(error) = setup_command_runner.run(&context, command) {
                warnings.push(format!("setup command '{command}' failed: {error}"));
            }
        }
    }

    Ok(CreateWorkspaceResult {
        workspace_path,
        branch,
        warnings,
    })
}

fn run_create_worktree_command(
    repo_root: &Path,
    workspace_path: &Path,
    request: &CreateWorkspaceRequest,
    git_runner: &impl GitCommandRunner,
) -> Result<(), WorkspaceLifecycleError> {
    let workspace_path_arg = workspace_path.to_string_lossy().to_string();
    let args = match &request.branch_mode {
        BranchMode::NewBranch { base_branch } => vec![
            "worktree".to_string(),
            "add".to_string(),
            "-b".to_string(),
            request.branch_name(),
            workspace_path_arg,
            base_branch.clone(),
        ],
        BranchMode::ExistingBranch { existing_branch } => vec![
            "worktree".to_string(),
            "add".to_string(),
            workspace_path_arg,
            existing_branch.clone(),
        ],
        BranchMode::PullRequest { number, .. } => {
            let fetch_args = vec![
                "fetch".to_string(),
                "origin".to_string(),
                format!("pull/{number}/head"),
            ];
            git_runner
                .run(repo_root, &fetch_args)
                .map_err(WorkspaceLifecycleError::GitCommandFailed)?;

            vec![
                "worktree".to_string(),
                "add".to_string(),
                "-b".to_string(),
                request.branch_name(),
                workspace_path_arg,
                "FETCH_HEAD".to_string(),
            ]
        }
    };

    git_runner
        .run(repo_root, &args)
        .map_err(WorkspaceLifecycleError::GitCommandFailed)
}

fn write_workspace_markers(
    workspace_path: &Path,
    agent: AgentType,
    base_branch: &str,
) -> Result<(), WorkspaceLifecycleError> {
    super::write_workspace_agent_marker(workspace_path, agent)?;
    super::write_workspace_base_marker(workspace_path, base_branch)
}

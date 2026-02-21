use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::application::commands::{
    AgentStartRequest, AgentStopRequest, ErrorCode as CommandErrorCode,
    InProcessLifecycleCommandService, LifecycleCommandService, RepoContext, WorkspaceCreateRequest,
    WorkspaceDeleteRequest, WorkspaceEditRequest, WorkspaceListRequest, WorkspaceMergeRequest,
    WorkspaceSelector, WorkspaceUpdateRequest,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::infrastructure::event_log::{FileEventLogger, now_millis};
use crate::interface::cli_contract::{CommandEnvelope, ErrorDetail, NextAction};
use crate::interface::cli_errors::{CliErrorCode, classify_error_message};
use crate::interface::next_actions::{
    NextActionsBuilder, after_agent_stop, after_workspace_create, after_workspace_merge,
};
use crate::interface::root_command_tree::{RootCommandTree, root_command_tree};

const DEBUG_RECORD_DIR: &str = ".grove";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CliArgs {
    print_hello: bool,
    event_log_path: Option<PathBuf>,
    debug_record: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TuiArgs {
    event_log_path: Option<PathBuf>,
    debug_record: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorkspaceListArgs {
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorkspaceCreateArgs {
    name: Option<String>,
    base_branch: Option<String>,
    existing_branch: Option<String>,
    agent: Option<AgentType>,
    start: bool,
    dry_run: bool,
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorkspaceEditArgs {
    workspace: Option<String>,
    workspace_path: Option<PathBuf>,
    agent: Option<AgentType>,
    base_branch: Option<String>,
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorkspaceDeleteArgs {
    workspace: Option<String>,
    workspace_path: Option<PathBuf>,
    delete_branch: bool,
    force_stop: bool,
    dry_run: bool,
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorkspaceMergeArgs {
    workspace: Option<String>,
    workspace_path: Option<PathBuf>,
    cleanup_workspace: bool,
    cleanup_branch: bool,
    dry_run: bool,
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorkspaceUpdateArgs {
    workspace: Option<String>,
    workspace_path: Option<PathBuf>,
    dry_run: bool,
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AgentStartArgs {
    workspace: Option<String>,
    workspace_path: Option<PathBuf>,
    prompt: Option<String>,
    pre_launch_command: Option<String>,
    skip_permissions: bool,
    dry_run: bool,
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AgentStopArgs {
    workspace: Option<String>,
    workspace_path: Option<PathBuf>,
    dry_run: bool,
    repo: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WorkspaceListResult {
    repo_root: String,
    workspaces: Vec<WorkspaceView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WorkspaceMutationResult {
    workspace: WorkspaceView,
    dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WorkspaceView {
    name: String,
    path: String,
    project_name: Option<String>,
    project_path: Option<String>,
    branch: String,
    base_branch: Option<String>,
    last_activity_unix_secs: Option<i64>,
    agent: String,
    status: String,
    is_main: bool,
    is_orphaned: bool,
    supported_agent: bool,
}

impl WorkspaceView {
    fn from_workspace(workspace: Workspace) -> Self {
        Self {
            name: workspace.name,
            path: workspace.path.display().to_string(),
            project_name: workspace.project_name,
            project_path: workspace
                .project_path
                .map(|path| path.display().to_string()),
            branch: workspace.branch,
            base_branch: workspace.base_branch,
            last_activity_unix_secs: workspace.last_activity_unix_secs,
            agent: agent_label(workspace.agent).to_string(),
            status: workspace_status_label(workspace.status).to_string(),
            is_main: workspace.is_main,
            is_orphaned: workspace.is_orphaned,
            supported_agent: workspace.supported_agent,
        }
    }
}

fn parse_cli_args(args: impl IntoIterator<Item = String>) -> std::io::Result<CliArgs> {
    let mut cli = CliArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--print-hello" => {
                cli.print_hello = true;
            }
            "--event-log" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--event-log requires a file path",
                    ));
                };
                cli.event_log_path = Some(PathBuf::from(path));
            }
            "--debug-record" => {
                cli.debug_record = true;
            }
            _ => {}
        }
    }

    Ok(cli)
}

fn parse_tui_args(args: impl IntoIterator<Item = String>) -> std::io::Result<TuiArgs> {
    let mut cli = TuiArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--event-log" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--event-log requires a file path",
                    ));
                };
                cli.event_log_path = Some(PathBuf::from(path));
            }
            "--debug-record" => {
                cli.debug_record = true;
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'tui': {argument}"),
                ));
            }
        }
    }

    Ok(cli)
}

fn parse_workspace_list_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<WorkspaceListArgs> {
    let mut parsed = WorkspaceListArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'workspace list': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn parse_workspace_create_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<WorkspaceCreateArgs> {
    let mut parsed = WorkspaceCreateArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--name" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--name requires a value",
                    ));
                };
                parsed.name = Some(name);
            }
            "--base" => {
                let Some(branch) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--base requires a value",
                    ));
                };
                parsed.base_branch = Some(branch);
            }
            "--existing-branch" => {
                let Some(branch) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--existing-branch requires a value",
                    ));
                };
                parsed.existing_branch = Some(branch);
            }
            "--agent" => {
                let Some(value) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--agent requires a value",
                    ));
                };
                parsed.agent = Some(parse_agent(&value)?);
            }
            "--start" => {
                parsed.start = true;
            }
            "--dry-run" => {
                parsed.dry_run = true;
            }
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'workspace create': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn parse_workspace_edit_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<WorkspaceEditArgs> {
    let mut parsed = WorkspaceEditArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workspace" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace requires a value",
                    ));
                };
                parsed.workspace = Some(name);
            }
            "--workspace-path" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace-path requires a path",
                    ));
                };
                parsed.workspace_path = Some(PathBuf::from(path));
            }
            "--agent" => {
                let Some(value) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--agent requires a value",
                    ));
                };
                parsed.agent = Some(parse_agent(&value)?);
            }
            "--base" => {
                let Some(branch) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--base requires a value",
                    ));
                };
                parsed.base_branch = Some(branch);
            }
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'workspace edit': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn workspace_selector(
    workspace_name: Option<String>,
    workspace_path: Option<PathBuf>,
) -> std::io::Result<WorkspaceSelector> {
    match (workspace_name, workspace_path) {
        (Some(name), Some(path)) => Ok(WorkspaceSelector::NameAndPath { name, path }),
        (Some(name), None) => Ok(WorkspaceSelector::Name(name)),
        (None, Some(path)) => Ok(WorkspaceSelector::Path(path)),
        (None, None) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "workspace selector is required (--workspace or --workspace-path)",
        )),
    }
}

fn parse_workspace_delete_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<WorkspaceDeleteArgs> {
    let mut parsed = WorkspaceDeleteArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workspace" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace requires a value",
                    ));
                };
                parsed.workspace = Some(name);
            }
            "--workspace-path" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace-path requires a path",
                    ));
                };
                parsed.workspace_path = Some(PathBuf::from(path));
            }
            "--delete-branch" => {
                parsed.delete_branch = true;
            }
            "--force-stop" => {
                parsed.force_stop = true;
            }
            "--dry-run" => {
                parsed.dry_run = true;
            }
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'workspace delete': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn parse_workspace_merge_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<WorkspaceMergeArgs> {
    let mut parsed = WorkspaceMergeArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workspace" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace requires a value",
                    ));
                };
                parsed.workspace = Some(name);
            }
            "--workspace-path" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace-path requires a path",
                    ));
                };
                parsed.workspace_path = Some(PathBuf::from(path));
            }
            "--cleanup-workspace" => {
                parsed.cleanup_workspace = true;
            }
            "--cleanup-branch" => {
                parsed.cleanup_branch = true;
            }
            "--dry-run" => {
                parsed.dry_run = true;
            }
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'workspace merge': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn parse_workspace_update_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<WorkspaceUpdateArgs> {
    let mut parsed = WorkspaceUpdateArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workspace" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace requires a value",
                    ));
                };
                parsed.workspace = Some(name);
            }
            "--workspace-path" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace-path requires a path",
                    ));
                };
                parsed.workspace_path = Some(PathBuf::from(path));
            }
            "--dry-run" => {
                parsed.dry_run = true;
            }
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'workspace update': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn parse_agent_start_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<AgentStartArgs> {
    let mut parsed = AgentStartArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workspace" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace requires a value",
                    ));
                };
                parsed.workspace = Some(name);
            }
            "--workspace-path" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace-path requires a path",
                    ));
                };
                parsed.workspace_path = Some(PathBuf::from(path));
            }
            "--prompt" => {
                let Some(prompt) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--prompt requires a value",
                    ));
                };
                parsed.prompt = Some(prompt);
            }
            "--pre-launch" => {
                let Some(command) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--pre-launch requires a value",
                    ));
                };
                parsed.pre_launch_command = Some(command);
            }
            "--skip-permissions" => {
                parsed.skip_permissions = true;
            }
            "--dry-run" => {
                parsed.dry_run = true;
            }
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'agent start': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn parse_agent_stop_args(args: impl IntoIterator<Item = String>) -> std::io::Result<AgentStopArgs> {
    let mut parsed = AgentStopArgs::default();
    let mut args = args.into_iter();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workspace" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace requires a value",
                    ));
                };
                parsed.workspace = Some(name);
            }
            "--workspace-path" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--workspace-path requires a path",
                    ));
                };
                parsed.workspace_path = Some(PathBuf::from(path));
            }
            "--dry-run" => {
                parsed.dry_run = true;
            }
            "--repo" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--repo requires a path",
                    ));
                };
                parsed.repo = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'agent stop': {argument}"),
                ));
            }
        }
    }

    Ok(parsed)
}

fn parse_agent(value: &str) -> std::io::Result<AgentType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "claude" => Ok(AgentType::Claude),
        "codex" => Ok(AgentType::Codex),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--agent must be one of: claude, codex",
        )),
    }
}

fn debug_record_path(app_start_ts: u64) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from(DEBUG_RECORD_DIR);
    fs::create_dir_all(&dir)?;

    let mut sequence = 0u32;
    loop {
        let file_name = if sequence == 0 {
            format!("debug-record-{app_start_ts}-{}.jsonl", std::process::id())
        } else {
            format!(
                "debug-record-{app_start_ts}-{}-{sequence}.jsonl",
                std::process::id()
            )
        };
        let path = dir.join(file_name);
        if !path.exists() {
            return Ok(path);
        }
        sequence = sequence.saturating_add(1);
    }
}

fn resolve_event_log_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    let grove_dir = Path::new(DEBUG_RECORD_DIR);
    if path.starts_with(grove_dir) {
        return path;
    }

    grove_dir.join(path)
}

fn ensure_event_log_parent_directory(path: &Path) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    fs::create_dir_all(parent)
}

fn root_next_actions() -> Vec<NextAction> {
    NextActionsBuilder::new()
        .push("grove tui", "Launch Grove TUI")
        .push("grove workspace list", "List repository workspaces")
        .build()
}

fn workspace_list_next_actions() -> Vec<NextAction> {
    NextActionsBuilder::new()
        .push(
            "grove workspace create --name <name> --base <branch>",
            "Create a workspace",
        )
        .push("grove", "Show root command tree")
        .build()
}

fn workspace_edit_next_actions(workspace_name: &str) -> Vec<NextAction> {
    let selector = format!("--workspace {workspace_name}");
    NextActionsBuilder::new()
        .push("workspace list", "Inspect workspace inventory")
        .push(
            format!("workspace update {selector}"),
            "Update workspace from base branch",
        )
        .push(
            format!("agent start {selector}"),
            "Start agent with updated workspace settings",
        )
        .build()
}

fn workspace_delete_next_actions() -> Vec<NextAction> {
    NextActionsBuilder::new()
        .push("workspace list", "Inspect workspace inventory")
        .push(
            "workspace create --name <name> --base <branch>",
            "Create a replacement workspace",
        )
        .build()
}

fn workspace_update_next_actions(workspace_name: &str) -> Vec<NextAction> {
    let selector = format!("--workspace {workspace_name}");
    NextActionsBuilder::new()
        .push("workspace list", "Inspect workspace inventory")
        .push(
            format!("workspace merge {selector}"),
            "Merge updated workspace branch",
        )
        .push(
            format!("agent start {selector}"),
            "Start or restart the workspace agent",
        )
        .build()
}

fn agent_start_next_actions(workspace_name: &str) -> Vec<NextAction> {
    let selector = format!("--workspace {workspace_name}");
    NextActionsBuilder::new()
        .push(
            format!("agent stop {selector}"),
            "Stop the running workspace agent",
        )
        .push(
            format!("workspace update {selector}"),
            "Update workspace from base branch",
        )
        .push("workspace list", "Inspect workspace inventory")
        .build()
}

fn root_command_envelope() -> CommandEnvelope<RootCommandTree> {
    CommandEnvelope::success(
        "grove",
        root_command_tree(),
        Vec::new(),
        root_next_actions(),
    )
}

fn emit_json<T: serde::Serialize>(payload: &T) -> std::io::Result<()> {
    let json =
        serde_json::to_string(payload).map_err(|error| std::io::Error::other(error.to_string()))?;
    println!("{json}");
    Ok(())
}

fn emit_error(
    command: &str,
    code: CliErrorCode,
    message: String,
    fix: &str,
) -> std::io::Result<()> {
    emit_json(&CommandEnvelope::<serde_json::Value>::error(
        command,
        ErrorDetail::from_code(code, message),
        fix.to_string(),
        Vec::new(),
        vec![NextAction::new("grove", "Show root command tree")],
    ))
}

fn command_error_code(error: CommandErrorCode, message: &str) -> CliErrorCode {
    match error {
        CommandErrorCode::InvalidArgument => classify_error_message(message),
        CommandErrorCode::NotFound => classify_error_message(message),
        CommandErrorCode::Conflict => CliErrorCode::Conflict,
        CommandErrorCode::RuntimeFailure => classify_error_message(message),
        CommandErrorCode::Internal => CliErrorCode::Internal,
    }
}

fn run_workspace_list(parsed: WorkspaceListArgs) -> std::io::Result<()> {
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let command = "grove workspace list";
    let request = WorkspaceListRequest {
        context: RepoContext {
            repo_root: repo_root.clone(),
        },
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.workspace_list(request);
    match response {
        Ok(result) => {
            let payload = WorkspaceListResult {
                repo_root: repo_root.display().to_string(),
                workspaces: result
                    .workspaces
                    .into_iter()
                    .map(WorkspaceView::from_workspace)
                    .collect(),
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                Vec::new(),
                workspace_list_next_actions(),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Verify repository path and retry",
        ),
    }
}

fn run_workspace_create(parsed: WorkspaceCreateArgs) -> std::io::Result<()> {
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let command = "grove workspace create";
    let request = WorkspaceCreateRequest {
        context: RepoContext {
            repo_root: repo_root.clone(),
        },
        name: parsed.name.unwrap_or_default(),
        base_branch: parsed.base_branch,
        existing_branch: parsed.existing_branch,
        agent: parsed.agent,
        start: parsed.start,
        dry_run: parsed.dry_run,
        setup_template: None,
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.workspace_create(request);
    match response {
        Ok(result) => {
            let started = result.workspace.status.is_running();
            let payload = WorkspaceMutationResult {
                workspace: WorkspaceView::from_workspace(result.workspace.clone()),
                dry_run: parsed.dry_run,
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                result.warnings,
                after_workspace_create(&result.workspace.name, started),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Adjust create arguments and retry",
        ),
    }
}

fn run_workspace_edit(parsed: WorkspaceEditArgs) -> std::io::Result<()> {
    let command = "grove workspace edit";
    let selector = match workspace_selector(parsed.workspace, parsed.workspace_path) {
        Ok(selector) => selector,
        Err(error) => {
            return emit_error(
                command,
                CliErrorCode::InvalidArgument,
                error.to_string(),
                "Retry with '--workspace <name>' or '--workspace-path <path>' plus edit flags",
            );
        }
    };
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let request = WorkspaceEditRequest {
        context: RepoContext {
            repo_root: repo_root.clone(),
        },
        selector,
        agent: parsed.agent,
        base_branch: parsed.base_branch,
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.workspace_edit(request);
    match response {
        Ok(result) => {
            let workspace_name = result.workspace.name.clone();
            let payload = WorkspaceMutationResult {
                workspace: WorkspaceView::from_workspace(result.workspace),
                dry_run: false,
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                result.warnings,
                workspace_edit_next_actions(&workspace_name),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Adjust selector and edit flags, then retry",
        ),
    }
}

fn run_workspace_delete(parsed: WorkspaceDeleteArgs) -> std::io::Result<()> {
    let command = "grove workspace delete";
    let selector = match workspace_selector(parsed.workspace, parsed.workspace_path) {
        Ok(selector) => selector,
        Err(error) => {
            return emit_error(
                command,
                CliErrorCode::InvalidArgument,
                error.to_string(),
                "Retry with '--workspace <name>' or '--workspace-path <path>'",
            );
        }
    };
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let request = WorkspaceDeleteRequest {
        context: RepoContext { repo_root },
        selector,
        delete_branch: parsed.delete_branch,
        force_stop: parsed.force_stop,
        dry_run: parsed.dry_run,
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.workspace_delete(request);
    match response {
        Ok(result) => {
            let payload = WorkspaceMutationResult {
                workspace: WorkspaceView::from_workspace(result.workspace),
                dry_run: parsed.dry_run,
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                result.warnings,
                workspace_delete_next_actions(),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Adjust selector/delete flags, then retry",
        ),
    }
}

fn run_workspace_merge(parsed: WorkspaceMergeArgs) -> std::io::Result<()> {
    let command = "grove workspace merge";
    let selector = match workspace_selector(parsed.workspace, parsed.workspace_path) {
        Ok(selector) => selector,
        Err(error) => {
            return emit_error(
                command,
                CliErrorCode::InvalidArgument,
                error.to_string(),
                "Retry with '--workspace <name>' or '--workspace-path <path>'",
            );
        }
    };
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let request = WorkspaceMergeRequest {
        context: RepoContext { repo_root },
        selector,
        cleanup_workspace: parsed.cleanup_workspace,
        cleanup_branch: parsed.cleanup_branch,
        dry_run: parsed.dry_run,
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.workspace_merge(request);
    match response {
        Ok(result) => {
            let workspace_name = result.workspace.name.clone();
            let payload = WorkspaceMutationResult {
                workspace: WorkspaceView::from_workspace(result.workspace),
                dry_run: parsed.dry_run,
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                result.warnings,
                after_workspace_merge(&workspace_name),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Resolve workspace/base branch state and retry merge",
        ),
    }
}

fn run_workspace_update(parsed: WorkspaceUpdateArgs) -> std::io::Result<()> {
    let command = "grove workspace update";
    let selector = match workspace_selector(parsed.workspace, parsed.workspace_path) {
        Ok(selector) => selector,
        Err(error) => {
            return emit_error(
                command,
                CliErrorCode::InvalidArgument,
                error.to_string(),
                "Retry with '--workspace <name>' or '--workspace-path <path>'",
            );
        }
    };
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let request = WorkspaceUpdateRequest {
        context: RepoContext { repo_root },
        selector,
        dry_run: parsed.dry_run,
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.workspace_update(request);
    match response {
        Ok(result) => {
            let workspace_name = result.workspace.name.clone();
            let payload = WorkspaceMutationResult {
                workspace: WorkspaceView::from_workspace(result.workspace),
                dry_run: parsed.dry_run,
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                result.warnings,
                workspace_update_next_actions(&workspace_name),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Resolve workspace/base branch state and retry update",
        ),
    }
}

fn run_agent_start(parsed: AgentStartArgs) -> std::io::Result<()> {
    let command = "grove agent start";
    let selector = match workspace_selector(parsed.workspace, parsed.workspace_path) {
        Ok(selector) => selector,
        Err(error) => {
            return emit_error(
                command,
                CliErrorCode::InvalidArgument,
                error.to_string(),
                "Retry with '--workspace <name>' or '--workspace-path <path>'",
            );
        }
    };
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let request = AgentStartRequest {
        context: RepoContext { repo_root },
        selector,
        workspace_hint: None,
        prompt: parsed.prompt,
        pre_launch_command: parsed.pre_launch_command,
        skip_permissions: parsed.skip_permissions,
        capture_cols: None,
        capture_rows: None,
        dry_run: parsed.dry_run,
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.agent_start(request);
    match response {
        Ok(result) => {
            let workspace_name = result.workspace.name.clone();
            let payload = WorkspaceMutationResult {
                workspace: WorkspaceView::from_workspace(result.workspace),
                dry_run: parsed.dry_run,
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                result.warnings,
                agent_start_next_actions(&workspace_name),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Resolve workspace/runtime state and retry start",
        ),
    }
}

fn run_agent_stop(parsed: AgentStopArgs) -> std::io::Result<()> {
    let command = "grove agent stop";
    let selector = match workspace_selector(parsed.workspace, parsed.workspace_path) {
        Ok(selector) => selector,
        Err(error) => {
            return emit_error(
                command,
                CliErrorCode::InvalidArgument,
                error.to_string(),
                "Retry with '--workspace <name>' or '--workspace-path <path>'",
            );
        }
    };
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let request = AgentStopRequest {
        context: RepoContext { repo_root },
        selector,
        workspace_hint: None,
        dry_run: parsed.dry_run,
    };
    let service = InProcessLifecycleCommandService::new();
    let response = service.agent_stop(request);
    match response {
        Ok(result) => {
            let workspace_name = result.workspace.name.clone();
            let payload = WorkspaceMutationResult {
                workspace: WorkspaceView::from_workspace(result.workspace),
                dry_run: parsed.dry_run,
            };
            emit_json(&CommandEnvelope::success(
                command,
                payload,
                result.warnings,
                after_agent_stop(&workspace_name),
            ))
        }
        Err(error) => emit_error(
            command,
            command_error_code(error.code, &error.message),
            error.message,
            "Resolve workspace/runtime state and retry stop",
        ),
    }
}

fn agent_label(agent: AgentType) -> &'static str {
    match agent {
        AgentType::Claude => "claude",
        AgentType::Codex => "codex",
    }
}

fn workspace_status_label(status: WorkspaceStatus) -> &'static str {
    match status {
        WorkspaceStatus::Main => "main",
        WorkspaceStatus::Idle => "idle",
        WorkspaceStatus::Active => "active",
        WorkspaceStatus::Thinking => "thinking",
        WorkspaceStatus::Waiting => "waiting",
        WorkspaceStatus::Done => "done",
        WorkspaceStatus::Error => "error",
        WorkspaceStatus::Unknown => "unknown",
        WorkspaceStatus::Unsupported => "unsupported",
    }
}

fn run_tui(cli: TuiArgs) -> std::io::Result<()> {
    let app_start_ts = now_millis();
    let debug_record_path = if cli.debug_record {
        Some(debug_record_path(app_start_ts)?)
    } else {
        None
    };
    if let Some(path) = debug_record_path.as_ref() {
        eprintln!("grove debug record: {}", path.display());
    }
    let event_log_path = debug_record_path.or(cli.event_log_path.map(resolve_event_log_path));
    if let Some(path) = event_log_path.as_ref() {
        ensure_event_log_parent_directory(path)?;
    }

    if cli.debug_record
        && let Some(path) = event_log_path
    {
        return crate::interface::tui::run_with_debug_record(path, app_start_ts);
    }

    crate::interface::tui::run_with_event_log(event_log_path)
}

pub fn run(args: impl IntoIterator<Item = String>) -> std::io::Result<()> {
    let args = args.into_iter().collect::<Vec<String>>();
    let Some((first, remaining)) = args.split_first() else {
        return emit_json(&root_command_envelope());
    };
    if first == "tui" {
        return run_tui(parse_tui_args(remaining.iter().cloned())?);
    }
    if first == "workspace" {
        let Some((workspace_command, workspace_args)) = remaining.split_first() else {
            return emit_error(
                "grove workspace",
                CliErrorCode::InvalidArgument,
                "workspace subcommand is required".to_string(),
                "Use 'grove workspace list' to inspect workspaces",
            );
        };
        if workspace_command == "list" {
            return match parse_workspace_list_args(workspace_args.iter().cloned()) {
                Ok(parsed) => run_workspace_list(parsed),
                Err(error) => emit_error(
                    "grove workspace list",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with '--repo <path>' or omit '--repo' to use current directory",
                ),
            };
        }
        if workspace_command == "create" {
            return match parse_workspace_create_args(workspace_args.iter().cloned()) {
                Ok(parsed) => run_workspace_create(parsed),
                Err(error) => emit_error(
                    "grove workspace create",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with '--name <name> --base <branch>' and optional flags",
                ),
            };
        }
        if workspace_command == "edit" {
            return match parse_workspace_edit_args(workspace_args.iter().cloned()) {
                Ok(parsed) => run_workspace_edit(parsed),
                Err(error) => emit_error(
                    "grove workspace edit",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with '--workspace <name>' or '--workspace-path <path>' plus edit flags",
                ),
            };
        }
        if workspace_command == "delete" {
            return match parse_workspace_delete_args(workspace_args.iter().cloned()) {
                Ok(parsed) => run_workspace_delete(parsed),
                Err(error) => emit_error(
                    "grove workspace delete",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with selector flags and optional '--delete-branch'/'--force-stop'",
                ),
            };
        }
        if workspace_command == "merge" {
            return match parse_workspace_merge_args(workspace_args.iter().cloned()) {
                Ok(parsed) => run_workspace_merge(parsed),
                Err(error) => emit_error(
                    "grove workspace merge",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with selector flags and optional cleanup/dry-run flags",
                ),
            };
        }
        if workspace_command == "update" {
            return match parse_workspace_update_args(workspace_args.iter().cloned()) {
                Ok(parsed) => run_workspace_update(parsed),
                Err(error) => emit_error(
                    "grove workspace update",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with selector flags and optional '--dry-run'",
                ),
            };
        }
    }
    if first == "agent" {
        let Some((agent_command, agent_args)) = remaining.split_first() else {
            return emit_error(
                "grove agent",
                CliErrorCode::InvalidArgument,
                "agent subcommand is required".to_string(),
                "Use 'grove agent start' or 'grove agent stop'",
            );
        };
        if agent_command == "start" {
            return match parse_agent_start_args(agent_args.iter().cloned()) {
                Ok(parsed) => run_agent_start(parsed),
                Err(error) => emit_error(
                    "grove agent start",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with selector flags and optional start arguments",
                ),
            };
        }
        if agent_command == "stop" {
            return match parse_agent_stop_args(agent_args.iter().cloned()) {
                Ok(parsed) => run_agent_stop(parsed),
                Err(error) => emit_error(
                    "grove agent stop",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with selector flags and optional '--dry-run'",
                ),
            };
        }
    }

    let cli = parse_cli_args(args)?;
    if cli.print_hello {
        let app_start_ts = now_millis();
        let debug_record_path = if cli.debug_record {
            Some(debug_record_path(app_start_ts)?)
        } else {
            None
        };
        if let Some(path) = debug_record_path.as_ref() {
            eprintln!("grove debug record: {}", path.display());
        }
        let event_log_path = debug_record_path.or(cli.event_log_path.map(resolve_event_log_path));
        if let Some(path) = event_log_path.as_ref() {
            ensure_event_log_parent_directory(path)?;
            let _ = FileEventLogger::open(path)?;
        }
        println!("Hello from grove.");
        return Ok(());
    }

    emit_error(
        "grove",
        CliErrorCode::InvalidArgument,
        "unknown command".to_string(),
        "Run 'grove' to view command tree and usage",
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CliArgs, TuiArgs, debug_record_path, ensure_event_log_parent_directory,
        parse_agent_start_args, parse_agent_stop_args, parse_cli_args, parse_tui_args,
        parse_workspace_create_args, parse_workspace_delete_args, parse_workspace_edit_args,
        parse_workspace_list_args, parse_workspace_merge_args, parse_workspace_update_args,
        resolve_event_log_path, root_command_envelope, workspace_selector,
    };
    use crate::application::commands::WorkspaceSelector;
    use crate::domain::AgentType;
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn cli_parser_reads_event_log_and_print_hello() {
        let parsed = parse_cli_args(vec![
            "--event-log".to_string(),
            "/tmp/events.jsonl".to_string(),
            "--print-hello".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            parsed,
            CliArgs {
                print_hello: true,
                event_log_path: Some(PathBuf::from("/tmp/events.jsonl")),
                debug_record: false,
            }
        );
    }

    #[test]
    fn cli_parser_requires_event_log_path() {
        let error = parse_cli_args(vec!["--event-log".to_string()])
            .expect_err("missing event log path should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn cli_parser_reads_debug_record_flag() {
        let parsed =
            parse_cli_args(vec!["--debug-record".to_string()]).expect("debug flag should parse");
        assert_eq!(
            parsed,
            CliArgs {
                print_hello: false,
                event_log_path: None,
                debug_record: true,
            }
        );
    }

    #[test]
    fn debug_record_path_uses_grove_directory_and_timestamp_prefix() {
        let app_start_ts = 1_771_023_000_555u64;
        let path = debug_record_path(app_start_ts).expect("path should resolve");
        let path_text = path.to_string_lossy();
        assert!(path_text.contains(".grove/"));
        assert!(path_text.contains(&format!("debug-record-{app_start_ts}")));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn resolve_event_log_path_places_relative_paths_under_grove_directory() {
        assert_eq!(
            resolve_event_log_path(PathBuf::from("events.jsonl")),
            PathBuf::from(".grove/events.jsonl")
        );
    }

    #[test]
    fn resolve_event_log_path_keeps_absolute_paths_unchanged() {
        assert_eq!(
            resolve_event_log_path(PathBuf::from("/tmp/events.jsonl")),
            PathBuf::from("/tmp/events.jsonl")
        );
    }

    #[test]
    fn resolve_event_log_path_keeps_grove_prefixed_relative_paths() {
        assert_eq!(
            resolve_event_log_path(PathBuf::from(".grove/custom/events.jsonl")),
            PathBuf::from(".grove/custom/events.jsonl")
        );
    }

    #[test]
    fn ensure_event_log_parent_directory_creates_missing_directories() {
        let root = std::env::temp_dir().join(format!(
            "grove-main-tests-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos()
        ));
        let path = root.join(".grove/nested/events.jsonl");

        ensure_event_log_parent_directory(&path).expect("parent directory should be created");
        assert!(root.join(".grove/nested").exists());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn tui_parser_rejects_unknown_flags() {
        let error = parse_tui_args(vec!["--wat".to_string()]).expect_err("unknown flag");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn tui_parser_reads_event_log_and_debug_record() {
        let parsed = parse_tui_args(vec![
            "--event-log".to_string(),
            ".grove/events.jsonl".to_string(),
            "--debug-record".to_string(),
        ])
        .expect("tui args should parse");
        assert_eq!(
            parsed,
            TuiArgs {
                event_log_path: Some(PathBuf::from(".grove/events.jsonl")),
                debug_record: true,
            }
        );
    }

    #[test]
    fn root_envelope_serializes_command_tree() {
        let value = serde_json::to_value(root_command_envelope()).expect("serialize root envelope");
        assert_eq!(value["ok"], Value::from(true));
        assert_eq!(value["command"], Value::from("grove"));
        assert_eq!(value["result"]["command"], Value::from("grove"));
        assert!(value["result"]["commands"].is_array());
    }

    #[test]
    fn workspace_list_parser_reads_repo_path() {
        let parsed =
            parse_workspace_list_args(vec!["--repo".to_string(), "/repos/grove".to_string()])
                .expect("workspace list args should parse");
        assert_eq!(
            parsed,
            super::WorkspaceListArgs {
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn workspace_list_parser_rejects_unknown_flag() {
        let error = parse_workspace_list_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn workspace_create_parser_reads_expected_flags() {
        let parsed = parse_workspace_create_args(vec![
            "--name".to_string(),
            "feature-auth".to_string(),
            "--base".to_string(),
            "main".to_string(),
            "--agent".to_string(),
            "claude".to_string(),
            "--start".to_string(),
            "--dry-run".to_string(),
            "--repo".to_string(),
            "/repos/grove".to_string(),
        ])
        .expect("workspace create args should parse");

        assert_eq!(
            parsed,
            super::WorkspaceCreateArgs {
                name: Some("feature-auth".to_string()),
                base_branch: Some("main".to_string()),
                existing_branch: None,
                agent: Some(AgentType::Claude),
                start: true,
                dry_run: true,
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn workspace_create_parser_rejects_invalid_agent() {
        let error = parse_workspace_create_args(vec![
            "--name".to_string(),
            "feature-auth".to_string(),
            "--base".to_string(),
            "main".to_string(),
            "--agent".to_string(),
            "wat".to_string(),
        ])
        .expect_err("invalid agent should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn workspace_create_parser_rejects_unknown_flag() {
        let error =
            parse_workspace_create_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn workspace_edit_parser_reads_expected_flags() {
        let parsed = parse_workspace_edit_args(vec![
            "--workspace".to_string(),
            "feature-auth".to_string(),
            "--agent".to_string(),
            "codex".to_string(),
            "--base".to_string(),
            "main".to_string(),
            "--repo".to_string(),
            "/repos/grove".to_string(),
        ])
        .expect("workspace edit args should parse");

        assert_eq!(
            parsed,
            super::WorkspaceEditArgs {
                workspace: Some("feature-auth".to_string()),
                workspace_path: None,
                agent: Some(AgentType::Codex),
                base_branch: Some("main".to_string()),
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn workspace_edit_parser_rejects_unknown_flag() {
        let error = parse_workspace_edit_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn workspace_selector_requires_name_or_path() {
        let error = workspace_selector(None, None).expect_err("selector required");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn workspace_selector_accepts_name_and_path() {
        let selector = workspace_selector(
            Some("feature-auth".to_string()),
            Some(PathBuf::from("/tmp/grove-feature-auth")),
        )
        .expect("selector should parse");
        assert_eq!(
            selector,
            WorkspaceSelector::NameAndPath {
                name: "feature-auth".to_string(),
                path: PathBuf::from("/tmp/grove-feature-auth"),
            }
        );
    }

    #[test]
    fn workspace_delete_parser_reads_expected_flags() {
        let parsed = parse_workspace_delete_args(vec![
            "--workspace".to_string(),
            "feature-auth".to_string(),
            "--delete-branch".to_string(),
            "--force-stop".to_string(),
            "--dry-run".to_string(),
            "--repo".to_string(),
            "/repos/grove".to_string(),
        ])
        .expect("workspace delete args should parse");

        assert_eq!(
            parsed,
            super::WorkspaceDeleteArgs {
                workspace: Some("feature-auth".to_string()),
                workspace_path: None,
                delete_branch: true,
                force_stop: true,
                dry_run: true,
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn workspace_delete_parser_rejects_unknown_flag() {
        let error =
            parse_workspace_delete_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn workspace_merge_parser_reads_expected_flags() {
        let parsed = parse_workspace_merge_args(vec![
            "--workspace".to_string(),
            "feature-auth".to_string(),
            "--cleanup-workspace".to_string(),
            "--cleanup-branch".to_string(),
            "--dry-run".to_string(),
            "--repo".to_string(),
            "/repos/grove".to_string(),
        ])
        .expect("workspace merge args should parse");

        assert_eq!(
            parsed,
            super::WorkspaceMergeArgs {
                workspace: Some("feature-auth".to_string()),
                workspace_path: None,
                cleanup_workspace: true,
                cleanup_branch: true,
                dry_run: true,
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn workspace_merge_parser_rejects_unknown_flag() {
        let error = parse_workspace_merge_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn workspace_update_parser_reads_expected_flags() {
        let parsed = parse_workspace_update_args(vec![
            "--workspace".to_string(),
            "feature-auth".to_string(),
            "--dry-run".to_string(),
            "--repo".to_string(),
            "/repos/grove".to_string(),
        ])
        .expect("workspace update args should parse");

        assert_eq!(
            parsed,
            super::WorkspaceUpdateArgs {
                workspace: Some("feature-auth".to_string()),
                workspace_path: None,
                dry_run: true,
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn workspace_update_parser_rejects_unknown_flag() {
        let error =
            parse_workspace_update_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn agent_start_parser_reads_expected_flags() {
        let parsed = parse_agent_start_args(vec![
            "--workspace".to_string(),
            "feature-auth".to_string(),
            "--prompt".to_string(),
            "ship it".to_string(),
            "--pre-launch".to_string(),
            "echo pre".to_string(),
            "--skip-permissions".to_string(),
            "--dry-run".to_string(),
            "--repo".to_string(),
            "/repos/grove".to_string(),
        ])
        .expect("agent start args should parse");

        assert_eq!(
            parsed,
            super::AgentStartArgs {
                workspace: Some("feature-auth".to_string()),
                workspace_path: None,
                prompt: Some("ship it".to_string()),
                pre_launch_command: Some("echo pre".to_string()),
                skip_permissions: true,
                dry_run: true,
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn agent_start_parser_rejects_unknown_flag() {
        let error = parse_agent_start_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn agent_stop_parser_reads_expected_flags() {
        let parsed = parse_agent_stop_args(vec![
            "--workspace".to_string(),
            "feature-auth".to_string(),
            "--dry-run".to_string(),
            "--repo".to_string(),
            "/repos/grove".to_string(),
        ])
        .expect("agent stop args should parse");

        assert_eq!(
            parsed,
            super::AgentStopArgs {
                workspace: Some("feature-auth".to_string()),
                workspace_path: None,
                dry_run: true,
                repo: Some(PathBuf::from("/repos/grove")),
            }
        );
    }

    #[test]
    fn agent_stop_parser_rejects_unknown_flag() {
        let error = parse_agent_stop_args(vec!["--wat".to_string()]).expect_err("unknown arg");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }
}

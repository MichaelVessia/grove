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
use crate::interface::daemon::{
    DaemonAgentStartPayload, DaemonAgentStopPayload, DaemonCommandError,
    DaemonWorkspaceCreatePayload, DaemonWorkspaceDeletePayload, DaemonWorkspaceEditPayload,
    DaemonWorkspaceMergePayload, DaemonWorkspaceUpdatePayload, DaemonWorkspaceView,
    agent_start_via_socket, agent_stop_via_socket, set_daemon_client_log_path,
    workspace_create_via_socket, workspace_delete_via_socket, workspace_edit_via_socket,
    workspace_list_via_socket, workspace_merge_via_socket, workspace_update_via_socket,
};
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
    evidence_log_path: Option<PathBuf>,
    render_trace_path: Option<PathBuf>,
    frame_timing_log_path: Option<PathBuf>,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct DebugBundleArgs {
    out: Option<PathBuf>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DebugBundleResult {
    bundle_path: String,
    copied: Vec<String>,
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

fn workspace_view_from_daemon(workspace: DaemonWorkspaceView) -> WorkspaceView {
    WorkspaceView {
        name: workspace.name,
        path: workspace.path,
        project_name: workspace.project_name,
        project_path: workspace.project_path,
        branch: workspace.branch,
        base_branch: workspace.base_branch,
        last_activity_unix_secs: workspace.last_activity_unix_secs,
        agent: workspace.agent,
        status: workspace.status,
        is_main: workspace.is_main,
        is_orphaned: workspace.is_orphaned,
        supported_agent: workspace.supported_agent,
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
            "--evidence-log" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--evidence-log requires a file path",
                    ));
                };
                cli.evidence_log_path = Some(PathBuf::from(path));
            }
            "--render-trace" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--render-trace requires a file path",
                    ));
                };
                cli.render_trace_path = Some(PathBuf::from(path));
            }
            "--frame-timing-log" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--frame-timing-log requires a file path",
                    ));
                };
                cli.frame_timing_log_path = Some(PathBuf::from(path));
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

fn parse_debug_bundle_args(
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<DebugBundleArgs> {
    let mut parsed = DebugBundleArgs::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--out" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--out requires a path",
                    ));
                };
                parsed.out = Some(PathBuf::from(path));
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("unknown argument for 'debug bundle': {argument}"),
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
    unique_debug_artifact_path("debug-record", app_start_ts, "jsonl")
}

fn unique_debug_artifact_path(
    label: &str,
    app_start_ts: u64,
    extension: &str,
) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from(DEBUG_RECORD_DIR);
    fs::create_dir_all(&dir)?;

    let mut sequence = 0u32;
    loop {
        let file_name = if sequence == 0 {
            format!(
                "{label}-{app_start_ts}-{}.{}",
                std::process::id(),
                extension
            )
        } else {
            format!(
                "{label}-{app_start_ts}-{}-{sequence}.{}",
                std::process::id(),
                extension
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
        .push("grove debug bundle", "Bundle observability artifacts")
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

fn unique_debug_bundle_path(app_start_ts: u64) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from(DEBUG_RECORD_DIR);
    fs::create_dir_all(&dir)?;
    let mut sequence = 0u32;
    loop {
        let file_name = if sequence == 0 {
            format!("debug-bundle-{app_start_ts}-{}", std::process::id())
        } else {
            format!(
                "debug-bundle-{app_start_ts}-{}-{sequence}",
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

fn debug_artifact_candidate(entry_name: &str) -> bool {
    entry_name.starts_with("debug-record-")
        || entry_name.starts_with("evidence-")
        || entry_name.starts_with("render-trace-")
        || entry_name.starts_with("frame-timing-")
        || entry_name.ends_with("_payloads")
}

fn copy_debug_artifact_dir(
    src_dir: &Path,
    dst_dir: &Path,
    copied: &mut Vec<String>,
) -> std::io::Result<()> {
    fs::create_dir_all(dst_dir)?;
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst_dir.join(entry.file_name());
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            copy_debug_artifact_dir(src_path.as_path(), dst_path.as_path(), copied)?;
            continue;
        }
        if metadata.is_file() {
            fs::copy(src_path.as_path(), dst_path.as_path())?;
            copied.push(dst_path.display().to_string());
        }
    }
    Ok(())
}

fn run_debug_bundle(parsed: DebugBundleArgs) -> std::io::Result<()> {
    let source_dir = PathBuf::from(DEBUG_RECORD_DIR);
    let bundle_path = match parsed.out {
        Some(path) => path,
        None => unique_debug_bundle_path(now_millis())?,
    };
    fs::create_dir_all(bundle_path.as_path())?;

    let mut copied = Vec::new();
    if source_dir.exists() {
        for entry in fs::read_dir(source_dir.as_path())? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if !debug_artifact_candidate(name.as_str()) {
                continue;
            }
            let src_path = entry.path();
            let dst_path = bundle_path.join(name);
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                copy_debug_artifact_dir(src_path.as_path(), dst_path.as_path(), &mut copied)?;
                continue;
            }
            if metadata.is_file() {
                fs::copy(src_path.as_path(), dst_path.as_path())?;
                copied.push(dst_path.display().to_string());
            }
        }
    }

    let manifest_path = bundle_path.join("manifest.json");
    let manifest = serde_json::json!({
        "created_at_unix_ms": now_millis(),
        "source_dir": source_dir.display().to_string(),
        "bundle_path": bundle_path.display().to_string(),
        "copied_files": copied,
    });
    fs::write(
        manifest_path.as_path(),
        serde_json::to_string_pretty(&manifest)
            .map_err(|error| std::io::Error::other(error.to_string()))?,
    )?;

    emit_json(&CommandEnvelope::success(
        "grove debug bundle",
        DebugBundleResult {
            bundle_path: bundle_path.display().to_string(),
            copied,
        },
        Vec::new(),
        vec![NextAction::new(
            "grove tui --debug-record",
            "Capture a fresh debug record session",
        )],
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

fn command_error_code_from_wire_label(code: &str) -> CommandErrorCode {
    match code {
        "invalid_argument" => CommandErrorCode::InvalidArgument,
        "not_found" => CommandErrorCode::NotFound,
        "conflict" => CommandErrorCode::Conflict,
        "runtime_failure" => CommandErrorCode::RuntimeFailure,
        _ => CommandErrorCode::Internal,
    }
}

fn daemon_command_error_to_cli_code(error: &DaemonCommandError) -> CliErrorCode {
    command_error_code(
        command_error_code_from_wire_label(&error.code),
        &error.message,
    )
}

fn run_workspace_list(
    parsed: WorkspaceListArgs,
    daemon_socket: Option<&Path>,
) -> std::io::Result<()> {
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let command = "grove workspace list";
    if let Some(socket_path) = daemon_socket {
        return match workspace_list_via_socket(socket_path, &repo_root) {
            Ok(Ok(result)) => emit_json(&CommandEnvelope::success(
                command,
                result,
                Vec::new(),
                workspace_list_next_actions(),
            )),
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Verify repository path and retry",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn split_global_socket_arg(args: Vec<String>) -> std::io::Result<(Option<PathBuf>, Vec<String>)> {
    let mut args_iter = args.into_iter();
    let Some(first) = args_iter.next() else {
        return Ok((None, Vec::new()));
    };

    if first != "--socket" {
        let mut remaining = vec![first];
        remaining.extend(args_iter);
        return Ok((None, remaining));
    }

    let Some(socket_path) = args_iter.next() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--socket requires a path",
        ));
    };

    Ok((Some(PathBuf::from(socket_path)), args_iter.collect()))
}

fn run_workspace_create(
    parsed: WorkspaceCreateArgs,
    daemon_socket: Option<&Path>,
) -> std::io::Result<()> {
    let repo_root = if let Some(path) = parsed.repo {
        path
    } else {
        std::env::current_dir()?
    };
    let command = "grove workspace create";
    if let Some(socket_path) = daemon_socket {
        return match workspace_create_via_socket(
            socket_path,
            DaemonWorkspaceCreatePayload {
                repo_root: repo_root.display().to_string(),
                name: parsed.name.unwrap_or_default(),
                base_branch: parsed.base_branch,
                existing_branch: parsed.existing_branch,
                agent: parsed.agent.map(|agent| agent_label(agent).to_string()),
                start: parsed.start,
                dry_run: parsed.dry_run,
            },
        ) {
            Ok(Ok(result)) => {
                let workspace_name = result.workspace.name.clone();
                let started = daemon_workspace_status_is_running(&result.workspace.status);
                let payload = WorkspaceMutationResult {
                    workspace: workspace_view_from_daemon(result.workspace),
                    dry_run: parsed.dry_run,
                };
                emit_json(&CommandEnvelope::success(
                    command,
                    payload,
                    result.warnings,
                    after_workspace_create(&workspace_name, started),
                ))
            }
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Adjust create arguments and retry",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn run_workspace_edit(
    parsed: WorkspaceEditArgs,
    daemon_socket: Option<&Path>,
) -> std::io::Result<()> {
    let command = "grove workspace edit";
    let workspace_name = parsed.workspace.clone();
    let workspace_path = parsed.workspace_path.clone();
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
    if let Some(socket_path) = daemon_socket {
        return match workspace_edit_via_socket(
            socket_path,
            DaemonWorkspaceEditPayload {
                repo_root: repo_root.display().to_string(),
                workspace: workspace_name,
                workspace_path: workspace_path.map(|path| path.display().to_string()),
                agent: parsed.agent.map(|agent| agent_label(agent).to_string()),
                base_branch: parsed.base_branch,
            },
        ) {
            Ok(Ok(result)) => {
                let workspace_name = result.workspace.name.clone();
                let payload = WorkspaceMutationResult {
                    workspace: workspace_view_from_daemon(result.workspace),
                    dry_run: false,
                };
                emit_json(&CommandEnvelope::success(
                    command,
                    payload,
                    result.warnings,
                    workspace_edit_next_actions(&workspace_name),
                ))
            }
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Adjust selector and edit flags, then retry",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn run_workspace_delete(
    parsed: WorkspaceDeleteArgs,
    daemon_socket: Option<&Path>,
) -> std::io::Result<()> {
    let command = "grove workspace delete";
    let workspace_name = parsed.workspace.clone();
    let workspace_path = parsed.workspace_path.clone();
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
    if let Some(socket_path) = daemon_socket {
        return match workspace_delete_via_socket(
            socket_path,
            DaemonWorkspaceDeletePayload {
                repo_root: repo_root.display().to_string(),
                workspace: workspace_name,
                workspace_path: workspace_path.map(|path| path.display().to_string()),
                delete_branch: parsed.delete_branch,
                force_stop: parsed.force_stop,
                dry_run: parsed.dry_run,
            },
        ) {
            Ok(Ok(result)) => {
                let payload = WorkspaceMutationResult {
                    workspace: workspace_view_from_daemon(result.workspace),
                    dry_run: parsed.dry_run,
                };
                emit_json(&CommandEnvelope::success(
                    command,
                    payload,
                    result.warnings,
                    workspace_delete_next_actions(),
                ))
            }
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Adjust selector/delete flags, then retry",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn run_workspace_merge(
    parsed: WorkspaceMergeArgs,
    daemon_socket: Option<&Path>,
) -> std::io::Result<()> {
    let command = "grove workspace merge";
    let workspace_name = parsed.workspace.clone();
    let workspace_path = parsed.workspace_path.clone();
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
    if let Some(socket_path) = daemon_socket {
        return match workspace_merge_via_socket(
            socket_path,
            DaemonWorkspaceMergePayload {
                repo_root: repo_root.display().to_string(),
                workspace: workspace_name,
                workspace_path: workspace_path.map(|path| path.display().to_string()),
                cleanup_workspace: parsed.cleanup_workspace,
                cleanup_branch: parsed.cleanup_branch,
                dry_run: parsed.dry_run,
            },
        ) {
            Ok(Ok(result)) => {
                let workspace_name = result.workspace.name.clone();
                let payload = WorkspaceMutationResult {
                    workspace: workspace_view_from_daemon(result.workspace),
                    dry_run: parsed.dry_run,
                };
                emit_json(&CommandEnvelope::success(
                    command,
                    payload,
                    result.warnings,
                    after_workspace_merge(&workspace_name),
                ))
            }
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Resolve workspace/base branch state and retry merge",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn run_workspace_update(
    parsed: WorkspaceUpdateArgs,
    daemon_socket: Option<&Path>,
) -> std::io::Result<()> {
    let command = "grove workspace update";
    let workspace_name = parsed.workspace.clone();
    let workspace_path = parsed.workspace_path.clone();
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
    if let Some(socket_path) = daemon_socket {
        return match workspace_update_via_socket(
            socket_path,
            DaemonWorkspaceUpdatePayload {
                repo_root: repo_root.display().to_string(),
                workspace: workspace_name,
                workspace_path: workspace_path.map(|path| path.display().to_string()),
                dry_run: parsed.dry_run,
            },
        ) {
            Ok(Ok(result)) => {
                let workspace_name = result.workspace.name.clone();
                let payload = WorkspaceMutationResult {
                    workspace: workspace_view_from_daemon(result.workspace),
                    dry_run: parsed.dry_run,
                };
                emit_json(&CommandEnvelope::success(
                    command,
                    payload,
                    result.warnings,
                    workspace_update_next_actions(&workspace_name),
                ))
            }
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Resolve workspace/base branch state and retry update",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn run_agent_start(parsed: AgentStartArgs, daemon_socket: Option<&Path>) -> std::io::Result<()> {
    let command = "grove agent start";
    let workspace_name = parsed.workspace.clone();
    let workspace_path = parsed.workspace_path.clone();
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
    if let Some(socket_path) = daemon_socket {
        return match agent_start_via_socket(
            socket_path,
            DaemonAgentStartPayload {
                repo_root: repo_root.display().to_string(),
                workspace: workspace_name,
                workspace_path: workspace_path.map(|path| path.display().to_string()),
                prompt: parsed.prompt,
                pre_launch_command: parsed.pre_launch_command,
                skip_permissions: parsed.skip_permissions,
                dry_run: parsed.dry_run,
                capture_cols: None,
                capture_rows: None,
            },
        ) {
            Ok(Ok(result)) => {
                let workspace_name = result.workspace.name.clone();
                let payload = WorkspaceMutationResult {
                    workspace: workspace_view_from_daemon(result.workspace),
                    dry_run: parsed.dry_run,
                };
                emit_json(&CommandEnvelope::success(
                    command,
                    payload,
                    result.warnings,
                    agent_start_next_actions(&workspace_name),
                ))
            }
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Resolve workspace/runtime state and retry start",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn run_agent_stop(parsed: AgentStopArgs, daemon_socket: Option<&Path>) -> std::io::Result<()> {
    let command = "grove agent stop";
    let workspace_name = parsed.workspace.clone();
    let workspace_path = parsed.workspace_path.clone();
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
    if let Some(socket_path) = daemon_socket {
        return match agent_stop_via_socket(
            socket_path,
            DaemonAgentStopPayload {
                repo_root: repo_root.display().to_string(),
                workspace: workspace_name,
                workspace_path: workspace_path.map(|path| path.display().to_string()),
                dry_run: parsed.dry_run,
            },
        ) {
            Ok(Ok(result)) => {
                let workspace_name = result.workspace.name.clone();
                let payload = WorkspaceMutationResult {
                    workspace: workspace_view_from_daemon(result.workspace),
                    dry_run: parsed.dry_run,
                };
                emit_json(&CommandEnvelope::success(
                    command,
                    payload,
                    result.warnings,
                    after_agent_stop(&workspace_name),
                ))
            }
            Ok(Err(error)) => emit_error(
                command,
                daemon_command_error_to_cli_code(&error),
                error.message,
                "Resolve workspace/runtime state and retry stop",
            ),
            Err(error) => emit_error(
                command,
                CliErrorCode::IoError,
                format!("daemon request failed: {error}"),
                "Verify daemon socket path and daemon availability, then retry",
            ),
        };
    }

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

fn daemon_workspace_status_is_running(status: &str) -> bool {
    matches!(status, "active" | "thinking" | "waiting")
}

fn run_tui(cli: TuiArgs, daemon_socket_path: Option<&Path>) -> std::io::Result<()> {
    let TuiArgs {
        event_log_path: cli_event_log_path,
        debug_record,
        evidence_log_path: cli_evidence_log_path,
        render_trace_path: cli_render_trace_path,
        frame_timing_log_path: cli_frame_timing_log_path,
    } = cli;
    let app_start_ts = now_millis();
    let debug_record_path = if debug_record {
        Some(debug_record_path(app_start_ts)?)
    } else {
        None
    };
    if let Some(path) = debug_record_path.as_ref() {
        eprintln!("grove debug record: {}", path.display());
    }
    let event_log_path = debug_record_path.or(cli_event_log_path.map(resolve_event_log_path));
    let evidence_log_path = match cli_evidence_log_path {
        Some(path) => Some(resolve_event_log_path(path)),
        None => {
            if debug_record {
                Some(unique_debug_artifact_path(
                    "evidence",
                    app_start_ts,
                    "jsonl",
                )?)
            } else {
                None
            }
        }
    };
    let render_trace_path = match cli_render_trace_path {
        Some(path) => Some(resolve_event_log_path(path)),
        None => {
            if debug_record {
                Some(unique_debug_artifact_path(
                    "render-trace",
                    app_start_ts,
                    "jsonl",
                )?)
            } else {
                None
            }
        }
    };
    let frame_timing_log_path = match cli_frame_timing_log_path {
        Some(path) => Some(resolve_event_log_path(path)),
        None => {
            if debug_record {
                Some(unique_debug_artifact_path(
                    "frame-timing",
                    app_start_ts,
                    "jsonl",
                )?)
            } else {
                None
            }
        }
    };
    if let Some(path) = event_log_path.as_ref() {
        ensure_event_log_parent_directory(path)?;
    }
    if let Some(path) = evidence_log_path.as_ref() {
        ensure_event_log_parent_directory(path)?;
    }
    if let Some(path) = render_trace_path.as_ref() {
        ensure_event_log_parent_directory(path)?;
    }
    if let Some(path) = frame_timing_log_path.as_ref() {
        ensure_event_log_parent_directory(path)?;
    }
    if debug_record {
        if let Some(path) = evidence_log_path.as_ref() {
            eprintln!("grove evidence log: {}", path.display());
        }
        if let Some(path) = render_trace_path.as_ref() {
            eprintln!("grove render trace: {}", path.display());
        }
        if let Some(path) = frame_timing_log_path.as_ref() {
            eprintln!("grove frame timing: {}", path.display());
        }
    }
    set_daemon_client_log_path(event_log_path.clone());
    let observability_paths = crate::interface::tui::RuntimeObservabilityPaths {
        evidence_log_path,
        render_trace_path,
        frame_timing_log_path,
    };

    if debug_record && let Some(path) = event_log_path {
        return crate::interface::tui::run_with_debug_record(
            path,
            app_start_ts,
            daemon_socket_path.map(Path::to_path_buf),
            observability_paths,
        );
    }

    crate::interface::tui::run_with_event_log(
        event_log_path,
        daemon_socket_path.map(Path::to_path_buf),
        observability_paths,
    )
}

pub fn run(args: impl IntoIterator<Item = String>) -> std::io::Result<()> {
    let raw_args = args.into_iter().collect::<Vec<String>>();
    let (daemon_socket, args) = split_global_socket_arg(raw_args)?;
    let Some((first, remaining)) = args.split_first() else {
        return emit_json(&root_command_envelope());
    };
    let daemon_socket_path = daemon_socket.as_deref();

    if first == "tui" {
        return run_tui(
            parse_tui_args(remaining.iter().cloned())?,
            daemon_socket_path,
        );
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
                Ok(parsed) => run_workspace_list(parsed, daemon_socket_path),
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
                Ok(parsed) => run_workspace_create(parsed, daemon_socket_path),
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
                Ok(parsed) => run_workspace_edit(parsed, daemon_socket_path),
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
                Ok(parsed) => run_workspace_delete(parsed, daemon_socket_path),
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
                Ok(parsed) => run_workspace_merge(parsed, daemon_socket_path),
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
                Ok(parsed) => run_workspace_update(parsed, daemon_socket_path),
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
                Ok(parsed) => run_agent_start(parsed, daemon_socket_path),
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
                Ok(parsed) => run_agent_stop(parsed, daemon_socket_path),
                Err(error) => emit_error(
                    "grove agent stop",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with selector flags and optional '--dry-run'",
                ),
            };
        }
    }
    if first == "debug" {
        if daemon_socket_path.is_some() {
            return emit_error(
                "grove debug",
                CliErrorCode::InvalidArgument,
                "--socket is not supported for debug commands".to_string(),
                "Retry without '--socket'",
            );
        }
        let Some((debug_command, debug_args)) = remaining.split_first() else {
            return emit_error(
                "grove debug",
                CliErrorCode::InvalidArgument,
                "debug subcommand is required".to_string(),
                "Use 'grove debug bundle'",
            );
        };
        if debug_command == "bundle" {
            return match parse_debug_bundle_args(debug_args.iter().cloned()) {
                Ok(parsed) => run_debug_bundle(parsed),
                Err(error) => emit_error(
                    "grove debug bundle",
                    CliErrorCode::InvalidArgument,
                    error.to_string(),
                    "Retry with optional '--out <path>'",
                ),
            };
        }
    }

    if daemon_socket_path.is_some() {
        return emit_error(
            "grove",
            CliErrorCode::InvalidArgument,
            "--socket currently supports lifecycle and tui commands only".to_string(),
            "Retry with 'grove --socket <path> tui' or '--socket <path> workspace|agent ...'",
        );
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
        resolve_event_log_path, root_command_envelope, split_global_socket_arg, workspace_selector,
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
                evidence_log_path: None,
                render_trace_path: None,
                frame_timing_log_path: None,
            }
        );
    }

    #[test]
    fn tui_parser_reads_forensic_paths() {
        let parsed = parse_tui_args(vec![
            "--evidence-log".to_string(),
            ".grove/evidence.jsonl".to_string(),
            "--render-trace".to_string(),
            ".grove/trace.jsonl".to_string(),
            "--frame-timing-log".to_string(),
            ".grove/frame-timing.jsonl".to_string(),
        ])
        .expect("tui args should parse");
        assert_eq!(
            parsed.evidence_log_path,
            Some(PathBuf::from(".grove/evidence.jsonl"))
        );
        assert_eq!(
            parsed.render_trace_path,
            Some(PathBuf::from(".grove/trace.jsonl"))
        );
        assert_eq!(
            parsed.frame_timing_log_path,
            Some(PathBuf::from(".grove/frame-timing.jsonl"))
        );
    }

    #[test]
    fn debug_bundle_parser_reads_out_path() {
        let parsed = super::parse_debug_bundle_args(vec![
            "--out".to_string(),
            ".grove/custom-bundle".to_string(),
        ])
        .expect("debug bundle args should parse");
        assert_eq!(
            parsed,
            super::DebugBundleArgs {
                out: Some(PathBuf::from(".grove/custom-bundle")),
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
    fn split_global_socket_arg_reads_socket_and_remaining_args() {
        let (socket_path, remaining) = split_global_socket_arg(vec![
            "--socket".to_string(),
            "/tmp/groved.sock".to_string(),
            "workspace".to_string(),
            "list".to_string(),
        ])
        .expect("global socket args should parse");

        assert_eq!(socket_path, Some(PathBuf::from("/tmp/groved.sock")));
        assert_eq!(remaining, vec!["workspace".to_string(), "list".to_string()]);
    }

    #[test]
    fn split_global_socket_arg_rejects_missing_socket_path() {
        let error = split_global_socket_arg(vec!["--socket".to_string()])
            .expect_err("missing socket path should fail");
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

use std::path::PathBuf;
use std::time::Duration;

use crate::domain::{AgentType, Workspace};

mod agents;
pub mod capture;
pub mod execution;
pub mod facade;
pub mod launch_plan;
pub mod polling;
pub mod reconciliation;
pub mod restart;
mod sessions;
pub mod status;

pub(crate) use capture::evaluate_capture_change;
#[cfg(test)]
use capture::strip_mouse_fragments;
pub use capture::tmux_capture_error_indicates_missing_session;
pub use execution::{
    CommandExecutionMode, CommandExecutor, DelegatingCommandExecutor, ProcessCommandExecutor,
    execute_command_with, execute_commands_for_mode, execute_commands_with,
    execute_commands_with_executor, execute_launch_plan_for_mode, execute_launch_plan_with,
    execute_launch_plan_with_executor, execute_launch_request_with_result_for_mode,
    execute_shell_launch_request_for_mode, execute_stop_workspace_with_result_for_mode,
    kill_workspace_session_command, kill_workspace_session_commands,
};
#[cfg(test)]
pub use execution::{execute_commands, execute_launch_plan};
#[cfg(test)]
use launch_plan::default_agent_command;
pub(crate) use launch_plan::trimmed_nonempty;
pub use launch_plan::{
    build_launch_plan, build_shell_launch_plan, launch_request_for_workspace,
    shell_launch_request_for_workspace, stop_plan, tmux_launch_error_indicates_duplicate_session,
};
pub use polling::{
    poll_interval, workspace_should_poll_status, workspace_status_session_target,
    workspace_status_targets_for_polling, workspace_status_targets_for_polling_with_live_preview,
};
pub use reconciliation::reconcile_with_sessions;
#[cfg(test)]
use restart::restart_workspace_in_pane_with_io_in_home;
pub use restart::{
    agent_supports_in_pane_restart, execute_restart_workspace_in_pane_with_result,
    extract_agent_resume_command, infer_workspace_skip_permissions,
    restart_workspace_in_pane_with_io,
};
#[cfg(test)]
use sessions::sanitize_workspace_name;
#[cfg(test)]
pub use sessions::session_name_for_workspace;
pub use sessions::{
    git_preview_session_if_ready, git_session_name_for_workspace, live_preview_agent_session,
    live_preview_capture_target_for_tab, live_preview_session_for_tab,
    session_name_for_workspace_in_project, session_name_for_workspace_ref,
    shell_session_name_for_workspace, workspace_can_enter_interactive, workspace_can_start_agent,
    workspace_can_stop_agent, workspace_session_for_preview_tab,
};
#[cfg(test)]
use status::{
    StatusOverrideContext, claude_project_dir_name, codex_session_skip_permissions_mode,
    detect_agent_session_status_in_home, detect_status_with_session_override_in_home,
    infer_claude_skip_permissions_in_home, infer_codex_skip_permissions_in_home,
    infer_opencode_skip_permissions_in_home, latest_claude_assistant_attention_marker_in_home,
    latest_codex_assistant_attention_marker_in_home,
    latest_opencode_assistant_attention_marker_in_home,
};
#[cfg(test)]
use status::{detect_status, detect_waiting_prompt};

pub const TMUX_SESSION_PREFIX: &str = "grove-ws-";
const GROVE_LAUNCHER_SCRIPT_PATH: &str = ".grove/start.sh";
const WAITING_PATTERNS: [&str; 9] = [
    "[y/n]",
    "(y/n)",
    "allow edit",
    "allow bash",
    "press enter",
    "continue?",
    "do you want",
    "approve",
    "confirm",
];
const WAITING_TAIL_LINES: usize = 8;
const STATUS_TAIL_LINES: usize = 60;
const SESSION_STATUS_TAIL_BYTES: usize = 256 * 1024;
const SESSION_ACTIVITY_THRESHOLD: Duration = Duration::from_secs(30);
const CODEX_SESSION_LOOKUP_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const OPENCODE_SESSION_LOOKUP_REFRESH_INTERVAL: Duration = Duration::from_millis(500);
const RESTART_RESUME_SCROLLBACK_LINES: usize = 240;
const RESTART_RESUME_CAPTURE_ATTEMPTS: usize = 30;
const RESTART_RESUME_ERROR_TAIL_LINES: usize = 8;
const RESTART_RESUME_ERROR_MAX_CHARS: usize = 320;
#[cfg(test)]
const RESTART_RESUME_RETRY_DELAY: Duration = Duration::from_millis(0);
#[cfg(not(test))]
const RESTART_RESUME_RETRY_DELAY: Duration = Duration::from_millis(120);
const OPENCODE_UNSAFE_PERMISSION_JSON: &str = r#"{"*":"allow"}"#;
const DONE_PATTERNS: [&str; 5] = [
    "task completed",
    "all done",
    "finished",
    "exited with code 0",
    "goodbye",
];
const ERROR_PATTERNS: [&str; 6] = [
    "error:",
    "failed",
    "exited with code 1",
    "panic:",
    "exception:",
    "traceback",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionActivity {
    Idle,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub project_name: Option<String>,
    pub workspace_name: String,
    pub workspace_path: PathBuf,
    pub agent: AgentType,
    pub prompt: Option<String>,
    pub workspace_init_command: Option<String>,
    pub skip_permissions: bool,
    pub agent_env: Vec<(String, String)>,
    pub capture_cols: Option<u16>,
    pub capture_rows: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherScript {
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub session_name: String,
    pub pane_lookup_cmd: Vec<String>,
    pub pre_launch_cmds: Vec<Vec<String>>,
    pub launch_cmd: Vec<String>,
    pub launcher_script: Option<LauncherScript>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionExecutionResult {
    pub workspace_name: String,
    pub workspace_path: PathBuf,
    pub session_name: String,
    pub result: Result<(), String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellLaunchRequest {
    pub session_name: String,
    pub workspace_path: PathBuf,
    pub command: String,
    pub workspace_init_command: Option<String>,
    pub capture_cols: Option<u16>,
    pub capture_rows: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationResult {
    pub workspaces: Vec<Workspace>,
    pub orphaned_sessions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceStatusTarget {
    pub workspace_name: String,
    pub workspace_path: PathBuf,
    pub session_name: String,
    pub supported_agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePreviewTarget {
    pub session_name: String,
    pub include_escape_sequences: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputDigest {
    pub raw_hash: u64,
    pub raw_len: usize,
    pub cleaned_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureChange {
    pub digest: OutputDigest,
    pub changed_raw: bool,
    pub changed_cleaned: bool,
    pub cleaned_output: String,
    pub render_output: String,
}

pub(super) enum RestartExitInput {
    Literal(&'static str),
    Named(&'static str),
}

#[cfg(test)]
mod tests;

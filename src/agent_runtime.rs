use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::config::MultiplexerKind;
use crate::domain::{AgentType, Workspace, WorkspaceStatus};

pub const TMUX_SESSION_PREFIX: &str = "grove-ws-";
pub(crate) const ZELLIJ_CAPTURE_COLS: u16 = 120;
pub(crate) const ZELLIJ_CAPTURE_ROWS: u16 = 40;
const DEFAULT_GROVE_ZELLIJ_CONFIG: &str = "show_startup_tips false\nshow_release_notes false\n";
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
const SESSION_STATUS_TAIL_BYTES: usize = 2 * 1024 * 1024;
const SESSION_ACTIVITY_THRESHOLD: Duration = Duration::from_secs(30);
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
    pub pre_launch_command: Option<String>,
    pub skip_permissions: bool,
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
pub struct ShellLaunchRequest {
    pub session_name: String,
    pub workspace_path: PathBuf,
    pub command: String,
    pub capture_cols: Option<u16>,
    pub capture_rows: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationResult {
    pub workspaces: Vec<Workspace>,
    pub orphaned_sessions: Vec<String>,
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

pub(crate) fn sanitize_workspace_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for character in name.chars() {
        let mapped = if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            character
        } else {
            '-'
        };

        if mapped == '-' {
            if !last_dash {
                out.push('-');
            }
            last_dash = true;
            continue;
        }

        out.push(mapped);
        last_dash = false;
    }

    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        return "workspace".to_string();
    }

    trimmed.to_string()
}

pub fn session_name_for_workspace(workspace_name: &str) -> String {
    session_name_for_workspace_in_project(None, workspace_name)
}

pub fn session_name_for_workspace_in_project(
    project_name: Option<&str>,
    workspace_name: &str,
) -> String {
    if let Some(project_name) = project_name {
        let project = sanitize_workspace_name(project_name);
        return format!(
            "{TMUX_SESSION_PREFIX}{project}-{}",
            sanitize_workspace_name(workspace_name)
        );
    }

    format!(
        "{TMUX_SESSION_PREFIX}{}",
        sanitize_workspace_name(workspace_name)
    )
}

fn default_zellij_capture_directory() -> PathBuf {
    if let Some(path) = dirs::cache_dir() {
        return path.join("grove").join("zellij");
    }

    if let Some(path) = dirs::home_dir() {
        return path.join(".cache").join("grove").join("zellij");
    }

    PathBuf::from(".grove").join("zellij")
}

fn default_grove_config_directory() -> PathBuf {
    if let Some(path) = dirs::config_dir() {
        return path.join("grove");
    }

    if let Some(path) = dirs::home_dir() {
        return path.join(".config").join("grove");
    }

    PathBuf::from(".grove")
}

pub(crate) fn zellij_config_path() -> PathBuf {
    default_grove_config_directory().join("zellij.kdl")
}

fn zellij_capture_log_path_in(base_directory: &Path, session_name: &str) -> PathBuf {
    base_directory.join(format!("{session_name}.ansi.log"))
}

pub fn zellij_capture_log_path(session_name: &str) -> PathBuf {
    zellij_capture_log_path_in(&default_zellij_capture_directory(), session_name)
}

pub fn build_launch_plan(request: &LaunchRequest, multiplexer: MultiplexerKind) -> LaunchPlan {
    let session_name = session_name_for_workspace_in_project(
        request.project_name.as_deref(),
        &request.workspace_name,
    );
    let agent_cmd = build_agent_command(request.agent, request.skip_permissions);
    let pre_launch_command = normalized_pre_launch_command(request.pre_launch_command.as_deref());
    let launch_agent_cmd =
        launch_command_with_pre_launch(&agent_cmd, pre_launch_command.as_deref());

    match multiplexer {
        MultiplexerKind::Tmux => tmux_launch_plan(request, session_name, launch_agent_cmd),
        MultiplexerKind::Zellij => zellij_launch_plan(request, session_name, launch_agent_cmd),
    }
}

pub fn build_shell_launch_plan(
    request: &ShellLaunchRequest,
    multiplexer: MultiplexerKind,
) -> LaunchPlan {
    let shared = LaunchRequest {
        project_name: None,
        workspace_name: request.session_name.clone(),
        workspace_path: request.workspace_path.clone(),
        agent: AgentType::Codex,
        prompt: None,
        pre_launch_command: None,
        skip_permissions: false,
        capture_cols: request.capture_cols,
        capture_rows: request.capture_rows,
    };

    match multiplexer {
        MultiplexerKind::Tmux => tmux_launch_plan(
            &shared,
            request.session_name.clone(),
            request.command.clone(),
        ),
        MultiplexerKind::Zellij => zellij_launch_plan(
            &shared,
            request.session_name.clone(),
            request.command.clone(),
        ),
    }
}

fn tmux_launch_plan(
    request: &LaunchRequest,
    session_name: String,
    launch_agent_cmd: String,
) -> LaunchPlan {
    let session_target = session_name.clone();
    let pre_launch_cmds = vec![
        vec![
            "tmux".to_string(),
            "new-session".to_string(),
            "-d".to_string(),
            "-s".to_string(),
            session_name.clone(),
            "-c".to_string(),
            request.workspace_path.to_string_lossy().to_string(),
        ],
        vec![
            "tmux".to_string(),
            "set-option".to_string(),
            "-t".to_string(),
            session_name.clone(),
            "history-limit".to_string(),
            "10000".to_string(),
        ],
    ];
    let pane_lookup_cmd = vec![
        "tmux".to_string(),
        "list-panes".to_string(),
        "-t".to_string(),
        session_name.clone(),
        "-F".to_string(),
        "#{pane_id}".to_string(),
    ];

    match &request.prompt {
        None => LaunchPlan {
            session_name,
            pane_lookup_cmd,
            pre_launch_cmds,
            launch_cmd: vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                session_target,
                launch_agent_cmd,
                "Enter".to_string(),
            ],
            launcher_script: None,
        },
        Some(prompt) => {
            let launcher_path = request.workspace_path.join(".grove-start.sh");
            let launcher_contents =
                build_launcher_script(&launch_agent_cmd, prompt, &launcher_path);
            LaunchPlan {
                session_name,
                pane_lookup_cmd,
                pre_launch_cmds,
                launch_cmd: vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    session_target,
                    format!("bash {}", launcher_path.to_string_lossy()),
                    "Enter".to_string(),
                ],
                launcher_script: Some(LauncherScript {
                    path: launcher_path,
                    contents: launcher_contents,
                }),
            }
        }
    }
}

fn zellij_launch_plan(
    request: &LaunchRequest,
    session_name: String,
    launch_agent_cmd: String,
) -> LaunchPlan {
    let capture_cols = request
        .capture_cols
        .filter(|value| *value > 0)
        .unwrap_or(ZELLIJ_CAPTURE_COLS);
    let capture_rows = request
        .capture_rows
        .filter(|value| *value > 0)
        .unwrap_or(ZELLIJ_CAPTURE_ROWS);

    fn zellij_script_capture_command(
        command: &str,
        capture_log_path_text: &str,
        capture_cols: u16,
        capture_rows: u16,
    ) -> String {
        format!(
            "stty cols {cols} rows {rows}; export COLUMNS={cols} LINES={rows} TERM=xterm-256color COLORTERM=truecolor; unset NO_COLOR; script -qefc {} {}",
            shell_single_quote(command),
            shell_single_quote(capture_log_path_text),
            cols = capture_cols,
            rows = capture_rows,
        )
    }

    let capture_log_path = zellij_capture_log_path(&session_name);
    let capture_log_path_text = capture_log_path.to_string_lossy().to_string();
    let capture_log_directory_text = capture_log_path.parent().map_or_else(
        || ".".to_string(),
        |path| path.to_string_lossy().to_string(),
    );
    let zellij_config_path = zellij_config_path();
    let zellij_config_path_text = zellij_config_path.to_string_lossy().to_string();
    let zellij_config_directory_text = zellij_config_path.parent().map_or_else(
        || ".".to_string(),
        |path| path.to_string_lossy().to_string(),
    );
    let pre_launch_cmds = vec![
        vec![
            "sh".to_string(),
            "-lc".to_string(),
            format!(
                "mkdir -p {config_dir} && if [ ! -f {config_file} ]; then printf '%s\\n' {config_lines} > {config_file}; fi",
                config_dir = shell_single_quote(&zellij_config_directory_text),
                config_file = shell_single_quote(&zellij_config_path_text),
                config_lines = shell_single_quote(DEFAULT_GROVE_ZELLIJ_CONFIG.trim_end()),
            ),
        ],
        vec![
            "sh".to_string(),
            "-lc".to_string(),
            format!(
                "zellij --config {config} kill-session {session} >/dev/null 2>&1 || true",
                config = shell_single_quote(&zellij_config_path_text),
                session = shell_single_quote(&session_name),
            ),
        ],
        vec![
            "sh".to_string(),
            "-lc".to_string(),
            format!(
                "mkdir -p {capture_dir} && : > {capture_file}",
                capture_dir = shell_single_quote(&capture_log_directory_text),
                capture_file = shell_single_quote(&capture_log_path_text),
            ),
        ],
        vec![
            "sh".to_string(),
            "-lc".to_string(),
            format!(
                "zellij --config {config} attach {session} --create --create-background >/dev/null 2>&1 || true",
                config = shell_single_quote(&zellij_config_path_text),
                session = shell_single_quote(&session_name),
            ),
        ],
        vec![
            "sh".to_string(),
            "-lc".to_string(),
            format!(
                "nohup script -q /dev/null -c \"stty cols {cols} rows {rows}; export COLUMNS={cols} LINES={rows} TERM=xterm-256color COLORTERM=truecolor; unset NO_COLOR; zellij --config {config} attach {session}\" >/dev/null 2>&1 &",
                cols = capture_cols,
                rows = capture_rows,
                config = shell_single_quote(&zellij_config_path_text),
                session = session_name,
            ),
        ],
        vec!["sh".to_string(), "-lc".to_string(), "sleep 1".to_string()],
    ];

    match &request.prompt {
        None => LaunchPlan {
            session_name: session_name.clone(),
            pane_lookup_cmd: Vec::new(),
            pre_launch_cmds,
            launch_cmd: vec![
                "zellij".to_string(),
                "--config".to_string(),
                zellij_config_path_text.clone(),
                "--session".to_string(),
                session_name,
                "run".to_string(),
                "--floating".to_string(),
                "--width".to_string(),
                "100%".to_string(),
                "--height".to_string(),
                "100%".to_string(),
                "--x".to_string(),
                "0".to_string(),
                "--y".to_string(),
                "0".to_string(),
                "--cwd".to_string(),
                request.workspace_path.to_string_lossy().to_string(),
                "--".to_string(),
                "bash".to_string(),
                "-lc".to_string(),
                zellij_script_capture_command(
                    &launch_agent_cmd,
                    &capture_log_path_text,
                    capture_cols,
                    capture_rows,
                ),
            ],
            launcher_script: None,
        },
        Some(prompt) => {
            let launcher_path = request.workspace_path.join(".grove-start.sh");
            let launcher_contents =
                build_launcher_script(&launch_agent_cmd, prompt, &launcher_path);
            let launcher_exec = format!(
                "bash {}",
                shell_single_quote(&launcher_path.to_string_lossy())
            );
            LaunchPlan {
                session_name: session_name.clone(),
                pane_lookup_cmd: Vec::new(),
                pre_launch_cmds,
                launch_cmd: vec![
                    "zellij".to_string(),
                    "--config".to_string(),
                    zellij_config_path_text,
                    "--session".to_string(),
                    session_name,
                    "run".to_string(),
                    "--floating".to_string(),
                    "--width".to_string(),
                    "100%".to_string(),
                    "--height".to_string(),
                    "100%".to_string(),
                    "--x".to_string(),
                    "0".to_string(),
                    "--y".to_string(),
                    "0".to_string(),
                    "--cwd".to_string(),
                    request.workspace_path.to_string_lossy().to_string(),
                    "--".to_string(),
                    "bash".to_string(),
                    "-lc".to_string(),
                    zellij_script_capture_command(
                        &launcher_exec,
                        &capture_log_path_text,
                        capture_cols,
                        capture_rows,
                    ),
                ],
                launcher_script: Some(LauncherScript {
                    path: launcher_path,
                    contents: launcher_contents,
                }),
            }
        }
    }
}

fn shell_single_quote(value: &str) -> String {
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

pub fn stop_plan(session_name: &str, multiplexer: MultiplexerKind) -> Vec<Vec<String>> {
    match multiplexer {
        MultiplexerKind::Tmux => vec![
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                "C-c".to_string(),
            ],
            vec![
                "tmux".to_string(),
                "kill-session".to_string(),
                "-t".to_string(),
                session_name.to_string(),
            ],
        ],
        MultiplexerKind::Zellij => vec![
            vec![
                "zellij".to_string(),
                "--config".to_string(),
                zellij_config_path().to_string_lossy().to_string(),
                "--session".to_string(),
                session_name.to_string(),
                "action".to_string(),
                "write".to_string(),
                "3".to_string(),
            ],
            vec![
                "zellij".to_string(),
                "--config".to_string(),
                zellij_config_path().to_string_lossy().to_string(),
                "kill-session".to_string(),
                session_name.to_string(),
            ],
        ],
    }
}

pub(crate) fn build_agent_command(agent: AgentType, skip_permissions: bool) -> String {
    if let Some(command_override) = env_agent_command_override(agent) {
        return command_override;
    }

    default_agent_command(agent, skip_permissions)
}

fn default_agent_command(agent: AgentType, skip_permissions: bool) -> String {
    match (agent, skip_permissions) {
        (AgentType::Claude, true) => "claude --dangerously-skip-permissions".to_string(),
        (AgentType::Claude, false) => "claude".to_string(),
        (AgentType::Codex, true) => "codex --dangerously-bypass-approvals-and-sandbox".to_string(),
        (AgentType::Codex, false) => "codex".to_string(),
    }
}

fn env_agent_command_override(agent: AgentType) -> Option<String> {
    let variable = match agent {
        AgentType::Claude => "GROVE_CLAUDE_CMD",
        AgentType::Codex => "GROVE_CODEX_CMD",
    };
    let override_value = std::env::var(variable).ok()?;
    normalized_agent_command_override(&override_value)
}

fn normalized_agent_command_override(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn normalized_pre_launch_command(value: Option<&str>) -> Option<String> {
    let raw = value?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn launch_command_with_pre_launch(agent_command: &str, pre_launch_command: Option<&str>) -> String {
    match pre_launch_command {
        Some(pre_launch) => format!("{pre_launch} && {agent_command}"),
        None => agent_command.to_string(),
    }
}

pub(crate) fn detect_waiting_prompt(output: &str) -> Option<String> {
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(WAITING_TAIL_LINES);
    let tail_lines = &lines[start..];

    for line in tail_lines {
        let lower = line.to_ascii_lowercase();
        if WAITING_PATTERNS
            .iter()
            .any(|pattern| lower.contains(pattern))
        {
            return Some(line.trim().to_string());
        }
    }

    for line in tail_lines.iter().rev() {
        if line.to_ascii_lowercase().contains("for shortcuts") {
            return Some(line.trim().to_string());
        }
    }

    if let Some(last_non_empty) = tail_lines.iter().rev().find(|line| !line.trim().is_empty()) {
        let trimmed = last_non_empty.trim_start();
        let prefix = trimmed.chars().next()?;
        if matches!(prefix, '>' | '›' | '❯' | '»') {
            return Some(trimmed.to_string());
        }
    }

    None
}

pub(crate) fn detect_status(
    output: &str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
) -> WorkspaceStatus {
    if is_main && !has_live_session {
        return WorkspaceStatus::Main;
    }

    if !supported_agent {
        return WorkspaceStatus::Unsupported;
    }

    if !has_live_session {
        return WorkspaceStatus::Idle;
    }

    if detect_waiting_prompt(output).is_some() {
        return WorkspaceStatus::Waiting;
    }

    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(STATUS_TAIL_LINES);
    let tail_text = lines[start..].join("\n");
    let tail_lower = tail_text.to_ascii_lowercase();

    if has_unclosed_tag(&tail_lower, "<thinking>", "</thinking>")
        || has_unclosed_tag(&tail_lower, "<internal_monologue>", "</internal_monologue>")
        || tail_lower.contains("thinking...")
        || tail_lower.contains("reasoning about")
    {
        return WorkspaceStatus::Thinking;
    }

    if DONE_PATTERNS
        .iter()
        .any(|pattern| tail_lower.contains(pattern))
    {
        return WorkspaceStatus::Done;
    }

    if ERROR_PATTERNS
        .iter()
        .any(|pattern| tail_lower.contains(pattern))
    {
        return WorkspaceStatus::Error;
    }

    match session_activity {
        SessionActivity::Active => WorkspaceStatus::Active,
        SessionActivity::Idle => WorkspaceStatus::Idle,
    }
}

pub(crate) fn detect_status_with_session_override(
    output: &str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
    agent: AgentType,
    workspace_path: &Path,
) -> WorkspaceStatus {
    let home_dir = dirs::home_dir();
    detect_status_with_session_override_in_home(StatusOverrideContext {
        output,
        session_activity,
        is_main,
        has_live_session,
        supported_agent,
        agent,
        workspace_path,
        home_dir: home_dir.as_deref(),
        activity_threshold: SESSION_ACTIVITY_THRESHOLD,
    })
}

struct StatusOverrideContext<'a> {
    output: &'a str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
    agent: AgentType,
    workspace_path: &'a Path,
    home_dir: Option<&'a Path>,
    activity_threshold: Duration,
}

fn detect_status_with_session_override_in_home(
    context: StatusOverrideContext<'_>,
) -> WorkspaceStatus {
    let detected = detect_status(
        context.output,
        context.session_activity,
        context.is_main,
        context.has_live_session,
        context.supported_agent,
    );
    if !matches!(detected, WorkspaceStatus::Active | WorkspaceStatus::Waiting) {
        return detected;
    }

    let Some(home_dir) = context.home_dir else {
        return detected;
    };
    if !context.workspace_path.exists() {
        return detected;
    }

    detect_agent_session_status_in_home(
        context.agent,
        context.workspace_path,
        home_dir,
        context.activity_threshold,
    )
    .unwrap_or(detected)
}

fn detect_agent_session_status_in_home(
    agent: AgentType,
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    match agent {
        AgentType::Claude => {
            detect_claude_session_status_in_home(workspace_path, home_dir, activity_threshold)
        }
        AgentType::Codex => {
            detect_codex_session_status_in_home(workspace_path, home_dir, activity_threshold)
        }
    }
}

fn detect_claude_session_status_in_home(
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    let workspace_path = absolute_path(workspace_path)?;
    let project_dir_name = claude_project_dir_name(&workspace_path);
    let project_dir = home_dir
        .join(".claude")
        .join("projects")
        .join(project_dir_name);
    let session_files = find_recent_jsonl_files(&project_dir, Some("agent-"))?;
    for session_file in session_files {
        if is_file_recently_modified(&session_file, activity_threshold) {
            return Some(WorkspaceStatus::Active);
        }

        let session_stem = session_file.file_stem()?;
        let subagents_dir = project_dir.join(session_stem).join("subagents");
        if any_file_recently_modified(&subagents_dir, ".jsonl", activity_threshold) {
            return Some(WorkspaceStatus::Active);
        }

        if let Some(status) =
            get_last_message_status_jsonl(&session_file, "type", "user", "assistant")
        {
            return Some(status);
        }
    }

    None
}

fn detect_codex_session_status_in_home(
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    let workspace_path = absolute_path(workspace_path)?;
    let sessions_dir = home_dir.join(".codex").join("sessions");
    let session_file = find_codex_session_for_path(&sessions_dir, &workspace_path)?;

    if is_file_recently_modified(&session_file, activity_threshold) {
        return Some(WorkspaceStatus::Active);
    }

    get_codex_last_message_status(&session_file)
}

fn absolute_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    let current = std::env::current_dir().ok()?;
    Some(current.join(path))
}

fn claude_project_dir_name(abs_path: &Path) -> String {
    abs_path
        .to_string_lossy()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn is_file_recently_modified(path: &Path, threshold: Duration) -> bool {
    let modified_at = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok();
    let Some(modified_at) = modified_at else {
        return false;
    };
    let Ok(age) = modified_at.elapsed() else {
        return false;
    };
    age < threshold
}

fn any_file_recently_modified(dir: &Path, suffix: &str, threshold: Duration) -> bool {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }
        if !entry.file_name().to_string_lossy().ends_with(suffix) {
            continue;
        }
        if is_file_recently_modified(&entry.path(), threshold) {
            return true;
        }
    }

    false
}

fn find_recent_jsonl_files(dir: &Path, exclude_prefix: Option<&str>) -> Option<Vec<PathBuf>> {
    let entries = fs::read_dir(dir).ok()?;
    let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if !file_type.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !file_name.ends_with(".jsonl") {
            continue;
        }
        if exclude_prefix.is_some_and(|prefix| file_name.starts_with(prefix)) {
            continue;
        }
        let modified = match entry.metadata().and_then(|metadata| metadata.modified()) {
            Ok(modified) => modified,
            Err(_) => continue,
        };
        files.push((entry.path(), modified));
    }

    files.sort_by(|left, right| right.1.cmp(&left.1));
    Some(files.into_iter().map(|(path, _)| path).collect())
}

fn read_tail_lines(path: &Path, max_bytes: usize) -> Option<Vec<String>> {
    let mut file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    if size == 0 {
        return Some(Vec::new());
    }

    let max_bytes_u64 = u64::try_from(max_bytes).ok()?;
    let start = size.saturating_sub(max_bytes_u64);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return None;
    }

    let mut bytes = Vec::new();
    if file.read_to_end(&mut bytes).is_err() {
        return None;
    }

    let mut lines: Vec<String> = String::from_utf8_lossy(&bytes)
        .lines()
        .map(|line| line.to_string())
        .collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    Some(lines)
}

fn get_last_message_status_jsonl(
    path: &Path,
    type_field: &str,
    user_value: &str,
    assistant_value: &str,
) -> Option<WorkspaceStatus> {
    let lines = read_tail_lines(path, SESSION_STATUS_TAIL_BYTES)?;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(message_type) = value.get(type_field).and_then(|value| value.as_str()) else {
            continue;
        };
        if message_type == user_value {
            return Some(WorkspaceStatus::Active);
        }
        if message_type == assistant_value {
            return Some(WorkspaceStatus::Waiting);
        }
    }
    None
}

fn find_codex_session_for_path(sessions_dir: &Path, workspace_path: &Path) -> Option<PathBuf> {
    let mut pending = vec![sessions_dir.to_path_buf()];
    let mut best_path: Option<PathBuf> = None;
    let mut best_time: Option<SystemTime> = None;

    while let Some(dir) = pending.pop() {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            let path = entry.path();
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !file_type.is_file()
                || path
                    .extension()
                    .is_none_or(|extension| extension != "jsonl")
            {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let Some(cwd) = get_codex_session_cwd(&path) else {
                continue;
            };
            if !cwd_matches(&cwd, workspace_path) {
                continue;
            }
            let modified = match metadata.modified() {
                Ok(modified) => modified,
                Err(_) => continue,
            };
            let replace = match best_time {
                Some(current_best) => modified > current_best,
                None => true,
            };
            if replace {
                best_time = Some(modified);
                best_path = Some(path);
            }
        }
    }

    best_path
}

fn get_codex_session_cwd(path: &Path) -> Option<PathBuf> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("session_meta") {
            continue;
        }
        let cwd = value
            .get("payload")
            .and_then(|payload| payload.get("cwd"))
            .and_then(|cwd| cwd.as_str())?;
        return Some(PathBuf::from(cwd));
    }
    None
}

fn cwd_matches(cwd: &Path, workspace_path: &Path) -> bool {
    let cwd = match absolute_path(cwd) {
        Some(path) => path,
        None => return false,
    };
    cwd == workspace_path || cwd.starts_with(workspace_path)
}

fn get_codex_last_message_status(path: &Path) -> Option<WorkspaceStatus> {
    let lines = read_tail_lines(path, SESSION_STATUS_TAIL_BYTES)?;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if value.get("type").and_then(|value| value.as_str()) != Some("response_item") {
            continue;
        }
        if value
            .get("payload")
            .and_then(|payload| payload.get("type"))
            .and_then(|value| value.as_str())
            != Some("message")
        {
            continue;
        }
        match value
            .get("payload")
            .and_then(|payload| payload.get("role"))
            .and_then(|value| value.as_str())
        {
            Some("assistant") => return Some(WorkspaceStatus::Waiting),
            Some("user") => return Some(WorkspaceStatus::Active),
            _ => continue,
        }
    }
    None
}

fn has_unclosed_tag(text: &str, open_tag: &str, close_tag: &str) -> bool {
    let Some(open_index) = text.rfind(open_tag) else {
        return false;
    };

    match text.rfind(close_tag) {
        Some(close_index) => close_index < open_index,
        None => true,
    }
}

pub fn reconcile_with_sessions(
    workspaces: &[Workspace],
    running_sessions: &HashSet<String>,
    previously_running_workspace_names: &HashSet<String>,
) -> ReconciliationResult {
    let mut mapped_workspaces = Vec::with_capacity(workspaces.len());
    let mut matched_sessions = HashSet::new();

    for workspace in workspaces {
        let mut updated = workspace.clone();
        let session_name = session_name_for_workspace_in_project(
            workspace.project_name.as_deref(),
            &workspace.name,
        );
        let has_live_session = running_sessions.contains(&session_name);
        if has_live_session {
            matched_sessions.insert(session_name);
            updated.status = detect_status(
                "",
                SessionActivity::Active,
                workspace.is_main,
                true,
                updated.supported_agent,
            );
            updated.is_orphaned = false;
        } else {
            updated.status = detect_status(
                "",
                SessionActivity::Idle,
                workspace.is_main,
                false,
                updated.supported_agent,
            );
            updated.is_orphaned = if workspace.is_main {
                false
            } else {
                previously_running_workspace_names.contains(&workspace.name)
            };
        }

        mapped_workspaces.push(updated);
    }

    let mut orphaned_sessions: Vec<String> = running_sessions
        .iter()
        .filter(|session_name| !matched_sessions.contains(*session_name))
        .cloned()
        .collect();
    orphaned_sessions.sort();

    ReconciliationResult {
        workspaces: mapped_workspaces,
        orphaned_sessions,
    }
}

pub fn poll_interval(
    status: WorkspaceStatus,
    is_selected: bool,
    is_preview_focused: bool,
    interactive_mode: bool,
    since_last_key: Duration,
    output_changing: bool,
) -> Duration {
    if interactive_mode && is_selected {
        if since_last_key < Duration::from_secs(2) {
            return Duration::from_millis(50);
        }
        if since_last_key < Duration::from_secs(10) {
            return Duration::from_millis(200);
        }
        return Duration::from_millis(500);
    }

    if !is_selected {
        return Duration::from_secs(10);
    }

    if output_changing {
        return Duration::from_millis(200);
    }

    if is_preview_focused {
        return Duration::from_millis(500);
    }

    match status {
        WorkspaceStatus::Active | WorkspaceStatus::Thinking => Duration::from_millis(200),
        WorkspaceStatus::Waiting | WorkspaceStatus::Idle => Duration::from_secs(2),
        WorkspaceStatus::Done | WorkspaceStatus::Error => Duration::from_secs(20),
        WorkspaceStatus::Main | WorkspaceStatus::Unknown | WorkspaceStatus::Unsupported => {
            Duration::from_secs(2)
        }
    }
}

pub(crate) fn evaluate_capture_change(
    previous: Option<&OutputDigest>,
    raw_output: &str,
) -> CaptureChange {
    let render_output = strip_non_sgr_control_sequences(raw_output);
    let cleaned_output = strip_mouse_fragments(&strip_sgr_sequences(&render_output));
    let digest = OutputDigest {
        raw_hash: content_hash(raw_output),
        raw_len: raw_output.len(),
        cleaned_hash: content_hash(&cleaned_output),
    };

    match previous {
        None => CaptureChange {
            digest,
            changed_raw: true,
            changed_cleaned: true,
            cleaned_output,
            render_output,
        },
        Some(previous_digest) => CaptureChange {
            changed_raw: previous_digest.raw_hash != digest.raw_hash
                || previous_digest.raw_len != digest.raw_len,
            changed_cleaned: previous_digest.cleaned_hash != digest.cleaned_hash,
            digest,
            cleaned_output,
            render_output,
        },
    }
}

fn is_safe_text_character(character: char) -> bool {
    matches!(character, '\n' | '\t') || !character.is_control()
}

pub(crate) fn strip_mouse_fragments(input: &str) -> String {
    let mut cleaned = input.to_string();

    for mode in [1000u16, 1002, 1003, 1005, 1006, 1015, 2004] {
        cleaned = cleaned.replace(&format!("\u{1b}[?{mode}h"), "");
        cleaned = cleaned.replace(&format!("\u{1b}[?{mode}l"), "");
        cleaned = cleaned.replace(&format!("[?{mode}h"), "");
        cleaned = cleaned.replace(&format!("[?{mode}l"), "");
    }

    strip_partial_mouse_sequences(&cleaned)
}

fn strip_non_sgr_control_sequences(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            if is_safe_text_character(character) {
                cleaned.push(character);
            }
            continue;
        }

        let Some(next) = chars.next() else {
            break;
        };

        match next {
            '[' => {
                let mut csi = String::from("\u{1b}[");
                if let Some(final_char) = consume_csi_sequence(&mut chars, &mut csi)
                    && final_char == 'm'
                {
                    cleaned.push_str(&csi);
                }
            }
            ']' => consume_osc_sequence(&mut chars),
            'P' | 'X' | '^' | '_' => consume_st_sequence(&mut chars),
            '(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' => {
                let _ = chars.next();
            }
            _ => {}
        }
    }

    cleaned
}

fn strip_sgr_sequences(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            if chars.next_if_eq(&'[').is_some() {
                let mut did_end = false;
                for value in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&value) {
                        did_end = true;
                        break;
                    }
                }
                if did_end {
                    continue;
                }
            }
            continue;
        }

        if is_safe_text_character(character) {
            cleaned.push(character);
        }
    }

    cleaned
}

fn consume_csi_sequence<I>(chars: &mut std::iter::Peekable<I>, buffer: &mut String) -> Option<char>
where
    I: Iterator<Item = char>,
{
    for character in chars.by_ref() {
        buffer.push(character);
        if ('\u{40}'..='\u{7e}').contains(&character) {
            return Some(character);
        }
    }

    None
}

fn consume_osc_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(character) = chars.next() {
        if character == '\u{7}' {
            return;
        }

        if character == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
            return;
        }
    }
}

fn consume_st_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
            return;
        }
    }
}

fn strip_partial_mouse_sequences(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(end) = parse_mouse_fragment_end(bytes, index) {
            index = end;
            continue;
        }

        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(output).unwrap_or_default()
}

fn parse_mouse_fragment_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) == Some(&b'[') && bytes.get(start.saturating_add(1)) == Some(&b'<') {
        return parse_sgr_mouse_tail(bytes, start.saturating_add(2));
    }
    if matches!(bytes.get(start), Some(b'M' | b'm'))
        && bytes.get(start.saturating_add(1)) == Some(&b'[')
        && bytes.get(start.saturating_add(2)) == Some(&b'<')
    {
        return parse_sgr_mouse_tail(bytes, start.saturating_add(3));
    }

    None
}

fn parse_sgr_mouse_tail(bytes: &[u8], mut index: usize) -> Option<usize> {
    index = consume_ascii_digits(bytes, index)?;

    if bytes.get(index) != Some(&b';') {
        return None;
    }
    index = index.saturating_add(1);
    index = consume_ascii_digits(bytes, index)?;

    if bytes.get(index) != Some(&b';') {
        return None;
    }
    index = index.saturating_add(1);
    index = consume_ascii_digits(bytes, index)?;

    if matches!(bytes.get(index), Some(b'M' | b'm')) {
        index = index.saturating_add(1);
    }

    Some(index)
}

fn consume_ascii_digits(bytes: &[u8], mut start: usize) -> Option<usize> {
    let initial = start;
    while matches!(bytes.get(start), Some(b'0'..=b'9')) {
        start = start.saturating_add(1);
    }

    if start == initial { None } else { Some(start) }
}

fn build_launcher_script(agent_cmd: &str, prompt: &str, launcher_path: &Path) -> String {
    format!(
        "#!/bin/bash\nexport NVM_DIR=\"${{NVM_DIR:-$HOME/.nvm}}\"\n[ -s \"$NVM_DIR/nvm.sh\" ] && source \"$NVM_DIR/nvm.sh\" 2>/dev/null\nif ! command -v node &>/dev/null; then\n  [ -f \"$HOME/.zshrc\" ] && source \"$HOME/.zshrc\" 2>/dev/null\n  [ -f \"$HOME/.bashrc\" ] && source \"$HOME/.bashrc\" 2>/dev/null\nfi\n{agent_cmd} \"$(cat <<'GROVE_PROMPT_EOF'\n{prompt}\nGROVE_PROMPT_EOF\n)\"\nrm -f {}\n",
        launcher_path.to_string_lossy()
    )
}

fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{
        CaptureChange, LaunchRequest, SessionActivity, build_launch_plan, default_agent_command,
        detect_agent_session_status_in_home, detect_status,
        detect_status_with_session_override_in_home, detect_waiting_prompt,
        evaluate_capture_change, normalized_agent_command_override, poll_interval,
        reconcile_with_sessions, sanitize_workspace_name, session_name_for_workspace, stop_plan,
        strip_mouse_fragments, zellij_capture_log_path, zellij_capture_log_path_in,
        zellij_config_path,
    };
    use crate::config::MultiplexerKind;
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};

    fn fixture_workspace(name: &str, is_main: bool) -> Workspace {
        Workspace::try_new(
            name.to_string(),
            PathBuf::from(format!("/repos/grove-{name}")),
            if is_main {
                "main".to_string()
            } else {
                name.to_string()
            },
            Some(1_700_000_100),
            AgentType::Claude,
            if is_main {
                WorkspaceStatus::Main
            } else {
                WorkspaceStatus::Idle
            },
            is_main,
        )
        .expect("workspace should be valid")
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", process::id()));
        fs::create_dir_all(&path).expect("test directory should be created");
        path
    }

    #[test]
    fn session_name_sanitizes_workspace_label() {
        assert_eq!(
            sanitize_workspace_name("feature/auth.v2"),
            "feature-auth-v2"
        );
        assert_eq!(
            session_name_for_workspace("feature/auth.v2"),
            "grove-ws-feature-auth-v2"
        );
        assert_eq!(sanitize_workspace_name("///"), "workspace");
    }

    #[test]
    fn codex_launch_command_matches_prd_flags() {
        assert_eq!(default_agent_command(AgentType::Codex, false), "codex");
        assert_eq!(
            default_agent_command(AgentType::Codex, true),
            "codex --dangerously-bypass-approvals-and-sandbox"
        );
    }

    #[test]
    fn agent_command_override_normalization_trims_whitespace() {
        assert_eq!(
            normalized_agent_command_override("  /tmp/fake-codex --flag  "),
            Some("/tmp/fake-codex --flag".to_string())
        );
    }

    #[test]
    fn agent_command_override_normalization_ignores_empty_values() {
        assert_eq!(normalized_agent_command_override(""), None);
        assert_eq!(normalized_agent_command_override("   "), None);
    }

    #[test]
    fn launch_plan_without_prompt_sends_agent_directly() {
        let request = LaunchRequest {
            project_name: None,
            workspace_name: "auth-flow".to_string(),
            workspace_path: PathBuf::from("/repos/grove-auth-flow"),
            agent: AgentType::Claude,
            prompt: None,
            pre_launch_command: None,
            skip_permissions: true,
            capture_cols: None,
            capture_rows: None,
        };

        let plan = build_launch_plan(&request, MultiplexerKind::Tmux);

        assert_eq!(plan.session_name, "grove-ws-auth-flow");
        assert!(plan.launcher_script.is_none());
        assert_eq!(
            plan.launch_cmd,
            vec![
                "tmux",
                "send-keys",
                "-t",
                "grove-ws-auth-flow",
                "claude --dangerously-skip-permissions",
                "Enter"
            ]
        );
    }

    #[test]
    fn launch_plan_with_prompt_writes_launcher_script() {
        let request = LaunchRequest {
            project_name: None,
            workspace_name: "db_migration".to_string(),
            workspace_path: PathBuf::from("/repos/grove-db_migration"),
            agent: AgentType::Codex,
            prompt: Some("fix migration".to_string()),
            pre_launch_command: None,
            skip_permissions: false,
            capture_cols: None,
            capture_rows: None,
        };

        let plan = build_launch_plan(&request, MultiplexerKind::Tmux);

        let script = plan.launcher_script.expect("script should be present");
        assert!(script.contents.contains("codex"));
        assert!(script.contents.contains("fix migration"));
        assert!(script.contents.contains("GROVE_PROMPT_EOF"));
        assert_eq!(
            plan.launch_cmd,
            vec![
                "tmux",
                "send-keys",
                "-t",
                "grove-ws-db_migration",
                "bash /repos/grove-db_migration/.grove-start.sh",
                "Enter"
            ]
        );
    }

    #[test]
    fn stop_plan_uses_ctrl_c_then_kill_session() {
        let plan = stop_plan("grove-ws-auth-flow", MultiplexerKind::Tmux);
        assert_eq!(plan.len(), 2);
        assert_eq!(
            plan[0],
            vec!["tmux", "send-keys", "-t", "grove-ws-auth-flow", "C-c"]
        );
        assert_eq!(
            plan[1],
            vec!["tmux", "kill-session", "-t", "grove-ws-auth-flow"]
        );
    }

    #[test]
    fn zellij_launch_plan_creates_background_session_and_runs_agent() {
        let request = LaunchRequest {
            project_name: None,
            workspace_name: "auth-flow".to_string(),
            workspace_path: PathBuf::from("/repos/grove-auth-flow"),
            agent: AgentType::Codex,
            prompt: None,
            pre_launch_command: None,
            skip_permissions: false,
            capture_cols: None,
            capture_rows: None,
        };

        let plan = build_launch_plan(&request, MultiplexerKind::Zellij);
        let capture_log_path = zellij_capture_log_path("grove-ws-auth-flow");
        let capture_log_path_text = capture_log_path.to_string_lossy().to_string();
        let capture_log_dir_text = capture_log_path
            .parent()
            .expect("capture path should have parent")
            .to_string_lossy()
            .to_string();
        let config_path = zellij_config_path();
        let config_path_text = config_path.to_string_lossy().to_string();
        let config_dir_text = config_path
            .parent()
            .expect("config path should have parent")
            .to_string_lossy()
            .to_string();

        assert_eq!(plan.session_name, "grove-ws-auth-flow");
        assert_eq!(
            plan.pre_launch_cmds[0],
            vec![
                "sh",
                "-lc",
                &format!(
                    "mkdir -p '{config_dir_text}' && if [ ! -f '{config_path_text}' ]; then printf '%s\\n' 'show_startup_tips false\nshow_release_notes false' > '{config_path_text}'; fi"
                ),
            ]
        );
        assert_eq!(
            plan.pre_launch_cmds[1],
            vec![
                "sh",
                "-lc",
                &format!(
                    "zellij --config '{config_path_text}' kill-session 'grove-ws-auth-flow' >/dev/null 2>&1 || true"
                ),
            ]
        );
        assert_eq!(
            plan.pre_launch_cmds[2],
            vec![
                "sh",
                "-lc",
                &format!(
                    "mkdir -p '{}' && : > '{}'",
                    capture_log_dir_text, capture_log_path_text
                ),
            ]
        );
        assert_eq!(
            plan.pre_launch_cmds[3],
            vec![
                "sh",
                "-lc",
                &format!(
                    "zellij --config '{config_path_text}' attach 'grove-ws-auth-flow' --create --create-background >/dev/null 2>&1 || true"
                ),
            ]
        );
        assert_eq!(
            plan.pre_launch_cmds[4],
            vec![
                "sh",
                "-lc",
                &format!(
                    "nohup script -q /dev/null -c \"stty cols 120 rows 40; export COLUMNS=120 LINES=40 TERM=xterm-256color COLORTERM=truecolor; unset NO_COLOR; zellij --config '{config_path_text}' attach grove-ws-auth-flow\" >/dev/null 2>&1 &"
                ),
            ]
        );
        assert_eq!(plan.pre_launch_cmds[5], vec!["sh", "-lc", "sleep 1"]);
        assert_eq!(
            plan.launch_cmd,
            vec![
                "zellij",
                "--config",
                &config_path_text,
                "--session",
                "grove-ws-auth-flow",
                "run",
                "--floating",
                "--width",
                "100%",
                "--height",
                "100%",
                "--x",
                "0",
                "--y",
                "0",
                "--cwd",
                "/repos/grove-auth-flow",
                "--",
                "bash",
                "-lc",
                &format!(
                    "stty cols 120 rows 40; export COLUMNS=120 LINES=40 TERM=xterm-256color COLORTERM=truecolor; unset NO_COLOR; script -qefc 'codex' '{}'",
                    capture_log_path_text
                ),
            ]
        );
    }

    #[test]
    fn zellij_capture_log_path_joins_session_file_name() {
        let path = zellij_capture_log_path_in(Path::new("/tmp/grove-zellij-capture"), "grove-ws-x");
        assert_eq!(
            path,
            PathBuf::from("/tmp/grove-zellij-capture/grove-ws-x.ansi.log")
        );
    }

    #[test]
    fn zellij_stop_plan_uses_ctrl_c_then_kill_session() {
        let plan = stop_plan("grove-ws-auth-flow", MultiplexerKind::Zellij);
        let config_path_text = zellij_config_path().to_string_lossy().to_string();
        assert_eq!(plan.len(), 2);
        assert_eq!(
            plan[0],
            vec![
                "zellij",
                "--config",
                &config_path_text,
                "--session",
                "grove-ws-auth-flow",
                "action",
                "write",
                "3",
            ]
        );
        assert_eq!(
            plan[1],
            vec![
                "zellij",
                "--config",
                &config_path_text,
                "kill-session",
                "grove-ws-auth-flow"
            ]
        );
    }

    #[test]
    fn launch_plan_with_pre_launch_command_runs_before_agent() {
        let request = LaunchRequest {
            project_name: None,
            workspace_name: "auth-flow".to_string(),
            workspace_path: PathBuf::from("/repos/grove-auth-flow"),
            agent: AgentType::Claude,
            prompt: None,
            pre_launch_command: Some("direnv allow".to_string()),
            skip_permissions: true,
            capture_cols: None,
            capture_rows: None,
        };

        let plan = build_launch_plan(&request, MultiplexerKind::Tmux);
        assert_eq!(
            plan.launch_cmd,
            vec![
                "tmux",
                "send-keys",
                "-t",
                "grove-ws-auth-flow",
                "direnv allow && claude --dangerously-skip-permissions",
                "Enter"
            ]
        );
    }

    #[test]
    fn waiting_prompt_checks_tail_lines_only() {
        let output = "approve earlier\nline\nline\nline\nline\nline\nline\nline\nline\n";
        assert_eq!(detect_waiting_prompt(output), None);

        let tail_output = "line\nline\nline\nline\nallow edit? [y/n]\n";
        assert_eq!(
            detect_waiting_prompt(tail_output),
            Some("allow edit? [y/n]".to_string())
        );
    }

    #[test]
    fn waiting_prompt_detects_codex_shortcuts_hint() {
        let output = "result\nresult\n> Implement {feature}\n? for shortcuts\n";
        assert_eq!(
            detect_waiting_prompt(output),
            Some("? for shortcuts".to_string())
        );
    }

    #[test]
    fn waiting_prompt_detects_unicode_prompt_prefix() {
        let output = "Claude Code v2\n› Try \"how does adapters.rs work?\"\n";
        assert_eq!(
            detect_waiting_prompt(output),
            Some("› Try \"how does adapters.rs work?\"".to_string())
        );
        assert_eq!(
            detect_status(output, SessionActivity::Active, false, true, true),
            WorkspaceStatus::Waiting
        );
    }

    #[test]
    fn claude_session_file_marks_waiting_when_last_message_is_assistant() {
        let root = unique_test_dir("grove-claude-session");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-alpha");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir_name = super::claude_project_dir_name(&workspace_path);
        let project_dir = home.join(".claude").join("projects").join(project_dir_name);
        fs::create_dir_all(&project_dir).expect("project directory should exist");

        let session_file = project_dir.join("session-1.jsonl");
        fs::write(
            &session_file,
            "{\"type\":\"system\"}\n{\"type\":\"assistant\"}\n",
        )
        .expect("session file should be written");

        let status = detect_agent_session_status_in_home(
            AgentType::Claude,
            &workspace_path,
            &home,
            Duration::from_secs(0),
        );
        assert_eq!(status, Some(WorkspaceStatus::Waiting));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn codex_session_file_marks_waiting_when_last_message_is_assistant() {
        let root = unique_test_dir("grove-codex-session");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-beta");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let sessions_dir = home.join(".codex").join("sessions").join("2026").join("02");
        fs::create_dir_all(&sessions_dir).expect("sessions directory should exist");
        let session_file = sessions_dir.join("rollout-1.jsonl");
        fs::write(
            &session_file,
            format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\"}}}}\n{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"assistant\"}}}}\n",
                workspace_path.display()
            ),
        )
        .expect("session file should be written");

        let status = detect_agent_session_status_in_home(
            AgentType::Codex,
            &workspace_path,
            &home,
            Duration::from_secs(0),
        );
        assert_eq!(status, Some(WorkspaceStatus::Waiting));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn status_override_uses_session_files_for_active_waiting() {
        let root = unique_test_dir("grove-status-override");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-gamma");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir_name = super::claude_project_dir_name(&workspace_path);
        let project_dir = home.join(".claude").join("projects").join(project_dir_name);
        fs::create_dir_all(&project_dir).expect("project directory should exist");
        let session_file = project_dir.join("session-2.jsonl");
        fs::write(&session_file, "{\"type\":\"assistant\"}\n")
            .expect("session file should be written");

        let status = detect_status_with_session_override_in_home(super::StatusOverrideContext {
            output: "plain output",
            session_activity: SessionActivity::Active,
            is_main: false,
            has_live_session: true,
            supported_agent: true,
            agent: AgentType::Claude,
            workspace_path: &workspace_path,
            home_dir: Some(&home),
            activity_threshold: Duration::from_secs(0),
        });
        assert_eq!(status, WorkspaceStatus::Waiting);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn status_resolution_uses_recent_tail_and_waiting_prompt_before_error() {
        assert_eq!(
            detect_status("panic: bad", SessionActivity::Active, false, true, true),
            WorkspaceStatus::Error
        );
        assert_eq!(
            detect_status(
                "task completed successfully",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Done
        );
        assert_eq!(
            detect_status("thinking...", SessionActivity::Active, false, true, true),
            WorkspaceStatus::Thinking
        );
        assert_eq!(
            detect_status(
                "allow edit? [y/n]",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Waiting
        );
        assert_eq!(
            detect_status("", SessionActivity::Active, false, true, true),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status("", SessionActivity::Idle, false, false, true),
            WorkspaceStatus::Idle
        );
        assert_eq!(
            detect_status("", SessionActivity::Active, false, true, false),
            WorkspaceStatus::Unsupported
        );

        assert_eq!(
            detect_status(
                "warning: failed to login mcp\nline\nline\n> Implement {feature}\n? for shortcuts\n",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Waiting
        );
        assert_eq!(
            detect_status(
                "Do you want to continue?",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Waiting
        );
        assert_eq!(
            detect_status("", SessionActivity::Active, true, true, true),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status("", SessionActivity::Idle, true, false, true),
            WorkspaceStatus::Main
        );
    }

    #[test]
    fn status_resolution_ignores_old_non_tail_errors() {
        let mut lines = vec!["failed: transient startup warning".to_string()];
        lines.extend((0..70).map(|index| format!("line {index}")));
        let output = lines.join("\n");
        assert_eq!(
            detect_status(&output, SessionActivity::Active, false, true, true),
            WorkspaceStatus::Active
        );
    }

    #[test]
    fn status_resolution_requires_unclosed_thinking_tags() {
        assert_eq!(
            detect_status(
                "<thinking>\nworking\n</thinking>",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status(
                "<thinking>\nworking\n",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Thinking
        );
        assert_eq!(
            detect_status(
                "<internal_monologue>\nworking\n",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Thinking
        );
    }

    #[test]
    fn reconciliation_marks_active_orphaned_and_orphan_sessions() {
        let workspaces = vec![
            fixture_workspace("grove", true),
            fixture_workspace("feature-a", false),
            fixture_workspace("feature-b", false),
        ];

        let running_sessions = HashSet::from([
            "grove-ws-grove".to_string(),
            "grove-ws-feature-a".to_string(),
            "grove-ws-zombie".to_string(),
        ]);
        let previously_running = HashSet::from(["feature-b".to_string()]);

        let result = reconcile_with_sessions(&workspaces, &running_sessions, &previously_running);
        assert_eq!(result.workspaces[0].status, WorkspaceStatus::Active);
        assert_eq!(result.workspaces[1].status, WorkspaceStatus::Active);
        assert_eq!(result.workspaces[2].status, WorkspaceStatus::Idle);
        assert!(result.workspaces[2].is_orphaned);
        assert_eq!(
            result.orphaned_sessions,
            vec!["grove-ws-zombie".to_string()]
        );
    }

    #[test]
    fn poll_intervals_follow_preview_and_interactive_rules() {
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_millis(100),
                true
            ),
            Duration::from_millis(50)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_secs(5),
                true
            ),
            Duration::from_millis(200)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_secs(15),
                false
            ),
            Duration::from_millis(500)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                false,
                false,
                false,
                Duration::from_secs(30),
                true
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Done,
                true,
                false,
                false,
                Duration::from_secs(30),
                false
            ),
            Duration::from_secs(20)
        );
    }

    #[test]
    fn capture_change_detects_mouse_fragment_noise() {
        let first = evaluate_capture_change(None, "hello\u{1b}[?1000h\u{1b}[<35;192;47M");
        assert!(first.changed_raw);
        assert!(first.changed_cleaned);

        let second = evaluate_capture_change(Some(&first.digest), "hello\u{1b}[?1000l");
        assert!(second.changed_raw);
        assert!(!second.changed_cleaned);
        assert_eq!(second.cleaned_output, "hello");

        let third = evaluate_capture_change(Some(&second.digest), "hello world");
        assert!(third.changed_cleaned);
    }

    #[test]
    fn capture_change_first_capture_marks_changed() {
        let change: CaptureChange = evaluate_capture_change(None, "one");
        assert!(change.changed_raw);
        assert!(change.changed_cleaned);
    }

    #[test]
    fn capture_change_strips_ansi_control_sequences() {
        let raw = "A\u{1b}[31mB\u{1b}[39m C\u{1b}]0;title\u{7}\n";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "AB C\n");
    }

    #[test]
    fn capture_change_strips_terminal_control_bytes() {
        let raw = "A\u{000e}B\u{000f}C\r\n";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "ABC\n");
        assert_eq!(change.render_output, "ABC\n");
    }

    #[test]
    fn capture_change_ignores_truncated_partial_mouse_fragments() {
        let first = evaluate_capture_change(None, "prompt [<65;103;31");
        assert_eq!(first.cleaned_output, "prompt ");

        let second = evaluate_capture_change(Some(&first.digest), "prompt [<65;103;32");
        assert!(!second.changed_cleaned);
        assert_eq!(second.cleaned_output, "prompt ");
    }

    #[test]
    fn strip_mouse_fragments_removes_terminal_modes_and_preserves_normal_brackets() {
        assert_eq!(strip_mouse_fragments("value[?1002h"), "value");
        assert_eq!(strip_mouse_fragments("keep [test]"), "keep [test]");
    }

    #[test]
    fn strip_mouse_fragments_removes_boundary_prefixed_partial_sequences() {
        assert_eq!(strip_mouse_fragments("prompt M[<64;107;16M"), "prompt ");
        assert_eq!(strip_mouse_fragments("prompt m[<65;107;14"), "prompt ");
    }
}

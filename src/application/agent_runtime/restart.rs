use std::path::Path;

use crate::domain::{AgentType, Workspace};

use super::agents;
use super::execution::execute_command_with;
use super::launch_plan::build_agent_env_command;
use super::sessions::session_name_for_workspace_ref;
use super::{
    RESTART_RESUME_CAPTURE_ATTEMPTS, RESTART_RESUME_ERROR_MAX_CHARS,
    RESTART_RESUME_ERROR_TAIL_LINES, RESTART_RESUME_RETRY_DELAY, RESTART_RESUME_SCROLLBACK_LINES,
    RestartExitInput, SessionExecutionResult,
};

pub fn restart_workspace(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
) -> SessionExecutionResult {
    execute_restart_workspace_in_pane_with_result(workspace, skip_permissions, agent_env)
}

pub fn agent_supports_in_pane_restart(agent: AgentType) -> bool {
    agents::supports_in_pane_restart(agent)
}

fn restart_exit_input(agent: AgentType) -> Option<RestartExitInput> {
    agents::restart_exit_input(agent)
}

fn restart_exit_plan(session_name: &str, exit_input: RestartExitInput) -> Vec<Vec<String>> {
    match exit_input {
        RestartExitInput::Literal(text) => vec![
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-l".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                text.to_string(),
            ],
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                session_name.to_string(),
                "Enter".to_string(),
            ],
        ],
        RestartExitInput::Named(key) => vec![vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            session_name.to_string(),
            key.to_string(),
        ]],
    }
}

fn resume_command_with_skip_permissions(
    agent: AgentType,
    command: &str,
    skip_permissions: bool,
) -> String {
    agents::resume_command_with_skip_permissions(agent, command, skip_permissions)
}

fn restart_resume_command(
    session_name: &str,
    agent: AgentType,
    command: &str,
    skip_permissions: bool,
) -> Vec<String> {
    let command = resume_command_with_skip_permissions(agent, command, skip_permissions);
    vec![
        "tmux".to_string(),
        "send-keys".to_string(),
        "-t".to_string(),
        session_name.to_string(),
        command,
        "Enter".to_string(),
    ]
}

pub fn extract_agent_resume_command(agent: AgentType, output: &str) -> Option<String> {
    agents::extract_resume_command(agent, output)
}

pub fn infer_workspace_skip_permissions(agent: AgentType, workspace_path: &Path) -> Option<bool> {
    let home_dir = dirs::home_dir()?;
    if !workspace_path.exists() {
        return None;
    }

    agents::infer_skip_permissions_in_home(agent, workspace_path, &home_dir)
}

fn wait_for_resume_command(
    agent: AgentType,
    session_name: &str,
    capture_output: &mut impl FnMut(&str, usize, bool) -> std::io::Result<String>,
) -> Result<String, String> {
    let mut last_output_excerpt = "<empty>".to_string();
    for attempt in 0..RESTART_RESUME_CAPTURE_ATTEMPTS {
        let output = capture_output(session_name, RESTART_RESUME_SCROLLBACK_LINES, false)
            .map_err(|error| error.to_string())?;
        last_output_excerpt = restart_capture_excerpt(output.as_str());
        if let Some(command) = extract_agent_resume_command(agent, output.as_str()) {
            return Ok(command);
        }
        if attempt + 1 < RESTART_RESUME_CAPTURE_ATTEMPTS {
            std::thread::sleep(RESTART_RESUME_RETRY_DELAY);
        }
    }

    Err(format!(
        "resume command not found in tmux output for '{session_name}' after {} attempts, last_output='{last_output_excerpt}'",
        RESTART_RESUME_CAPTURE_ATTEMPTS
    ))
}

fn restart_capture_excerpt(output: &str) -> String {
    let mut tail = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_whitespace().collect::<Vec<&str>>().join(" "))
        .rev()
        .take(RESTART_RESUME_ERROR_TAIL_LINES)
        .collect::<Vec<String>>();
    if tail.is_empty() {
        return "<empty>".to_string();
    }
    tail.reverse();
    let joined = tail.join(" | ");
    truncate_excerpt(joined.as_str(), RESTART_RESUME_ERROR_MAX_CHARS)
}

fn truncate_excerpt(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return "...".chars().take(max_chars).collect();
    }

    let trimmed: String = value.chars().take(max_chars - 3).collect();
    format!("{trimmed}...")
}

pub fn restart_workspace_in_pane_with_io(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: &[(String, String)],
    mut execute: impl FnMut(&[String]) -> std::io::Result<()>,
    mut capture_output: impl FnMut(&str, usize, bool) -> std::io::Result<String>,
) -> Result<(), String> {
    let home_dir = dirs::home_dir();
    restart_workspace_in_pane_with_io_in_home(
        workspace,
        skip_permissions,
        agent_env,
        &mut execute,
        &mut capture_output,
        home_dir.as_deref(),
    )
}

pub(super) fn restart_workspace_in_pane_with_io_in_home(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: &[(String, String)],
    mut execute: impl FnMut(&[String]) -> std::io::Result<()>,
    mut capture_output: impl FnMut(&str, usize, bool) -> std::io::Result<String>,
    home_dir: Option<&Path>,
) -> Result<(), String> {
    let Some(exit_input) = restart_exit_input(workspace.agent) else {
        return Err(format!(
            "in-pane restart unsupported for {}",
            workspace.agent.label()
        ));
    };
    let session_name = session_name_for_workspace_ref(workspace);

    for command in restart_exit_plan(&session_name, exit_input) {
        execute_command_with(command.as_slice(), |command| execute(command)).map_err(|error| {
            format!("restart exit command failed for '{session_name}': {error}")
        })?;
    }

    let resume_command =
        wait_for_resume_command(workspace.agent, &session_name, &mut capture_output).or_else(
            |error| {
                let Some(home_dir) = home_dir else {
                    return Err(error);
                };
                agents::infer_resume_command_in_home(workspace.agent, &workspace.path, home_dir)
                    .ok_or(error)
            },
        )?;
    if let Some(command) = restart_agent_env_command(&session_name, agent_env) {
        execute_command_with(command.as_slice(), |command| execute(command))
            .map_err(|error| format!("restart env apply failed for '{session_name}': {error}"))?;
    }
    let command = restart_resume_command(
        &session_name,
        workspace.agent,
        resume_command.as_str(),
        skip_permissions,
    );
    execute_command_with(command.as_slice(), |command| execute(command))
        .map_err(|error| format!("restart resume command failed for '{session_name}': {error}"))
}

fn capture_output_with_process(
    target_session: &str,
    scrollback_lines: usize,
    include_escape_sequences: bool,
) -> std::io::Result<String> {
    let mut args = vec![
        "capture-pane".to_string(),
        "-p".to_string(),
        "-N".to_string(),
    ];
    if include_escape_sequences {
        args.push("-e".to_string());
    }
    args.push("-t".to_string());
    args.push(target_session.to_string());
    args.push("-S".to_string());
    args.push(format!("-{scrollback_lines}"));

    let output = std::process::Command::new("tmux").args(args).output()?;
    if !output.status.success() {
        let stderr = crate::infrastructure::process::stderr_or_status(&output);
        return Err(std::io::Error::other(format!(
            "tmux capture-pane failed for '{target_session}': {stderr}"
        )));
    }

    String::from_utf8(output.stdout)
        .map_err(|error| std::io::Error::other(format!("tmux output utf8 decode failed: {error}")))
}

fn restart_agent_env_command(
    session_name: &str,
    agent_env: &[(String, String)],
) -> Option<Vec<String>> {
    let env_command = build_agent_env_command(agent_env)?;
    Some(vec![
        "tmux".to_string(),
        "send-keys".to_string(),
        "-t".to_string(),
        session_name.to_string(),
        env_command,
        "Enter".to_string(),
    ])
}

pub fn execute_restart_workspace_in_pane_with_result(
    workspace: &Workspace,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
) -> SessionExecutionResult {
    let workspace_name = workspace.name.clone();
    let workspace_path = workspace.path.clone();
    let session_name = session_name_for_workspace_ref(workspace);
    let result = restart_workspace_in_pane_with_io(
        workspace,
        skip_permissions,
        &agent_env,
        crate::infrastructure::process::execute_command,
        capture_output_with_process,
    );
    SessionExecutionResult {
        workspace_name,
        workspace_path,
        session_name,
        result,
    }
}

mod claude;
mod codex;
mod opencode;
mod shared;

use std::path::Path;
use std::time::Duration;

use crate::domain::{AgentType, WorkspaceStatus};

#[cfg(test)]
pub(super) use claude::project_dir_name as claude_project_dir_name;

pub(super) fn restart_exit_input(agent: AgentType) -> Option<super::RestartExitInput> {
    match agent {
        AgentType::Claude => Some(super::RestartExitInput::Literal("/exit")),
        AgentType::Codex => Some(super::RestartExitInput::Named("C-c")),
        AgentType::OpenCode => Some(super::RestartExitInput::Named("C-c")),
    }
}

pub(super) fn resume_command_with_skip_permissions(
    agent: AgentType,
    command: &str,
    skip_permissions: bool,
) -> String {
    if !skip_permissions {
        return command.to_string();
    }

    match agent {
        AgentType::Claude => {
            if command.contains("--dangerously-skip-permissions") {
                return command.to_string();
            }
            if let Some(remainder) = command.strip_prefix("claude ") {
                return format!("claude --dangerously-skip-permissions {remainder}");
            }
            command.to_string()
        }
        AgentType::Codex => {
            if command.contains("--dangerously-bypass-approvals-and-sandbox") {
                return command.to_string();
            }
            if let Some(remainder) = command.strip_prefix("codex ") {
                return format!("codex --dangerously-bypass-approvals-and-sandbox {remainder}");
            }
            command.to_string()
        }
        AgentType::OpenCode => {
            if command.contains("OPENCODE_PERMISSION=") {
                return command.to_string();
            }
            format!(
                "OPENCODE_PERMISSION='{}' {command}",
                super::OPENCODE_UNSAFE_PERMISSION_JSON
            )
        }
    }
}

pub(super) fn extract_resume_command(agent: AgentType, output: &str) -> Option<String> {
    match agent {
        AgentType::Claude => claude::extract_resume_command(output),
        AgentType::Codex => codex::extract_resume_command(output),
        AgentType::OpenCode => opencode::extract_resume_command(output),
    }
}

pub(super) fn infer_skip_permissions_in_home(
    agent: AgentType,
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<bool> {
    match agent {
        AgentType::Claude => claude::infer_skip_permissions_in_home(workspace_path, home_dir),
        AgentType::Codex => codex::infer_skip_permissions_in_home(workspace_path, home_dir),
        AgentType::OpenCode => opencode::infer_skip_permissions_in_home(workspace_path, home_dir),
    }
}

pub(super) fn detect_session_status_in_home(
    agent: AgentType,
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    match agent {
        AgentType::Claude => {
            claude::detect_session_status_in_home(workspace_path, home_dir, activity_threshold)
        }
        AgentType::Codex => {
            codex::detect_session_status_in_home(workspace_path, home_dir, activity_threshold)
        }
        AgentType::OpenCode => {
            opencode::detect_session_status_in_home(workspace_path, home_dir, activity_threshold)
        }
    }
}

pub(super) fn latest_attention_marker_in_home(
    agent: AgentType,
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    match agent {
        AgentType::Claude => claude::latest_attention_marker_in_home(workspace_path, home_dir),
        AgentType::Codex => codex::latest_attention_marker_in_home(workspace_path, home_dir),
        AgentType::OpenCode => opencode::latest_attention_marker_in_home(workspace_path, home_dir),
    }
}

pub(super) fn infer_resume_command_in_home(
    agent: AgentType,
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    match agent {
        AgentType::OpenCode => opencode::infer_resume_command_in_home(workspace_path, home_dir),
        _ => None,
    }
}

#[cfg(test)]
pub(super) fn codex_session_skip_permissions_mode(path: &Path) -> Option<bool> {
    codex::session_skip_permissions_mode(path)
}

pub(super) fn normalize_resume_session_id(value: &str) -> Option<String> {
    if value.contains('<') || value.contains('>') {
        return None;
    }

    let trimmed = value.trim_matches(|character: char| {
        matches!(
            character,
            '\'' | '"' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '.' | ':'
        )
    });
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        return None;
    }
    Some(trimmed.to_string())
}

pub(super) fn normalize_codex_resume_session_id(value: &str) -> Option<String> {
    let normalized = normalize_resume_session_id(value)?;
    if normalized
        .chars()
        .any(|character| character.is_ascii_digit())
        || normalized.contains('-')
        || normalized.contains('_')
    {
        return Some(normalized);
    }

    None
}

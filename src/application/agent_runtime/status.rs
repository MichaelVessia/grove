use std::path::Path;
use std::time::Duration;

use crate::domain::{AgentType, WorkspaceStatus};

use super::agents;
use super::{
    DONE_PATTERNS, ERROR_PATTERNS, SESSION_ACTIVITY_THRESHOLD, STATUS_TAIL_LINES, SessionActivity,
    WAITING_PATTERNS, WAITING_TAIL_LINES,
};

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
        if matches!(prefix, '›' | '❯' | '»') {
            let without_prefix = trimmed.trim_start_matches(['›', '❯', '»']).trim_start();
            if without_prefix.to_ascii_lowercase().starts_with("try ") {
                return Some(trimmed.to_string());
            }
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

    if lines[start..].iter().any(|line| {
        let normalized = line
            .trim()
            .trim_start_matches(['•', '-', '*', '·', '✓', '✔', '☑'])
            .trim()
            .to_ascii_lowercase();
        normalized == "done" || normalized == "done."
    }) {
        return WorkspaceStatus::Done;
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

pub(crate) struct StatusOverrideContext<'a> {
    pub(crate) output: &'a str,
    pub(crate) session_activity: SessionActivity,
    pub(crate) is_main: bool,
    pub(crate) has_live_session: bool,
    pub(crate) supported_agent: bool,
    pub(crate) agent: AgentType,
    pub(crate) workspace_path: &'a Path,
    pub(crate) home_dir: Option<&'a Path>,
    pub(crate) activity_threshold: Duration,
}

pub(crate) fn detect_status_with_session_override_in_home(
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

pub(crate) fn detect_agent_session_status_in_home(
    agent: AgentType,
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    agents::detect_session_status_in_home(agent, workspace_path, home_dir, activity_threshold)
}

pub(crate) fn latest_assistant_attention_marker(
    agent: AgentType,
    workspace_path: &Path,
) -> Option<String> {
    let home_dir = dirs::home_dir()?;
    if !workspace_path.exists() {
        return None;
    }

    agents::latest_attention_marker_in_home(agent, workspace_path, &home_dir)
}

#[cfg(test)]
pub(super) fn infer_claude_skip_permissions_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<bool> {
    agents::infer_skip_permissions_in_home(AgentType::Claude, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn infer_codex_skip_permissions_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<bool> {
    agents::infer_skip_permissions_in_home(AgentType::Codex, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn codex_session_skip_permissions_mode(path: &Path) -> Option<bool> {
    agents::codex_session_skip_permissions_mode(path)
}

#[cfg(test)]
pub(super) fn infer_opencode_skip_permissions_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<bool> {
    agents::infer_skip_permissions_in_home(AgentType::OpenCode, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn latest_claude_assistant_attention_marker_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    agents::latest_attention_marker_in_home(AgentType::Claude, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn latest_codex_assistant_attention_marker_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    agents::latest_attention_marker_in_home(AgentType::Codex, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn latest_opencode_assistant_attention_marker_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    agents::latest_attention_marker_in_home(AgentType::OpenCode, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn claude_project_dir_name(abs_path: &Path) -> String {
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

fn has_unclosed_tag(text: &str, open_tag: &str, close_tag: &str) -> bool {
    let Some(open_index) = text.rfind(open_tag) else {
        return false;
    };

    match text.rfind(close_tag) {
        Some(close_index) => close_index < open_index,
        None => true,
    }
}

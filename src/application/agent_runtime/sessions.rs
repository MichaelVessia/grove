use std::collections::HashSet;

use crate::domain::{Workspace, WorkspaceStatus};

use super::{LivePreviewTarget, TMUX_SESSION_PREFIX};

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

#[cfg(test)]
pub fn session_name_for_workspace(workspace_name: &str) -> String {
    session_name_for_workspace_in_project(None, workspace_name)
}

pub fn session_name_for_workspace_ref(workspace: &Workspace) -> String {
    session_name_for_workspace_in_project(workspace.project_name.as_deref(), &workspace.name)
}

pub fn git_session_name_for_workspace(workspace: &Workspace) -> String {
    format!("{}-git", session_name_for_workspace_ref(workspace))
}

pub fn shell_session_name_for_workspace(workspace: &Workspace) -> String {
    format!("{}-shell", session_name_for_workspace_ref(workspace))
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

pub fn live_preview_agent_session(workspace: Option<&Workspace>) -> Option<String> {
    let workspace = workspace?;
    if !workspace.status.has_session() {
        return None;
    }

    Some(session_name_for_workspace_ref(workspace))
}

pub fn workspace_can_enter_interactive(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
) -> bool {
    if preview_tab_is_git {
        return workspace.is_some();
    }

    live_preview_agent_session(workspace).is_some()
}

pub fn workspace_can_start_agent(workspace: Option<&Workspace>) -> bool {
    let Some(workspace) = workspace else {
        return false;
    };
    if !workspace.supported_agent {
        return false;
    }

    matches!(
        workspace.status,
        WorkspaceStatus::Main
            | WorkspaceStatus::Idle
            | WorkspaceStatus::Done
            | WorkspaceStatus::Error
            | WorkspaceStatus::Unknown
    )
}

pub fn workspace_can_stop_agent(workspace: Option<&Workspace>) -> bool {
    let Some(workspace) = workspace else {
        return false;
    };

    workspace.status.has_session()
}

pub fn workspace_session_for_preview_tab(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
    git_preview_session: Option<&str>,
) -> Option<String> {
    if preview_tab_is_git {
        workspace?;
        return git_preview_session.map(str::to_string);
    }

    live_preview_agent_session(workspace)
}

pub fn git_preview_session_if_ready(
    workspace: Option<&Workspace>,
    ready_sessions: &HashSet<String>,
) -> Option<String> {
    let workspace = workspace?;
    let session_name = git_session_name_for_workspace(workspace);
    if !ready_sessions.contains(&session_name) {
        return None;
    }

    Some(session_name)
}

pub fn live_preview_session_for_tab(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
    ready_sessions: &HashSet<String>,
) -> Option<String> {
    if preview_tab_is_git {
        return git_preview_session_if_ready(workspace, ready_sessions);
    }

    live_preview_agent_session(workspace)
}

pub fn live_preview_capture_target_for_tab(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
    ready_sessions: &HashSet<String>,
) -> Option<LivePreviewTarget> {
    let session_name = live_preview_session_for_tab(workspace, preview_tab_is_git, ready_sessions)?;
    Some(LivePreviewTarget {
        session_name,
        include_escape_sequences: true,
    })
}

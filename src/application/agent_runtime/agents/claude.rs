use std::path::Path;
use std::time::Duration;

use crate::domain::WorkspaceStatus;

use super::shared;

pub(super) fn extract_resume_command(output: &str) -> Option<String> {
    let mut found = None;
    for line in output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }

        for index in 0..tokens.len().saturating_sub(2) {
            if tokens[index] != "claude" {
                continue;
            }

            for resume_index in index + 1..tokens.len().saturating_sub(1) {
                let resume_flag = tokens[resume_index];
                if resume_flag != "--resume" && resume_flag != "resume" && resume_flag != "-r" {
                    continue;
                }

                let Some(session_id) = super::normalize_resume_session_id(tokens[resume_index + 1])
                else {
                    continue;
                };
                found = Some(format!("claude --resume {session_id}"));
            }
        }
    }

    found
}

pub(super) fn infer_skip_permissions_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<bool> {
    let workspace_path = shared::absolute_path(workspace_path)?;
    let project_dir_name = project_dir_name(&workspace_path);
    let project_dir = home_dir
        .join(".claude")
        .join("projects")
        .join(project_dir_name);
    let session_files = shared::find_recent_jsonl_files(&project_dir, Some("agent-"))?;
    for session_file in session_files {
        if let Some(skip_permissions) =
            shared::session_file_skip_permissions_mode(&session_file, 96)
        {
            return Some(skip_permissions);
        }
    }

    None
}

pub(super) fn detect_session_status_in_home(
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    let workspace_path = shared::absolute_path(workspace_path)?;
    let project_dir_name = project_dir_name(&workspace_path);
    let project_dir = home_dir
        .join(".claude")
        .join("projects")
        .join(project_dir_name);
    let session_files = shared::find_recent_jsonl_files(&project_dir, Some("agent-"))?;
    for session_file in session_files {
        if shared::is_file_recently_modified(&session_file, activity_threshold) {
            return Some(WorkspaceStatus::Active);
        }

        let session_stem = session_file.file_stem()?;
        let subagents_dir = project_dir.join(session_stem).join("subagents");
        if shared::any_file_recently_modified(&subagents_dir, ".jsonl", activity_threshold) {
            return Some(WorkspaceStatus::Active);
        }

        if let Some(status) = shared::get_last_message_status_jsonl(
            &session_file,
            "type",
            "user",
            "assistant",
            super::super::SESSION_STATUS_TAIL_BYTES,
        ) {
            return Some(status);
        }
    }

    None
}

pub(super) fn latest_attention_marker_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<String> {
    let workspace_path = shared::absolute_path(workspace_path)?;
    let project_dir_name = project_dir_name(&workspace_path);
    let project_dir = home_dir
        .join(".claude")
        .join("projects")
        .join(project_dir_name);
    let session_files = shared::find_recent_jsonl_files(&project_dir, Some("agent-"))?;
    for session_file in session_files {
        let Some((is_assistant, marker)) = shared::get_last_message_marker_jsonl(
            &session_file,
            "type",
            "user",
            "assistant",
            super::super::SESSION_STATUS_TAIL_BYTES,
        ) else {
            continue;
        };
        if is_assistant {
            return Some(marker);
        }
        return None;
    }

    None
}

pub(crate) fn project_dir_name(abs_path: &Path) -> String {
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

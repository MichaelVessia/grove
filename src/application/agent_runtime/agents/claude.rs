use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::application::agent_runtime::status::WorkspaceStatusObservation;
use crate::domain::{PermissionMode, WorkspaceStatus};

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

pub(super) fn infer_permission_mode_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<PermissionMode> {
    let workspace_path = shared::absolute_path(workspace_path)?;
    let project_dir_name = project_dir_name(&workspace_path);
    let project_dir = home_dir
        .join(".claude")
        .join("projects")
        .join(project_dir_name);
    let session_files = shared::find_recent_jsonl_files(&project_dir, Some("agent-"))?;
    for session_file in session_files {
        if let Some(permission_mode) = shared::session_file_permission_mode(&session_file, 96) {
            return Some(permission_mode);
        }
    }

    None
}

pub(super) fn detect_session_status_in_home(
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatus> {
    status_observation_in_home(workspace_path, home_dir, activity_threshold)
        .map(|observation| observation.status)
}

pub(super) fn status_observation_in_home(
    workspace_path: &Path,
    home_dir: &Path,
    activity_threshold: Duration,
) -> Option<WorkspaceStatusObservation> {
    let workspace_path = shared::absolute_path(workspace_path)?;
    let project_dir_name = project_dir_name(&workspace_path);
    let project_dir = home_dir
        .join(".claude")
        .join("projects")
        .join(project_dir_name);
    let session_files = shared::find_recent_jsonl_files(&project_dir, Some("agent-"))?;
    for session_file in session_files {
        let session_stem = session_file.file_stem()?;
        let subagents_dir = project_dir.join(session_stem).join("subagents");
        let recent_activity = shared::is_file_recently_modified(&session_file, activity_threshold)
            || shared::any_file_recently_modified(&subagents_dir, ".jsonl", activity_threshold);
        if recent_activity {
            return Some(WorkspaceStatusObservation {
                status: WorkspaceStatus::Active,
                recent_activity: true,
                waiting_excerpt: None,
            });
        }

        if let Some(observation) = last_message_observation(&session_file) {
            return Some(observation);
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

fn last_message_observation(path: &Path) -> Option<WorkspaceStatusObservation> {
    let lines = shared::read_tail_lines(path, super::super::SESSION_STATUS_TAIL_BYTES)?;
    for line in lines.iter().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(message_type) = value.get("type").and_then(Value::as_str) else {
            continue;
        };
        if message_type == "user" {
            return Some(WorkspaceStatusObservation {
                status: WorkspaceStatus::Active,
                recent_activity: false,
                waiting_excerpt: None,
            });
        }
        if message_type == "assistant" {
            return Some(WorkspaceStatusObservation {
                status: WorkspaceStatus::Waiting,
                recent_activity: false,
                waiting_excerpt: shared::best_effort_excerpt_from_json_value(&value),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use crate::application::agent_runtime::status::WorkspaceStatusObservation;
    use crate::test_support::unique_test_dir;

    use super::{project_dir_name, status_observation_in_home};

    #[test]
    fn session_signal_claude_extracts_waiting_excerpt_from_message_content() {
        let root = unique_test_dir("claude-observation-excerpt");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-alpha");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir = home
            .join(".claude")
            .join("projects")
            .join(project_dir_name(&workspace_path));
        fs::create_dir_all(&project_dir).expect("project directory should exist");
        let session_file = project_dir.join("session-1.jsonl");
        fs::write(
            &session_file,
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"approve command\"}]}}\n",
        )
        .expect("session file should be written");

        assert_eq!(
            status_observation_in_home(&workspace_path, &home, Duration::from_secs(0)),
            Some(WorkspaceStatusObservation {
                status: crate::domain::WorkspaceStatus::Waiting,
                recent_activity: false,
                waiting_excerpt: Some("approve command".to_string()),
            })
        );
    }

    #[test]
    fn session_signal_claude_marks_recent_activity_independently_of_excerpt() {
        let root = unique_test_dir("claude-observation-recent");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-beta");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir = home
            .join(".claude")
            .join("projects")
            .join(project_dir_name(&workspace_path));
        fs::create_dir_all(&project_dir).expect("project directory should exist");
        let session_file = project_dir.join("session-2.jsonl");
        fs::write(
            &session_file,
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"approve command\"}]}}\n",
        )
        .expect("session file should be written");

        assert_eq!(
            status_observation_in_home(&workspace_path, &home, Duration::from_secs(60)),
            Some(WorkspaceStatusObservation {
                status: crate::domain::WorkspaceStatus::Active,
                recent_activity: true,
                waiting_excerpt: None,
            })
        );
    }
}

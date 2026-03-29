use std::path::Path;
use std::time::Duration;

#[cfg(test)]
use crate::domain::PermissionMode;
use crate::domain::{AgentType, WorkspaceStatus};

use super::agents;
use super::{SESSION_ACTIVITY_THRESHOLD, SessionActivity, WAITING_PATTERNS, WAITING_TAIL_LINES};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct WorkspaceStatusObservation {
    pub(crate) status: WorkspaceStatus,
    pub(crate) recent_activity: bool,
    pub(crate) waiting_excerpt: Option<String>,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct BackgroundStatusObservationContext<'a> {
    pub(crate) agent: AgentType,
    pub(crate) workspace_path: &'a Path,
    pub(crate) home_dir: Option<&'a Path>,
    pub(crate) activity_threshold: Duration,
    pub(crate) session_name: &'a str,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn detect_status_with_session_override(
    output: &str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
    agent: AgentType,
    workspace_path: &Path,
    session_name: &str,
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
        session_name,
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
    session_name: &str,
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

    if let Some(exit_code) = read_session_exit_code(session_name) {
        return if exit_code == 0 {
            WorkspaceStatus::Done
        } else {
            WorkspaceStatus::Error
        };
    }

    if detect_waiting_prompt(output).is_some() {
        return WorkspaceStatus::Waiting;
    }

    if has_unclosed_tag_ci(output, b"<thinking>", b"</thinking>")
        || has_unclosed_tag_ci(output, b"<internal_monologue>", b"</internal_monologue>")
        || contains_ascii_ci(output.as_bytes(), b"thinking...")
        || contains_ascii_ci(output.as_bytes(), b"reasoning about")
    {
        return WorkspaceStatus::Thinking;
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
    pub(crate) session_name: &'a str,
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
        context.session_name,
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

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn workspace_status_observation_in_home(
    context: BackgroundStatusObservationContext<'_>,
) -> Option<WorkspaceStatusObservation> {
    if !context.workspace_path.exists() {
        return None;
    }

    if let Some(exit_code) = read_session_exit_code(context.session_name) {
        return Some(WorkspaceStatusObservation {
            status: if exit_code == 0 {
                WorkspaceStatus::Done
            } else {
                WorkspaceStatus::Error
            },
            recent_activity: false,
            waiting_excerpt: None,
        });
    }

    let home_dir = context.home_dir?;
    agents::status_observation_in_home(
        context.agent,
        context.workspace_path,
        home_dir,
        context.activity_threshold,
    )
}

#[cfg(test)]
pub(super) fn infer_claude_permission_mode_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<PermissionMode> {
    agents::infer_permission_mode_in_home(AgentType::Claude, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn infer_codex_permission_mode_in_home(
    workspace_path: &Path,
    home_dir: &Path,
) -> Option<PermissionMode> {
    agents::infer_permission_mode_in_home(AgentType::Codex, workspace_path, home_dir)
}

#[cfg(test)]
pub(super) fn codex_session_permission_mode(path: &Path) -> Option<PermissionMode> {
    agents::codex_session_permission_mode(path)
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

pub(crate) fn exit_code_file_path(session_name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/tmp/grove-exit-{session_name}"))
}

fn read_session_exit_code(session_name: &str) -> Option<i32> {
    let path = exit_code_file_path(session_name);
    let content = std::fs::read_to_string(path).ok()?;
    content.trim().parse::<i32>().ok()
}

fn rfind_ascii_ci(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > haystack.len() {
        return None;
    }
    for i in (0..=haystack.len() - needle.len()).rev() {
        if haystack[i..i + needle.len()]
            .iter()
            .zip(needle)
            .all(|(h, n)| h.to_ascii_lowercase() == *n)
        {
            return Some(i);
        }
    }
    None
}

fn contains_ascii_ci(haystack: &[u8], needle: &[u8]) -> bool {
    rfind_ascii_ci(haystack, needle).is_some()
}

fn has_unclosed_tag_ci(text: &str, open_tag: &[u8], close_tag: &[u8]) -> bool {
    let bytes = text.as_bytes();
    let Some(open_index) = rfind_ascii_ci(bytes, open_tag) else {
        return false;
    };

    match rfind_ascii_ci(bytes, close_tag) {
        Some(close_index) => close_index < open_index,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use crate::domain::{AgentType, WorkspaceStatus};
    use crate::test_support::unique_test_dir;

    use super::super::SessionActivity;
    use super::super::agents::claude_project_dir_name;
    use super::{
        BackgroundStatusObservationContext, StatusOverrideContext, WorkspaceStatusObservation,
        detect_agent_session_status_in_home, detect_status,
        detect_status_with_session_override_in_home, detect_waiting_prompt, exit_code_file_path,
        latest_claude_assistant_attention_marker_in_home,
        latest_codex_assistant_attention_marker_in_home, read_session_exit_code,
        workspace_status_observation_in_home,
    };

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
            detect_status(
                output,
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Waiting
        );
    }

    #[test]
    fn waiting_prompt_does_not_treat_plain_shell_angle_prompt_as_waiting() {
        let output = "build finished\n> \n";
        assert_eq!(detect_waiting_prompt(output), None);
    }

    #[test]
    fn waiting_prompt_does_not_treat_generic_skills_hint_as_waiting() {
        let output = "Done.\n› Use /skills to list available skills\n";
        assert_eq!(detect_waiting_prompt(output), None);
    }

    #[test]
    fn claude_session_file_marks_waiting_when_last_message_is_assistant() {
        let root = unique_test_dir("grove-claude-session");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-alpha");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir_name = claude_project_dir_name(&workspace_path);
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
    fn claude_attention_marker_exists_when_last_message_is_assistant() {
        let root = unique_test_dir("grove-claude-attention-marker");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-alpha");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir_name = claude_project_dir_name(&workspace_path);
        let project_dir = home.join(".claude").join("projects").join(project_dir_name);
        fs::create_dir_all(&project_dir).expect("project directory should exist");

        let session_file = project_dir.join("session-1.jsonl");
        fs::write(
            &session_file,
            "{\"type\":\"system\"}\n{\"type\":\"assistant\"}\n",
        )
        .expect("session file should be written");

        let marker = latest_claude_assistant_attention_marker_in_home(&workspace_path, &home);
        assert!(marker.is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn codex_attention_marker_exists_when_last_message_is_assistant() {
        let root = unique_test_dir("grove-codex-attention-marker");
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

        let marker = latest_codex_assistant_attention_marker_in_home(&workspace_path, &home);
        assert!(marker.is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn status_override_uses_session_files_for_active_waiting() {
        let root = unique_test_dir("grove-status-override");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-gamma");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir_name = claude_project_dir_name(&workspace_path);
        let project_dir = home.join(".claude").join("projects").join(project_dir_name);
        fs::create_dir_all(&project_dir).expect("project directory should exist");
        let session_file = project_dir.join("session-2.jsonl");
        fs::write(&session_file, "{\"type\":\"assistant\"}\n")
            .expect("session file should be written");

        let status = detect_status_with_session_override_in_home(StatusOverrideContext {
            output: "plain output",
            session_activity: SessionActivity::Active,
            is_main: false,
            has_live_session: true,
            supported_agent: true,
            agent: AgentType::Claude,
            workspace_path: &workspace_path,
            home_dir: Some(&home),
            activity_threshold: Duration::from_secs(0),
            session_name: "no-session",
        });
        assert_eq!(status, WorkspaceStatus::Waiting);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn background_status_signal_reads_waiting_from_session_files_without_pane_text() {
        let root = unique_test_dir("grove-background-signal");
        let home = root.join("home");
        let workspace_path = root.join("ws").join("feature-delta");
        fs::create_dir_all(&home).expect("home directory should exist");
        fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

        let project_dir_name = claude_project_dir_name(&workspace_path);
        let project_dir = home.join(".claude").join("projects").join(project_dir_name);
        fs::create_dir_all(&project_dir).expect("project directory should exist");
        let session_file = project_dir.join("session-3.jsonl");
        fs::write(
            &session_file,
            "{\"type\":\"system\"}\n{\"type\":\"assistant\"}\n",
        )
        .expect("session file should be written");

        let observation =
            workspace_status_observation_in_home(BackgroundStatusObservationContext {
                agent: AgentType::Claude,
                workspace_path: &workspace_path,
                home_dir: Some(&home),
                activity_threshold: Duration::from_secs(0),
                session_name: "no-session",
            });
        assert_eq!(
            observation,
            Some(WorkspaceStatusObservation {
                status: WorkspaceStatus::Waiting,
                recent_activity: false,
                waiting_excerpt: None,
            })
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn status_resolution_core_priority_order() {
        assert_eq!(
            detect_status(
                "thinking...",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Thinking
        );
        assert_eq!(
            detect_status(
                "allow edit? [y/n]",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Waiting
        );
        assert_eq!(
            detect_status("", SessionActivity::Active, false, true, true, "no-session"),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status("", SessionActivity::Idle, false, false, true, "no-session"),
            WorkspaceStatus::Idle
        );
        assert_eq!(
            detect_status(
                "",
                SessionActivity::Active,
                false,
                true,
                false,
                "no-session"
            ),
            WorkspaceStatus::Unsupported
        );
        assert_eq!(
            detect_status(
                "warning: failed to login mcp\nline\nline\n> Implement {feature}\n? for shortcuts\n",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Waiting
        );
        assert_eq!(
            detect_status(
                "Do you want to continue?",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Waiting
        );
        assert_eq!(
            detect_status("", SessionActivity::Active, true, true, true, "no-session"),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status("", SessionActivity::Idle, true, false, true, "no-session"),
            WorkspaceStatus::Main
        );
    }

    #[test]
    fn exit_code_file_determines_done_status() {
        let session = "grove-test-exit-done";
        let path = exit_code_file_path(session);
        fs::write(&path, "0\n").expect("exit code file should be written");

        let status = detect_status("", SessionActivity::Active, false, true, true, session);
        assert_eq!(status, WorkspaceStatus::Done);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn exit_code_file_determines_error_status() {
        let session = "grove-test-exit-error";
        let path = exit_code_file_path(session);
        fs::write(&path, "1\n").expect("exit code file should be written");

        let status = detect_status("", SessionActivity::Active, false, true, true, session);
        assert_eq!(status, WorkspaceStatus::Error);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn exit_code_file_nonzero_is_error_regardless_of_code() {
        let session = "grove-test-exit-signal";
        let path = exit_code_file_path(session);
        fs::write(&path, "130\n").expect("exit code file should be written");

        let status = detect_status("", SessionActivity::Active, false, true, true, session);
        assert_eq!(status, WorkspaceStatus::Error);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_exit_code_file_falls_through_to_heuristics() {
        let status = detect_status(
            "allow edit? [y/n]",
            SessionActivity::Active,
            false,
            true,
            true,
            "grove-test-no-such-session",
        );
        assert_eq!(status, WorkspaceStatus::Waiting);
    }

    #[test]
    fn invalid_exit_code_file_falls_through_to_heuristics() {
        let session = "grove-test-exit-invalid";
        let path = exit_code_file_path(session);
        fs::write(&path, "not-a-number\n").expect("exit code file should be written");

        let status = detect_status(
            "thinking...",
            SessionActivity::Active,
            false,
            true,
            true,
            session,
        );
        assert_eq!(status, WorkspaceStatus::Thinking);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn exit_code_file_takes_priority_over_text_content() {
        let session = "grove-test-exit-priority";
        let path = exit_code_file_path(session);
        fs::write(&path, "0\n").expect("exit code file should be written");

        let status = detect_status(
            "allow edit? [y/n]",
            SessionActivity::Active,
            false,
            true,
            true,
            session,
        );
        assert_eq!(status, WorkspaceStatus::Done);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn text_based_done_error_patterns_no_longer_trigger_without_exit_file() {
        assert_eq!(
            detect_status(
                "panic: bad",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status(
                "task completed successfully",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status(
                "error: permission denied\n",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Active
        );
    }

    #[test]
    fn read_session_exit_code_parses_valid_file() {
        let session = "grove-test-read-exit";
        let path = exit_code_file_path(session);
        fs::write(&path, "42\n").expect("exit code file should be written");

        assert_eq!(read_session_exit_code(session), Some(42));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_session_exit_code_returns_none_for_missing_file() {
        assert_eq!(read_session_exit_code("grove-test-nonexistent"), None);
    }

    #[test]
    fn read_session_exit_code_returns_none_for_invalid_content() {
        let session = "grove-test-read-exit-bad";
        let path = exit_code_file_path(session);
        fs::write(&path, "garbage").expect("exit code file should be written");

        assert_eq!(read_session_exit_code(session), None);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn status_resolution_requires_unclosed_thinking_tags() {
        assert_eq!(
            detect_status(
                "<thinking>\nworking\n</thinking>",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status(
                "<thinking>\nworking\n",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Thinking
        );
        assert_eq!(
            detect_status(
                "<internal_monologue>\nworking\n",
                SessionActivity::Active,
                false,
                true,
                true,
                "no-session"
            ),
            WorkspaceStatus::Thinking
        );
    }
}

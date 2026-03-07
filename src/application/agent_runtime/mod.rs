use std::path::PathBuf;
use std::time::Duration;

use crate::domain::{AgentType, Workspace};

mod agents;
pub mod capture;
pub mod execution;
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
    kill_workspace_session_commands_for_existing_sessions, workspace_session_name_matches,
    workspace_session_names_for_cleanup,
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
pub use reconciliation::{reconcile_with_sessions, reconcile_with_sessions_owned};
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
    live_preview_capture_target_for_tab, live_preview_session_for_tab, session_name_for_task,
    session_name_for_task_worktree, session_name_for_workspace_in_project,
    session_name_for_workspace_ref, shell_session_name_for_workspace,
    workspace_can_enter_interactive, workspace_can_start_agent, workspace_can_stop_agent,
    workspace_session_for_preview_tab,
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
pub(crate) use status::{detect_status_with_session_override, latest_assistant_attention_marker};

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
const SESSION_LOOKUP_CACHE_MAX_ENTRIES: usize = 1024;
const SESSION_LOOKUP_EVICTION_TTL: Duration = Duration::from_secs(300);
const MESSAGE_STATUS_CACHE_MAX_ENTRIES: usize = 1024;
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
pub struct TaskLaunchRequest {
    pub task_slug: String,
    pub task_root: PathBuf,
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
mod tests {
    mod shared {
        pub(super) use std::collections::HashSet;
        pub(super) use std::fs;
        pub(super) use std::path::PathBuf;
        pub(super) use std::process;
        pub(super) use std::time::{Duration, SystemTime, UNIX_EPOCH};

        pub(super) use rusqlite::Connection;

        pub(super) use super::super::{
            CaptureChange, CommandExecutionMode, CommandExecutor, LaunchPlan, LaunchRequest,
            LauncherScript, LivePreviewTarget, SessionActivity, agent_supports_in_pane_restart,
            build_launch_plan, build_shell_launch_plan, default_agent_command,
            detect_agent_session_status_in_home, detect_status,
            detect_status_with_session_override_in_home, detect_waiting_prompt,
            evaluate_capture_change, execute_command_with, execute_commands,
            execute_commands_for_mode, execute_commands_with, execute_commands_with_executor,
            execute_launch_plan, execute_launch_plan_for_mode, execute_launch_plan_with,
            execute_launch_plan_with_executor, execute_launch_request_with_result_for_mode,
            execute_restart_workspace_in_pane_with_result,
            execute_stop_workspace_with_result_for_mode, extract_agent_resume_command,
            git_preview_session_if_ready, git_session_name_for_workspace,
            kill_workspace_session_command, kill_workspace_session_commands,
            kill_workspace_session_commands_for_existing_sessions, launch_request_for_workspace,
            live_preview_agent_session, live_preview_capture_target_for_tab,
            live_preview_session_for_tab, poll_interval, reconcile_with_sessions,
            restart_workspace_in_pane_with_io, sanitize_workspace_name, session_name_for_workspace,
            session_name_for_workspace_ref, shell_launch_request_for_workspace,
            shell_session_name_for_workspace, stop_plan, strip_mouse_fragments,
            tmux_capture_error_indicates_missing_session,
            tmux_launch_error_indicates_duplicate_session, trimmed_nonempty,
            workspace_can_enter_interactive, workspace_can_start_agent, workspace_can_stop_agent,
            workspace_session_for_preview_tab, workspace_session_name_matches,
            workspace_session_names_for_cleanup, workspace_should_poll_status,
            workspace_status_session_target, workspace_status_targets_for_polling,
            workspace_status_targets_for_polling_with_live_preview,
        };
        pub(super) use crate::domain::{AgentType, Workspace, WorkspaceStatus};

        pub(super) fn fixture_workspace(name: &str, is_main: bool) -> Workspace {
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

        pub(super) fn unique_test_dir(prefix: &str) -> PathBuf {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", process::id()));
            fs::create_dir_all(&path).expect("test directory should be created");
            path
        }

        #[derive(Default)]
        pub(super) struct RecordingCommandExecutor {
            pub(super) commands: Vec<Vec<String>>,
            pub(super) launcher_scripts: Vec<(PathBuf, String)>,
        }

        impl CommandExecutor for RecordingCommandExecutor {
            fn execute(&mut self, command: &[String]) -> std::io::Result<()> {
                self.commands.push(command.to_vec());
                Ok(())
            }

            fn write_launcher_script(&mut self, script: &LauncherScript) -> std::io::Result<()> {
                self.launcher_scripts
                    .push((script.path.clone(), script.contents.clone()));
                Ok(())
            }
        }
    }

    use shared::*;

    mod capture_clean {
        use super::*;

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
            assert_eq!(change.render_output, "A\u{1b}[31mB\u{1b}[39m C\n");
        }

        #[test]
        fn capture_change_strips_sgr_and_preserves_unicode_text() {
            let raw = "start🙂\u{1b}[31m中\u{1b}[0mend\n";
            let change = evaluate_capture_change(None, raw);
            assert_eq!(change.cleaned_output, "start🙂中end\n");
        }

        #[test]
        fn capture_change_strips_terminal_control_bytes() {
            let raw = "A\u{000e}B\u{000f}C\r\n";
            let change = evaluate_capture_change(None, raw);
            assert_eq!(change.cleaned_output, "ABC\n");
            assert_eq!(change.render_output, "ABC\n");
        }

        #[test]
        fn capture_change_drops_charset_escape_with_multibyte_suffix() {
            let raw = "A\u{1b}(🙂B";
            let change = evaluate_capture_change(None, raw);
            assert_eq!(change.cleaned_output, "AB");
            assert_eq!(change.render_output, "AB");
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
            assert_eq!(strip_mouse_fragments("value[?9999h"), "value[?9999h");
        }

        #[test]
        fn strip_mouse_fragments_removes_boundary_prefixed_partial_sequences() {
            assert_eq!(strip_mouse_fragments("prompt M[<64;107;16M"), "prompt ");
            assert_eq!(strip_mouse_fragments("prompt m[<65;107;14"), "prompt ");
        }

        #[test]
        fn strip_mouse_fragments_removes_modes_and_partials_in_single_input() {
            let raw = "A\u{1b}[?1002hB[<65;107;16MC";
            assert_eq!(strip_mouse_fragments(raw), "ABC");
        }

        #[test]
        fn strip_mouse_fragments_preserves_non_mouse_candidate_prefixes() {
            assert_eq!(strip_mouse_fragments("Mnot-mouse"), "Mnot-mouse");
            assert_eq!(strip_mouse_fragments("text [<x;1;2"), "text [<x;1;2");
        }
    }
    mod restart_exec {
        use super::*;

        #[test]
        fn extract_agent_resume_command_returns_latest_claude_resume_command() {
            let output = "\
        old hint: claude --resume old-session\n\
        another line\n\
        run this next: claude --resume abc123xyz.\n";
            let resume = extract_agent_resume_command(AgentType::Claude, output);
            assert_eq!(resume.as_deref(), Some("claude --resume abc123xyz"));
        }

        #[test]
        fn extract_agent_resume_command_parses_real_claude_exit_output() {
            let output = "\
        Press Ctrl-C again to exit\n\
        \n\
        Resume this session with:\n\
        claude --resume e610b734-e6b8-4b1f-b42f-f3ddeb817467\n";
            let resume = extract_agent_resume_command(AgentType::Claude, output);
            assert_eq!(
                resume.as_deref(),
                Some("claude --resume e610b734-e6b8-4b1f-b42f-f3ddeb817467")
            );
        }

        #[test]
        fn extract_agent_resume_command_parses_claude_resume_with_flag_before_resume() {
            let output = "\
        Resume this session with:\n\
        claude --dangerously-skip-permissions --resume e610b734-e6b8-4b1f-b42f-f3ddeb817467\n";
            let resume = extract_agent_resume_command(AgentType::Claude, output);
            assert_eq!(
                resume.as_deref(),
                Some("claude --resume e610b734-e6b8-4b1f-b42f-f3ddeb817467")
            );
        }

        #[test]
        fn extract_agent_resume_command_parses_claude_short_resume_flag() {
            let output = "Run this next: claude -r e610b734-e6b8-4b1f-b42f-f3ddeb817467";
            let resume = extract_agent_resume_command(AgentType::Claude, output);
            assert_eq!(
                resume.as_deref(),
                Some("claude --resume e610b734-e6b8-4b1f-b42f-f3ddeb817467")
            );
        }

        #[test]
        fn extract_agent_resume_command_parses_real_codex_exit_output() {
            let output = "\
        To continue this session, run codex resume 019c83c1-26c3-7fb0-bd4d-51bb9d6e7701\n";
            let resume = extract_agent_resume_command(AgentType::Codex, output);
            assert_eq!(
                resume.as_deref(),
                Some("codex resume 019c83c1-26c3-7fb0-bd4d-51bb9d6e7701")
            );
        }

        #[test]
        fn extract_agent_resume_command_normalizes_codex_dash_resume_to_subcommand() {
            let output = "To continue this session, run codex --resume 019c83c1-26c3-7fb0-bd4d-51bb9d6e7701\n";
            let resume = extract_agent_resume_command(AgentType::Codex, output);
            assert_eq!(
                resume.as_deref(),
                Some("codex resume 019c83c1-26c3-7fb0-bd4d-51bb9d6e7701")
            );
        }

        #[test]
        fn extract_agent_resume_command_ignores_codex_prose_resume_phrase() {
            let output = "\
        No TODOs.\n\
        codex resume is\n\
        Token usage: total=1011482\n\
        To continue this session, run codex resume 019c92cf-2410-7ec3-a8bd-b203b83a6fba\n";
            let resume = extract_agent_resume_command(AgentType::Codex, output);
            assert_eq!(
                resume.as_deref(),
                Some("codex resume 019c92cf-2410-7ec3-a8bd-b203b83a6fba")
            );
        }

        #[test]
        fn extract_agent_resume_command_ignores_codex_placeholder_id() {
            let output = "\
        codex resume <id>\n\
        Token usage: total=587331\n\
        To continue this session, run codex resume 019c92cf-2410-7ec3-a8bd-b203b83a6fba\n";
            let resume = extract_agent_resume_command(AgentType::Codex, output);
            assert_eq!(
                resume.as_deref(),
                Some("codex resume 019c92cf-2410-7ec3-a8bd-b203b83a6fba")
            );
        }

        #[test]
        fn extract_agent_resume_command_ignores_codex_rust_string_artifact() {
            let output = "\
        codex resume run-1234\".to_string(),\n\
        Token usage: total=587331\n\
        To continue this session, run codex resume 019c92cf-2410-7ec3-a8bd-b203b83a6fba\n";
            let resume = extract_agent_resume_command(AgentType::Codex, output);
            assert_eq!(
                resume.as_deref(),
                Some("codex resume 019c92cf-2410-7ec3-a8bd-b203b83a6fba")
            );
        }

        #[test]
        fn codex_session_skip_permissions_mode_detects_approval_policy() {
            let root = unique_test_dir("grove-codex-skip-mode");
            let session_file = root.join("session.jsonl");
            fs::write(
                &session_file,
                "\
        {\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/tmp/ws\"}}\n\
        {\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"developer\",\"content\":[{\"type\":\"input_text\",\"text\":\"Approval policy is currently never.\"}]}}\n",
            )
            .expect("session file should be written");
            assert_eq!(
                super::super::codex_session_skip_permissions_mode(&session_file),
                Some(true)
            );

            fs::write(
                &session_file,
                "\
        {\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/tmp/ws\"}}\n\
        {\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"<approval_policy>on-request</approval_policy>\"}]}}\n",
            )
            .expect("session file should be rewritten");
            assert_eq!(
                super::super::codex_session_skip_permissions_mode(&session_file),
                Some(false)
            );

            let _ = fs::remove_dir_all(root);
        }

        #[test]
        fn infer_codex_skip_permissions_in_home_uses_workspace_session_file() {
            let root = unique_test_dir("grove-codex-skip-infer");
            let home = root.join("home");
            let workspace_path = root.join("ws").join("feature-gamma");
            fs::create_dir_all(&home).expect("home directory should exist");
            fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

            let sessions_dir = home
                .join(".codex")
                .join("sessions")
                .join("2026")
                .join("02")
                .join("25");
            fs::create_dir_all(&sessions_dir).expect("sessions directory should exist");

            let older = sessions_dir.join("older.jsonl");
            fs::write(
                &older,
                format!(
                    "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\"}}}}\n\
        {{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"developer\",\"content\":[{{\"type\":\"input_text\",\"text\":\"<approval_policy>on-request</approval_policy>\"}}]}}}}\n",
                    workspace_path.display()
                ),
            )
            .expect("older session should be written");

            let newer = sessions_dir.join("newer.jsonl");
            fs::write(
                &newer,
                format!(
                    "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\"}}}}\n\
        {{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"developer\",\"content\":[{{\"type\":\"input_text\",\"text\":\"Approval policy is currently never.\"}}]}}}}\n",
                    workspace_path.display()
                ),
            )
            .expect("newer session should be written");

            std::thread::sleep(Duration::from_millis(10));
            fs::write(
                &newer,
                format!(
                    "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{}\"}}}}\n\
        {{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"developer\",\"content\":[{{\"type\":\"input_text\",\"text\":\"Approval policy is currently never.\"}}]}}}}\n",
                    workspace_path.display()
                ),
            )
            .expect("newer session rewrite should refresh mtime");

            assert_eq!(
                super::super::infer_codex_skip_permissions_in_home(&workspace_path, &home),
                Some(true)
            );

            let _ = fs::remove_dir_all(root);
        }

        #[test]
        fn infer_claude_skip_permissions_in_home_uses_project_session_file() {
            let root = unique_test_dir("grove-claude-skip-infer");
            let home = root.join("home");
            let workspace_path = root.join("ws").join("feature-zeta");
            fs::create_dir_all(&home).expect("home directory should exist");
            fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

            let project_dir_name = super::super::claude_project_dir_name(&workspace_path);
            let project_dir = home.join(".claude").join("projects").join(project_dir_name);
            fs::create_dir_all(&project_dir).expect("project directory should exist");
            let session_file = project_dir.join("session-1.jsonl");

            fs::write(
                &session_file,
                "{\"type\":\"user\",\"message\":\"<approval_policy>never</approval_policy>\"}\n",
            )
            .expect("session file should be written");
            assert_eq!(
                super::super::infer_claude_skip_permissions_in_home(&workspace_path, &home),
                Some(true)
            );

            fs::write(
                &session_file,
                "{\"type\":\"user\",\"message\":\"<approval_policy>on-request</approval_policy>\"}\n",
            )
            .expect("session file should be rewritten");
            assert_eq!(
                super::super::infer_claude_skip_permissions_in_home(&workspace_path, &home),
                Some(false)
            );

            fs::write(
                &session_file,
                "{\"type\":\"user\",\"permissionMode\":\"bypassPermissions\"}\n",
            )
            .expect("session file should be rewritten");
            assert_eq!(
                super::super::infer_claude_skip_permissions_in_home(&workspace_path, &home),
                Some(true)
            );

            fs::write(
                &session_file,
                "{\"type\":\"user\",\"permissionMode\":\"default\"}\n",
            )
            .expect("session file should be rewritten");
            assert_eq!(
                super::super::infer_claude_skip_permissions_in_home(&workspace_path, &home),
                Some(false)
            );

            let _ = fs::remove_dir_all(root);
        }

        #[test]
        fn infer_opencode_skip_permissions_in_home_uses_message_data() {
            let root = unique_test_dir("grove-opencode-skip-infer");
            let home = root.join("home");
            let workspace_path = root.join("ws").join("feature-eta");
            fs::create_dir_all(&home).expect("home directory should exist");
            fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

            let data_dir = home.join(".local").join("share").join("opencode");
            fs::create_dir_all(&data_dir).expect("opencode data directory should exist");
            let database_path = data_dir.join("opencode.db");
            let connection = Connection::open(&database_path).expect("database should open");
            connection
                .execute_batch(
                    "CREATE TABLE session (id TEXT PRIMARY KEY, directory TEXT NOT NULL, time_updated INTEGER NOT NULL);
                     CREATE TABLE message (
                       id TEXT PRIMARY KEY,
                       session_id TEXT NOT NULL,
                       time_created INTEGER NOT NULL,
                       time_updated INTEGER NOT NULL,
                       data TEXT NOT NULL
                     );",
                )
                .expect("schema should be created");
            connection
                .execute(
                    "INSERT INTO session (id, directory, time_updated) VALUES (?1, ?2, ?3)",
                    (
                        "session-skip",
                        workspace_path.to_string_lossy().to_string(),
                        1_i64,
                    ),
                )
                .expect("session row should be inserted");
            connection
                .execute(
                    "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                    (
                        "message-skip",
                        "session-skip",
                        1_i64,
                        1_i64,
                        "{\"role\":\"user\",\"text\":\"Approval policy is currently never.\"}",
                    ),
                )
                .expect("message row should be inserted");

            assert_eq!(
                super::super::infer_opencode_skip_permissions_in_home(&workspace_path, &home),
                Some(true)
            );

            let _ = fs::remove_dir_all(root);
        }

        #[test]
        fn extract_agent_resume_command_parses_real_opencode_banner_output() {
            let output = "opencode -s ses_36d243142ffeYteys2MXS86Nnt";
            let resume = extract_agent_resume_command(AgentType::OpenCode, output);
            assert_eq!(
                resume.as_deref(),
                Some("opencode -s ses_36d243142ffeYteys2MXS86Nnt")
            );
        }

        #[test]
        fn extract_agent_resume_command_parses_opencode_continue_output() {
            let output = "To continue this session, run opencode --continue";
            let resume = extract_agent_resume_command(AgentType::OpenCode, output);
            assert_eq!(resume.as_deref(), Some("opencode --continue"));
        }

        #[test]
        fn extract_agent_resume_command_ignores_unrelated_output() {
            let output = "claude --resume abc123xyz";
            assert!(extract_agent_resume_command(AgentType::OpenCode, output).is_none());
        }

        #[test]
        fn restart_workspace_in_pane_with_io_sends_exit_and_resume_commands() {
            let workspace = fixture_workspace("feature-a", false);
            let mut commands = Vec::new();
            let mut captures = vec![
                "still shutting down".to_string(),
                "resume with: claude --resume run-1234".to_string(),
            ];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                false,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands,
                vec![
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-l".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "/exit".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "Enter".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "claude --resume run-1234".to_string(),
                        "Enter".to_string(),
                    ],
                ]
            );
        }

        #[test]
        fn restart_workspace_in_pane_with_io_returns_error_when_resume_missing() {
            let workspace = fixture_workspace("feature-a", false);
            let mut captures = vec![
                "still shutting down".to_string(),
                "no resume command".to_string(),
            ];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                false,
                &[],
                |_command| Ok(()),
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            let error = result.expect_err("missing resume command should fail");
            assert!(error.contains("resume command not found"));
        }

        #[test]
        fn restart_workspace_in_pane_with_io_resume_missing_error_includes_capture_excerpt() {
            let workspace = fixture_workspace("feature-a", false);
            let mut call_count = 0_u8;

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                false,
                &[],
                |_command| Ok(()),
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    call_count = call_count.saturating_add(1);
                    if call_count == 1 {
                        return Ok("still shutting down".to_string());
                    }
                    Ok(
                        "\n\n  no resume command in output  \n  Claude exited successfully  "
                            .to_string(),
                    )
                },
            );

            let error = result.expect_err("missing resume command should fail");
            assert!(error.contains("resume command not found"));
            assert!(error.contains("last_output='"));
            assert!(error.contains("no resume command in output"));
            assert!(error.contains("Claude exited successfully"));
        }

        #[test]
        fn restart_workspace_in_pane_with_io_sends_ctrl_c_for_opencode() {
            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::OpenCode;
            let mut commands = Vec::new();
            let mut captures = vec!["opencode -s ses_36d243142ffeYteys2MXS86Nnt".to_string()];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                false,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands,
                vec![
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "C-c".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "opencode -s ses_36d243142ffeYteys2MXS86Nnt".to_string(),
                        "Enter".to_string(),
                    ],
                ]
            );
        }

        #[test]
        fn restart_workspace_in_pane_with_io_uses_opencode_db_resume_when_output_missing() {
            let root = unique_test_dir("grove-opencode-restart-resume-fallback");
            let home = root.join("home");
            let workspace_path = root.join("ws").join("feature-opencode");
            fs::create_dir_all(&home).expect("home directory should exist");
            fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

            let data_dir = home.join(".local").join("share").join("opencode");
            fs::create_dir_all(&data_dir).expect("opencode data directory should exist");
            let database_path = data_dir.join("opencode.db");
            let connection = Connection::open(&database_path).expect("database should open");
            connection
                .execute_batch(
                    "CREATE TABLE session (id TEXT PRIMARY KEY, directory TEXT NOT NULL, time_updated INTEGER NOT NULL);",
                )
                .expect("schema should be created");
            connection
                .execute(
                    "INSERT INTO session (id, directory, time_updated) VALUES (?1, ?2, ?3)",
                    (
                        "session-fallback",
                        workspace_path.to_string_lossy().to_string(),
                        1_i64,
                    ),
                )
                .expect("session row should be inserted");

            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::OpenCode;
            workspace.path = workspace_path.clone();
            let mut commands = Vec::new();

            let result = super::super::restart_workspace_in_pane_with_io_in_home(
                &workspace,
                false,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    Ok("no resume command in output".to_string())
                },
                Some(home.as_path()),
            );

            assert!(result.is_ok());
            assert_eq!(
                commands[1],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "opencode -s session-fallback".to_string(),
                    "Enter".to_string(),
                ]
            );

            let _ = fs::remove_dir_all(root);
        }

        #[test]
        fn restart_workspace_in_pane_with_io_sends_ctrl_c_for_codex() {
            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::Codex;
            let mut commands = Vec::new();
            let mut captures = vec!["run codex resume run-1234".to_string()];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                false,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands,
                vec![
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "C-c".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "codex resume run-1234".to_string(),
                        "Enter".to_string(),
                    ],
                ]
            );
        }

        #[test]
        fn restart_workspace_in_pane_with_io_applies_agent_env_before_resume() {
            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::Codex;
            let mut commands = Vec::new();
            let mut captures = vec!["run codex resume run-1234".to_string()];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                false,
                &[
                    ("FOO".to_string(), "bar".to_string()),
                    ("BAR".to_string(), "baz".to_string()),
                ],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands[1],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "export FOO='bar' BAR='baz'".to_string(),
                    "Enter".to_string(),
                ]
            );
            assert_eq!(
                commands[2],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "codex resume run-1234".to_string(),
                    "Enter".to_string(),
                ]
            );
        }

        #[test]
        fn execute_restart_workspace_in_pane_with_result_returns_workspace_context() {
            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::OpenCode;

            let result =
                execute_restart_workspace_in_pane_with_result(&workspace, false, Vec::new());
            assert_eq!(result.workspace_name, "feature-a");
            assert_eq!(
                result.workspace_path,
                PathBuf::from("/repos/grove-feature-a")
            );
            assert_eq!(result.session_name, "grove-ws-feature-a");
            assert!(result.result.is_err());
        }

        #[test]
        fn restart_workspace_in_pane_with_io_adds_skip_permissions_for_codex_resume() {
            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::Codex;
            let mut commands = Vec::new();
            let mut captures = vec!["run codex resume run-1234".to_string()];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                true,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands[1],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "codex --dangerously-bypass-approvals-and-sandbox resume run-1234".to_string(),
                    "Enter".to_string(),
                ]
            );
        }

        #[test]
        fn restart_workspace_in_pane_with_io_adds_skip_permissions_for_claude_resume() {
            let workspace = fixture_workspace("feature-a", false);
            let mut commands = Vec::new();
            let mut captures = vec!["resume with: claude --resume run-1234".to_string()];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                true,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands[2],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "claude --dangerously-skip-permissions --resume run-1234".to_string(),
                    "Enter".to_string(),
                ]
            );
        }

        #[test]
        fn restart_workspace_in_pane_with_io_adds_skip_permissions_for_codex_dash_resume() {
            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::Codex;
            let mut commands = Vec::new();
            let mut captures = vec!["run codex --resume run-1234".to_string()];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                true,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands[1],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "codex --dangerously-bypass-approvals-and-sandbox resume run-1234".to_string(),
                    "Enter".to_string(),
                ]
            );
        }

        #[test]
        fn restart_workspace_in_pane_with_io_adds_skip_permissions_for_opencode_resume() {
            let mut workspace = fixture_workspace("feature-a", false);
            workspace.agent = AgentType::OpenCode;
            let mut commands = Vec::new();
            let mut captures = vec!["opencode -s ses_36d243142ffeYteys2MXS86Nnt".to_string()];

            let result = restart_workspace_in_pane_with_io(
                &workspace,
                true,
                &[],
                |command| {
                    commands.push(command.to_vec());
                    Ok(())
                },
                |_session_name, _scrollback_lines, _include_escape_sequences| {
                    if captures.is_empty() {
                        return Ok(String::new());
                    }
                    Ok(captures.remove(0))
                },
            );

            assert!(result.is_ok());
            assert_eq!(
                commands[1],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-a".to_string(),
                    "OPENCODE_PERMISSION='{\"*\":\"allow\"}' opencode -s ses_36d243142ffeYteys2MXS86Nnt"
                        .to_string(),
                    "Enter".to_string(),
                ]
            );
        }

        #[test]
        fn execute_commands_runs_successful_command_sequence() {
            let commands = vec![
                vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
                Vec::new(),
            ];
            assert!(execute_commands(&commands).is_ok());
        }

        #[test]
        fn execute_commands_returns_error_for_missing_program() {
            let commands = vec![vec![
                "grove-this-command-does-not-exist".to_string(),
                "arg".to_string(),
            ]];
            assert!(execute_commands(&commands).is_err());
        }

        #[test]
        fn execute_commands_for_mode_process_returns_string_errors() {
            let commands = vec![vec![
                "grove-this-command-does-not-exist".to_string(),
                "arg".to_string(),
            ]];
            let result = execute_commands_for_mode(&commands, CommandExecutionMode::Process);
            let error_text = result.expect_err("missing program should error");

            assert!(!error_text.is_empty());
        }

        #[test]
        fn execute_launch_request_with_result_for_mode_includes_workspace_context() {
            let request = LaunchRequest {
                project_name: Some("project.one".to_string()),
                workspace_name: "auth-flow".to_string(),
                workspace_path: PathBuf::from("/repos/project.one/worktrees/auth-flow"),
                agent: AgentType::Claude,
                prompt: None,
                workspace_init_command: None,
                skip_permissions: false,
                agent_env: Vec::new(),
                capture_cols: Some(120),
                capture_rows: Some(40),
            };
            let result = execute_launch_request_with_result_for_mode(
                &request,
                CommandExecutionMode::Delegating(&mut |_command| {
                    Err(std::io::Error::other("synthetic execution failure"))
                }),
            );

            assert_eq!(result.workspace_name, "auth-flow");
            assert_eq!(
                result.workspace_path,
                PathBuf::from("/repos/project.one/worktrees/auth-flow")
            );
            assert_eq!(result.session_name, "grove-ws-project-one-auth-flow");
            assert!(result.result.is_err());
        }

        #[test]
        fn execute_stop_workspace_with_result_for_mode_includes_workspace_context() {
            let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
                "project.one".to_string(),
                PathBuf::from("/repos/project.one"),
            );
            let mut commands = Vec::new();
            let result = execute_stop_workspace_with_result_for_mode(
                &workspace,
                CommandExecutionMode::Delegating(&mut |command| {
                    commands.push(command.to_vec());
                    Ok(())
                }),
            );

            assert_eq!(result.workspace_name, "feature/auth.v2");
            assert_eq!(
                result.workspace_path,
                PathBuf::from("/repos/grove-feature/auth.v2")
            );
            assert_eq!(result.session_name, "grove-ws-project-one-feature-auth-v2");
            assert!(result.result.is_ok());
            assert_eq!(
                commands,
                vec![
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2".to_string(),
                        "C-c".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2".to_string(),
                    ],
                ]
            );
        }

        #[test]
        fn execute_launch_plan_writes_launcher_script_and_executes_commands() {
            let temp_dir = unique_test_dir("execute-launch-plan");
            let script_path = temp_dir.join(".grove/start.sh");
            let launch_plan = LaunchPlan {
                session_name: "grove-ws-test".to_string(),
                pane_lookup_cmd: Vec::new(),
                pre_launch_cmds: vec![vec![
                    "sh".to_string(),
                    "-lc".to_string(),
                    "true".to_string(),
                ]],
                launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
                launcher_script: Some(LauncherScript {
                    path: script_path.clone(),
                    contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
                }),
            };

            let result = execute_launch_plan(launch_plan);
            assert!(result.is_ok());
            assert_eq!(
                fs::read_to_string(script_path).expect("launcher script should be written"),
                "#!/usr/bin/env bash\necho hi\n"
            );
        }

        #[test]
        fn execute_commands_with_uses_supplied_executor() {
            let commands = vec![
                vec!["echo".to_string(), "first".to_string()],
                vec!["echo".to_string(), "second".to_string()],
            ];
            let mut observed = Vec::new();

            let result = execute_commands_with(&commands, |command| {
                observed.push(command.join(" "));
                Ok(())
            });

            assert!(result.is_ok());
            assert_eq!(observed, vec!["echo first", "echo second"]);
        }

        #[test]
        fn execute_commands_with_executor_skips_empty_commands() {
            let commands = vec![
                Vec::new(),
                vec!["echo".to_string(), "ran".to_string()],
                Vec::new(),
            ];
            let mut executor = RecordingCommandExecutor::default();

            let result = execute_commands_with_executor(&commands, &mut executor);

            assert!(result.is_ok());
            assert_eq!(
                executor.commands,
                vec![vec!["echo".to_string(), "ran".to_string()]]
            );
        }

        #[test]
        fn execute_launch_plan_with_executor_runs_prelaunch_then_launch() {
            let launch_plan = LaunchPlan {
                session_name: "grove-ws-test".to_string(),
                pane_lookup_cmd: Vec::new(),
                pre_launch_cmds: vec![
                    vec!["echo".to_string(), "one".to_string()],
                    vec!["echo".to_string(), "two".to_string()],
                ],
                launch_cmd: vec!["echo".to_string(), "launch".to_string()],
                launcher_script: Some(LauncherScript {
                    path: PathBuf::from("/tmp/.grove/start.sh"),
                    contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
                }),
            };
            let mut executor = RecordingCommandExecutor::default();

            let result = execute_launch_plan_with_executor(&launch_plan, &mut executor);

            assert!(result.is_ok());
            assert_eq!(
                executor.commands,
                vec![
                    vec!["echo".to_string(), "one".to_string()],
                    vec!["echo".to_string(), "two".to_string()],
                    vec!["echo".to_string(), "launch".to_string()],
                ]
            );
            assert_eq!(executor.launcher_scripts.len(), 1);
        }

        #[test]
        fn execute_command_with_skips_empty_commands() {
            let mut executed = false;

            let result = execute_command_with(&Vec::new(), |_command| {
                executed = true;
                Ok(())
            });

            assert!(result.is_ok());
            assert!(!executed);
        }

        #[test]
        fn execute_command_with_invokes_executor_for_non_empty_commands() {
            let command = vec!["echo".to_string(), "ok".to_string()];
            let mut observed = String::new();

            let result = execute_command_with(&command, |command| {
                observed = command.join(" ");
                Ok(())
            });

            assert!(result.is_ok());
            assert_eq!(observed, "echo ok");
        }

        #[test]
        fn execute_launch_plan_with_prefixes_script_write_errors() {
            let temp_dir = unique_test_dir("execute-launch-plan-sync");
            let blocked_path = temp_dir.join("blocked");
            fs::write(&blocked_path, "not a directory").expect("blocked path should be writable");
            let launch_plan = LaunchPlan {
                session_name: "grove-ws-test".to_string(),
                pane_lookup_cmd: Vec::new(),
                pre_launch_cmds: Vec::new(),
                launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
                launcher_script: Some(LauncherScript {
                    path: blocked_path.join(".grove/start.sh"),
                    contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
                }),
            };

            let result = execute_launch_plan_with(&launch_plan, |_command| Ok(()));
            let error_text = result.expect_err("script write should fail").to_string();

            assert!(error_text.starts_with("launcher script write failed: "));
        }

        #[test]
        fn execute_launch_plan_for_mode_delegating_prefixes_script_write_errors() {
            let temp_dir = unique_test_dir("execute-launch-plan-sync-mode");
            let blocked_path = temp_dir.join("blocked");
            fs::write(&blocked_path, "not a directory").expect("blocked path should be writable");
            let launch_plan = LaunchPlan {
                session_name: "grove-ws-test".to_string(),
                pane_lookup_cmd: Vec::new(),
                pre_launch_cmds: Vec::new(),
                launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
                launcher_script: Some(LauncherScript {
                    path: blocked_path.join(".grove/start.sh"),
                    contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
                }),
            };

            let result = execute_launch_plan_for_mode(
                &launch_plan,
                CommandExecutionMode::Delegating(&mut |_command| Ok(())),
            );
            let error_text = result.expect_err("script write should fail");

            assert!(error_text.starts_with("launcher script write failed: "));
        }

        #[test]
        fn execute_launch_plan_keeps_unprefixed_script_write_errors() {
            let temp_dir = unique_test_dir("execute-launch-plan");
            let blocked_path = temp_dir.join("blocked");
            fs::write(&blocked_path, "not a directory").expect("blocked path should be writable");
            let launch_plan = LaunchPlan {
                session_name: "grove-ws-test".to_string(),
                pane_lookup_cmd: Vec::new(),
                pre_launch_cmds: Vec::new(),
                launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
                launcher_script: Some(LauncherScript {
                    path: blocked_path.join(".grove/start.sh"),
                    contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
                }),
            };

            let result = execute_launch_plan(launch_plan);
            let error_text = result.expect_err("script write should fail").to_string();

            assert!(!error_text.starts_with("launcher script write failed: "));
        }

        #[test]
        fn kill_workspace_session_command_uses_project_scoped_tmux_session_name() {
            assert_eq!(
                kill_workspace_session_command(Some("project.one"), "feature/auth.v2"),
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2".to_string(),
                ]
            );
        }

        #[test]
        fn kill_workspace_session_commands_include_agent_git_and_shell_sessions() {
            assert_eq!(
                kill_workspace_session_commands(Some("project.one"), "feature/auth.v2"),
                vec![
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2-git".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2-shell".to_string(),
                    ],
                ]
            );
        }

        #[test]
        fn workspace_session_name_matches_accepts_numbered_agent_and_shell_tabs() {
            assert!(workspace_session_name_matches(
                Some("project.one"),
                "feature/auth.v2",
                "grove-ws-project-one-feature-auth-v2-agent-1",
            ));
            assert!(workspace_session_name_matches(
                Some("project.one"),
                "feature/auth.v2",
                "grove-ws-project-one-feature-auth-v2-shell-2",
            ));
            assert!(workspace_session_name_matches(
                Some("project.one"),
                "feature/auth.v2",
                "grove-ws-project-one-feature-auth-v2-git",
            ));
            assert!(!workspace_session_name_matches(
                Some("project.one"),
                "feature/auth.v2",
                "grove-ws-project-one-feature-auth-v2-agent-x",
            ));
            assert!(!workspace_session_name_matches(
                Some("project.one"),
                "feature/auth.v2",
                "grove-ws-project-one-other-agent-1",
            ));
        }

        #[test]
        fn workspace_session_names_for_cleanup_filters_to_workspace_tabs() {
            let sessions = vec![
                "grove-ws-project-one-feature-auth-v2".to_string(),
                "grove-ws-project-one-feature-auth-v2-git".to_string(),
                "grove-ws-project-one-feature-auth-v2-shell-1".to_string(),
                "grove-ws-project-one-feature-auth-v2-agent-1".to_string(),
                "grove-ws-project-one-other-agent-1".to_string(),
            ];
            assert_eq!(
                workspace_session_names_for_cleanup(
                    Some("project.one"),
                    "feature/auth.v2",
                    sessions.as_slice(),
                ),
                vec![
                    "grove-ws-project-one-feature-auth-v2".to_string(),
                    "grove-ws-project-one-feature-auth-v2-git".to_string(),
                    "grove-ws-project-one-feature-auth-v2-shell-1".to_string(),
                    "grove-ws-project-one-feature-auth-v2-agent-1".to_string(),
                ],
            );
            assert_eq!(
                kill_workspace_session_commands_for_existing_sessions(
                    Some("project.one"),
                    "feature/auth.v2",
                    sessions.as_slice(),
                ),
                vec![
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2-git".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2-shell-1".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "kill-session".to_string(),
                        "-t".to_string(),
                        "grove-ws-project-one-feature-auth-v2-agent-1".to_string(),
                    ],
                ],
            );
        }

        #[test]
        fn launch_plan_with_workspace_init_runs_before_agent() {
            let request = LaunchRequest {
                project_name: None,
                workspace_name: "auth-flow".to_string(),
                workspace_path: PathBuf::from("/repos/grove-auth-flow"),
                agent: AgentType::Claude,
                prompt: None,
                workspace_init_command: Some("direnv allow".to_string()),
                skip_permissions: true,
                agent_env: Vec::new(),
                capture_cols: None,
                capture_rows: None,
            };

            let plan = build_launch_plan(&request);
            assert_eq!(plan.launch_cmd.len(), 6);
            assert_eq!(plan.launch_cmd[0], "tmux");
            assert_eq!(plan.launch_cmd[1], "send-keys");
            assert_eq!(plan.launch_cmd[2], "-t");
            assert_eq!(plan.launch_cmd[3], "grove-ws-auth-flow");
            assert!(
                plan.launch_cmd[4].contains("bash -lc"),
                "expected shell wrapper command, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("direnv allow"),
                "expected init command in wrapper, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("claude --dangerously-skip-permissions"),
                "expected agent command in wrapper, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("direnv exec . bash -lc"),
                "expected direnv exec wrapper, got {}",
                plan.launch_cmd[4]
            );
            assert_eq!(plan.launch_cmd[5], "Enter");
        }
    }
    mod session_launch {
        use super::*;

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
        fn session_name_for_workspace_ref_uses_project_context_when_present() {
            let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
                "project.one".to_string(),
                PathBuf::from("/repos/project.one"),
            );
            assert_eq!(
                session_name_for_workspace_ref(&workspace),
                "grove-ws-project-one-feature-auth-v2"
            );
        }

        #[test]
        fn git_session_name_uses_project_context_when_present() {
            let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
                "project.one".to_string(),
                PathBuf::from("/repos/project.one"),
            );
            assert_eq!(
                git_session_name_for_workspace(&workspace),
                "grove-ws-project-one-feature-auth-v2-git"
            );
        }

        #[test]
        fn shell_session_name_uses_project_context_when_present() {
            let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
                "project.one".to_string(),
                PathBuf::from("/repos/project.one"),
            );
            assert_eq!(
                shell_session_name_for_workspace(&workspace),
                "grove-ws-project-one-feature-auth-v2-shell"
            );
        }

        #[test]
        fn launch_request_for_workspace_copies_workspace_context_and_options() {
            let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
                "project.one".to_string(),
                PathBuf::from("/repos/project.one"),
            );
            let request = launch_request_for_workspace(
                &workspace,
                Some("run checks".to_string()),
                Some("direnv allow".to_string()),
                true,
                vec![(
                    "CLAUDE_CONFIG_DIR".to_string(),
                    "~/.claude-work".to_string(),
                )],
                Some(132),
                Some(44),
            );

            assert_eq!(request.project_name.as_deref(), Some("project.one"));
            assert_eq!(request.workspace_name, "feature/auth.v2");
            assert_eq!(
                request.workspace_path,
                PathBuf::from("/repos/grove-feature/auth.v2")
            );
            assert_eq!(request.agent, AgentType::Claude);
            assert_eq!(request.prompt.as_deref(), Some("run checks"));
            assert_eq!(
                request.workspace_init_command.as_deref(),
                Some("direnv allow")
            );
            assert!(request.skip_permissions);
            assert_eq!(
                request.agent_env,
                vec![(
                    "CLAUDE_CONFIG_DIR".to_string(),
                    "~/.claude-work".to_string()
                )]
            );
            assert_eq!(request.capture_cols, Some(132));
            assert_eq!(request.capture_rows, Some(44));
        }

        #[test]
        fn shell_launch_request_for_workspace_uses_workspace_path_and_options() {
            let workspace = fixture_workspace("feature", false);
            let request = shell_launch_request_for_workspace(
                &workspace,
                "grove-ws-feature-git".to_string(),
                "lazygit".to_string(),
                Some("direnv allow".to_string()),
                Some(120),
                Some(40),
            );

            assert_eq!(request.session_name, "grove-ws-feature-git");
            assert_eq!(
                request.workspace_path,
                PathBuf::from("/repos/grove-feature")
            );
            assert_eq!(request.command, "lazygit");
            assert_eq!(
                request.workspace_init_command.as_deref(),
                Some("direnv allow")
            );
            assert_eq!(request.capture_cols, Some(120));
            assert_eq!(request.capture_rows, Some(40));
        }

        #[test]
        fn build_shell_launch_plan_skips_send_keys_when_command_is_empty() {
            let request = shell_launch_request_for_workspace(
                &fixture_workspace("feature", false),
                "grove-ws-feature-shell".to_string(),
                String::new(),
                None,
                Some(120),
                Some(40),
            );
            let plan = build_shell_launch_plan(&request);

            assert!(plan.launch_cmd.is_empty());
        }

        #[test]
        fn build_shell_launch_plan_with_workspace_init_runs_before_empty_command() {
            let request = shell_launch_request_for_workspace(
                &fixture_workspace("feature", false),
                "grove-ws-feature-shell".to_string(),
                String::new(),
                Some("direnv allow".to_string()),
                Some(120),
                Some(40),
            );
            let plan = build_shell_launch_plan(&request);

            assert_eq!(plan.launch_cmd.len(), 6);
            assert!(
                plan.launch_cmd[4].contains("bash -lc"),
                "expected init wrapper command, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("direnv allow"),
                "expected init command in wrapper, got {}",
                plan.launch_cmd[4]
            );
        }

        #[test]
        fn workspace_init_runs_directly_without_lock_wrapper() {
            let request = shell_launch_request_for_workspace(
                &fixture_workspace("feature", false),
                "grove-ws-feature-shell".to_string(),
                String::new(),
                Some("echo init".to_string()),
                Some(120),
                Some(40),
            );
            let plan = build_shell_launch_plan(&request);
            let command = &plan.launch_cmd[4];

            assert!(
                command.contains("bash -lc"),
                "expected shell wrapper, got {command}"
            );
            assert!(
                command.contains("echo init"),
                "expected init command, got {command}"
            );
            assert!(
                !command.contains("workspace-init-"),
                "lock wrapper should be removed, got {command}"
            );
        }

        #[test]
        fn build_shell_launch_plan_with_direnv_init_wraps_run_command_in_direnv_exec() {
            let request = shell_launch_request_for_workspace(
                &fixture_workspace("feature", false),
                "grove-ws-feature-shell".to_string(),
                "yarn test".to_string(),
                Some("direnv allow".to_string()),
                Some(120),
                Some(40),
            );
            let plan = build_shell_launch_plan(&request);

            assert!(
                plan.launch_cmd[4].contains("direnv exec . bash -lc"),
                "expected direnv exec wrapper, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("yarn test"),
                "expected shell command in wrapped launch, got {}",
                plan.launch_cmd[4]
            );
        }

        #[test]
        fn build_shell_launch_plan_with_capture_dimensions_resizes_before_send_keys() {
            let request = shell_launch_request_for_workspace(
                &fixture_workspace("feature", false),
                "grove-ws-feature-shell".to_string(),
                "bash".to_string(),
                None,
                Some(120),
                Some(40),
            );
            let plan = build_shell_launch_plan(&request);

            assert_eq!(
                plan.pre_launch_cmds.last(),
                Some(&vec![
                    "tmux".to_string(),
                    "resize-window".to_string(),
                    "-t".to_string(),
                    "grove-ws-feature-shell".to_string(),
                    "-x".to_string(),
                    "120".to_string(),
                    "-y".to_string(),
                    "40".to_string(),
                ])
            );
        }

        #[test]
        fn workspace_status_poll_policy_requires_supported_agent() {
            let workspace = fixture_workspace("feature", false).with_supported_agent(false);
            assert!(!workspace_should_poll_status(&workspace));
        }

        #[test]
        fn live_preview_agent_session_requires_live_workspace_session() {
            let idle_workspace = fixture_workspace("feature", false);
            assert_eq!(live_preview_agent_session(Some(&idle_workspace)), None);

            let mut active_workspace = fixture_workspace("feature", false);
            active_workspace.status = WorkspaceStatus::Active;
            assert_eq!(
                live_preview_agent_session(Some(&active_workspace)),
                Some("grove-ws-feature".to_string())
            );
        }

        #[test]
        fn workspace_status_session_target_skips_selected_live_session() {
            let mut workspace = fixture_workspace("feature", false);
            workspace.status = WorkspaceStatus::Active;

            assert_eq!(
                workspace_status_session_target(&workspace, None),
                Some("grove-ws-feature".to_string())
            );
            assert_eq!(
                workspace_status_session_target(&workspace, Some("grove-ws-feature")),
                None
            );
        }

        #[test]
        fn workspace_status_targets_for_polling_skip_selected_session() {
            let mut selected = fixture_workspace("selected", false);
            selected.status = WorkspaceStatus::Active;
            let mut other = fixture_workspace("other", false);
            other.status = WorkspaceStatus::Active;
            let workspaces = vec![selected, other];

            let targets =
                workspace_status_targets_for_polling(&workspaces, Some("grove-ws-selected"));
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].workspace_name, "other");
            assert_eq!(targets[0].session_name, "grove-ws-other");
        }

        #[test]
        fn workspace_status_targets_for_polling_with_live_preview_skips_selected_session() {
            let mut selected = fixture_workspace("selected", false);
            selected.status = WorkspaceStatus::Active;
            let mut other = fixture_workspace("other", false);
            other.status = WorkspaceStatus::Active;
            let workspaces = vec![selected, other];

            let live_preview = LivePreviewTarget {
                session_name: "grove-ws-selected".to_string(),
                include_escape_sequences: true,
            };
            let targets = workspace_status_targets_for_polling_with_live_preview(
                &workspaces,
                Some(&live_preview),
            );
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].workspace_name, "other");
        }

        #[test]
        fn workspace_can_enter_interactive_depends_on_preview_tab_mode() {
            let idle_workspace = fixture_workspace("feature", false);
            assert!(!workspace_can_enter_interactive(
                Some(&idle_workspace),
                false
            ));
            assert!(workspace_can_enter_interactive(Some(&idle_workspace), true));
            assert!(!workspace_can_enter_interactive(None, false));

            let mut active_workspace = fixture_workspace("feature", false);
            active_workspace.status = WorkspaceStatus::Active;
            assert!(workspace_can_enter_interactive(
                Some(&active_workspace),
                false
            ));
        }

        #[test]
        fn workspace_can_start_agent_depends_on_status_and_support() {
            let idle_workspace = fixture_workspace("feature", false);
            assert!(workspace_can_start_agent(Some(&idle_workspace)));
            assert!(!workspace_can_start_agent(None));

            let unsupported_workspace =
                fixture_workspace("feature", false).with_supported_agent(false);
            assert!(!workspace_can_start_agent(Some(&unsupported_workspace)));

            let mut waiting_workspace = fixture_workspace("feature", false);
            waiting_workspace.status = WorkspaceStatus::Waiting;
            assert!(!workspace_can_start_agent(Some(&waiting_workspace)));

            let mut done_workspace = fixture_workspace("feature", false);
            done_workspace.status = WorkspaceStatus::Done;
            assert!(workspace_can_start_agent(Some(&done_workspace)));
        }

        #[test]
        fn workspace_can_stop_agent_depends_on_session_status() {
            let idle_workspace = fixture_workspace("feature", false);
            assert!(!workspace_can_stop_agent(Some(&idle_workspace)));
            assert!(!workspace_can_stop_agent(None));

            let mut active_workspace = fixture_workspace("feature", false);
            active_workspace.status = WorkspaceStatus::Active;
            assert!(workspace_can_stop_agent(Some(&active_workspace)));
        }

        #[test]
        fn workspace_session_for_preview_tab_respects_preview_tab_mode() {
            let idle_workspace = fixture_workspace("feature", false);
            assert_eq!(
                workspace_session_for_preview_tab(
                    Some(&idle_workspace),
                    true,
                    Some("grove-ws-feature-git"),
                ),
                Some("grove-ws-feature-git".to_string())
            );
            assert_eq!(
                workspace_session_for_preview_tab(Some(&idle_workspace), true, None),
                None
            );
            assert_eq!(
                workspace_session_for_preview_tab(None, true, Some("grove-ws-feature-git")),
                None
            );
            assert_eq!(
                workspace_session_for_preview_tab(
                    Some(&idle_workspace),
                    false,
                    Some("grove-ws-feature-git"),
                ),
                None
            );

            let mut active_workspace = fixture_workspace("feature", false);
            active_workspace.status = WorkspaceStatus::Active;
            assert_eq!(
                workspace_session_for_preview_tab(Some(&active_workspace), false, None),
                Some("grove-ws-feature".to_string())
            );
        }

        #[test]
        fn git_preview_session_if_ready_requires_matching_ready_session() {
            let workspace = fixture_workspace("feature", false);
            let mut ready_sessions = HashSet::new();
            assert_eq!(
                git_preview_session_if_ready(Some(&workspace), &ready_sessions),
                None
            );
            ready_sessions.insert("grove-ws-feature-git".to_string());
            assert_eq!(
                git_preview_session_if_ready(Some(&workspace), &ready_sessions),
                Some("grove-ws-feature-git".to_string())
            );
            assert_eq!(git_preview_session_if_ready(None, &ready_sessions), None);
        }

        #[test]
        fn live_preview_session_for_tab_uses_git_or_agent_policy() {
            let idle_workspace = fixture_workspace("feature", false);
            let mut active_workspace = fixture_workspace("feature", false);
            active_workspace.status = WorkspaceStatus::Active;
            let mut ready_sessions = HashSet::new();
            ready_sessions.insert("grove-ws-feature-git".to_string());

            assert_eq!(
                live_preview_session_for_tab(Some(&idle_workspace), true, &ready_sessions),
                Some("grove-ws-feature-git".to_string())
            );
            assert_eq!(
                live_preview_session_for_tab(Some(&idle_workspace), false, &ready_sessions),
                None
            );
            assert_eq!(
                live_preview_session_for_tab(Some(&active_workspace), false, &ready_sessions),
                Some("grove-ws-feature".to_string())
            );
        }

        #[test]
        fn live_preview_capture_target_for_tab_sets_capture_mode() {
            let mut active_workspace = fixture_workspace("feature", false);
            active_workspace.status = WorkspaceStatus::Active;
            let mut ready_sessions = HashSet::new();
            ready_sessions.insert("grove-ws-feature-git".to_string());

            assert_eq!(
                live_preview_capture_target_for_tab(
                    Some(&active_workspace),
                    false,
                    &ready_sessions
                ),
                Some(LivePreviewTarget {
                    session_name: "grove-ws-feature".to_string(),
                    include_escape_sequences: true,
                })
            );
            assert_eq!(
                live_preview_capture_target_for_tab(Some(&active_workspace), true, &ready_sessions),
                Some(LivePreviewTarget {
                    session_name: "grove-ws-feature-git".to_string(),
                    include_escape_sequences: true,
                })
            );
        }

        #[test]
        fn tmux_missing_session_error_detection_matches_known_patterns() {
            assert!(tmux_capture_error_indicates_missing_session(
                "can't find session: grove-ws-main"
            ));
            assert!(tmux_capture_error_indicates_missing_session(
                "No active session found"
            ));
            assert!(!tmux_capture_error_indicates_missing_session(
                "permission denied"
            ));
        }

        #[test]
        fn tmux_duplicate_session_error_detection_matches_known_patterns() {
            assert!(tmux_launch_error_indicates_duplicate_session(
                "duplicate session: grove-ws-main-git"
            ));
            assert!(tmux_launch_error_indicates_duplicate_session(
                "command failed: tmux new-session -d -s foo; Duplicate Session: foo"
            ));
            assert!(!tmux_launch_error_indicates_duplicate_session(
                "permission denied"
            ));
        }

        #[test]
        fn codex_launch_command_matches_prd_flags() {
            assert_eq!(default_agent_command(AgentType::Codex, false), "codex");
            assert_eq!(
                default_agent_command(AgentType::Codex, true),
                "codex --dangerously-bypass-approvals-and-sandbox"
            );
            assert_eq!(
                default_agent_command(AgentType::OpenCode, false),
                "opencode"
            );
            assert_eq!(
                default_agent_command(AgentType::OpenCode, true),
                "OPENCODE_PERMISSION='{\"*\":\"allow\"}' opencode"
            );
        }

        #[test]
        fn agent_command_override_normalization_trims_whitespace() {
            assert_eq!(
                trimmed_nonempty("  /tmp/fake-codex --flag  "),
                Some("/tmp/fake-codex --flag".to_string())
            );
        }

        #[test]
        fn agent_command_override_normalization_ignores_empty_values() {
            assert_eq!(trimmed_nonempty(""), None);
            assert_eq!(trimmed_nonempty("   "), None);
        }

        #[test]
        fn launch_plan_without_prompt_sends_agent_directly() {
            let request = LaunchRequest {
                project_name: None,
                workspace_name: "auth-flow".to_string(),
                workspace_path: PathBuf::from("/repos/grove-auth-flow"),
                agent: AgentType::Claude,
                prompt: None,
                workspace_init_command: None,
                skip_permissions: true,
                agent_env: Vec::new(),
                capture_cols: None,
                capture_rows: None,
            };

            let plan = build_launch_plan(&request);

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
        fn launch_plan_with_workspace_init_wraps_agent_start_command() {
            let request = LaunchRequest {
                project_name: None,
                workspace_name: "auth-flow".to_string(),
                workspace_path: PathBuf::from("/repos/grove-auth-flow"),
                agent: AgentType::Claude,
                prompt: None,
                workspace_init_command: Some("direnv allow".to_string()),
                skip_permissions: false,
                agent_env: Vec::new(),
                capture_cols: None,
                capture_rows: None,
            };

            let plan = build_launch_plan(&request);
            assert_eq!(plan.launch_cmd.len(), 6);
            assert!(
                plan.launch_cmd[4].contains("bash -lc"),
                "expected init wrapper command, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("direnv allow"),
                "expected init command in wrapper, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("claude"),
                "expected agent command in wrapper, got {}",
                plan.launch_cmd[4]
            );
            assert!(
                plan.launch_cmd[4].contains("direnv exec . bash -lc"),
                "expected direnv exec wrapper, got {}",
                plan.launch_cmd[4]
            );
        }

        #[test]
        fn launch_plan_with_non_direnv_init_does_not_wrap_agent_command_in_direnv_exec() {
            let request = LaunchRequest {
                project_name: None,
                workspace_name: "auth-flow".to_string(),
                workspace_path: PathBuf::from("/repos/grove-auth-flow"),
                agent: AgentType::Claude,
                prompt: None,
                workspace_init_command: Some("echo init".to_string()),
                skip_permissions: false,
                agent_env: Vec::new(),
                capture_cols: None,
                capture_rows: None,
            };

            let plan = build_launch_plan(&request);
            assert!(
                !plan.launch_cmd[4].contains("direnv exec . bash -lc"),
                "did not expect direnv exec wrapper, got {}",
                plan.launch_cmd[4]
            );
        }

        #[test]
        fn launch_plan_with_capture_dimensions_resizes_before_send_keys() {
            let request = LaunchRequest {
                project_name: None,
                workspace_name: "auth-flow".to_string(),
                workspace_path: PathBuf::from("/repos/grove-auth-flow"),
                agent: AgentType::Claude,
                prompt: None,
                workspace_init_command: None,
                skip_permissions: true,
                agent_env: Vec::new(),
                capture_cols: Some(132),
                capture_rows: Some(44),
            };

            let plan = build_launch_plan(&request);

            assert_eq!(
                plan.pre_launch_cmds.last(),
                Some(&vec![
                    "tmux".to_string(),
                    "resize-window".to_string(),
                    "-t".to_string(),
                    "grove-ws-auth-flow".to_string(),
                    "-x".to_string(),
                    "132".to_string(),
                    "-y".to_string(),
                    "44".to_string(),
                ])
            );
        }

        #[test]
        fn launch_plan_with_agent_env_exports_before_agent_start() {
            let request = LaunchRequest {
                project_name: None,
                workspace_name: "auth-flow".to_string(),
                workspace_path: PathBuf::from("/repos/grove-auth-flow"),
                agent: AgentType::Claude,
                prompt: None,
                workspace_init_command: None,
                skip_permissions: false,
                agent_env: vec![
                    (
                        "CLAUDE_CONFIG_DIR".to_string(),
                        "~/.claude-work".to_string(),
                    ),
                    (
                        "OPENAI_API_BASE".to_string(),
                        "https://api.example.com/v1".to_string(),
                    ),
                ],
                capture_cols: None,
                capture_rows: None,
            };

            let plan = build_launch_plan(&request);

            assert_eq!(
                plan.pre_launch_cmds[2],
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-auth-flow".to_string(),
                    "export CLAUDE_CONFIG_DIR='~/.claude-work' OPENAI_API_BASE='https://api.example.com/v1'"
                        .to_string(),
                    "Enter".to_string(),
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
                workspace_init_command: None,
                skip_permissions: false,
                agent_env: Vec::new(),
                capture_cols: None,
                capture_rows: None,
            };

            let plan = build_launch_plan(&request);

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
                    "bash /repos/grove-db_migration/.grove/start.sh",
                    "Enter"
                ]
            );
        }

        #[test]
        fn stop_plan_uses_ctrl_c_then_kill_session() {
            let plan = stop_plan("grove-ws-auth-flow");
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
        fn agent_supports_in_pane_restart_is_enabled_for_all_agents() {
            assert!(agent_supports_in_pane_restart(AgentType::Claude));
            assert!(agent_supports_in_pane_restart(AgentType::Codex));
            assert!(agent_supports_in_pane_restart(AgentType::OpenCode));
        }
    }
    mod status_reconcile {
        use super::*;

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

            let project_dir_name = super::super::claude_project_dir_name(&workspace_path);
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

            let project_dir_name = super::super::claude_project_dir_name(&workspace_path);
            let project_dir = home.join(".claude").join("projects").join(project_dir_name);
            fs::create_dir_all(&project_dir).expect("project directory should exist");

            let session_file = project_dir.join("session-1.jsonl");
            fs::write(
                &session_file,
                "{\"type\":\"system\"}\n{\"type\":\"assistant\"}\n",
            )
            .expect("session file should be written");

            let marker = super::super::latest_claude_assistant_attention_marker_in_home(
                &workspace_path,
                &home,
            );
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

            let marker = super::super::latest_codex_assistant_attention_marker_in_home(
                &workspace_path,
                &home,
            );
            assert!(marker.is_some());

            let _ = fs::remove_dir_all(root);
        }

        #[test]
        fn opencode_session_file_marks_waiting_when_last_message_is_assistant() {
            let root = unique_test_dir("grove-opencode-session");
            let home = root.join("home");
            let workspace_path = root.join("ws").join("feature-delta");
            fs::create_dir_all(&home).expect("home directory should exist");
            fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

            let data_dir = home.join(".local").join("share").join("opencode");
            fs::create_dir_all(&data_dir).expect("opencode data directory should exist");
            let database_path = data_dir.join("opencode.db");
            let connection = Connection::open(&database_path).expect("database should open");
            connection
                .execute_batch(
                    "CREATE TABLE session (id TEXT PRIMARY KEY, directory TEXT NOT NULL, time_updated INTEGER NOT NULL);
                     CREATE TABLE message (
                       id TEXT PRIMARY KEY,
                       session_id TEXT NOT NULL,
                       time_created INTEGER NOT NULL,
                       time_updated INTEGER NOT NULL,
                       data TEXT NOT NULL
                     );",
                )
                .expect("schema should be created");
            connection
                .execute(
                    "INSERT INTO session (id, directory, time_updated) VALUES (?1, ?2, ?3)",
                    (
                        "session-alpha",
                        workspace_path.to_string_lossy().to_string(),
                        1_i64,
                    ),
                )
                .expect("session row should be inserted");
            connection
                .execute(
                    "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                    (
                        "message-alpha",
                        "session-alpha",
                        1_i64,
                        1_i64,
                        "{\"role\":\"assistant\"}",
                    ),
                )
                .expect("message row should be inserted");

            let status = detect_agent_session_status_in_home(
                AgentType::OpenCode,
                &workspace_path,
                &home,
                Duration::from_secs(0),
            );
            assert_eq!(status, Some(WorkspaceStatus::Waiting));

            let _ = fs::remove_dir_all(root);
        }

        #[test]
        fn opencode_attention_marker_exists_when_last_message_is_assistant() {
            let root = unique_test_dir("grove-opencode-attention-marker");
            let home = root.join("home");
            let workspace_path = root.join("ws").join("feature-epsilon");
            fs::create_dir_all(&home).expect("home directory should exist");
            fs::create_dir_all(&workspace_path).expect("workspace directory should exist");

            let data_dir = home.join(".local").join("share").join("opencode");
            fs::create_dir_all(&data_dir).expect("opencode data directory should exist");
            let database_path = data_dir.join("opencode.db");
            let connection = Connection::open(&database_path).expect("database should open");
            connection
                .execute_batch(
                    "CREATE TABLE session (id TEXT PRIMARY KEY, directory TEXT NOT NULL, time_updated INTEGER NOT NULL);
                     CREATE TABLE message (
                       id TEXT PRIMARY KEY,
                       session_id TEXT NOT NULL,
                       time_created INTEGER NOT NULL,
                       time_updated INTEGER NOT NULL,
                       data TEXT NOT NULL
                     );",
                )
                .expect("schema should be created");
            connection
                .execute(
                    "INSERT INTO session (id, directory, time_updated) VALUES (?1, ?2, ?3)",
                    (
                        "session-beta",
                        workspace_path.to_string_lossy().to_string(),
                        1_i64,
                    ),
                )
                .expect("session row should be inserted");
            connection
                .execute(
                    "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                    (
                        "message-beta",
                        "session-beta",
                        1_i64,
                        2_i64,
                        "{\"role\":\"assistant\"}",
                    ),
                )
                .expect("message row should be inserted");

            let marker = super::super::latest_opencode_assistant_attention_marker_in_home(
                &workspace_path,
                &home,
            );
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

            let project_dir_name = super::super::claude_project_dir_name(&workspace_path);
            let project_dir = home.join(".claude").join("projects").join(project_dir_name);
            fs::create_dir_all(&project_dir).expect("project directory should exist");
            let session_file = project_dir.join("session-2.jsonl");
            fs::write(&session_file, "{\"type\":\"assistant\"}\n")
                .expect("session file should be written");

            let status =
                detect_status_with_session_override_in_home(super::super::StatusOverrideContext {
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
            assert_eq!(
                detect_status(
                    "task output\n• Done.\n› Use /skills to list available skills\n",
                    SessionActivity::Active,
                    false,
                    true,
                    true
                ),
                WorkspaceStatus::Done
            );
        }

        #[test]
        fn status_resolution_detects_done_marker_beyond_last_twelve_lines() {
            let mut lines = vec!["header".to_string(), "• Done.".to_string()];
            lines.extend((0..20).map(|index| format!("detail line {index}")));
            let output = lines.join("\n");
            assert_eq!(
                detect_status(&output, SessionActivity::Active, false, true, true),
                WorkspaceStatus::Done
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

            let result =
                reconcile_with_sessions(&workspaces, &running_sessions, &previously_running);
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
    }
}

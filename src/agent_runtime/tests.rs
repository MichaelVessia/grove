use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{
    CaptureChange, LaunchRequest, LivePreviewTarget, SessionActivity, build_launch_plan,
    default_agent_command, detect_agent_session_status_in_home, detect_status,
    detect_status_with_session_override_in_home, detect_waiting_prompt, evaluate_capture_change,
    git_preview_session_if_ready, git_session_name_for_workspace, kill_workspace_session_command,
    live_preview_agent_session, live_preview_capture_target_for_tab, live_preview_session_for_tab,
    normalized_agent_command_override, poll_interval, reconcile_with_sessions,
    sanitize_workspace_name, session_name_for_workspace, session_name_for_workspace_ref, stop_plan,
    strip_mouse_fragments, tmux_capture_error_indicates_missing_session,
    workspace_can_enter_interactive, workspace_can_start_agent, workspace_can_stop_agent,
    workspace_session_for_preview_tab, workspace_should_poll_status,
    workspace_status_session_target, workspace_status_targets_for_polling,
    workspace_status_targets_for_polling_with_live_preview, zellij_capture_log_path,
    zellij_capture_log_path_in, zellij_config_path,
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
fn workspace_status_poll_policy_requires_supported_agent_for_all_multiplexers() {
    let workspace = fixture_workspace("feature", false).with_supported_agent(false);
    assert!(!workspace_should_poll_status(
        &workspace,
        MultiplexerKind::Tmux
    ));
    assert!(!workspace_should_poll_status(
        &workspace,
        MultiplexerKind::Zellij
    ));
}

#[test]
fn workspace_status_poll_policy_differs_between_tmux_and_zellij_for_idle_non_main() {
    let workspace = fixture_workspace("feature", false);
    assert!(!workspace_should_poll_status(
        &workspace,
        MultiplexerKind::Tmux
    ));
    assert!(workspace_should_poll_status(
        &workspace,
        MultiplexerKind::Zellij
    ));
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
        workspace_status_session_target(&workspace, MultiplexerKind::Tmux, None),
        Some("grove-ws-feature".to_string())
    );
    assert_eq!(
        workspace_status_session_target(
            &workspace,
            MultiplexerKind::Tmux,
            Some("grove-ws-feature")
        ),
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

    let targets = workspace_status_targets_for_polling(
        &workspaces,
        MultiplexerKind::Tmux,
        Some("grove-ws-selected"),
    );
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].workspace_name, "other");
    assert_eq!(targets[0].session_name, "grove-ws-other");
}

#[test]
fn workspace_status_targets_for_polling_include_idle_non_main_for_zellij() {
    let idle_workspace = fixture_workspace("feature", false);
    let workspaces = vec![idle_workspace];

    let targets = workspace_status_targets_for_polling(&workspaces, MultiplexerKind::Zellij, None);
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].workspace_name, "feature");
    assert_eq!(targets[0].session_name, "grove-ws-feature");
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
        MultiplexerKind::Tmux,
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

    let unsupported_workspace = fixture_workspace("feature", false).with_supported_agent(false);
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
        live_preview_capture_target_for_tab(Some(&active_workspace), false, &ready_sessions),
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
fn kill_workspace_session_command_uses_project_scoped_tmux_session_name() {
    assert_eq!(
        kill_workspace_session_command(
            Some("project.one"),
            "feature/auth.v2",
            MultiplexerKind::Tmux
        ),
        vec![
            "tmux".to_string(),
            "kill-session".to_string(),
            "-t".to_string(),
            "grove-ws-project-one-feature-auth-v2".to_string(),
        ]
    );
}

#[test]
fn kill_workspace_session_command_uses_zellij_config_for_zellij() {
    assert_eq!(
        kill_workspace_session_command(None, "feature", MultiplexerKind::Zellij),
        vec![
            "zellij".to_string(),
            "--config".to_string(),
            zellij_config_path().to_string_lossy().to_string(),
            "kill-session".to_string(),
            "grove-ws-feature".to_string(),
        ]
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
    fs::write(&session_file, "{\"type\":\"assistant\"}\n").expect("session file should be written");

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

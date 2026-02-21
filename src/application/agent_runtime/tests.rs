use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{
    CaptureChange, CommandExecutionMode, CommandExecutor, LaunchPlan, LaunchRequest,
    LauncherScript, LivePreviewTarget, SessionActivity, build_launch_plan, build_shell_launch_plan,
    default_agent_command, detect_agent_session_status_in_home, detect_status,
    detect_status_with_session_override_in_home, detect_waiting_prompt, evaluate_capture_change,
    execute_command_with, execute_commands, execute_commands_for_mode, execute_commands_with,
    execute_commands_with_executor, execute_launch_plan, execute_launch_plan_for_mode,
    execute_launch_plan_with, execute_launch_plan_with_executor,
    execute_launch_request_with_result_for_mode, execute_stop_workspace_with_result_for_mode,
    git_preview_session_if_ready, git_session_name_for_workspace, kill_workspace_session_command,
    kill_workspace_session_commands, launch_request_for_workspace, live_preview_agent_session,
    live_preview_capture_target_for_tab, live_preview_session_for_tab, poll_interval,
    reconcile_with_sessions, sanitize_workspace_name, session_name_for_workspace,
    session_name_for_workspace_ref, shell_launch_request_for_workspace,
    shell_session_name_for_workspace, stop_plan, strip_mouse_fragments,
    tmux_capture_error_indicates_missing_session, tmux_launch_error_indicates_duplicate_session,
    trimmed_nonempty, workspace_can_enter_interactive, workspace_can_start_agent,
    workspace_can_stop_agent, workspace_session_for_preview_tab, workspace_should_poll_status,
    workspace_status_session_target, workspace_status_targets_for_polling,
    workspace_status_targets_for_polling_with_live_preview,
};
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

#[derive(Default)]
struct RecordingCommandExecutor {
    commands: Vec<Vec<String>>,
    launcher_scripts: Vec<(PathBuf, String)>,
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
        Some("direnv exec .".to_string()),
        true,
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
    assert_eq!(request.pre_launch_command.as_deref(), Some("direnv exec ."));
    assert!(request.skip_permissions);
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
        Some(120),
        Some(40),
    );

    assert_eq!(request.session_name, "grove-ws-feature-git");
    assert_eq!(
        request.workspace_path,
        PathBuf::from("/repos/grove-feature")
    );
    assert_eq!(request.command, "lazygit");
    assert_eq!(request.capture_cols, Some(120));
    assert_eq!(request.capture_rows, Some(40));
}

#[test]
fn build_shell_launch_plan_skips_send_keys_when_command_is_empty() {
    let request = shell_launch_request_for_workspace(
        &fixture_workspace("feature", false),
        "grove-ws-feature-shell".to_string(),
        String::new(),
        Some(120),
        Some(40),
    );
    let plan = build_shell_launch_plan(&request);

    assert!(plan.launch_cmd.is_empty());
}

#[test]
fn build_shell_launch_plan_with_capture_dimensions_resizes_before_send_keys() {
    let request = shell_launch_request_for_workspace(
        &fixture_workspace("feature", false),
        "grove-ws-feature-shell".to_string(),
        "bash".to_string(),
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

    let targets = workspace_status_targets_for_polling(&workspaces, Some("grove-ws-selected"));
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
        daemon_socket_path: None,
    };
    let targets =
        workspace_status_targets_for_polling_with_live_preview(&workspaces, Some(&live_preview));
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
            daemon_socket_path: None,
        })
    );
    assert_eq!(
        live_preview_capture_target_for_tab(Some(&active_workspace), true, &ready_sessions),
        Some(LivePreviewTarget {
            session_name: "grove-ws-feature-git".to_string(),
            include_escape_sequences: true,
            daemon_socket_path: None,
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
        pre_launch_command: None,
        skip_permissions: true,
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
fn launch_plan_with_capture_dimensions_resizes_before_send_keys() {
    let request = LaunchRequest {
        project_name: None,
        workspace_name: "auth-flow".to_string(),
        workspace_path: PathBuf::from("/repos/grove-auth-flow"),
        agent: AgentType::Claude,
        prompt: None,
        pre_launch_command: None,
        skip_permissions: true,
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
        pre_launch_command: None,
        skip_permissions: false,
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

    let plan = build_launch_plan(&request);
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
fn claude_attention_marker_exists_when_last_message_is_assistant() {
    let root = unique_test_dir("grove-claude-attention-marker");
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

    let marker = super::latest_claude_assistant_attention_marker_in_home(&workspace_path, &home);
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

    let marker = super::latest_codex_assistant_attention_marker_in_home(&workspace_path, &home);
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

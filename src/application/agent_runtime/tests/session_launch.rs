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
fn build_shell_launch_plan_with_workspace_init_runs_guard_when_command_is_empty() {
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
        plan.launch_cmd[4].contains("workspace-init-"),
        "expected init guard command, got {}",
        plan.launch_cmd[4]
    );
    assert!(
        plan.launch_cmd[4].contains("direnv allow"),
        "expected init command in guard, got {}",
        plan.launch_cmd[4]
    );
}

#[test]
fn workspace_init_guard_recovers_stale_lock_dirs() {
    let request = shell_launch_request_for_workspace(
        &fixture_workspace("feature", false),
        "grove-ws-feature-shell".to_string(),
        String::new(),
        Some("echo init".to_string()),
        Some(120),
        Some(40),
    );
    let plan = build_shell_launch_plan(&request);
    let guard = &plan.launch_cmd[4];

    assert!(
        guard.contains("lock_stale_checks=0"),
        "expected stale lock counter, got {guard}"
    );
    assert!(
        guard.contains("if [ -f") && guard.contains("/pid ]"),
        "expected pid probe for lock holder, got {guard}"
    );
    assert!(
        guard.contains("kill -0"),
        "expected liveness check before stale cleanup, got {guard}"
    );
    assert!(
        guard.contains("rm -rf") && guard.contains(".lock"),
        "expected stale lock cleanup, got {guard}"
    );
    assert!(
        guard.contains("echo \"$$\" >") && guard.contains("/pid"),
        "expected pid file write when lock acquired, got {guard}"
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
        plan.launch_cmd[4].contains("workspace-init-"),
        "expected init guard command, got {}",
        plan.launch_cmd[4]
    );
    assert!(
        plan.launch_cmd[4].contains("direnv allow"),
        "expected init command in guard, got {}",
        plan.launch_cmd[4]
    );
    assert!(
        plan.launch_cmd[4].contains("claude"),
        "expected agent command in guard, got {}",
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

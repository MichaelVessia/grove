use super::{AgentType, Workspace, WorkspaceStatus, WorkspaceValidationError};
use std::path::PathBuf;

#[test]
fn main_workspace_requires_main_status() {
    let workspace = Workspace::try_new(
        "grove".to_string(),
        PathBuf::from("/repos/grove"),
        "main".to_string(),
        Some(1_700_000_000),
        AgentType::Claude,
        WorkspaceStatus::Idle,
        true,
    );
    assert_eq!(
        workspace,
        Err(WorkspaceValidationError::MainWorkspaceMustUseMainStatus)
    );
}

#[test]
fn workspace_requires_non_empty_name_and_branch() {
    assert_eq!(
        Workspace::try_new(
            "".to_string(),
            PathBuf::from("/repos/grove"),
            "main".to_string(),
            Some(1_700_000_000),
            AgentType::Claude,
            WorkspaceStatus::Idle,
            false
        ),
        Err(WorkspaceValidationError::EmptyName)
    );
    assert_eq!(
        Workspace::try_new(
            "feature-x".to_string(),
            PathBuf::from("/repos/grove-feature-x"),
            "".to_string(),
            Some(1_700_000_000),
            AgentType::Claude,
            WorkspaceStatus::Idle,
            false
        ),
        Err(WorkspaceValidationError::EmptyBranch)
    );
    assert_eq!(
        Workspace::try_new(
            "feature-x".to_string(),
            PathBuf::new(),
            "feature-x".to_string(),
            Some(1_700_000_000),
            AgentType::Claude,
            WorkspaceStatus::Idle,
            false
        ),
        Err(WorkspaceValidationError::EmptyPath)
    );
}

#[test]
fn workspace_accepts_valid_values() {
    let workspace = Workspace::try_new(
        "feature-x".to_string(),
        PathBuf::from("/repos/grove-feature-x"),
        "feature-x".to_string(),
        None,
        AgentType::Codex,
        WorkspaceStatus::Unknown,
        false,
    )
    .expect("workspace should be valid")
    .with_base_branch(Some("main".to_string()))
    .with_orphaned(true)
    .with_supported_agent(false);

    assert_eq!(workspace.agent.label(), "Codex");
    assert_eq!(workspace.path, PathBuf::from("/repos/grove-feature-x"));
    assert_eq!(workspace.base_branch.as_deref(), Some("main"));
    assert!(workspace.is_orphaned);
    assert!(!workspace.supported_agent);
}

#[test]
fn agent_type_metadata_roundtrips_marker() {
    for agent in AgentType::all() {
        assert_eq!(AgentType::from_marker(agent.marker()), Some(*agent));
        assert!(!agent.label().is_empty());
        assert!(!agent.command_override_env_var().is_empty());
    }
}

#[test]
fn agent_type_cycles_all_variants() {
    let mut forward = AgentType::Claude;
    for _ in 0..AgentType::all().len() {
        forward = forward.next();
    }
    assert_eq!(forward, AgentType::Claude);

    let mut backward = AgentType::Claude;
    for _ in 0..AgentType::all().len() {
        backward = backward.previous();
    }
    assert_eq!(backward, AgentType::Claude);
}

#[test]
fn claude_agent_supports_graceful_restart() {
    assert_eq!(AgentType::Claude.exit_command(), Some("/exit"));
    assert!(AgentType::Claude.resume_command_pattern().is_some());
}

#[test]
fn codex_and_opencode_do_not_support_graceful_restart() {
    assert_eq!(AgentType::Codex.exit_command(), None);
    assert_eq!(AgentType::Codex.resume_command_pattern(), None);
    assert_eq!(AgentType::OpenCode.exit_command(), None);
    assert_eq!(AgentType::OpenCode.resume_command_pattern(), None);
}

#[test]
fn claude_resume_pattern_matches_resume_command() {
    let pattern = AgentType::Claude.resume_command_pattern().unwrap();
    let regex = regex::Regex::new(pattern).unwrap();

    let output =
        "Session saved. To resume this conversation, run:\nclaude --resume abc123-def456\n$";
    let captures = regex.captures(output);
    assert!(captures.is_some());
    let matched = captures.unwrap().get(1).unwrap().as_str();
    assert_eq!(matched, "claude --resume abc123-def456");
}

#[test]
fn claude_resume_pattern_does_not_match_unrelated_output() {
    let pattern = AgentType::Claude.resume_command_pattern().unwrap();
    let regex = regex::Regex::new(pattern).unwrap();

    let output = "Working on your task...\nDone!\n$";
    assert!(regex.captures(output).is_none());
}

#[test]
fn claude_resume_pattern_captures_trailing_flags() {
    let pattern = AgentType::Claude.resume_command_pattern().unwrap();
    let regex = regex::Regex::new(pattern).unwrap();

    let output =
        "Session saved. To resume this conversation, run:\nclaude --resume abc123 --continue\n$";
    let captures = regex.captures(output);
    assert!(captures.is_some());
    let matched = captures.unwrap().get(1).unwrap().as_str();
    assert_eq!(matched, "claude --resume abc123 --continue");
}

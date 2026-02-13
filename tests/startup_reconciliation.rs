use std::collections::HashSet;
use std::path::PathBuf;

use grove::agent_runtime::reconcile_with_sessions;
use grove::domain::{AgentType, Workspace, WorkspaceStatus};
use grove::hardening::{missing_workspace_paths, orphaned_sessions};

fn workspace(name: &str, status: WorkspaceStatus, is_main: bool, path: PathBuf) -> Workspace {
    Workspace::try_new(
        name.to_string(),
        path,
        if is_main {
            "main".to_string()
        } else {
            name.to_string()
        },
        Some(1_700_000_000),
        AgentType::Claude,
        status,
        is_main,
    )
    .expect("workspace should be valid")
}

#[test]
fn startup_reconciliation_identifies_orphaned_workspaces_and_sessions() {
    let workspaces = vec![
        workspace(
            "grove",
            WorkspaceStatus::Main,
            true,
            PathBuf::from("/repo/grove"),
        ),
        workspace(
            "feature-a",
            WorkspaceStatus::Idle,
            false,
            PathBuf::from("/repo/grove-feature-a"),
        ),
        workspace(
            "feature-b",
            WorkspaceStatus::Idle,
            false,
            PathBuf::from("/repo/grove-feature-b"),
        ),
    ];

    let running_sessions = HashSet::from([
        "grove-ws-feature-a".to_string(),
        "grove-ws-orphaned".to_string(),
    ]);
    let previously_running = HashSet::from(["feature-b".to_string()]);

    let reconciled = reconcile_with_sessions(&workspaces, &running_sessions, &previously_running);

    assert_eq!(reconciled.workspaces[1].status, WorkspaceStatus::Active);
    assert!(reconciled.workspaces[2].is_orphaned);
    assert_eq!(
        reconciled.orphaned_sessions,
        vec!["grove-ws-orphaned".to_string()]
    );
}

#[test]
fn startup_reconciliation_flags_missing_worktrees_for_prune() {
    let workspaces = vec![
        workspace(
            "grove",
            WorkspaceStatus::Main,
            true,
            PathBuf::from("/repo/grove"),
        ),
        workspace(
            "feature-a",
            WorkspaceStatus::Idle,
            false,
            PathBuf::from("/definitely/missing/path"),
        ),
    ];

    let missing = missing_workspace_paths(&workspaces);
    assert_eq!(missing, vec![PathBuf::from("/definitely/missing/path")]);
    assert!(!missing.is_empty());
}

#[test]
fn orphaned_session_detection_matches_runtime_cleanup_candidates() {
    let workspaces = vec![
        workspace(
            "grove",
            WorkspaceStatus::Main,
            true,
            PathBuf::from("/repo/grove"),
        ),
        workspace(
            "feature-a",
            WorkspaceStatus::Idle,
            false,
            PathBuf::from("/repo/grove-feature-a"),
        ),
    ];

    let running = HashSet::from([
        "grove-ws-feature-a".to_string(),
        "grove-ws-stray".to_string(),
    ]);

    assert_eq!(
        orphaned_sessions(&running, &workspaces),
        vec!["grove-ws-stray".to_string()]
    );
}

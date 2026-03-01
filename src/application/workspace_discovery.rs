use std::collections::HashSet;

use crate::application::agent_runtime::reconciliation::reconcile_with_sessions;
use crate::infrastructure::adapters::{
    BootstrapData, DiscoveryState, GitAdapter, MultiplexerAdapter, SystemAdapter, bootstrap_data,
};

pub(crate) fn discover_bootstrap_data(
    git: &impl GitAdapter,
    multiplexer: &impl MultiplexerAdapter,
    system: &impl SystemAdapter,
) -> BootstrapData {
    let mut bootstrap = bootstrap_data(git, multiplexer, system);
    if !matches!(bootstrap.discovery_state, DiscoveryState::Ready) {
        return bootstrap;
    }
    if bootstrap.workspaces.is_empty() {
        return bootstrap;
    }

    let running_sessions = multiplexer.running_sessions();
    let reconciled =
        reconcile_with_sessions(&bootstrap.workspaces, &running_sessions, &HashSet::new());
    bootstrap.workspaces = reconciled.workspaces;
    bootstrap
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use super::*;
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};
    use crate::infrastructure::adapters::GitAdapterError;

    struct FakeMultiplexerAdapter {
        running: HashSet<String>,
    }

    impl MultiplexerAdapter for FakeMultiplexerAdapter {
        fn running_sessions(&self) -> HashSet<String> {
            self.running.clone()
        }
    }

    struct FakeSystemAdapter;

    impl SystemAdapter for FakeSystemAdapter {
        fn repo_name(&self) -> String {
            "grove".to_string()
        }
    }

    struct FakeGitSuccess;

    impl GitAdapter for FakeGitSuccess {
        fn list_workspaces(&self) -> Result<Vec<Workspace>, GitAdapterError> {
            Ok(vec![
                Workspace::try_new(
                    "grove".to_string(),
                    PathBuf::from("/repos/grove"),
                    "main".to_string(),
                    Some(1_700_000_300),
                    AgentType::Claude,
                    WorkspaceStatus::Main,
                    true,
                )
                .expect("workspace should be valid"),
                Workspace::try_new(
                    "feature-a".to_string(),
                    PathBuf::from("/repos/grove-feature-a"),
                    "feature-a".to_string(),
                    Some(1_700_000_200),
                    AgentType::Codex,
                    WorkspaceStatus::Idle,
                    false,
                )
                .expect("workspace should be valid"),
            ])
        }
    }

    #[test]
    fn discover_bootstrap_data_reconciles_running_sessions() {
        let bootstrap = discover_bootstrap_data(
            &FakeGitSuccess,
            &FakeMultiplexerAdapter {
                running: HashSet::from(["grove-ws-feature-a".to_string()]),
            },
            &FakeSystemAdapter,
        );

        assert_eq!(bootstrap.discovery_state, DiscoveryState::Ready);
        assert_eq!(bootstrap.workspaces.len(), 2);
        assert_eq!(bootstrap.workspaces[1].status, WorkspaceStatus::Active);
    }
}

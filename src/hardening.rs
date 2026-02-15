use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::agent_runtime::session_name_for_workspace_in_project;
use crate::domain::Workspace;

pub fn recover_working_directory(current_dir: &Path, repo_root: &Path) -> PathBuf {
    if current_dir.exists() {
        return current_dir.to_path_buf();
    }

    repo_root.to_path_buf()
}

pub fn missing_workspace_paths(workspaces: &[Workspace]) -> Vec<PathBuf> {
    let mut missing: Vec<PathBuf> = workspaces
        .iter()
        .filter(|workspace| !workspace.is_main && !workspace.path.exists())
        .map(|workspace| workspace.path.clone())
        .collect();
    missing.sort();
    missing.dedup();
    missing
}

pub fn orphaned_sessions(
    running_sessions: &HashSet<String>,
    workspaces: &[Workspace],
) -> Vec<String> {
    let expected_sessions: HashSet<String> = workspaces
        .iter()
        .filter(|workspace| !workspace.is_main)
        .map(|workspace| {
            session_name_for_workspace_in_project(
                workspace.project_name.as_deref(),
                &workspace.name,
            )
        })
        .collect();

    let mut orphaned: Vec<String> = running_sessions
        .iter()
        .filter(|session| !expected_sessions.contains(*session))
        .cloned()
        .collect();
    orphaned.sort();
    orphaned
}

pub fn bump_generation(generations: &mut HashMap<String, u64>, workspace_name: &str) -> u64 {
    let entry = generations.entry(workspace_name.to_string()).or_insert(0);
    *entry = entry.saturating_add(1);
    *entry
}

pub fn drop_missing_generations(generations: &mut HashMap<String, u64>, workspaces: &[Workspace]) {
    let active_names: HashSet<String> = workspaces
        .iter()
        .map(|workspace| workspace.name.clone())
        .collect();
    generations.retain(|name, _| active_names.contains(name));
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{
        bump_generation, drop_missing_generations, missing_workspace_paths, orphaned_sessions,
        recover_working_directory,
    };
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};

    #[derive(Debug)]
    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "grove-hardening-{label}-{}-{timestamp}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test dir should be created");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn workspace(name: &str, is_main: bool, path: PathBuf) -> Workspace {
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
            if is_main {
                WorkspaceStatus::Main
            } else {
                WorkspaceStatus::Idle
            },
            is_main,
        )
        .expect("workspace should be valid")
    }

    #[test]
    fn missing_workspaces_trigger_prune_signal() {
        let temp = TestDir::new("missing");
        let existing = temp.path.join("existing");
        fs::create_dir_all(&existing).expect("existing path should exist");

        let workspaces = vec![
            workspace("grove", true, temp.path.join("repo")),
            workspace("feature-a", false, existing),
            workspace("feature-b", false, temp.path.join("missing")),
        ];

        let missing = missing_workspace_paths(&workspaces);
        assert_eq!(missing, vec![temp.path.join("missing")]);
    }

    #[test]
    fn working_directory_recovers_to_repo_root_when_deleted() {
        let temp = TestDir::new("cwd");
        let current = temp.path.join("current");
        fs::create_dir_all(&current).expect("current dir should exist");
        let repo_root = temp.path.join("repo");
        fs::create_dir_all(&repo_root).expect("repo root should exist");

        assert_eq!(recover_working_directory(&current, &repo_root), current);

        fs::remove_dir_all(&current).expect("current dir should be removable");
        assert_eq!(recover_working_directory(&current, &repo_root), repo_root);
    }

    #[test]
    fn orphaned_sessions_are_sessions_without_matching_workspace() {
        let workspaces = vec![
            workspace("grove", true, PathBuf::from("/repo/grove")),
            workspace("feature-a", false, PathBuf::from("/repo/grove-feature-a")),
        ];
        let running = HashSet::from([
            "grove-ws-feature-a".to_string(),
            "grove-ws-lost".to_string(),
        ]);

        assert_eq!(
            orphaned_sessions(&running, &workspaces),
            vec!["grove-ws-lost".to_string()]
        );
    }

    #[test]
    fn generation_helpers_increment_and_cleanup() {
        let mut generations = HashMap::new();
        assert_eq!(bump_generation(&mut generations, "feature-a"), 1);
        assert_eq!(bump_generation(&mut generations, "feature-a"), 2);
        assert_eq!(bump_generation(&mut generations, "feature-b"), 1);

        let workspaces = vec![workspace("feature-a", false, PathBuf::from("/repo/a"))];
        drop_missing_generations(&mut generations, &workspaces);

        assert_eq!(generations.get("feature-a"), Some(&2));
        assert!(!generations.contains_key("feature-b"));
    }
}

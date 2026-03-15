use std::collections::HashSet;
use std::process::Command;

use crate::application::task_discovery::{
    TaskBootstrapData,
    bootstrap_task_data_for_root_with_sessions as discover_task_bootstrap_for_root,
};
use std::path::Path;

pub(super) fn bootstrap_task_data_for_root(tasks_root: &Path) -> TaskBootstrapData {
    discover_task_bootstrap_for_root(tasks_root, &running_task_sessions())
}

fn running_task_sessions() -> HashSet<String> {
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output();

    match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .map(|content| {
                content
                    .lines()
                    .filter(|name| name.starts_with("grove-task-") || name.starts_with("grove-wt-"))
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        _ => HashSet::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::bootstrap_task_data_for_root;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
                "grove-bootstrap-discovery-{label}-{}-{timestamp}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test dir should exist");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn bootstrap_task_data_ignores_configured_repo_without_manifest() {
        let temp = TestDir::new("no-manifest");
        let tasks_root = temp.path.join("tasks");
        fs::create_dir_all(&tasks_root).expect("tasks root should exist");

        let bootstrap = bootstrap_task_data_for_root(tasks_root.as_path());

        assert!(bootstrap.tasks.is_empty());
        assert_eq!(
            bootstrap.discovery_state,
            crate::application::task_discovery::TaskDiscoveryState::Empty
        );
    }

    #[test]
    fn bootstrap_task_data_keeps_manifest_tasks_only() {
        let temp = TestDir::new("manifest-only");
        let tasks_root = temp.path.join("tasks");
        let task_dir = tasks_root.join("feature-a").join(".grove");
        let feature_path = temp.path.join("worktrees").join("feature-a");
        fs::create_dir_all(&task_dir).expect("task dir should exist");
        fs::create_dir_all(&feature_path).expect("feature path should exist");
        fs::write(
            task_dir.join("task.toml"),
            format!(
                "name = \"feature-a\"\nslug = \"feature-a\"\nroot_path = \"{}\"\nbranch = \"feature-a\"\n\n[[worktrees]]\nrepository_name = \"web-monorepo\"\nrepository_path = \"{}\"\npath = \"{}\"\nbranch = \"feature-a\"\nagent = \"codex\"\nstatus = \"idle\"\nis_orphaned = false\nsupported_agent = true\npull_requests = []\n",
                tasks_root.join("feature-a").display(),
                temp.path.join("repos").join("web-monorepo").display(),
                feature_path.display(),
            ),
        )
        .expect("task manifest should write");

        let bootstrap = bootstrap_task_data_for_root(tasks_root.as_path());

        assert_eq!(bootstrap.tasks.len(), 1);
        assert_eq!(bootstrap.tasks[0].slug, "feature-a");
    }
}

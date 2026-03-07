use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::application::agent_runtime::session_name_for_task_worktree;
use crate::domain::Task;
use crate::infrastructure::task_manifest::decode_task_manifest;

const TASK_MANIFEST_FILE: &str = ".grove/task.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskDiscoveryState {
    Ready,
    Empty,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBootstrapData {
    pub tasks: Vec<Task>,
    pub discovery_state: TaskDiscoveryState,
}

pub fn bootstrap_task_data_for_root(tasks_root: &Path) -> TaskBootstrapData {
    bootstrap_task_data_for_root_with_sessions(tasks_root, &HashSet::new())
}

pub fn bootstrap_task_data_for_root_with_sessions(
    tasks_root: &Path,
    running_sessions: &HashSet<String>,
) -> TaskBootstrapData {
    match load_tasks_from_root(tasks_root) {
        Ok(tasks) if tasks.is_empty() => TaskBootstrapData {
            tasks,
            discovery_state: TaskDiscoveryState::Empty,
        },
        Ok(tasks) => TaskBootstrapData {
            tasks: reconcile_tasks_with_sessions(tasks, running_sessions),
            discovery_state: TaskDiscoveryState::Ready,
        },
        Err(error) => TaskBootstrapData {
            tasks: Vec::new(),
            discovery_state: TaskDiscoveryState::Error(error),
        },
    }
}

fn load_tasks_from_root(tasks_root: &Path) -> Result<Vec<Task>, String> {
    if !tasks_root.exists() {
        return Ok(Vec::new());
    }

    let entries =
        fs::read_dir(tasks_root).map_err(|error| format!("task root read failed: {error}"))?;
    let mut tasks = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("task root entry failed: {error}"))?;
        let task_root = entry.path();
        if !task_root.is_dir() {
            continue;
        }

        let manifest_path = task_manifest_path(&task_root);
        if !manifest_path.exists() {
            continue;
        }

        let raw = fs::read_to_string(&manifest_path).map_err(|error| {
            format!(
                "task manifest read failed for {}: {error}",
                manifest_path.display()
            )
        })?;
        let task = decode_task_manifest(raw.as_str()).map_err(|error| {
            format!(
                "task manifest decode failed for {}: {error}",
                manifest_path.display()
            )
        })?;
        tasks.push(task);
    }

    tasks.sort_by(|left, right| left.slug.cmp(&right.slug));
    Ok(tasks)
}

fn reconcile_tasks_with_sessions(
    tasks: Vec<Task>,
    running_sessions: &HashSet<String>,
) -> Vec<Task> {
    tasks
        .into_iter()
        .map(|mut task| {
            for worktree in &mut task.worktrees {
                let session_name = session_name_for_task_worktree(
                    task.slug.as_str(),
                    worktree.repository_name.as_str(),
                );
                if running_sessions.contains(&session_name) {
                    worktree.status = crate::domain::WorkspaceStatus::Active;
                } else if worktree.status.has_session() {
                    worktree.status = crate::domain::WorkspaceStatus::Idle;
                }
            }
            task
        })
        .collect()
}

fn task_manifest_path(task_root: &Path) -> PathBuf {
    task_root.join(TASK_MANIFEST_FILE)
}

#[cfg(test)]
mod tests {
    use super::{
        TaskDiscoveryState, bootstrap_task_data_for_root,
        bootstrap_task_data_for_root_with_sessions,
    };
    use crate::domain::{AgentType, Task, WorkspaceStatus, Worktree};
    use crate::infrastructure::task_manifest::encode_task_manifest;
    use std::collections::HashSet;
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
                "grove-task-discovery-{label}-{}-{timestamp}",
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

    fn fixture_task(slug: &str, repository_name: &str) -> Task {
        let worktree = Worktree::try_new(
            repository_name.to_string(),
            PathBuf::from(format!("/repos/{repository_name}")),
            PathBuf::from(format!("/tmp/.grove/tasks/{slug}/{repository_name}")),
            slug.to_string(),
            AgentType::Codex,
            WorkspaceStatus::Idle,
        )
        .expect("worktree should be valid")
        .with_base_branch(Some("main".to_string()));

        Task::try_new(
            slug.to_string(),
            slug.to_string(),
            PathBuf::from(format!("/tmp/.grove/tasks/{slug}")),
            slug.to_string(),
            vec![worktree],
        )
        .expect("task should be valid")
    }

    #[test]
    fn bootstrap_task_data_loads_tasks_from_task_manifests() {
        let temp = TestDir::new("bootstrap");
        let tasks_root = temp.path.join("tasks");
        fs::create_dir_all(&tasks_root).expect("tasks root should exist");

        for (slug, repository_name) in [
            ("flohome-launch", "flohome"),
            ("infra-rollout", "terraform-fastly"),
        ] {
            let task = fixture_task(slug, repository_name);
            let task_dir = tasks_root.join(slug).join(".grove");
            fs::create_dir_all(&task_dir).expect("task dir should exist");
            let raw = encode_task_manifest(&task).expect("task manifest should encode");
            fs::write(task_dir.join("task.toml"), raw).expect("task manifest should write");
        }

        let bootstrap = bootstrap_task_data_for_root(&tasks_root);

        assert_eq!(bootstrap.discovery_state, TaskDiscoveryState::Ready);
        assert_eq!(bootstrap.tasks.len(), 2);
        assert_eq!(bootstrap.tasks[0].slug, "flohome-launch");
        assert_eq!(bootstrap.tasks[1].slug, "infra-rollout");
    }

    #[test]
    fn bootstrap_task_data_reports_invalid_manifests() {
        let temp = TestDir::new("invalid");
        let tasks_root = temp.path.join("tasks");
        let broken_task_dir = tasks_root.join("broken").join(".grove");
        fs::create_dir_all(&broken_task_dir).expect("broken task dir should exist");
        fs::write(broken_task_dir.join("task.toml"), "not = [valid").expect("broken manifest");

        let bootstrap = bootstrap_task_data_for_root(&tasks_root);

        assert!(matches!(
            bootstrap.discovery_state,
            TaskDiscoveryState::Error(_)
        ));
        assert!(bootstrap.tasks.is_empty());
    }

    #[test]
    fn bootstrap_task_data_reconciles_running_worktree_sessions() {
        let temp = TestDir::new("sessions");
        let tasks_root = temp.path.join("tasks");
        let task = fixture_task("flohome-launch", "flohome");
        let task_dir = tasks_root.join("flohome-launch").join(".grove");
        fs::create_dir_all(&task_dir).expect("task dir should exist");
        let raw = encode_task_manifest(&task).expect("task manifest should encode");
        fs::write(task_dir.join("task.toml"), raw).expect("task manifest should write");

        let bootstrap = bootstrap_task_data_for_root_with_sessions(
            &tasks_root,
            &HashSet::from(["grove-wt-flohome-launch-flohome".to_string()]),
        );

        assert_eq!(bootstrap.discovery_state, TaskDiscoveryState::Ready);
        assert_eq!(bootstrap.tasks.len(), 1);
        assert_eq!(
            bootstrap.tasks[0].worktrees[0].status,
            WorkspaceStatus::Active
        );
    }
}

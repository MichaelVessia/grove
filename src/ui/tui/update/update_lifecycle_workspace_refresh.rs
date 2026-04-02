use super::update_prelude::*;
use crate::application::task_discovery::TaskDiscoveryState;
use crate::infrastructure::config::ProjectConfig;
use crate::infrastructure::paths::tasks_root;
use crate::ui::state::AppState;
use crate::ui::tui::bootstrap_discovery::bootstrap_task_data_for_root;

struct RefreshedAppState {
    repo_name: String,
    discovery_state: DiscoveryState,
    state: AppState,
}

fn refreshed_app_state(
    tasks_root_path: Option<&Path>,
    projects: &[ProjectConfig],
    hidden_base_project_paths: &[PathBuf],
) -> RefreshedAppState {
    let bootstrap = tasks_root_path
        .map(|tasks_root| {
            bootstrap_task_data_for_root(tasks_root, projects, hidden_base_project_paths)
        })
        .unwrap_or_else(|| crate::application::task_discovery::TaskBootstrapData {
            tasks: Vec::new(),
            discovery_state: TaskDiscoveryState::Empty,
        });
    RefreshedAppState {
        repo_name: if bootstrap.tasks.is_empty() {
            "tasks".to_string()
        } else {
            format!("{} tasks", bootstrap.tasks.len())
        },
        discovery_state: match bootstrap.discovery_state {
            TaskDiscoveryState::Ready => DiscoveryState::Ready,
            TaskDiscoveryState::Empty => DiscoveryState::Empty,
            TaskDiscoveryState::Error(message) => DiscoveryState::Error(message),
        },
        state: AppState::new(bootstrap.tasks),
    }
}

impl GroveApp {
    const MANUAL_WORKSPACE_REFRESH_COOLDOWN: Duration = Duration::from_secs(10);

    pub(super) fn resolved_tasks_root(&self) -> Option<PathBuf> {
        #[cfg(test)]
        if let Some(path) = self.task_root_override.clone() {
            return Some(path);
        }

        tasks_root()
    }

    fn finalize_manual_workspace_refresh_feedback(&mut self) {
        if !self.dialogs.manual_refresh_feedback_pending {
            return;
        }
        self.dialogs.manual_refresh_feedback_pending = false;

        match &self.discovery_state {
            DiscoveryState::Ready => self.show_success_toast("workspace refresh complete"),
            DiscoveryState::Empty => {
                self.show_info_toast("workspace refresh complete, no workspaces found")
            }
            DiscoveryState::Error(message) => {
                self.show_error_toast(format!("workspace refresh failed: {message}"))
            }
        }
    }

    pub(super) fn request_manual_workspace_refresh(&mut self) {
        let now = Instant::now();
        if self.dialogs.refresh_in_flight {
            self.show_info_toast("workspace refresh already in progress");
            return;
        }

        if let Some(last_requested_at) = self.dialogs.last_manual_refresh_requested_at {
            let elapsed = now.saturating_duration_since(last_requested_at);
            if elapsed < Self::MANUAL_WORKSPACE_REFRESH_COOLDOWN {
                let remaining = Self::MANUAL_WORKSPACE_REFRESH_COOLDOWN.saturating_sub(elapsed);
                let remaining_seconds = remaining.as_secs().max(1);
                self.show_info_toast(format!("refresh throttled, retry in {remaining_seconds}s"));
                return;
            }
        }

        self.dialogs.last_manual_refresh_requested_at = Some(now);
        self.dialogs.manual_refresh_feedback_pending = true;
        self.show_info_toast("refreshing workspaces...");
        self.refresh_workspaces(None);
    }

    pub(super) fn refresh_workspaces(&mut self, preferred_workspace_path: Option<PathBuf>) {
        if !self.tmux_input.supports_background_launch() {
            self.refresh_workspaces_sync_with_root(
                preferred_workspace_path,
                self.resolved_tasks_root(),
            );
            return;
        }

        if self.dialogs.refresh_in_flight {
            return;
        }

        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let tasks_root_path = self.resolved_tasks_root();
        let projects = self.projects.clone();
        let hidden_base_project_paths = self.hidden_base_project_paths_for_config();
        self.dialogs.refresh_in_flight = true;
        self.queue_cmd(Cmd::task(move || {
            let refreshed = refreshed_app_state(
                tasks_root_path.as_deref(),
                &projects,
                &hidden_base_project_paths,
            );
            Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
                preferred_workspace_path: target_path,
                repo_name: refreshed.repo_name,
                discovery_state: refreshed.discovery_state,
                tasks: refreshed.state.tasks,
            })
        }));
    }

    fn refresh_workspaces_sync_with_root(
        &mut self,
        preferred_workspace_path: Option<PathBuf>,
        tasks_root_path: Option<PathBuf>,
    ) {
        let target_path = preferred_workspace_path.or_else(|| self.selected_workspace_path());
        let previous_mode = self.state.mode;
        let refreshed = refreshed_app_state(
            tasks_root_path.as_deref(),
            &self.projects,
            &self.hidden_base_project_paths_for_config(),
        );

        self.repo_name = refreshed.repo_name;
        self.discovery_state = refreshed.discovery_state;
        self.state = refreshed.state;
        if let Some(path) = target_path
            && self.state.select_workspace_path(path)
        {}
        self.prune_workspace_visit_order();
        self.record_selected_workspace_visit();
        self.reconcile_task_order();
        self.reorder_tasks_for_task_order();
        self.state.mode = previous_mode;
        self.sync_workspace_tab_maps();
        self.reconcile_workspace_attention_tracking();
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
        self.finalize_manual_workspace_refresh_feedback();
    }

    pub(super) fn apply_refresh_workspaces_completion(
        &mut self,
        completion: RefreshWorkspacesCompletion,
    ) {
        let previous_mode = self.state.mode;

        self.repo_name = completion.repo_name;
        self.discovery_state = completion.discovery_state;
        self.state = AppState::new(completion.tasks);
        if let Some(path) = completion.preferred_workspace_path
            && self.state.select_workspace_path(path)
        {}
        self.prune_workspace_visit_order();
        self.record_selected_workspace_visit();
        self.reconcile_task_order();
        self.reorder_tasks_for_task_order();
        self.state.mode = previous_mode;
        self.sync_workspace_tab_maps();
        self.dialogs.refresh_in_flight = false;
        self.reconcile_workspace_attention_tracking();
        self.clear_agent_activity_tracking();
        self.clear_status_tracking();
        self.poll_preview();
        self.finalize_manual_workspace_refresh_feedback();
    }
}

#[cfg(test)]
mod tests {
    use super::refreshed_app_state;
    use crate::domain::{AgentType, Task, WorkspaceStatus, Worktree};
    use crate::infrastructure::adapters::DiscoveryState;
    use crate::infrastructure::config::{ProjectConfig, ProjectDefaults};
    use crate::infrastructure::task_manifest::encode_task_manifest;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
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
                "grove-refresh-{label}-{}-{timestamp}",
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

    fn fixture_task() -> Task {
        let app = Worktree::try_new(
            "flohome".to_string(),
            PathBuf::from("/repos/flohome"),
            PathBuf::from("/tmp/.grove/tasks/flohome-launch/flohome"),
            "flohome-launch".to_string(),
            AgentType::Codex,
            WorkspaceStatus::Idle,
        )
        .expect("app worktree should be valid");
        let infra = Worktree::try_new(
            "terraform-fastly".to_string(),
            PathBuf::from("/repos/terraform-fastly"),
            PathBuf::from("/tmp/.grove/tasks/flohome-launch/terraform-fastly"),
            "flohome-launch".to_string(),
            AgentType::Codex,
            WorkspaceStatus::Idle,
        )
        .expect("infra worktree should be valid");
        Task::try_new(
            "flohome-launch".to_string(),
            "flohome-launch".to_string(),
            PathBuf::from("/tmp/.grove/tasks/flohome-launch"),
            "flohome-launch".to_string(),
            vec![app, infra],
        )
        .expect("task should be valid")
    }

    fn init_git_repo(repo_root: &PathBuf, branch: &str) {
        fs::create_dir_all(repo_root).expect("repo root should exist");
        for args in [
            vec!["init", &format!("--initial-branch={branch}")],
            vec!["config", "user.name", "Grove Tests"],
            vec!["config", "user.email", "grove@example.com"],
        ] {
            let output = Command::new("git")
                .current_dir(repo_root)
                .args(args)
                .output()
                .expect("git command should run");
            assert!(output.status.success(), "git command should succeed");
        }
        fs::write(repo_root.join("README.md"), "hello\n").expect("readme should write");
        for args in [vec!["add", "README.md"], vec!["commit", "-m", "init"]] {
            let output = Command::new("git")
                .current_dir(repo_root)
                .args(args)
                .output()
                .expect("git command should run");
            assert!(output.status.success(), "git command should succeed");
        }
    }

    #[test]
    fn refreshed_app_state_loads_tasks_from_manifests() {
        let temp = TestDir::new("task-state");
        let task = fixture_task();
        let task_dir = temp.path.join("flohome-launch").join(".grove");
        fs::create_dir_all(&task_dir).expect("task dir should exist");
        let raw = encode_task_manifest(&task).expect("task manifest should encode");
        fs::write(task_dir.join("task.toml"), raw).expect("task manifest should write");

        let refreshed = refreshed_app_state(Some(temp.path.as_path()), &[], &[]);

        assert_eq!(refreshed.repo_name, "1 tasks");
        assert_eq!(refreshed.discovery_state, DiscoveryState::Ready);
        assert_eq!(refreshed.state.tasks.len(), 1);
        assert_eq!(refreshed.state.tasks[0].worktrees.len(), 2);
        assert_eq!(refreshed.state.tasks[0].slug, "flohome-launch");
    }

    #[test]
    fn refreshed_app_state_migrates_configured_repo_into_base_task_manifest() {
        let temp = TestDir::new("configured-repo");
        let tasks_root = temp.path.join("tasks");
        let repo_path = temp.path.join("repos").join("mcp");
        init_git_repo(&repo_path, "main");

        let refreshed = refreshed_app_state(
            Some(tasks_root.as_path()),
            &[ProjectConfig {
                name: "mcp".to_string(),
                path: repo_path.clone(),
                defaults: ProjectDefaults::default(),
            }],
            &[],
        );

        assert_eq!(refreshed.discovery_state, DiscoveryState::Ready);
        assert_eq!(refreshed.state.tasks.len(), 1);
        assert_eq!(refreshed.state.tasks[0].slug, "mcp");
        assert_eq!(
            refreshed.state.tasks[0].worktrees[0].repository_path,
            repo_path
        );
        assert!(
            tasks_root
                .join("mcp")
                .join(".grove")
                .join("task.toml")
                .exists()
        );
    }

    #[test]
    fn refreshed_app_state_skips_hidden_base_project_paths() {
        let temp = TestDir::new("hidden-configured-repo");
        let tasks_root = temp.path.join("tasks");
        let repo_path = temp.path.join("repos").join("mcp");
        init_git_repo(&repo_path, "main");

        let refreshed = refreshed_app_state(
            Some(tasks_root.as_path()),
            &[ProjectConfig {
                name: "mcp".to_string(),
                path: repo_path.clone(),
                defaults: ProjectDefaults::default(),
            }],
            std::slice::from_ref(&repo_path),
        );

        assert_eq!(refreshed.discovery_state, DiscoveryState::Empty);
        assert!(refreshed.state.tasks.is_empty());
        assert!(
            !tasks_root
                .join("mcp")
                .join(".grove")
                .join("task.toml")
                .exists()
        );
    }
}

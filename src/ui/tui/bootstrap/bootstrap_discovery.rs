use std::collections::HashSet;
use std::process::Command;

use crate::application::task_discovery::{
    TaskBootstrapData, TaskDiscoveryState,
    bootstrap_task_data_for_root_with_sessions as discover_task_bootstrap_for_root,
};
use crate::domain::{AgentType, Task, WorkspaceStatus, Worktree};
use crate::infrastructure::config::ProjectConfig;
use crate::infrastructure::paths::refer_to_same_location;
use std::path::Path;

pub(super) fn bootstrap_task_data_for_root(
    tasks_root: &Path,
    projects: &[ProjectConfig],
) -> TaskBootstrapData {
    let bootstrap = discover_task_bootstrap_for_root(tasks_root, &running_task_sessions());
    merge_bootstrap_with_configured_projects(bootstrap, projects)
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

fn merge_bootstrap_with_configured_projects(
    bootstrap: TaskBootstrapData,
    projects: &[ProjectConfig],
) -> TaskBootstrapData {
    let mut tasks = bootstrap.tasks;
    let mut taken_slugs = tasks
        .iter()
        .map(|task| task.slug.clone())
        .collect::<HashSet<String>>();

    for project in projects {
        let already_present = tasks.iter().any(|task| {
            task.worktrees
                .iter()
                .any(|worktree| refer_to_same_location(worktree.path.as_path(), &project.path))
        });
        if already_present {
            continue;
        }

        let Some(task) = synthesize_base_task(project, &taken_slugs) else {
            continue;
        };
        taken_slugs.insert(task.slug.clone());
        tasks.push(task);
    }

    tasks.sort_by(|left, right| left.slug.cmp(&right.slug));
    let discovery_state = match bootstrap.discovery_state {
        TaskDiscoveryState::Empty if !tasks.is_empty() => TaskDiscoveryState::Ready,
        other => other,
    };
    TaskBootstrapData {
        tasks,
        discovery_state,
    }
}

fn synthesize_base_task(project: &ProjectConfig, taken_slugs: &HashSet<String>) -> Option<Task> {
    let task_name = task_name_for_project(project);
    let base_slug = sanitize_task_slug(task_name.as_str())?;
    let task_slug = unique_task_slug(base_slug, taken_slugs);
    let branch = detect_repository_branch(project.path.as_path())
        .or_else(|| configured_base_branch(project))
        .unwrap_or_else(|| "main".to_string());
    let base_branch = detect_repository_base_branch(project.path.as_path())
        .or_else(|| configured_base_branch(project))
        .or_else(|| Some(branch.clone()));
    let worktree = Worktree::try_new(
        task_name.clone(),
        project.path.clone(),
        project.path.clone(),
        branch.clone(),
        AgentType::Codex,
        WorkspaceStatus::Main,
    )
    .ok()?
    .with_base_branch(base_branch);

    Task::try_new(
        task_name,
        task_slug,
        project.path.clone(),
        branch,
        vec![worktree],
    )
    .ok()
}

fn task_name_for_project(project: &ProjectConfig) -> String {
    project
        .path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| project.name.clone())
}

fn configured_base_branch(project: &ProjectConfig) -> Option<String> {
    let branch = project.defaults.base_branch.trim();
    if branch.is_empty() {
        return None;
    }

    Some(branch.to_string())
}

fn sanitize_task_slug(raw: &str) -> Option<String> {
    let mut slug = String::new();

    for character in raw.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed)
}

fn unique_task_slug(base_slug: String, taken_slugs: &HashSet<String>) -> String {
    if !taken_slugs.contains(base_slug.as_str()) {
        return base_slug;
    }

    let mut ordinal = 2usize;
    loop {
        let candidate = format!("{base_slug}-{ordinal}");
        if !taken_slugs.contains(candidate.as_str()) {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
}

fn detect_repository_branch(repo_root: &Path) -> Option<String> {
    git_optional_stdout(repo_root, &["branch", "--show-current"])
}

fn detect_repository_base_branch(repo_root: &Path) -> Option<String> {
    if let Some(remote_head) = git_optional_stdout(
        repo_root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    ) {
        let branch = remote_head
            .strip_prefix("origin/")
            .unwrap_or(remote_head.as_str())
            .trim();
        if !branch.is_empty() {
            return Some(branch.to_string());
        }
    }

    if let Some(current_branch) = detect_repository_branch(repo_root) {
        return Some(current_branch);
    }

    if git_branch_exists(repo_root, "main") {
        return Some("main".to_string());
    }
    if git_branch_exists(repo_root, "master") {
        return Some("master".to_string());
    }

    None
}

fn git_optional_stdout(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn git_branch_exists(repo_root: &Path, branch: &str) -> bool {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            format!("refs/heads/{branch}").as_str(),
        ])
        .output();
    matches!(output, Ok(result) if result.status.success())
}

#[cfg(test)]
mod tests {
    use super::bootstrap_task_data_for_root;
    use crate::domain::WorkspaceStatus;
    use crate::infrastructure::config::{AgentEnvDefaults, ProjectConfig, ProjectDefaults};
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

    fn project(path: PathBuf, base_branch: &str) -> ProjectConfig {
        ProjectConfig {
            name: "ignored-display-name".to_string(),
            path,
            defaults: ProjectDefaults {
                base_branch: base_branch.to_string(),
                workspace_init_command: String::new(),
                agent_env: AgentEnvDefaults::default(),
            },
        }
    }

    #[test]
    fn bootstrap_task_data_synthesizes_base_task_for_configured_repo_without_manifest() {
        let temp = TestDir::new("synth-base-task");
        let tasks_root = temp.path.join("tasks");
        let repo_path = temp.path.join("repos").join("mcp");
        fs::create_dir_all(&tasks_root).expect("tasks root should exist");
        fs::create_dir_all(&repo_path).expect("repo path should exist");

        let bootstrap =
            bootstrap_task_data_for_root(tasks_root.as_path(), &[project(repo_path, "main")]);

        assert_eq!(bootstrap.tasks.len(), 1);
        assert_eq!(bootstrap.tasks[0].slug, "mcp");
        assert_eq!(
            bootstrap.tasks[0].root_path,
            bootstrap.tasks[0].worktrees[0].path
        );
        assert_eq!(bootstrap.tasks[0].worktrees[0].branch, "main");
        assert_eq!(
            bootstrap.tasks[0].worktrees[0].status,
            WorkspaceStatus::Main
        );
    }

    #[test]
    fn bootstrap_task_data_keeps_feature_task_and_synthesizes_missing_base_checkout() {
        let temp = TestDir::new("synth-base-with-feature");
        let tasks_root = temp.path.join("tasks");
        let task_dir = tasks_root.join("feature-a").join(".grove");
        let repo_path = temp.path.join("repos").join("web-monorepo");
        let feature_path = temp.path.join("worktrees").join("feature-a");
        fs::create_dir_all(&task_dir).expect("task dir should exist");
        fs::create_dir_all(&repo_path).expect("repo path should exist");
        fs::create_dir_all(&feature_path).expect("feature path should exist");
        fs::write(
            task_dir.join("task.toml"),
            format!(
                "name = \"feature-a\"\nslug = \"feature-a\"\nroot_path = \"{}\"\nbranch = \"feature-a\"\n\n[[worktrees]]\nrepository_name = \"web-monorepo\"\nrepository_path = \"{}\"\npath = \"{}\"\nbranch = \"feature-a\"\nagent = \"codex\"\nstatus = \"idle\"\nis_orphaned = false\nsupported_agent = true\npull_requests = []\n",
                tasks_root.join("feature-a").display(),
                repo_path.display(),
                feature_path.display(),
            ),
        )
        .expect("task manifest should write");

        let bootstrap = bootstrap_task_data_for_root(
            tasks_root.as_path(),
            &[project(repo_path.clone(), "main")],
        );

        assert_eq!(bootstrap.tasks.len(), 2);
        assert!(bootstrap.tasks.iter().any(|task| task.slug == "feature-a"));
        assert!(
            bootstrap
                .tasks
                .iter()
                .any(|task| task.slug == "web-monorepo" && task.root_path == repo_path)
        );
    }
}

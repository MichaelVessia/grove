use std::collections::HashSet;
use std::process::Command;

use crate::application::services::discovery_service::bootstrap_data_for_projects as discover_bootstrap_for_projects;
use crate::application::task_discovery::{
    TaskBootstrapData,
    bootstrap_task_data_for_root_with_sessions as discover_task_bootstrap_for_root,
};
use crate::infrastructure::adapters::BootstrapData;
use crate::infrastructure::config::ProjectConfig;
use std::path::Path;

pub(super) fn bootstrap_data_for_projects(projects: &[ProjectConfig]) -> BootstrapData {
    discover_bootstrap_for_projects(projects)
}

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

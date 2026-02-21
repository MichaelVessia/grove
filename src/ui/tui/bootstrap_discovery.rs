use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::{
    BootstrapData, CommandGitAdapter, CommandMultiplexerAdapter, CommandSystemAdapter,
    DiscoveryState, MultiplexerAdapter, bootstrap_data,
};
use crate::infrastructure::config::ProjectConfig;
use crate::interface::daemon::{DaemonWorkspaceView, workspace_list_via_socket};

#[derive(Debug, Clone)]
struct StaticMultiplexerAdapter {
    running_sessions: HashSet<String>,
}

impl MultiplexerAdapter for StaticMultiplexerAdapter {
    fn running_sessions(&self) -> HashSet<String> {
        self.running_sessions.clone()
    }
}

pub(super) fn bootstrap_data_for_projects_with_transport(
    projects: &[ProjectConfig],
    daemon_socket_path: Option<&Path>,
) -> BootstrapData {
    if projects.is_empty() {
        return BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Empty,
        };
    }

    let static_multiplexer = daemon_socket_path.map(|_| StaticMultiplexerAdapter {
        running_sessions: HashSet::new(),
    });
    let static_multiplexer = static_multiplexer.unwrap_or_else(|| {
        let live_multiplexer = CommandMultiplexerAdapter;
        StaticMultiplexerAdapter {
            running_sessions: live_multiplexer.running_sessions(),
        }
    });
    let mut workspaces = Vec::new();
    let mut errors = Vec::new();
    for project in projects {
        if let Some(socket_path) = daemon_socket_path {
            match list_workspaces_for_project_via_socket(socket_path, project) {
                Ok(project_workspaces) => workspaces.extend(project_workspaces),
                Err(error) => errors.push(format!("{}: {error}", project.name)),
            }
            continue;
        }

        let git = CommandGitAdapter::for_repo(project.path.clone());
        let system = CommandSystemAdapter::for_repo(project.path.clone());
        let bootstrap = bootstrap_data(&git, &static_multiplexer, &system);
        if let DiscoveryState::Error(message) = &bootstrap.discovery_state {
            errors.push(format!("{}: {message}", project.name));
        }
        workspaces.extend(bootstrap.workspaces);
    }

    let discovery_state = if !workspaces.is_empty() {
        DiscoveryState::Ready
    } else if !errors.is_empty() {
        DiscoveryState::Error(errors.join("; "))
    } else {
        DiscoveryState::Empty
    };
    let repo_name = if projects.len() == 1 {
        projects[0].name.clone()
    } else {
        format!("{} projects", projects.len())
    };

    BootstrapData {
        repo_name,
        workspaces,
        discovery_state,
    }
}

fn list_workspaces_for_project_via_socket(
    socket_path: &Path,
    project: &ProjectConfig,
) -> Result<Vec<Workspace>, String> {
    let response = workspace_list_via_socket(socket_path, &project.path)
        .map_err(|error| format!("daemon request failed: {error}"))?;
    let result = response.map_err(|error| error.message)?;
    result
        .workspaces
        .into_iter()
        .map(|workspace| daemon_workspace_to_domain(workspace, project))
        .collect()
}

fn daemon_workspace_to_domain(
    workspace: DaemonWorkspaceView,
    project: &ProjectConfig,
) -> Result<Workspace, String> {
    let agent = parse_agent_label(&workspace.agent)?;
    let status = parse_status_label(&workspace.status)?;
    Ok(Workspace {
        name: workspace.name,
        path: PathBuf::from(workspace.path),
        project_name: workspace.project_name.or(Some(project.name.clone())),
        project_path: workspace
            .project_path
            .map(PathBuf::from)
            .or(Some(project.path.clone())),
        branch: workspace.branch,
        base_branch: workspace.base_branch,
        last_activity_unix_secs: workspace.last_activity_unix_secs,
        agent,
        status,
        is_main: workspace.is_main,
        is_orphaned: workspace.is_orphaned,
        supported_agent: workspace.supported_agent,
    })
}

fn parse_agent_label(label: &str) -> Result<AgentType, String> {
    match label.trim().to_ascii_lowercase().as_str() {
        "claude" => Ok(AgentType::Claude),
        "codex" => Ok(AgentType::Codex),
        _ => Err(format!("invalid daemon agent label: {label}")),
    }
}

fn parse_status_label(label: &str) -> Result<WorkspaceStatus, String> {
    match label.trim().to_ascii_lowercase().as_str() {
        "main" => Ok(WorkspaceStatus::Main),
        "idle" => Ok(WorkspaceStatus::Idle),
        "active" => Ok(WorkspaceStatus::Active),
        "thinking" => Ok(WorkspaceStatus::Thinking),
        "waiting" => Ok(WorkspaceStatus::Waiting),
        "done" => Ok(WorkspaceStatus::Done),
        "error" => Ok(WorkspaceStatus::Error),
        "unknown" => Ok(WorkspaceStatus::Unknown),
        "unsupported" => Ok(WorkspaceStatus::Unsupported),
        _ => Err(format!("invalid daemon workspace status label: {label}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{daemon_workspace_to_domain, parse_agent_label, parse_status_label};
    use crate::infrastructure::config::ProjectConfig;
    use crate::interface::daemon::DaemonWorkspaceView;
    use std::path::PathBuf;

    #[test]
    fn daemon_workspace_conversion_defaults_project_context() {
        let project = ProjectConfig {
            name: "grove".to_string(),
            path: PathBuf::from("/repos/grove"),
            defaults: Default::default(),
        };
        let workspace = DaemonWorkspaceView {
            name: "feature-a".to_string(),
            path: "/repos/grove-feature-a".to_string(),
            project_name: None,
            project_path: None,
            branch: "feature-a".to_string(),
            base_branch: Some("main".to_string()),
            last_activity_unix_secs: Some(1),
            agent: "codex".to_string(),
            status: "idle".to_string(),
            is_main: false,
            is_orphaned: false,
            supported_agent: true,
        };

        let converted =
            daemon_workspace_to_domain(workspace, &project).expect("workspace should convert");

        assert_eq!(converted.project_name.as_deref(), Some("grove"));
        assert_eq!(
            converted.project_path.as_deref(),
            Some(PathBuf::from("/repos/grove").as_path())
        );
    }

    #[test]
    fn daemon_workspace_label_parsers_reject_unknown_values() {
        assert!(parse_agent_label("unknown-agent").is_err());
        assert!(parse_status_label("unknown-status").is_err());
    }
}

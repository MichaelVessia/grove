use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::application::agent_runtime::{
    TMUX_SESSION_PREFIX, kill_task_session_commands, task_session_names_for_cleanup,
    workspace_session_name_matches,
};
use crate::application::task_discovery::bootstrap_task_data_for_root;
use crate::application::workspace_discovery::discover_bootstrap_data;
use crate::domain::{Task, Workspace};
use crate::infrastructure::adapters::{
    CommandGitAdapter, CommandSystemAdapter, DiscoveryState, MultiplexerAdapter,
};
use crate::infrastructure::config::ProjectConfig;
use crate::infrastructure::paths::refer_to_same_location;
use crate::infrastructure::paths::tasks_root;
use crate::infrastructure::process::{execute_command, stderr_trimmed};

const STALE_AUXILIARY_MIN_AGE_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCleanupOptions {
    pub include_stale: bool,
    pub include_attached: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCleanupReason {
    Orphaned,
    StaleAuxiliary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCleanupEntry {
    pub session_name: String,
    pub reason: SessionCleanupReason,
    pub created_unix_secs: Option<u64>,
    pub age_secs: Option<u64>,
    pub attached_clients: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCleanupPlan {
    pub candidates: Vec<SessionCleanupEntry>,
    pub skipped_attached: Vec<SessionCleanupEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCleanupApplyResult {
    pub killed: Vec<String>,
    pub already_gone: Vec<String>,
    pub failures: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionRecord {
    name: String,
    created_unix_secs: Option<u64>,
    attached_clients: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct NoopMultiplexerAdapter;

impl MultiplexerAdapter for NoopMultiplexerAdapter {
    fn running_sessions(&self) -> HashSet<String> {
        HashSet::new()
    }
}

pub fn plan_session_cleanup(options: SessionCleanupOptions) -> Result<SessionCleanupPlan, String> {
    if let Some(task_root) = tasks_root() {
        let bootstrap = bootstrap_task_data_for_root(task_root.as_path());
        return plan_session_cleanup_for_tasks(bootstrap.tasks.as_slice(), options);
    }

    let projects = cleanup_projects()?;
    let workspaces = discover_workspaces_for_projects(projects.as_slice());
    plan_session_cleanup_for_workspaces(workspaces.as_slice(), options)
}

pub fn plan_session_cleanup_for_tasks(
    tasks: &[Task],
    options: SessionCleanupOptions,
) -> Result<SessionCleanupPlan, String> {
    let sessions = list_tmux_sessions()?;
    let now_unix_secs = now_unix_secs();
    Ok(plan_session_cleanup_from_task_inputs(
        tasks,
        sessions.as_slice(),
        options,
        now_unix_secs,
    ))
}

pub fn plan_session_cleanup_for_workspaces(
    workspaces: &[Workspace],
    options: SessionCleanupOptions,
) -> Result<SessionCleanupPlan, String> {
    let sessions = list_tmux_sessions()?;
    let now_unix_secs = now_unix_secs();
    Ok(plan_session_cleanup_from_inputs(
        workspaces,
        sessions.as_slice(),
        options,
        now_unix_secs,
    ))
}

pub fn apply_session_cleanup(plan: &SessionCleanupPlan) -> SessionCleanupApplyResult {
    let mut killed = Vec::new();
    let mut already_gone = Vec::new();
    let mut failures = Vec::new();

    for candidate in &plan.candidates {
        let command = vec![
            "tmux".to_string(),
            "kill-session".to_string(),
            "-t".to_string(),
            candidate.session_name.clone(),
        ];
        match execute_command(command.as_slice()) {
            Ok(()) => {
                killed.push(candidate.session_name.clone());
            }
            Err(error) => {
                let message = error.to_string();
                if session_missing_error(message.as_str()) {
                    already_gone.push(candidate.session_name.clone());
                } else {
                    failures.push((candidate.session_name.clone(), message));
                }
            }
        }
    }

    SessionCleanupApplyResult {
        killed,
        already_gone,
        failures,
    }
}

pub fn cleanup_commands_for_task(task: &Task) -> Vec<Vec<String>> {
    kill_task_session_commands(task)
}

fn cleanup_projects() -> Result<Vec<ProjectConfig>, String> {
    let mut projects = match crate::infrastructure::config::load() {
        Ok(loaded) => loaded.config.projects,
        Err(_) => Vec::new(),
    };

    if let Some(repo_root) = current_repo_root() {
        let exists = projects
            .iter()
            .any(|project| refer_to_same_location(project.path.as_path(), repo_root.as_path()));
        if !exists {
            projects.push(ProjectConfig {
                name: project_display_name(repo_root.as_path()),
                path: repo_root,
                defaults: Default::default(),
            });
        }
    }

    if projects.is_empty() {
        return Err("no projects configured and no current git repository detected".to_string());
    }

    Ok(projects)
}

fn current_repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = PathBuf::from(trimmed);
    path.canonicalize().ok().or(Some(path))
}

fn project_display_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string())
}

fn discover_workspaces_for_projects(projects: &[ProjectConfig]) -> Vec<Workspace> {
    let multiplexer = NoopMultiplexerAdapter;
    let mut workspaces = Vec::new();
    for project in projects {
        let git = CommandGitAdapter::for_repo(project.path.clone());
        let system = CommandSystemAdapter::for_repo(project.path.clone());
        let bootstrap = discover_bootstrap_data(&git, &multiplexer, &system);
        if matches!(bootstrap.discovery_state, DiscoveryState::Ready) {
            workspaces.extend(bootstrap.workspaces);
        }
    }
    workspaces
}

fn list_tmux_sessions() -> Result<Vec<SessionRecord>, String> {
    let output = Command::new("tmux")
        .args([
            "list-sessions",
            "-F",
            "#{session_name}\t#{session_created}\t#{session_attached}",
        ])
        .output()
        .map_err(|error| format!("tmux list-sessions failed: {error}"))?;
    if !output.status.success() {
        let stderr = stderr_trimmed(&output);
        if stderr.contains("no server running") {
            return Ok(Vec::new());
        }
        return Err(format!("tmux list-sessions failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("tmux output invalid UTF-8: {error}"))?;
    let mut sessions = Vec::new();
    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.splitn(3, '\t');
        let Some(name_part) = parts.next() else {
            continue;
        };
        let Some(created_part) = parts.next() else {
            continue;
        };
        let Some(attached_part) = parts.next() else {
            continue;
        };

        let name = name_part.trim().to_string();
        if name.is_empty() || !name.starts_with(TMUX_SESSION_PREFIX) {
            continue;
        }

        let created_unix_secs = created_part.trim().parse::<u64>().ok();
        let attached_clients = attached_part.trim().parse::<u32>().unwrap_or(0);
        sessions.push(SessionRecord {
            name,
            created_unix_secs,
            attached_clients,
        });
    }

    Ok(sessions)
}

fn plan_session_cleanup_from_inputs(
    workspaces: &[Workspace],
    sessions: &[SessionRecord],
    options: SessionCleanupOptions,
    now_unix_secs: u64,
) -> SessionCleanupPlan {
    let mut candidates = Vec::new();
    let mut skipped_attached = Vec::new();

    for session in sessions {
        let reason = match cleanup_reason(session, workspaces, options.include_stale, now_unix_secs)
        {
            Some(reason) => reason,
            None => continue,
        };
        let age_secs = session
            .created_unix_secs
            .and_then(|created| now_unix_secs.checked_sub(created));
        let entry = SessionCleanupEntry {
            session_name: session.name.clone(),
            reason,
            created_unix_secs: session.created_unix_secs,
            age_secs,
            attached_clients: session.attached_clients,
        };

        if !options.include_attached && session.attached_clients > 0 {
            skipped_attached.push(entry);
        } else {
            candidates.push(entry);
        }
    }

    candidates.sort_by(|left, right| left.session_name.cmp(&right.session_name));
    skipped_attached.sort_by(|left, right| left.session_name.cmp(&right.session_name));
    SessionCleanupPlan {
        candidates,
        skipped_attached,
    }
}

fn plan_session_cleanup_from_task_inputs(
    tasks: &[Task],
    sessions: &[SessionRecord],
    options: SessionCleanupOptions,
    now_unix_secs: u64,
) -> SessionCleanupPlan {
    let mut candidates = Vec::new();
    let mut skipped_attached = Vec::new();

    for session in sessions {
        let reason =
            match cleanup_reason_for_tasks(session, tasks, options.include_stale, now_unix_secs) {
                Some(reason) => reason,
                None => continue,
            };
        let age_secs = session
            .created_unix_secs
            .and_then(|created| now_unix_secs.checked_sub(created));
        let entry = SessionCleanupEntry {
            session_name: session.name.clone(),
            reason,
            created_unix_secs: session.created_unix_secs,
            age_secs,
            attached_clients: session.attached_clients,
        };

        if !options.include_attached && session.attached_clients > 0 {
            skipped_attached.push(entry);
        } else {
            candidates.push(entry);
        }
    }

    candidates.sort_by(|left, right| left.session_name.cmp(&right.session_name));
    skipped_attached.sort_by(|left, right| left.session_name.cmp(&right.session_name));
    SessionCleanupPlan {
        candidates,
        skipped_attached,
    }
}

fn cleanup_reason(
    session: &SessionRecord,
    workspaces: &[Workspace],
    include_stale: bool,
    now_unix_secs: u64,
) -> Option<SessionCleanupReason> {
    let belongs_to_workspace = workspaces.iter().any(|workspace| {
        workspace_session_name_matches(
            workspace.project_name.as_deref(),
            workspace.name.as_str(),
            session.name.as_str(),
        )
    });
    if !belongs_to_workspace {
        return Some(SessionCleanupReason::Orphaned);
    }

    if include_stale && stale_auxiliary_session(session, now_unix_secs) {
        return Some(SessionCleanupReason::StaleAuxiliary);
    }

    None
}

fn cleanup_reason_for_tasks(
    session: &SessionRecord,
    tasks: &[Task],
    include_stale: bool,
    now_unix_secs: u64,
) -> Option<SessionCleanupReason> {
    let session_names = [session.name.clone()];
    let belongs_to_task = tasks
        .iter()
        .any(|task| !task_session_names_for_cleanup(task, session_names.as_slice()).is_empty());
    if !belongs_to_task {
        return Some(SessionCleanupReason::Orphaned);
    }

    if include_stale && stale_auxiliary_session(session, now_unix_secs) {
        return Some(SessionCleanupReason::StaleAuxiliary);
    }

    None
}

fn stale_auxiliary_session(session: &SessionRecord, now_unix_secs: u64) -> bool {
    if !is_auxiliary_session(session.name.as_str()) {
        return false;
    }
    let Some(created_unix_secs) = session.created_unix_secs else {
        return false;
    };
    let Some(age) = now_unix_secs.checked_sub(created_unix_secs) else {
        return false;
    };
    age >= STALE_AUXILIARY_MIN_AGE_SECS
}

fn is_auxiliary_session(session_name: &str) -> bool {
    if session_name.ends_with("-git") || session_name.ends_with("-shell") {
        return true;
    }

    let Some((_, ordinal)) = session_name.rsplit_once("-shell-") else {
        return false;
    };
    !ordinal.is_empty() && ordinal.chars().all(|character| character.is_ascii_digit())
}

fn session_missing_error(message: &str) -> bool {
    message.contains("can't find session")
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::{
        SessionCleanupOptions, SessionCleanupReason, SessionRecord, cleanup_commands_for_task,
        plan_session_cleanup_from_inputs,
    };
    use crate::domain::{AgentType, Task, Workspace, WorkspaceStatus, Worktree};
    use std::path::PathBuf;

    fn fixture_workspace(name: &str) -> Workspace {
        Workspace::try_new(
            name.to_string(),
            PathBuf::from(format!("/tmp/grove-{name}")),
            name.to_string(),
            Some(1_700_000_000),
            AgentType::Claude,
            WorkspaceStatus::Idle,
            false,
        )
        .expect("workspace should be valid")
        .with_project_context("grove".to_string(), PathBuf::from("/tmp/grove"))
    }

    fn fixture_task() -> Task {
        Task::try_new(
            "flohome-launch".to_string(),
            "flohome-launch".to_string(),
            PathBuf::from("/tmp/.grove/tasks/flohome-launch"),
            "flohome-launch".to_string(),
            vec![
                Worktree::try_new(
                    "flohome".to_string(),
                    PathBuf::from("/repos/flohome"),
                    PathBuf::from("/tmp/.grove/tasks/flohome-launch/flohome"),
                    "flohome-launch".to_string(),
                    AgentType::Codex,
                    WorkspaceStatus::Idle,
                )
                .expect("worktree should be valid"),
                Worktree::try_new(
                    "terraform-fastly".to_string(),
                    PathBuf::from("/repos/terraform-fastly"),
                    PathBuf::from("/tmp/.grove/tasks/flohome-launch/terraform-fastly"),
                    "flohome-launch".to_string(),
                    AgentType::Codex,
                    WorkspaceStatus::Idle,
                )
                .expect("worktree should be valid"),
            ],
        )
        .expect("task should be valid")
    }

    #[test]
    fn session_cleanup_targets_task_and_worktree_sessions() {
        let commands = cleanup_commands_for_task(&fixture_task());
        let rendered = commands
            .iter()
            .map(|command| command.join(" "))
            .collect::<Vec<String>>();

        assert!(
            rendered
                .iter()
                .any(|command| command.contains("grove-task-"))
        );
        assert!(rendered.iter().any(|command| command.contains("grove-wt-")));
    }

    #[test]
    fn plan_marks_orphan_sessions() {
        let workspace = fixture_workspace("feature-a");
        let sessions = vec![
            SessionRecord {
                name: "grove-ws-grove-feature-a".to_string(),
                created_unix_secs: Some(1_700_000_100),
                attached_clients: 0,
            },
            SessionRecord {
                name: "grove-ws-grove-lost".to_string(),
                created_unix_secs: Some(1_700_000_050),
                attached_clients: 0,
            },
        ];

        let plan = plan_session_cleanup_from_inputs(
            &[workspace],
            sessions.as_slice(),
            SessionCleanupOptions {
                include_stale: false,
                include_attached: false,
            },
            1_700_001_000,
        );

        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].session_name, "grove-ws-grove-lost");
        assert_eq!(plan.candidates[0].reason, SessionCleanupReason::Orphaned);
        assert!(plan.skipped_attached.is_empty());
    }

    #[test]
    fn plan_includes_stale_auxiliary_only_when_opted_in() {
        let workspace = fixture_workspace("feature-a");
        let stale_git = SessionRecord {
            name: "grove-ws-grove-feature-a-git".to_string(),
            created_unix_secs: Some(1_700_000_000),
            attached_clients: 0,
        };

        let without_stale = plan_session_cleanup_from_inputs(
            std::slice::from_ref(&workspace),
            std::slice::from_ref(&stale_git),
            SessionCleanupOptions {
                include_stale: false,
                include_attached: false,
            },
            1_700_090_000,
        );
        assert!(without_stale.candidates.is_empty());

        let with_stale = plan_session_cleanup_from_inputs(
            &[workspace],
            &[stale_git],
            SessionCleanupOptions {
                include_stale: true,
                include_attached: false,
            },
            1_700_090_000,
        );
        assert_eq!(with_stale.candidates.len(), 1);
        assert_eq!(
            with_stale.candidates[0].reason,
            SessionCleanupReason::StaleAuxiliary
        );
    }

    #[test]
    fn plan_skips_attached_by_default() {
        let workspace = fixture_workspace("feature-a");
        let orphan_attached = SessionRecord {
            name: "grove-ws-grove-lost".to_string(),
            created_unix_secs: Some(1_700_000_100),
            attached_clients: 1,
        };

        let default_plan = plan_session_cleanup_from_inputs(
            std::slice::from_ref(&workspace),
            std::slice::from_ref(&orphan_attached),
            SessionCleanupOptions {
                include_stale: false,
                include_attached: false,
            },
            1_700_090_000,
        );
        assert!(default_plan.candidates.is_empty());
        assert_eq!(default_plan.skipped_attached.len(), 1);

        let include_attached_plan = plan_session_cleanup_from_inputs(
            &[workspace],
            &[orphan_attached],
            SessionCleanupOptions {
                include_stale: false,
                include_attached: true,
            },
            1_700_090_000,
        );
        assert_eq!(include_attached_plan.candidates.len(), 1);
        assert!(include_attached_plan.skipped_attached.is_empty());
    }

    #[test]
    fn plan_recognizes_numbered_agent_and_shell_sessions_as_expected() {
        let workspace = fixture_workspace("feature-a");
        let sessions = vec![
            SessionRecord {
                name: "grove-ws-grove-feature-a-agent-1".to_string(),
                created_unix_secs: Some(1_700_000_100),
                attached_clients: 0,
            },
            SessionRecord {
                name: "grove-ws-grove-feature-a-shell-2".to_string(),
                created_unix_secs: Some(1_700_000_100),
                attached_clients: 0,
            },
            SessionRecord {
                name: "grove-ws-grove-lost-agent-1".to_string(),
                created_unix_secs: Some(1_700_000_100),
                attached_clients: 0,
            },
        ];

        let plan = plan_session_cleanup_from_inputs(
            &[workspace],
            sessions.as_slice(),
            SessionCleanupOptions {
                include_stale: false,
                include_attached: false,
            },
            1_700_010_000,
        );
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(
            plan.candidates[0].session_name,
            "grove-ws-grove-lost-agent-1"
        );
        assert_eq!(plan.candidates[0].reason, SessionCleanupReason::Orphaned);
    }

    #[test]
    fn plan_marks_numbered_shell_session_stale_when_enabled() {
        let workspace = fixture_workspace("feature-a");
        let sessions = vec![SessionRecord {
            name: "grove-ws-grove-feature-a-shell-3".to_string(),
            created_unix_secs: Some(1_700_000_000),
            attached_clients: 0,
        }];

        let plan = plan_session_cleanup_from_inputs(
            &[workspace],
            sessions.as_slice(),
            SessionCleanupOptions {
                include_stale: true,
                include_attached: false,
            },
            1_700_090_000,
        );
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(
            plan.candidates[0].reason,
            SessionCleanupReason::StaleAuxiliary
        );
    }
}

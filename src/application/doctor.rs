use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::application::session_cleanup::SessionRecord;
use crate::application::session_cleanup::{
    SessionCleanupReason, cleanup_reason_for_tasks, list_tmux_sessions, now_unix_secs,
};
use crate::domain::Task;
use crate::infrastructure::config::ProjectConfig;
use crate::infrastructure::paths::{refer_to_same_location, tasks_root};
use crate::infrastructure::task_manifest::decode_task_manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorSeverity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorFindingKind {
    InvalidTaskManifest,
    DuplicateTaskSlug,
    MissingWorktreePath,
    MissingBaseMarker,
    ConfiguredRepoMissingBaseTaskManifest,
    OrphanedGroveSession,
    StaleAuxiliarySession,
    LegacyGroveSessionMissingMetadata,
    ManifestRepositoryMismatch,
    SessionCheckSkipped,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DoctorSubject {
    pub task_slug: Option<String>,
    pub manifest_path: Option<String>,
    pub repository_path: Option<String>,
    pub worktree_path: Option<String>,
    pub session_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorFinding {
    pub severity: DoctorSeverity,
    pub kind: DoctorFindingKind,
    pub subject: DoctorSubject,
    pub evidence: String,
    pub recommended_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorSummary {
    pub total: usize,
    pub info: usize,
    pub warn: usize,
    pub error: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorRepairAction {
    InspectOrRewriteManifest,
    RemoveDuplicateManifestOwner,
    RestoreOrRemoveMissingWorktree,
    WriteBaseMarker,
    MaterializeBaseTaskManifest,
    KillOrAdoptSession,
    InspectRepositoryMapping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorRepairStep {
    pub priority: u8,
    pub action: DoctorRepairAction,
    pub reason: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub summary: DoctorSummary,
    pub findings: Vec<DoctorFinding>,
    pub repair_plan: Vec<DoctorRepairStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DoctorTmuxState {
    Available(Vec<SessionRecord>),
    Unavailable(String),
}

impl DoctorReport {
    pub fn from_findings(findings: Vec<DoctorFinding>) -> Self {
        let summary = DoctorSummary::from_findings(findings.as_slice());
        let repair_plan = repair_plan_from_findings(findings.as_slice());
        Self {
            summary,
            findings,
            repair_plan,
        }
    }
}

pub fn diagnose() -> Result<DoctorReport, String> {
    let loaded_config = crate::infrastructure::config::load()?;
    let tasks_root = tasks_root().ok_or_else(|| "task root unavailable".to_string())?;
    let tmux_state = match list_tmux_sessions() {
        Ok(sessions) => DoctorTmuxState::Available(sessions),
        Err(error) => DoctorTmuxState::Unavailable(error),
    };

    Ok(diagnose_from_inputs(
        Some(tasks_root.as_path()),
        loaded_config.config.projects.as_slice(),
        tmux_state,
    ))
}

pub(crate) fn diagnose_from_inputs(
    tasks_root: Option<&Path>,
    projects: &[ProjectConfig],
    tmux_state: DoctorTmuxState,
) -> DoctorReport {
    diagnose_from_inputs_at(tasks_root, projects, tmux_state, now_unix_secs())
}

pub(crate) fn diagnose_from_inputs_at(
    tasks_root: Option<&Path>,
    projects: &[ProjectConfig],
    tmux_state: DoctorTmuxState,
    now_unix_secs: u64,
) -> DoctorReport {
    let mut findings = Vec::new();
    let tasks = load_doctor_tasks(tasks_root, &mut findings);

    collect_duplicate_slug_findings(tasks.as_slice(), &mut findings);
    collect_worktree_findings(tasks.as_slice(), &mut findings);
    collect_missing_base_task_findings(tasks.as_slice(), projects, &mut findings);
    collect_tmux_findings(tasks.as_slice(), tmux_state, now_unix_secs, &mut findings);

    DoctorReport::from_findings(findings)
}

impl DoctorSummary {
    fn from_findings(findings: &[DoctorFinding]) -> Self {
        let mut summary = Self {
            total: findings.len(),
            info: 0,
            warn: 0,
            error: 0,
        };

        for finding in findings {
            match finding.severity {
                DoctorSeverity::Info => summary.info += 1,
                DoctorSeverity::Warn => summary.warn += 1,
                DoctorSeverity::Error => summary.error += 1,
            }
        }

        summary
    }
}

fn repair_plan_from_findings(findings: &[DoctorFinding]) -> Vec<DoctorRepairStep> {
    let mut deduped: BTreeMap<(u8, DoctorRepairAction, Vec<String>), DoctorRepairStep> =
        BTreeMap::new();

    for finding in findings {
        let Some(step) = repair_step_for_finding(finding) else {
            continue;
        };
        deduped
            .entry((step.priority, step.action, step.targets.clone()))
            .or_insert(step);
    }

    deduped.into_values().collect()
}

fn repair_step_for_finding(finding: &DoctorFinding) -> Option<DoctorRepairStep> {
    let (priority, action, reason) = match finding.kind {
        DoctorFindingKind::InvalidTaskManifest => (
            10,
            DoctorRepairAction::InspectOrRewriteManifest,
            "manifest is invalid".to_string(),
        ),
        DoctorFindingKind::DuplicateTaskSlug => (
            20,
            DoctorRepairAction::RemoveDuplicateManifestOwner,
            "multiple manifests claim the same task slug".to_string(),
        ),
        DoctorFindingKind::MissingWorktreePath => (
            30,
            DoctorRepairAction::RestoreOrRemoveMissingWorktree,
            "manifest references a missing worktree path".to_string(),
        ),
        DoctorFindingKind::MissingBaseMarker => (
            40,
            DoctorRepairAction::WriteBaseMarker,
            "worktree is missing a Grove base marker".to_string(),
        ),
        DoctorFindingKind::ConfiguredRepoMissingBaseTaskManifest => (
            50,
            DoctorRepairAction::MaterializeBaseTaskManifest,
            "configured repository is missing a base task manifest".to_string(),
        ),
        DoctorFindingKind::OrphanedGroveSession
        | DoctorFindingKind::StaleAuxiliarySession
        | DoctorFindingKind::LegacyGroveSessionMissingMetadata => (
            60,
            DoctorRepairAction::KillOrAdoptSession,
            "tmux session state no longer matches Grove task state".to_string(),
        ),
        DoctorFindingKind::ManifestRepositoryMismatch => (
            70,
            DoctorRepairAction::InspectRepositoryMapping,
            "manifest repository mapping does not match configured repository state".to_string(),
        ),
        DoctorFindingKind::SessionCheckSkipped => return None,
    };

    let targets = finding.subject.targets();

    Some(DoctorRepairStep {
        priority,
        action,
        reason,
        targets: if targets.is_empty() {
            vec![finding.kind.label().to_string()]
        } else {
            targets
        },
    })
}

impl DoctorSubject {
    pub(crate) fn targets(&self) -> Vec<String> {
        [
            self.manifest_path.clone(),
            self.task_slug.clone(),
            self.repository_path.clone(),
            self.worktree_path.clone(),
            self.session_name.clone(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

impl DoctorFindingKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            DoctorFindingKind::InvalidTaskManifest => "invalid_task_manifest",
            DoctorFindingKind::DuplicateTaskSlug => "duplicate_task_slug",
            DoctorFindingKind::MissingWorktreePath => "missing_worktree_path",
            DoctorFindingKind::MissingBaseMarker => "missing_base_marker",
            DoctorFindingKind::ConfiguredRepoMissingBaseTaskManifest => {
                "configured_repo_missing_base_task_manifest"
            }
            DoctorFindingKind::OrphanedGroveSession => "orphaned_grove_session",
            DoctorFindingKind::StaleAuxiliarySession => "stale_auxiliary_session",
            DoctorFindingKind::LegacyGroveSessionMissingMetadata => {
                "legacy_grove_session_missing_metadata"
            }
            DoctorFindingKind::ManifestRepositoryMismatch => "manifest_repository_mismatch",
            DoctorFindingKind::SessionCheckSkipped => "session_check_skipped",
        }
    }
}

impl DoctorRepairAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            DoctorRepairAction::InspectOrRewriteManifest => "inspect_or_rewrite_manifest",
            DoctorRepairAction::RemoveDuplicateManifestOwner => "remove_duplicate_manifest_owner",
            DoctorRepairAction::RestoreOrRemoveMissingWorktree => {
                "restore_or_remove_missing_worktree"
            }
            DoctorRepairAction::WriteBaseMarker => "write_base_marker",
            DoctorRepairAction::MaterializeBaseTaskManifest => "materialize_base_task_manifest",
            DoctorRepairAction::KillOrAdoptSession => "kill_or_adopt_session",
            DoctorRepairAction::InspectRepositoryMapping => "inspect_repository_mapping",
        }
    }
}

#[derive(Debug, Clone)]
struct LoadedDoctorTask {
    manifest_path: PathBuf,
    task: Task,
}

fn load_doctor_tasks(
    tasks_root: Option<&Path>,
    findings: &mut Vec<DoctorFinding>,
) -> Vec<LoadedDoctorTask> {
    let Some(tasks_root) = tasks_root else {
        return Vec::new();
    };
    if !tasks_root.exists() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(tasks_root) else {
        return Vec::new();
    };

    let mut tasks = Vec::new();
    for entry_result in entries {
        let Ok(entry) = entry_result else {
            continue;
        };
        let task_root = entry.path();
        if !task_root.is_dir() {
            continue;
        }

        let manifest_path = task_root.join(".grove").join("task.toml");
        if !manifest_path.exists() {
            continue;
        }

        let raw = match fs::read_to_string(&manifest_path) {
            Ok(raw) => raw,
            Err(error) => {
                findings.push(DoctorFinding {
                    severity: DoctorSeverity::Error,
                    kind: DoctorFindingKind::InvalidTaskManifest,
                    subject: DoctorSubject {
                        task_slug: None,
                        manifest_path: Some(manifest_path.to_string_lossy().into_owned()),
                        repository_path: None,
                        worktree_path: None,
                        session_name: None,
                    },
                    evidence: format!("task manifest read failed: {error}"),
                    recommended_action: "inspect or rewrite the task manifest".to_string(),
                });
                continue;
            }
        };

        match decode_task_manifest(raw.as_str()) {
            Ok(task) => tasks.push(LoadedDoctorTask {
                manifest_path,
                task,
            }),
            Err(error) => findings.push(DoctorFinding {
                severity: DoctorSeverity::Error,
                kind: DoctorFindingKind::InvalidTaskManifest,
                subject: DoctorSubject {
                    task_slug: None,
                    manifest_path: Some(manifest_path.to_string_lossy().into_owned()),
                    repository_path: None,
                    worktree_path: None,
                    session_name: None,
                },
                evidence: error,
                recommended_action: "inspect or rewrite the task manifest".to_string(),
            }),
        }
    }

    tasks.sort_by(|left, right| left.task.slug.cmp(&right.task.slug));
    tasks
}

fn collect_duplicate_slug_findings(tasks: &[LoadedDoctorTask], findings: &mut Vec<DoctorFinding>) {
    let mut by_slug: BTreeMap<&str, Vec<&LoadedDoctorTask>> = BTreeMap::new();
    for task in tasks {
        by_slug
            .entry(task.task.slug.as_str())
            .or_default()
            .push(task);
    }

    for (slug, owners) in by_slug {
        if owners.len() < 2 {
            continue;
        }

        for owner in owners {
            findings.push(DoctorFinding {
                severity: DoctorSeverity::Error,
                kind: DoctorFindingKind::DuplicateTaskSlug,
                subject: DoctorSubject {
                    task_slug: Some(slug.to_string()),
                    manifest_path: Some(owner.manifest_path.to_string_lossy().into_owned()),
                    repository_path: None,
                    worktree_path: None,
                    session_name: None,
                },
                evidence: format!("multiple manifests claim task slug '{slug}'"),
                recommended_action: "remove or rename one manifest owner".to_string(),
            });
        }
    }
}

fn collect_worktree_findings(tasks: &[LoadedDoctorTask], findings: &mut Vec<DoctorFinding>) {
    for loaded in tasks {
        for worktree in &loaded.task.worktrees {
            if !worktree.path.exists() {
                findings.push(DoctorFinding {
                    severity: DoctorSeverity::Error,
                    kind: DoctorFindingKind::MissingWorktreePath,
                    subject: DoctorSubject {
                        task_slug: Some(loaded.task.slug.clone()),
                        manifest_path: Some(loaded.manifest_path.to_string_lossy().into_owned()),
                        repository_path: Some(
                            worktree.repository_path.to_string_lossy().into_owned(),
                        ),
                        worktree_path: Some(worktree.path.to_string_lossy().into_owned()),
                        session_name: None,
                    },
                    evidence: format!("worktree path does not exist: {}", worktree.path.display()),
                    recommended_action: "restore the worktree path or remove it from the manifest"
                        .to_string(),
                });
                continue;
            }

            if worktree.is_main_checkout() {
                continue;
            }

            let base_marker_path = worktree.path.join(".grove").join("base");
            let missing_or_empty = fs::read_to_string(&base_marker_path)
                .map(|content| content.trim().is_empty())
                .unwrap_or(true);
            if missing_or_empty {
                findings.push(DoctorFinding {
                    severity: DoctorSeverity::Warn,
                    kind: DoctorFindingKind::MissingBaseMarker,
                    subject: DoctorSubject {
                        task_slug: Some(loaded.task.slug.clone()),
                        manifest_path: Some(loaded.manifest_path.to_string_lossy().into_owned()),
                        repository_path: Some(
                            worktree.repository_path.to_string_lossy().into_owned(),
                        ),
                        worktree_path: Some(worktree.path.to_string_lossy().into_owned()),
                        session_name: None,
                    },
                    evidence: format!(
                        "missing or empty base marker: {}",
                        base_marker_path.display()
                    ),
                    recommended_action: "write the worktree base marker".to_string(),
                });
            }
        }
    }
}

fn collect_missing_base_task_findings(
    tasks: &[LoadedDoctorTask],
    projects: &[ProjectConfig],
    findings: &mut Vec<DoctorFinding>,
) {
    for project in projects {
        let has_base_task = tasks.iter().any(|loaded| {
            loaded.task.worktrees.iter().any(|worktree| {
                worktree.is_main_checkout()
                    && refer_to_same_location(
                        worktree.repository_path.as_path(),
                        project.path.as_path(),
                    )
            })
        });
        if has_base_task {
            continue;
        }

        findings.push(DoctorFinding {
            severity: DoctorSeverity::Warn,
            kind: DoctorFindingKind::ConfiguredRepoMissingBaseTaskManifest,
            subject: DoctorSubject {
                task_slug: None,
                manifest_path: None,
                repository_path: Some(project.path.to_string_lossy().into_owned()),
                worktree_path: None,
                session_name: None,
            },
            evidence: format!(
                "configured repository '{}' has no manifest-backed base task",
                project.name
            ),
            recommended_action: "materialize a base task manifest".to_string(),
        });
    }
}

fn collect_tmux_findings(
    tasks: &[LoadedDoctorTask],
    tmux_state: DoctorTmuxState,
    now_unix_secs: u64,
    findings: &mut Vec<DoctorFinding>,
) {
    match tmux_state {
        DoctorTmuxState::Available(sessions) => {
            let task_snapshot = tasks
                .iter()
                .map(|loaded| loaded.task.clone())
                .collect::<Vec<Task>>();
            for session in sessions {
                if session.name.starts_with("grove-ws-") {
                    findings.push(DoctorFinding {
                        severity: DoctorSeverity::Warn,
                        kind: DoctorFindingKind::LegacyGroveSessionMissingMetadata,
                        subject: DoctorSubject {
                            task_slug: None,
                            manifest_path: None,
                            repository_path: None,
                            worktree_path: None,
                            session_name: Some(session.name.clone()),
                        },
                        evidence: format!(
                            "legacy Grove session uses pre-task-model naming: {}",
                            session.name
                        ),
                        recommended_action: "kill or adopt the legacy tmux session outside Grove"
                            .to_string(),
                    });
                    continue;
                }

                let Some(reason) = cleanup_reason_for_tasks(
                    &session,
                    task_snapshot.as_slice(),
                    true,
                    now_unix_secs,
                ) else {
                    continue;
                };

                let (kind, evidence, recommended_action) = match reason {
                    SessionCleanupReason::Orphaned => (
                        DoctorFindingKind::OrphanedGroveSession,
                        format!(
                            "session is not owned by any discovered task: {}",
                            session.name
                        ),
                        "kill or adopt the orphaned tmux session".to_string(),
                    ),
                    SessionCleanupReason::StaleAuxiliary => (
                        DoctorFindingKind::StaleAuxiliarySession,
                        format!("auxiliary session is stale: {}", session.name),
                        "kill the stale auxiliary tmux session".to_string(),
                    ),
                };

                findings.push(DoctorFinding {
                    severity: DoctorSeverity::Warn,
                    kind,
                    subject: DoctorSubject {
                        task_slug: None,
                        manifest_path: None,
                        repository_path: None,
                        worktree_path: None,
                        session_name: Some(session.name),
                    },
                    evidence,
                    recommended_action,
                });
            }
        }
        DoctorTmuxState::Unavailable(error) => findings.push(DoctorFinding {
            severity: DoctorSeverity::Warn,
            kind: DoctorFindingKind::SessionCheckSkipped,
            subject: DoctorSubject {
                task_slug: None,
                manifest_path: None,
                repository_path: None,
                worktree_path: None,
                session_name: None,
            },
            evidence: format!("tmux session checks skipped: {error}"),
            recommended_action: "inspect tmux availability before relying on session diagnosis"
                .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DoctorFinding, DoctorFindingKind, DoctorRepairAction, DoctorReport, DoctorSeverity,
        DoctorSubject, DoctorTmuxState, diagnose_from_inputs, diagnose_from_inputs_at,
    };
    use crate::application::session_cleanup::SessionRecord;
    use crate::domain::{AgentType, Task, WorkspaceStatus, Worktree};
    use crate::infrastructure::config::{ProjectConfig, ProjectDefaults};
    use crate::infrastructure::task_manifest::encode_task_manifest;
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
                "grove-doctor-{label}-{}-{timestamp}",
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

    fn write_manifest(tasks_root: &std::path::Path, dir_name: &str, task: &Task) {
        let manifest_dir = tasks_root.join(dir_name).join(".grove");
        fs::create_dir_all(&manifest_dir).expect("manifest dir should exist");
        let encoded = encode_task_manifest(task).expect("manifest should encode");
        fs::write(manifest_dir.join("task.toml"), encoded).expect("manifest should write");
    }

    fn task_fixture(
        slug: &str,
        repository_name: &str,
        repository_path: PathBuf,
        worktree_path: PathBuf,
        status: WorkspaceStatus,
    ) -> Task {
        let worktree = Worktree::try_new(
            repository_name.to_string(),
            repository_path,
            worktree_path,
            slug.to_string(),
            AgentType::Codex,
            status,
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

    fn finding(
        severity: DoctorSeverity,
        kind: DoctorFindingKind,
        task_slug: Option<&str>,
        manifest_path: Option<&str>,
    ) -> DoctorFinding {
        DoctorFinding {
            severity,
            kind,
            subject: DoctorSubject {
                task_slug: task_slug.map(ToOwned::to_owned),
                manifest_path: manifest_path.map(ToOwned::to_owned),
                repository_path: None,
                worktree_path: None,
                session_name: None,
            },
            evidence: "evidence".to_string(),
            recommended_action: "action".to_string(),
        }
    }

    #[test]
    fn report_counts_findings_by_severity() {
        let report = DoctorReport::from_findings(vec![
            finding(
                DoctorSeverity::Error,
                DoctorFindingKind::InvalidTaskManifest,
                Some("broken-a"),
                Some("/tmp/a.toml"),
            ),
            finding(
                DoctorSeverity::Warn,
                DoctorFindingKind::MissingWorktreePath,
                Some("broken-b"),
                None,
            ),
            finding(
                DoctorSeverity::Warn,
                DoctorFindingKind::MissingBaseMarker,
                Some("broken-c"),
                None,
            ),
        ]);

        assert_eq!(report.summary.total, 3);
        assert_eq!(report.summary.info, 0);
        assert_eq!(report.summary.warn, 2);
        assert_eq!(report.summary.error, 1);
    }

    #[test]
    fn report_deduplicates_repair_steps_for_same_root_cause() {
        let report = DoctorReport::from_findings(vec![
            finding(
                DoctorSeverity::Error,
                DoctorFindingKind::InvalidTaskManifest,
                Some("broken-a"),
                Some("/tmp/a.toml"),
            ),
            finding(
                DoctorSeverity::Error,
                DoctorFindingKind::InvalidTaskManifest,
                Some("broken-a"),
                Some("/tmp/a.toml"),
            ),
        ]);

        assert_eq!(report.repair_plan.len(), 1);
        assert_eq!(
            report.repair_plan[0].action,
            DoctorRepairAction::InspectOrRewriteManifest
        );
    }

    #[test]
    fn report_orders_repair_steps_deterministically() {
        let report = DoctorReport::from_findings(vec![
            finding(
                DoctorSeverity::Warn,
                DoctorFindingKind::ConfiguredRepoMissingBaseTaskManifest,
                None,
                None,
            ),
            finding(
                DoctorSeverity::Error,
                DoctorFindingKind::InvalidTaskManifest,
                Some("broken-z"),
                Some("/tmp/z.toml"),
            ),
            finding(
                DoctorSeverity::Error,
                DoctorFindingKind::DuplicateTaskSlug,
                Some("broken-a"),
                Some("/tmp/a.toml"),
            ),
        ]);

        assert_eq!(report.repair_plan.len(), 3);
        assert_eq!(
            report.repair_plan[0].action,
            DoctorRepairAction::InspectOrRewriteManifest
        );
        assert_eq!(
            report.repair_plan[1].action,
            DoctorRepairAction::RemoveDuplicateManifestOwner
        );
        assert_eq!(
            report.repair_plan[2].action,
            DoctorRepairAction::MaterializeBaseTaskManifest
        );
    }

    #[test]
    fn diagnose_reports_invalid_task_manifest() {
        let temp = TestDir::new("invalid-manifest");
        let tasks_root = temp.path.join("tasks");
        let broken_dir = tasks_root.join("broken").join(".grove");
        fs::create_dir_all(&broken_dir).expect("broken dir should exist");
        fs::write(broken_dir.join("task.toml"), "not = [valid").expect("manifest should write");

        let report = diagnose_from_inputs(
            Some(tasks_root.as_path()),
            &[],
            DoctorTmuxState::Available(vec![]),
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == DoctorFindingKind::InvalidTaskManifest)
        );
    }

    #[test]
    fn diagnose_reports_duplicate_task_slug() {
        let temp = TestDir::new("duplicate-slug");
        let tasks_root = temp.path.join("tasks");
        let repo_root = temp.path.join("repos").join("web");
        let worktree_a = temp.path.join("worktrees").join("a");
        let worktree_b = temp.path.join("worktrees").join("b");
        fs::create_dir_all(&repo_root).expect("repo should exist");
        fs::create_dir_all(&worktree_a).expect("worktree should exist");
        fs::create_dir_all(&worktree_b).expect("worktree should exist");
        fs::create_dir_all(worktree_a.join(".grove")).expect("grove dir should exist");
        fs::create_dir_all(worktree_b.join(".grove")).expect("grove dir should exist");
        fs::write(worktree_a.join(".grove/base"), "main\n").expect("base marker should write");
        fs::write(worktree_b.join(".grove/base"), "main\n").expect("base marker should write");

        let task_a = task_fixture(
            "shared",
            "web",
            repo_root.clone(),
            worktree_a,
            WorkspaceStatus::Idle,
        );
        let task_b = task_fixture(
            "shared",
            "web",
            repo_root,
            worktree_b,
            WorkspaceStatus::Idle,
        );
        write_manifest(tasks_root.as_path(), "task-a", &task_a);
        write_manifest(tasks_root.as_path(), "task-b", &task_b);

        let report = diagnose_from_inputs(
            Some(tasks_root.as_path()),
            &[],
            DoctorTmuxState::Available(vec![]),
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == DoctorFindingKind::DuplicateTaskSlug)
        );
    }

    #[test]
    fn diagnose_reports_missing_worktree_path() {
        let temp = TestDir::new("missing-worktree");
        let tasks_root = temp.path.join("tasks");
        let repo_root = temp.path.join("repos").join("web");
        fs::create_dir_all(&repo_root).expect("repo should exist");
        let missing_worktree = temp.path.join("worktrees").join("missing");
        let task = task_fixture(
            "missing-worktree",
            "web",
            repo_root,
            missing_worktree,
            WorkspaceStatus::Idle,
        );
        write_manifest(tasks_root.as_path(), "missing-worktree", &task);

        let report = diagnose_from_inputs(
            Some(tasks_root.as_path()),
            &[],
            DoctorTmuxState::Available(vec![]),
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == DoctorFindingKind::MissingWorktreePath)
        );
    }

    #[test]
    fn diagnose_reports_missing_base_marker() {
        let temp = TestDir::new("missing-base");
        let tasks_root = temp.path.join("tasks");
        let repo_root = temp.path.join("repos").join("web");
        let worktree = temp.path.join("worktrees").join("feature");
        fs::create_dir_all(&repo_root).expect("repo should exist");
        fs::create_dir_all(&worktree).expect("worktree should exist");
        let task = task_fixture(
            "missing-base",
            "web",
            repo_root,
            worktree,
            WorkspaceStatus::Idle,
        );
        write_manifest(tasks_root.as_path(), "missing-base", &task);

        let report = diagnose_from_inputs(
            Some(tasks_root.as_path()),
            &[],
            DoctorTmuxState::Available(vec![]),
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == DoctorFindingKind::MissingBaseMarker)
        );
    }

    #[test]
    fn diagnose_reports_configured_repo_missing_base_task_manifest() {
        let temp = TestDir::new("missing-base-manifest");
        let tasks_root = temp.path.join("tasks");
        let repo_root = temp.path.join("repos").join("api");
        fs::create_dir_all(&repo_root).expect("repo should exist");

        let report = diagnose_from_inputs(
            Some(tasks_root.as_path()),
            &[ProjectConfig {
                name: "api".to_string(),
                path: repo_root.clone(),
                defaults: ProjectDefaults::default(),
            }],
            DoctorTmuxState::Available(vec![]),
        );

        assert!(report.findings.iter().any(|finding| {
            finding.kind == DoctorFindingKind::ConfiguredRepoMissingBaseTaskManifest
                && finding.subject.repository_path.as_deref()
                    == Some(repo_root.to_string_lossy().as_ref())
        }));
    }

    #[test]
    fn diagnose_reports_orphaned_grove_session() {
        let temp = TestDir::new("orphaned-session");
        let tasks_root = temp.path.join("tasks");
        let repo_root = temp.path.join("repos").join("flohome");
        let worktree = temp.path.join("worktrees").join("flohome");
        fs::create_dir_all(&repo_root).expect("repo should exist");
        fs::create_dir_all(worktree.join(".grove")).expect("worktree should exist");
        fs::write(worktree.join(".grove/base"), "main\n").expect("base marker should write");
        let task = task_fixture(
            "flohome-launch",
            "flohome",
            repo_root,
            worktree,
            WorkspaceStatus::Idle,
        );
        write_manifest(tasks_root.as_path(), "flohome-launch", &task);

        let report = diagnose_from_inputs_at(
            Some(tasks_root.as_path()),
            &[],
            DoctorTmuxState::Available(vec![SessionRecord {
                name: "grove-wt-flohome-launch-lost".to_string(),
                created_unix_secs: Some(1_700_000_100),
                attached_clients: 0,
            }]),
            1_700_090_000,
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == DoctorFindingKind::OrphanedGroveSession)
        );
    }

    #[test]
    fn diagnose_reports_stale_auxiliary_session() {
        let temp = TestDir::new("stale-session");
        let tasks_root = temp.path.join("tasks");
        let repo_root = temp.path.join("repos").join("flohome");
        let worktree = temp.path.join("worktrees").join("flohome");
        fs::create_dir_all(&repo_root).expect("repo should exist");
        fs::create_dir_all(worktree.join(".grove")).expect("worktree should exist");
        fs::write(worktree.join(".grove/base"), "main\n").expect("base marker should write");
        let task = task_fixture(
            "flohome-launch",
            "flohome",
            repo_root,
            worktree,
            WorkspaceStatus::Idle,
        );
        write_manifest(tasks_root.as_path(), "flohome-launch", &task);

        let report = diagnose_from_inputs_at(
            Some(tasks_root.as_path()),
            &[],
            DoctorTmuxState::Available(vec![SessionRecord {
                name: "grove-wt-flohome-launch-flohome-git".to_string(),
                created_unix_secs: Some(1_700_000_000),
                attached_clients: 0,
            }]),
            1_700_090_000,
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.kind == DoctorFindingKind::StaleAuxiliarySession)
        );
    }

    #[test]
    fn diagnose_reports_legacy_grove_session_missing_metadata() {
        let report = diagnose_from_inputs_at(
            None,
            &[],
            DoctorTmuxState::Available(vec![SessionRecord {
                name: "grove-ws-legacy-feature".to_string(),
                created_unix_secs: Some(1_700_000_100),
                attached_clients: 0,
            }]),
            1_700_090_000,
        );

        assert!(report.findings.iter().any(|finding| {
            finding.kind == DoctorFindingKind::LegacyGroveSessionMissingMetadata
                && finding.subject.session_name.as_deref() == Some("grove-ws-legacy-feature")
        }));
    }

    #[test]
    fn diagnose_warns_when_tmux_checks_are_unavailable() {
        let report = diagnose_from_inputs(
            None,
            &[],
            DoctorTmuxState::Unavailable("tmux missing".to_string()),
        );

        assert_eq!(report.summary.warn, 1);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.evidence.contains("tmux session checks skipped"))
        );
    }
}

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::application::agent_runtime::{
    LivePreviewTarget, SessionActivity, detect_status_with_session_override,
    reconcile_with_sessions, session_name_for_workspace_in_project,
    workspace_status_targets_for_polling_with_live_preview,
};
use crate::domain::{Task, Workspace, WorkspaceStatus, Worktree};
use crate::infrastructure::adapters::benchmark_discovery_from_synthetic_fixture;

const TASK_COUNTS: [usize; 3] = [10, 100, 500];
const WARMUP_RUNS: usize = 2;
const MEASURED_RUNS: usize = 15;
const DEFAULT_SEVERE_REGRESSION_PCT: u64 = 35;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScaleBenchmarkOptions {
    pub(crate) json_output: bool,
    pub(crate) baseline_path: Option<PathBuf>,
    pub(crate) write_baseline_path: Option<PathBuf>,
    pub(crate) severe_regression_pct: u64,
}

impl Default for ScaleBenchmarkOptions {
    fn default() -> Self {
        Self {
            json_output: false,
            baseline_path: None,
            write_baseline_path: None,
            severe_regression_pct: DEFAULT_SEVERE_REGRESSION_PCT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScaleBenchmarkReport {
    generated_at_unix_secs: u64,
    package_version: String,
    config: ScaleBenchmarkConfig,
    cases: Vec<ScaleBenchmarkCaseReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScaleBenchmarkConfig {
    warmup_runs: usize,
    measured_runs: usize,
    task_counts: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScaleBenchmarkCaseReport {
    task_count: usize,
    discovery: FlowStats,
    status_target_generation: FlowStats,
    sort_update_pipeline: FlowStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlowStats {
    warmup_runs: usize,
    measured_runs: usize,
    min_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    #[serde(default)]
    p99_ms: f64,
    max_ms: f64,
    mean_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ScaleBenchmarkOutput {
    warnings: Vec<String>,
    report: ScaleBenchmarkReport,
}

pub(crate) fn run_scale_benchmark(options: ScaleBenchmarkOptions) -> io::Result<()> {
    let report = run_scale_benchmarks()?;

    if let Some(path) = options.write_baseline_path.as_ref() {
        write_report_to_path(path, &report)?;
    }

    let warnings = if let Some(path) = options.baseline_path.as_ref() {
        let baseline = read_report_from_path(path)?;
        compare_against_baseline(&report, &baseline, options.severe_regression_pct)
    } else {
        Vec::new()
    };

    if options.json_output {
        let output = ScaleBenchmarkOutput { warnings, report };
        let serialized = serde_json::to_string_pretty(&output).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to encode benchmark report: {error}"),
            )
        })?;
        println!("{serialized}");
        return Ok(());
    }

    print_human_report(&report, &warnings);
    Ok(())
}

fn run_scale_benchmarks() -> io::Result<ScaleBenchmarkReport> {
    let mut cases = Vec::with_capacity(TASK_COUNTS.len());

    for task_count in TASK_COUNTS {
        let fixture = SyntheticScaleFixture::create(task_count)?;

        let discovery = measure_flow(WARMUP_RUNS, MEASURED_RUNS, || {
            let discovered = fixture.discover_tasks()?;
            Ok(discovered.len())
        })?;

        let discovered_tasks = fixture.discover_tasks()?;
        let discovered = flatten_benchmark_tasks(discovered_tasks.as_slice());
        let seeded_for_status_targets = reconcile_with_sessions(
            &discovered,
            &fixture.running_sessions,
            &fixture.previously_running_workspace_names,
        )
        .workspaces;
        let live_preview = fixture.live_preview_target();

        let status_target_generation = measure_flow(WARMUP_RUNS, MEASURED_RUNS, || {
            let targets = workspace_status_targets_for_polling_with_live_preview(
                &seeded_for_status_targets,
                live_preview.as_ref(),
            );
            Ok(targets.len())
        })?;

        let sort_update_pipeline = measure_flow(WARMUP_RUNS, MEASURED_RUNS, || {
            let mut updated = reconcile_with_sessions(
                &discovered,
                &fixture.running_sessions,
                &fixture.previously_running_workspace_names,
            )
            .workspaces;
            let targets = workspace_status_targets_for_polling_with_live_preview(
                &updated,
                live_preview.as_ref(),
            );
            let target_updates = {
                let workspace_index_by_path: HashMap<&Path, usize> = updated
                    .iter()
                    .enumerate()
                    .map(|(index, workspace)| (workspace.path.as_path(), index))
                    .collect();
                targets
                    .into_iter()
                    .filter_map(|target| {
                        workspace_index_by_path
                            .get(target.workspace_path.as_path())
                            .copied()
                    })
                    .collect::<Vec<_>>()
            };
            for index in target_updates {
                let workspace = &mut updated[index];
                let output = fixture
                    .status_output_by_workspace
                    .get(&workspace.name)
                    .map(String::as_str)
                    .unwrap_or("processing...");
                workspace.status = detect_status_with_session_override(
                    output,
                    SessionActivity::Active,
                    workspace.is_main,
                    true,
                    workspace.supported_agent,
                    workspace.agent,
                    workspace.path.as_path(),
                );
                workspace.is_orphaned = false;
            }
            updated.sort_by(workspace_scale_sort);
            Ok(updated.len())
        })?;

        cases.push(ScaleBenchmarkCaseReport {
            task_count,
            discovery,
            status_target_generation,
            sort_update_pipeline,
        });
    }

    Ok(ScaleBenchmarkReport {
        generated_at_unix_secs: unix_now_secs(),
        package_version: env!("CARGO_PKG_VERSION").to_string(),
        config: ScaleBenchmarkConfig {
            warmup_runs: WARMUP_RUNS,
            measured_runs: MEASURED_RUNS,
            task_counts: TASK_COUNTS.to_vec(),
        },
        cases,
    })
}

fn print_human_report(report: &ScaleBenchmarkReport, warnings: &[String]) {
    println!("task scale benchmark");
    println!(
        "runs: warmup={} measured={} counts={:?}",
        report.config.warmup_runs, report.config.measured_runs, report.config.task_counts
    );
    println!("generated_at_unix_secs={}", report.generated_at_unix_secs);

    for case in &report.cases {
        println!("N={}", case.task_count);
        print_flow_line("discovery", &case.discovery);
        print_flow_line("status-targets", &case.status_target_generation);
        print_flow_line("sort-update", &case.sort_update_pipeline);
    }

    if warnings.is_empty() {
        println!("regression-check: no severe regressions");
        return;
    }

    println!("regression-check: {} warning(s)", warnings.len());
    for warning in warnings {
        println!("warning: {warning}");
    }
}

fn print_flow_line(name: &str, stats: &FlowStats) {
    println!(
        "  {:<14} p50={:>8.3}ms p95={:>8.3}ms p99={:>8.3}ms min={:>8.3}ms max={:>8.3}ms mean={:>8.3}ms",
        name, stats.p50_ms, stats.p95_ms, stats.p99_ms, stats.min_ms, stats.max_ms, stats.mean_ms
    );
}

fn write_report_to_path(path: &Path, report: &ScaleBenchmarkReport) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let serialized = serde_json::to_string_pretty(report).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to encode benchmark report: {error}"),
        )
    })?;
    fs::write(path, serialized)
}

fn read_report_from_path(path: &Path) -> io::Result<ScaleBenchmarkReport> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str::<ScaleBenchmarkReport>(&content).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "failed to decode baseline report '{}': {error}",
                path.display()
            ),
        )
    })
}

fn compare_against_baseline(
    current: &ScaleBenchmarkReport,
    baseline: &ScaleBenchmarkReport,
    severe_regression_pct: u64,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for current_case in &current.cases {
        let Some(baseline_case) = baseline
            .cases
            .iter()
            .find(|case| case.task_count == current_case.task_count)
        else {
            warnings.push(format!(
                "missing baseline case for N={} tasks",
                current_case.task_count
            ));
            continue;
        };

        let current_flows = case_flows(current_case);
        let baseline_flows = case_flows(baseline_case);
        for (flow_name, current_flow) in current_flows {
            let Some((_, baseline_flow)) = baseline_flows
                .iter()
                .find(|(baseline_name, _)| *baseline_name == flow_name)
            else {
                warnings.push(format!(
                    "missing baseline flow '{flow_name}' for N={}",
                    current_case.task_count
                ));
                continue;
            };

            let allowed_p95 = baseline_flow.p95_ms * (100.0 + severe_regression_pct as f64) / 100.0;
            if current_flow.p95_ms > allowed_p95 {
                warnings.push(format!(
                    "N={} flow={} p95 {:.3}ms exceeded baseline {:.3}ms (+{}%, allowed {:.3}ms)",
                    current_case.task_count,
                    flow_name,
                    current_flow.p95_ms,
                    baseline_flow.p95_ms,
                    severe_regression_pct,
                    allowed_p95
                ));
            }

            if baseline_flow.p99_ms > 0.0 {
                let allowed_p99 =
                    baseline_flow.p99_ms * (100.0 + severe_regression_pct as f64) / 100.0;
                if current_flow.p99_ms > allowed_p99 {
                    warnings.push(format!(
                        "N={} flow={} p99 {:.3}ms exceeded baseline {:.3}ms (+{}%, allowed {:.3}ms)",
                        current_case.task_count,
                        flow_name,
                        current_flow.p99_ms,
                        baseline_flow.p99_ms,
                        severe_regression_pct,
                        allowed_p99
                    ));
                }
            }
        }
    }
    warnings
}

fn case_flows(case: &ScaleBenchmarkCaseReport) -> [(&'static str, &FlowStats); 3] {
    [
        ("discovery", &case.discovery),
        ("status-target-generation", &case.status_target_generation),
        ("sort-update-pipeline", &case.sort_update_pipeline),
    ]
}

fn measure_flow<F>(
    warmup_runs: usize,
    measured_runs: usize,
    mut run_once: F,
) -> io::Result<FlowStats>
where
    F: FnMut() -> io::Result<usize>,
{
    if measured_runs == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "measured_runs must be greater than zero",
        ));
    }

    for _ in 0..warmup_runs {
        black_box(run_once()?);
    }

    let mut samples_ms = Vec::with_capacity(measured_runs);
    for _ in 0..measured_runs {
        let started_at = Instant::now();
        black_box(run_once()?);
        samples_ms.push(started_at.elapsed().as_secs_f64() * 1_000.0);
    }

    Ok(FlowStats::from_samples(
        warmup_runs,
        measured_runs,
        &samples_ms,
    ))
}

impl FlowStats {
    fn from_samples(warmup_runs: usize, measured_runs: usize, samples_ms: &[f64]) -> Self {
        let mut sorted = samples_ms.to_vec();
        sorted.sort_by(sort_float);
        let sum_ms = sorted.iter().sum::<f64>();
        let mean_ms = if sorted.is_empty() {
            0.0
        } else {
            sum_ms / sorted.len() as f64
        };
        let min_ms = sorted.first().copied().unwrap_or(0.0);
        let max_ms = sorted.last().copied().unwrap_or(0.0);
        let p50_ms = percentile_ms(&sorted, 0.50);
        let p95_ms = percentile_ms(&sorted, 0.95);
        let p99_ms = percentile_ms(&sorted, 0.99);

        Self {
            warmup_runs,
            measured_runs,
            min_ms,
            p50_ms,
            p95_ms,
            p99_ms,
            max_ms,
            mean_ms,
        }
    }
}

fn percentile_ms(sorted_samples: &[f64], percentile: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }

    let rank = (sorted_samples.len() as f64 * percentile).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_samples.len() - 1);
    sorted_samples[index]
}

fn sort_float(left: &f64, right: &f64) -> Ordering {
    match left.partial_cmp(right) {
        Some(ordering) => ordering,
        None => Ordering::Equal,
    }
}

fn workspace_scale_sort(left: &Workspace, right: &Workspace) -> Ordering {
    match (left.is_main, right.is_main) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }

    let activity_order = right
        .last_activity_unix_secs
        .cmp(&left.last_activity_unix_secs);
    if activity_order != Ordering::Equal {
        return activity_order;
    }

    left.name.cmp(&right.name)
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn unix_now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

struct SyntheticScaleFixture {
    root: PathBuf,
    repo_root: PathBuf,
    repo_name: String,
    porcelain_worktrees: String,
    branch_activity: String,
    running_sessions: HashSet<String>,
    previously_running_workspace_names: HashSet<String>,
    status_output_by_workspace: HashMap<String, String>,
    live_preview_session: Option<String>,
}

impl SyntheticScaleFixture {
    fn create(task_count: usize) -> io::Result<Self> {
        if task_count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "task_count must be greater than zero",
            ));
        }

        let root = std::env::temp_dir().join(format!(
            "grove-scale-bench-{}-{}-{task_count}",
            std::process::id(),
            unix_now_nanos()
        ));
        fs::create_dir_all(&root)?;

        let repo_name = "grove".to_string();
        let repo_root = root.join(&repo_name);
        fs::create_dir_all(repo_root.join(".grove"))?;

        let mut porcelain_worktrees = String::new();
        append_worktree_block(&mut porcelain_worktrees, repo_root.as_path(), Some("main"));

        let now_secs_i64 = i64::try_from(unix_now_secs()).unwrap_or(i64::MAX);
        let mut branch_activity_lines = vec![format!("main {now_secs_i64}")];
        let mut running_sessions = HashSet::new();
        let mut previously_running_workspace_names = HashSet::new();
        let mut status_output_by_workspace = HashMap::new();
        let mut live_preview_session = None;

        for index in 1..task_count {
            let workspace_name = format!("feature-{index:04}");
            let branch_name = workspace_name.clone();
            let workspace_directory = root.join(format!("{repo_name}-{workspace_name}"));
            fs::create_dir_all(workspace_directory.join(".grove"))?;
            fs::write(workspace_directory.join(".grove/base"), "main\n")?;

            append_worktree_block(
                &mut porcelain_worktrees,
                workspace_directory.as_path(),
                Some(branch_name.as_str()),
            );

            let offset = i64::try_from(index).unwrap_or(i64::MAX);
            branch_activity_lines.push(format!(
                "{branch_name} {}",
                now_secs_i64.saturating_sub(offset)
            ));

            let session_name = session_name_for_workspace_in_project(
                Some(repo_name.as_str()),
                workspace_name.as_str(),
            );
            if index % 3 != 0 {
                if live_preview_session.is_none() {
                    live_preview_session = Some(session_name.clone());
                }
                running_sessions.insert(session_name);
            }

            if index % 5 == 0 {
                previously_running_workspace_names.insert(workspace_name.clone());
            }

            status_output_by_workspace.insert(workspace_name, synthetic_status_output(index));
        }

        let branch_activity = format!("{}\n", branch_activity_lines.join("\n"));

        Ok(Self {
            root,
            repo_root,
            repo_name,
            porcelain_worktrees,
            branch_activity,
            running_sessions,
            previously_running_workspace_names,
            status_output_by_workspace,
            live_preview_session,
        })
    }

    fn discover(&self) -> io::Result<Vec<Workspace>> {
        benchmark_discovery_from_synthetic_fixture(
            self.porcelain_worktrees.as_str(),
            self.branch_activity.as_str(),
            self.repo_root.as_path(),
            self.repo_name.as_str(),
        )
        .map_err(|error| io::Error::other(error.message()))
    }

    fn discover_tasks(&self) -> io::Result<Vec<Task>> {
        let workspaces = self.discover()?;
        Ok(workspaces
            .iter()
            .map(task_from_benchmark_workspace)
            .collect())
    }

    fn live_preview_target(&self) -> Option<LivePreviewTarget> {
        self.live_preview_session
            .as_ref()
            .map(|session| LivePreviewTarget {
                session_name: session.clone(),
                include_escape_sequences: true,
            })
    }
}

impl Drop for SyntheticScaleFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn append_worktree_block(buffer: &mut String, path: &Path, branch: Option<&str>) {
    buffer.push_str("worktree ");
    buffer.push_str(path.to_string_lossy().as_ref());
    buffer.push('\n');
    buffer.push_str("HEAD 0000000000000000000000000000000000000000\n");
    if let Some(branch) = branch {
        buffer.push_str("branch refs/heads/");
        buffer.push_str(branch);
        buffer.push('\n');
    } else {
        buffer.push_str("detached\n");
    }
    buffer.push('\n');
}

fn synthetic_status_output(index: usize) -> String {
    if index.is_multiple_of(11) {
        return "continue? [y/n]\n".to_string();
    }
    if index.is_multiple_of(7) {
        return "task completed\n".to_string();
    }
    if index.is_multiple_of(13) {
        return "error: command failed\n".to_string();
    }

    "applying changes\n".to_string()
}

fn flatten_benchmark_tasks(tasks: &[Task]) -> Vec<Workspace> {
    tasks
        .iter()
        .flat_map(|task| {
            task.worktrees
                .iter()
                .map(|worktree| Workspace {
                    name: worktree.repository_name.clone(),
                    path: worktree.path.clone(),
                    project_name: Some(worktree.repository_name.clone()),
                    project_path: Some(worktree.repository_path.clone()),
                    branch: worktree.branch.clone(),
                    base_branch: worktree.base_branch.clone(),
                    last_activity_unix_secs: worktree.last_activity_unix_secs,
                    agent: worktree.agent,
                    status: if worktree.status == WorkspaceStatus::Main {
                        WorkspaceStatus::Idle
                    } else {
                        worktree.status
                    },
                    is_main: false,
                    is_orphaned: worktree.is_orphaned,
                    supported_agent: worktree.supported_agent,
                    pull_requests: worktree.pull_requests.clone(),
                })
                .collect::<Vec<Workspace>>()
        })
        .collect()
}

fn task_from_benchmark_workspace(workspace: &Workspace) -> Task {
    let repository_path = workspace
        .project_path
        .clone()
        .unwrap_or_else(|| workspace.path.clone());
    let repository_name = workspace.project_name.clone().unwrap_or_else(|| {
        repository_path
            .file_name()
            .and_then(|name| name.to_str())
            .map_or_else(|| workspace.name.clone(), ToString::to_string)
    });
    let worktree = Worktree::try_new(
        repository_name,
        repository_path,
        workspace.path.clone(),
        workspace.branch.clone(),
        workspace.agent,
        workspace.status,
    )
    .expect("benchmark worktree should be valid")
    .with_base_branch(workspace.base_branch.clone())
    .with_last_activity_unix_secs(workspace.last_activity_unix_secs)
    .with_supported_agent(workspace.supported_agent)
    .with_orphaned(workspace.is_orphaned)
    .with_pull_requests(workspace.pull_requests.clone());

    Task::try_new(
        workspace.name.clone(),
        workspace.name.clone(),
        workspace.path.clone(),
        workspace.branch.clone(),
        vec![worktree],
    )
    .expect("benchmark task should be valid")
}

#[cfg(test)]
mod tests {
    use super::{
        FlowStats, ScaleBenchmarkCaseReport, ScaleBenchmarkConfig, ScaleBenchmarkReport,
        SyntheticScaleFixture, compare_against_baseline,
    };

    fn stats(p50_ms: f64, p95_ms: f64) -> FlowStats {
        FlowStats {
            warmup_runs: 0,
            measured_runs: 1,
            min_ms: p50_ms,
            p50_ms,
            p95_ms,
            p99_ms: p95_ms,
            max_ms: p95_ms,
            mean_ms: p50_ms,
        }
    }

    fn case(task_count: usize, p95_ms: f64) -> ScaleBenchmarkCaseReport {
        ScaleBenchmarkCaseReport {
            task_count,
            discovery: stats(1.0, p95_ms),
            status_target_generation: stats(1.0, p95_ms),
            sort_update_pipeline: stats(1.0, p95_ms),
        }
    }

    #[test]
    fn flow_stats_percentiles_use_nearest_rank() {
        let stats = FlowStats::from_samples(2, 5, &[3.0, 1.0, 5.0, 4.0, 2.0]);
        assert_eq!(stats.min_ms, 1.0);
        assert_eq!(stats.p50_ms, 3.0);
        assert_eq!(stats.p95_ms, 5.0);
        assert_eq!(stats.p99_ms, 5.0);
        assert_eq!(stats.max_ms, 5.0);
    }

    #[test]
    fn compare_baseline_flags_severe_p95_regression() {
        let current = ScaleBenchmarkReport {
            generated_at_unix_secs: 100,
            package_version: "0.1.0".to_string(),
            config: ScaleBenchmarkConfig {
                warmup_runs: 2,
                measured_runs: 15,
                task_counts: vec![100],
            },
            cases: vec![case(100, 150.0)],
        };
        let baseline = ScaleBenchmarkReport {
            generated_at_unix_secs: 90,
            package_version: "0.1.0".to_string(),
            config: ScaleBenchmarkConfig {
                warmup_runs: 2,
                measured_runs: 15,
                task_counts: vec![100],
            },
            cases: vec![case(100, 100.0)],
        };

        let warnings = compare_against_baseline(&current, &baseline, 35);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("N=100"));
    }

    #[test]
    fn synthetic_scale_fixture_discovers_tasks() {
        let fixture = SyntheticScaleFixture::create(10).expect("fixture should build");
        let tasks = fixture.discover_tasks().expect("tasks should discover");

        assert_eq!(tasks.len(), 10);
        assert!(tasks.iter().all(|task| task.worktrees.len() == 1));
    }
}

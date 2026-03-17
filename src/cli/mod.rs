use std::fs;
use std::path::{Path, PathBuf};

use crate::application::doctor::DoctorReport;
use crate::application::session_cleanup::{
    SessionCleanupEntry, SessionCleanupOptions, SessionCleanupReason, apply_session_cleanup,
    plan_session_cleanup,
};
use crate::infrastructure::event_log::now_millis;

const DEBUG_RECORD_DIR: &str = ".grove";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct CliArgs {
    pub(crate) print_hello: bool,
    pub(crate) event_log_path: Option<PathBuf>,
    pub(crate) debug_record: bool,
    pub(crate) replay_trace_path: Option<PathBuf>,
    pub(crate) replay_snapshot_path: Option<PathBuf>,
    pub(crate) replay_emit_test_name: Option<String>,
    pub(crate) replay_invariant_only: bool,
    pub(crate) benchmark_scale: bool,
    pub(crate) benchmark_json_output: bool,
    pub(crate) benchmark_baseline_path: Option<PathBuf>,
    pub(crate) benchmark_write_baseline_path: Option<PathBuf>,
    pub(crate) benchmark_warn_regression_pct: Option<u64>,
    pub(crate) doctor: bool,
    pub(crate) doctor_json_output: bool,
    pub(crate) cleanup_sessions: bool,
    pub(crate) cleanup_sessions_apply: bool,
    pub(crate) cleanup_sessions_include_stale: bool,
    pub(crate) cleanup_sessions_include_attached: bool,
}

pub(crate) fn parse_cli_args(args: impl IntoIterator<Item = String>) -> std::io::Result<CliArgs> {
    let mut cli = CliArgs::default();
    let mut args = args.into_iter();
    let mut saw_json_output = false;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--print-hello" => {
                cli.print_hello = true;
            }
            "--event-log" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--event-log requires a file path",
                    ));
                };
                cli.event_log_path = Some(PathBuf::from(path));
            }
            "--debug-record" => {
                cli.debug_record = true;
            }
            "replay" => {
                if cli.benchmark_scale {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "replay cannot be combined with benchmark-scale",
                    ));
                }
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "replay requires a trace path",
                    ));
                };
                cli.replay_trace_path = Some(PathBuf::from(path));
            }
            "benchmark-scale" => {
                if cli.replay_trace_path.is_some() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "benchmark-scale cannot be combined with replay",
                    ));
                }
                cli.benchmark_scale = true;
            }
            "doctor" => {
                cli.doctor = true;
            }
            "cleanup" => {
                let Some(target) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "cleanup requires a target (`sessions`)",
                    ));
                };
                if target != "sessions" {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("unsupported cleanup target `{target}`"),
                    ));
                }
                cli.cleanup_sessions = true;
            }
            "--snapshot" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--snapshot requires a file path",
                    ));
                };
                cli.replay_snapshot_path = Some(PathBuf::from(path));
            }
            "--emit-test" => {
                let Some(name) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--emit-test requires a fixture name",
                    ));
                };
                cli.replay_emit_test_name = Some(name);
            }
            "--invariant-only" => {
                cli.replay_invariant_only = true;
            }
            "--json" => {
                saw_json_output = true;
            }
            "--baseline" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--baseline requires a file path",
                    ));
                };
                cli.benchmark_baseline_path = Some(PathBuf::from(path));
            }
            "--write-baseline" => {
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--write-baseline requires a file path",
                    ));
                };
                cli.benchmark_write_baseline_path = Some(PathBuf::from(path));
            }
            "--warn-regression-pct" => {
                let Some(value) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--warn-regression-pct requires a positive integer",
                    ));
                };
                let parsed = value.parse::<u64>().map_err(|error| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("--warn-regression-pct must be an integer: {error}"),
                    )
                })?;
                if parsed == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "--warn-regression-pct must be greater than zero",
                    ));
                }
                cli.benchmark_warn_regression_pct = Some(parsed);
            }
            "--apply" => {
                cli.cleanup_sessions_apply = true;
            }
            "--include-stale" => {
                cli.cleanup_sessions_include_stale = true;
            }
            "--include-attached" => {
                cli.cleanup_sessions_include_attached = true;
            }
            _ => {}
        }
    }

    if saw_json_output {
        if cli.benchmark_scale {
            cli.benchmark_json_output = true;
        } else if cli.doctor {
            cli.doctor_json_output = true;
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "json output requires `benchmark-scale` or `doctor`",
            ));
        }
    }

    if cli.replay_trace_path.is_none()
        && (cli.replay_snapshot_path.is_some()
            || cli.replay_emit_test_name.is_some()
            || cli.replay_invariant_only)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "replay flags require `replay <trace-path>`",
        ));
    }

    if !cli.benchmark_scale
        && (cli.benchmark_json_output
            || cli.benchmark_baseline_path.is_some()
            || cli.benchmark_write_baseline_path.is_some()
            || cli.benchmark_warn_regression_pct.is_some())
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "benchmark flags require `benchmark-scale`",
        ));
    }

    if !cli.cleanup_sessions
        && (cli.cleanup_sessions_apply
            || cli.cleanup_sessions_include_stale
            || cli.cleanup_sessions_include_attached)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cleanup flags require `cleanup sessions`",
        ));
    }

    if cli.cleanup_sessions
        && (cli.replay_trace_path.is_some()
            || cli.benchmark_scale
            || cli.doctor
            || cli.debug_record
            || cli.event_log_path.is_some()
            || cli.print_hello)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "cleanup sessions cannot be combined with other command modes",
        ));
    }

    if cli.doctor
        && (cli.replay_trace_path.is_some()
            || cli.benchmark_scale
            || cli.cleanup_sessions
            || cli.debug_record
            || cli.event_log_path.is_some()
            || cli.print_hello)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "doctor cannot be combined with other command modes",
        ));
    }

    Ok(cli)
}

pub(crate) fn debug_record_path(app_start_ts: u64) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from(DEBUG_RECORD_DIR);
    fs::create_dir_all(&dir)?;

    let mut sequence = 0u32;
    loop {
        let file_name = if sequence == 0 {
            format!("debug-record-{app_start_ts}-{}.jsonl", std::process::id())
        } else {
            format!(
                "debug-record-{app_start_ts}-{}-{sequence}.jsonl",
                std::process::id()
            )
        };
        let path = dir.join(file_name);
        if !path.exists() {
            return Ok(path);
        }
        sequence = sequence.saturating_add(1);
    }
}

pub(crate) fn resolve_event_log_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    let grove_dir = Path::new(DEBUG_RECORD_DIR);
    if path.starts_with(grove_dir) {
        return path;
    }

    grove_dir.join(path)
}

pub(crate) fn ensure_event_log_parent_directory(path: &Path) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    fs::create_dir_all(parent)
}

fn format_age(age_secs: Option<u64>) -> String {
    let Some(age_secs) = age_secs else {
        return "unknown".to_string();
    };
    if age_secs < 60 {
        return format!("{age_secs}s");
    }
    if age_secs < 60 * 60 {
        return format!("{}m", age_secs / 60);
    }
    if age_secs < 24 * 60 * 60 {
        return format!("{}h", age_secs / (60 * 60));
    }
    format!("{}d", age_secs / (24 * 60 * 60))
}

fn reason_label(reason: SessionCleanupReason) -> &'static str {
    match reason {
        SessionCleanupReason::Orphaned => "orphan",
        SessionCleanupReason::StaleAuxiliary => "stale",
    }
}

fn print_cleanup_entries(prefix: &str, entries: &[SessionCleanupEntry]) {
    for entry in entries {
        println!(
            "{prefix} {} [{}] age={} attached={}",
            entry.session_name,
            reason_label(entry.reason),
            format_age(entry.age_secs),
            entry.attached_clients
        );
    }
}

fn run_cleanup_sessions(cli: &CliArgs) -> std::io::Result<()> {
    let options = SessionCleanupOptions {
        include_stale: cli.cleanup_sessions_include_stale,
        include_attached: cli.cleanup_sessions_include_attached,
    };
    let plan = plan_session_cleanup(options).map_err(std::io::Error::other)?;
    if plan.candidates.is_empty() {
        println!("cleanup sessions: no candidates");
    } else {
        println!("cleanup sessions: {} candidate(s)", plan.candidates.len());
        print_cleanup_entries("-", plan.candidates.as_slice());
    }
    if !plan.skipped_attached.is_empty() {
        println!(
            "cleanup sessions: {} attached session(s) skipped, use --include-attached to include",
            plan.skipped_attached.len()
        );
        print_cleanup_entries("~", plan.skipped_attached.as_slice());
    }

    if !cli.cleanup_sessions_apply {
        if !plan.candidates.is_empty() {
            println!("dry run only, rerun with `cleanup sessions --apply` to kill candidates");
        }
        return Ok(());
    }

    let applied = apply_session_cleanup(&plan);
    for session_name in &applied.killed {
        println!("killed {session_name}");
    }
    for session_name in &applied.already_gone {
        println!("gone {session_name}");
    }
    for (session_name, error) in &applied.failures {
        eprintln!("failed {session_name}: {error}");
    }

    if applied.failures.is_empty() {
        return Ok(());
    }

    Err(std::io::Error::other(format!(
        "cleanup sessions failed for {} session(s)",
        applied.failures.len()
    )))
}

fn format_doctor_summary(report: &DoctorReport) -> String {
    if report.summary.total == 0 {
        return "doctor: clean".to_string();
    }

    format!(
        "doctor: {} findings ({} error, {} warn, {} info)",
        report.summary.total, report.summary.error, report.summary.warn, report.summary.info
    )
}

fn print_doctor_findings(
    findings: &[crate::application::doctor::DoctorFinding],
    severity: crate::application::doctor::DoctorSeverity,
    label: &str,
) {
    let matching = findings
        .iter()
        .filter(|finding| finding.severity == severity)
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return;
    }

    println!();
    println!("{label}");
    for finding in matching {
        println!("- {}", finding.kind.label());
        for target in finding.subject.targets() {
            println!("  target: {target}");
        }
        println!("  evidence: {}", finding.evidence);
        println!("  action: {}", finding.recommended_action);
    }
}

fn print_doctor_report(report: &DoctorReport) -> std::io::Result<()> {
    println!("{}", format_doctor_summary(report));

    print_doctor_findings(
        report.findings.as_slice(),
        crate::application::doctor::DoctorSeverity::Error,
        "errors",
    );
    print_doctor_findings(
        report.findings.as_slice(),
        crate::application::doctor::DoctorSeverity::Warn,
        "warnings",
    );
    print_doctor_findings(
        report.findings.as_slice(),
        crate::application::doctor::DoctorSeverity::Info,
        "info",
    );

    if !report.repair_plan.is_empty() {
        println!();
        println!("repair plan");
        for (index, step) in report.repair_plan.iter().enumerate() {
            let targets = step.targets.join(", ");
            println!(
                "{}. {}: {} [{}]",
                index + 1,
                step.action.label(),
                step.goal,
                targets
            );
            println!("   why: {}", step.reason);
            if !step.preconditions.is_empty() {
                println!("   before you start:");
                for precondition in &step.preconditions {
                    println!("   - {precondition}");
                }
            }
            if !step.steps.is_empty() {
                println!("   do this:");
                for (step_index, instruction) in step.steps.iter().enumerate() {
                    println!("   {}. {}", step_index + 1, instruction);
                }
            }
            if !step.verification.is_empty() {
                println!("   verify:");
                for verification in &step.verification {
                    println!("   - {verification}");
                }
            }
        }
    }

    Ok(())
}

fn run_doctor(cli: &CliArgs) -> std::io::Result<()> {
    let report = crate::application::doctor::diagnose().map_err(std::io::Error::other)?;
    if cli.doctor_json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(std::io::Error::other)?
        );
    } else {
        print_doctor_report(&report)?;
    }

    let exit_code = doctor_exit_code(&report);
    if exit_code == 0 {
        return Ok(());
    }

    Err(std::io::Error::other("doctor found actionable issues"))
}

fn doctor_exit_code(report: &DoctorReport) -> i32 {
    if report.summary.warn > 0 || report.summary.error > 0 {
        1
    } else {
        0
    }
}

pub fn run(args: impl IntoIterator<Item = String>) -> std::io::Result<()> {
    let cli = parse_cli_args(args)?;

    if cli.doctor {
        return run_doctor(&cli);
    }

    if cli.cleanup_sessions {
        return run_cleanup_sessions(&cli);
    }

    if cli.benchmark_scale {
        let options = crate::application::scale_benchmark::ScaleBenchmarkOptions {
            json_output: cli.benchmark_json_output,
            baseline_path: cli.benchmark_baseline_path,
            write_baseline_path: cli.benchmark_write_baseline_path,
            severe_regression_pct: cli.benchmark_warn_regression_pct.unwrap_or(
                crate::application::scale_benchmark::ScaleBenchmarkOptions::default()
                    .severe_regression_pct,
            ),
        };
        return crate::application::scale_benchmark::run_scale_benchmark(options);
    }

    if let Some(trace_path) = cli.replay_trace_path.as_ref() {
        if let Some(name) = cli.replay_emit_test_name.as_deref() {
            let fixture_path = crate::ui::tui::emit_replay_fixture(trace_path, name)?;
            println!("replay fixture written: {}", fixture_path.display());
        }

        let options = crate::ui::tui::ReplayOptions {
            invariant_only: cli.replay_invariant_only,
            snapshot_path: cli.replay_snapshot_path.clone(),
        };
        let outcome = crate::ui::tui::replay_debug_record(trace_path, &options)?;
        println!(
            "replay ok: steps={} states={} frames={}",
            outcome.steps_replayed, outcome.states_compared, outcome.frames_compared
        );
        return Ok(());
    }

    let app_start_ts = now_millis();
    let debug_record_path = if cli.debug_record {
        Some(debug_record_path(app_start_ts)?)
    } else {
        None
    };
    if let Some(path) = debug_record_path.as_ref() {
        eprintln!("grove debug record: {}", path.display());
    }
    let event_log_path = debug_record_path.or(cli.event_log_path.map(resolve_event_log_path));
    if let Some(path) = event_log_path.as_ref() {
        ensure_event_log_parent_directory(path)?;
    }

    if cli.print_hello {
        if let Some(event_log_path) = event_log_path.as_ref() {
            let _ = crate::infrastructure::event_log::FileEventLogger::open(event_log_path)?;
        }
        println!("Hello from grove.");
        return Ok(());
    }

    if cli.debug_record
        && let Some(path) = event_log_path
    {
        return crate::ui::tui::run_with_debug_record(path, app_start_ts);
    }

    crate::ui::tui::run_with_event_log(event_log_path)
}

#[cfg(test)]
mod tests {
    use super::{
        CliArgs, debug_record_path, doctor_exit_code, ensure_event_log_parent_directory,
        parse_cli_args, resolve_event_log_path,
    };
    use crate::application::doctor::{
        DoctorFinding, DoctorFindingKind, DoctorReport, DoctorSeverity, DoctorSubject,
    };
    use std::path::PathBuf;

    #[test]
    fn cli_parser_reads_event_log_and_print_hello() {
        let parsed = parse_cli_args(vec![
            "--event-log".to_string(),
            "/tmp/events.jsonl".to_string(),
            "--print-hello".to_string(),
        ])
        .expect("arguments should parse");

        assert_eq!(
            parsed,
            CliArgs {
                print_hello: true,
                event_log_path: Some(PathBuf::from("/tmp/events.jsonl")),
                debug_record: false,
                replay_trace_path: None,
                replay_snapshot_path: None,
                replay_emit_test_name: None,
                replay_invariant_only: false,
                benchmark_scale: false,
                benchmark_json_output: false,
                benchmark_baseline_path: None,
                benchmark_write_baseline_path: None,
                benchmark_warn_regression_pct: None,
                doctor: false,
                doctor_json_output: false,
                cleanup_sessions: false,
                cleanup_sessions_apply: false,
                cleanup_sessions_include_stale: false,
                cleanup_sessions_include_attached: false,
            }
        );
    }

    #[test]
    fn cli_parser_requires_event_log_path() {
        let error = parse_cli_args(vec!["--event-log".to_string()])
            .expect_err("missing event log path should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn cli_parser_reads_debug_record_flag() {
        let parsed =
            parse_cli_args(vec!["--debug-record".to_string()]).expect("debug flag should parse");
        assert_eq!(
            parsed,
            CliArgs {
                print_hello: false,
                event_log_path: None,
                debug_record: true,
                replay_trace_path: None,
                replay_snapshot_path: None,
                replay_emit_test_name: None,
                replay_invariant_only: false,
                benchmark_scale: false,
                benchmark_json_output: false,
                benchmark_baseline_path: None,
                benchmark_write_baseline_path: None,
                benchmark_warn_regression_pct: None,
                doctor: false,
                doctor_json_output: false,
                cleanup_sessions: false,
                cleanup_sessions_apply: false,
                cleanup_sessions_include_stale: false,
                cleanup_sessions_include_attached: false,
            }
        );
    }

    #[test]
    fn cli_parser_reads_replay_options() {
        let parsed = parse_cli_args(vec![
            "replay".to_string(),
            "/tmp/debug-record.jsonl".to_string(),
            "--snapshot".to_string(),
            "/tmp/replay-snapshot.json".to_string(),
            "--emit-test".to_string(),
            "flow-a".to_string(),
            "--invariant-only".to_string(),
        ])
        .expect("replay arguments should parse");

        assert_eq!(
            parsed,
            CliArgs {
                print_hello: false,
                event_log_path: None,
                debug_record: false,
                replay_trace_path: Some(PathBuf::from("/tmp/debug-record.jsonl")),
                replay_snapshot_path: Some(PathBuf::from("/tmp/replay-snapshot.json")),
                replay_emit_test_name: Some("flow-a".to_string()),
                replay_invariant_only: true,
                benchmark_scale: false,
                benchmark_json_output: false,
                benchmark_baseline_path: None,
                benchmark_write_baseline_path: None,
                benchmark_warn_regression_pct: None,
                doctor: false,
                doctor_json_output: false,
                cleanup_sessions: false,
                cleanup_sessions_apply: false,
                cleanup_sessions_include_stale: false,
                cleanup_sessions_include_attached: false,
            }
        );
    }

    #[test]
    fn cli_parser_rejects_replay_flags_without_replay_subcommand() {
        let error = parse_cli_args(vec!["--snapshot".to_string(), "/tmp/out.json".to_string()])
            .expect_err("replay-only flags without replay should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn cli_parser_reads_benchmark_scale_options() {
        let parsed = parse_cli_args(vec![
            "benchmark-scale".to_string(),
            "--json".to_string(),
            "--baseline".to_string(),
            "/tmp/baseline.json".to_string(),
            "--write-baseline".to_string(),
            "/tmp/new-baseline.json".to_string(),
            "--warn-regression-pct".to_string(),
            "25".to_string(),
        ])
        .expect("benchmark arguments should parse");

        assert_eq!(
            parsed,
            CliArgs {
                print_hello: false,
                event_log_path: None,
                debug_record: false,
                replay_trace_path: None,
                replay_snapshot_path: None,
                replay_emit_test_name: None,
                replay_invariant_only: false,
                benchmark_scale: true,
                benchmark_json_output: true,
                benchmark_baseline_path: Some(PathBuf::from("/tmp/baseline.json")),
                benchmark_write_baseline_path: Some(PathBuf::from("/tmp/new-baseline.json")),
                benchmark_warn_regression_pct: Some(25),
                doctor: false,
                doctor_json_output: false,
                cleanup_sessions: false,
                cleanup_sessions_apply: false,
                cleanup_sessions_include_stale: false,
                cleanup_sessions_include_attached: false,
            }
        );
    }

    #[test]
    fn cli_parser_rejects_benchmark_flags_without_benchmark_subcommand() {
        let error = parse_cli_args(vec!["--json".to_string()])
            .expect_err("benchmark flags without benchmark command should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn cli_parser_rejects_benchmark_and_replay_combination() {
        let error = parse_cli_args(vec![
            "benchmark-scale".to_string(),
            "replay".to_string(),
            "/tmp/trace.jsonl".to_string(),
        ])
        .expect_err("benchmark and replay should not combine");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn cli_parser_reads_cleanup_sessions_options() {
        let parsed = parse_cli_args(vec![
            "cleanup".to_string(),
            "sessions".to_string(),
            "--apply".to_string(),
            "--include-stale".to_string(),
            "--include-attached".to_string(),
        ])
        .expect("cleanup arguments should parse");

        assert_eq!(
            parsed,
            CliArgs {
                print_hello: false,
                event_log_path: None,
                debug_record: false,
                replay_trace_path: None,
                replay_snapshot_path: None,
                replay_emit_test_name: None,
                replay_invariant_only: false,
                benchmark_scale: false,
                benchmark_json_output: false,
                benchmark_baseline_path: None,
                benchmark_write_baseline_path: None,
                benchmark_warn_regression_pct: None,
                doctor: false,
                doctor_json_output: false,
                cleanup_sessions: true,
                cleanup_sessions_apply: true,
                cleanup_sessions_include_stale: true,
                cleanup_sessions_include_attached: true,
            }
        );
    }

    #[test]
    fn cli_parser_rejects_cleanup_flags_without_cleanup_subcommand() {
        let error = parse_cli_args(vec!["--apply".to_string()])
            .expect_err("cleanup flags without cleanup command should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn cli_parser_reads_doctor_options() {
        let parsed = parse_cli_args(vec!["doctor".to_string(), "--json".to_string()])
            .expect("doctor arguments should parse");

        assert_eq!(
            parsed,
            CliArgs {
                print_hello: false,
                event_log_path: None,
                debug_record: false,
                replay_trace_path: None,
                replay_snapshot_path: None,
                replay_emit_test_name: None,
                replay_invariant_only: false,
                benchmark_scale: false,
                benchmark_json_output: false,
                benchmark_baseline_path: None,
                benchmark_write_baseline_path: None,
                benchmark_warn_regression_pct: None,
                doctor: true,
                doctor_json_output: true,
                cleanup_sessions: false,
                cleanup_sessions_apply: false,
                cleanup_sessions_include_stale: false,
                cleanup_sessions_include_attached: false,
            }
        );
    }

    #[test]
    fn cli_parser_rejects_doctor_combined_with_other_modes() {
        let error = parse_cli_args(vec![
            "doctor".to_string(),
            "replay".to_string(),
            "/tmp/trace.jsonl".to_string(),
        ])
        .expect_err("doctor should not combine with replay");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn doctor_exit_code_is_zero_for_clean_report() {
        let report = DoctorReport::from_findings(Vec::new());
        assert_eq!(doctor_exit_code(&report), 0);
    }

    #[test]
    fn doctor_exit_code_is_nonzero_for_actionable_findings() {
        let report = DoctorReport::from_findings(vec![DoctorFinding {
            severity: DoctorSeverity::Warn,
            kind: DoctorFindingKind::MissingBaseMarker,
            subject: DoctorSubject {
                task_slug: Some("task-a".to_string()),
                manifest_path: None,
                repository_path: None,
                worktree_path: Some("/tmp/task-a".to_string()),
                session_name: None,
            },
            evidence: "missing base marker".to_string(),
            recommended_action: "write the base marker".to_string(),
        }]);
        assert_eq!(doctor_exit_code(&report), 1);
    }

    #[test]
    fn debug_record_path_uses_grove_directory_and_timestamp_prefix() {
        let app_start_ts = 1_771_023_000_555u64;
        let path = debug_record_path(app_start_ts).expect("path should resolve");
        let path_text = path.to_string_lossy();
        assert!(path_text.contains(".grove/"));
        assert!(path_text.contains(&format!("debug-record-{app_start_ts}")));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn resolve_event_log_path_places_relative_paths_under_grove_directory() {
        assert_eq!(
            resolve_event_log_path(PathBuf::from("events.jsonl")),
            PathBuf::from(".grove/events.jsonl")
        );
    }

    #[test]
    fn resolve_event_log_path_keeps_absolute_paths_unchanged() {
        assert_eq!(
            resolve_event_log_path(PathBuf::from("/tmp/events.jsonl")),
            PathBuf::from("/tmp/events.jsonl")
        );
    }

    #[test]
    fn resolve_event_log_path_keeps_grove_prefixed_relative_paths() {
        assert_eq!(
            resolve_event_log_path(PathBuf::from(".grove/custom/events.jsonl")),
            PathBuf::from(".grove/custom/events.jsonl")
        );
    }

    #[test]
    fn ensure_event_log_parent_directory_creates_missing_directories() {
        let root = std::env::temp_dir().join(format!(
            "grove-main-tests-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos()
        ));
        let path = root.join(".grove/nested/events.jsonl");

        ensure_event_log_parent_directory(&path).expect("parent directory should be created");
        assert!(root.join(".grove/nested").exists());

        let _ = std::fs::remove_dir_all(root);
    }
}

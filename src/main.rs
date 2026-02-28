use std::fs;
use std::path::{Path, PathBuf};

use grove::infrastructure::event_log::now_millis;

const DEBUG_RECORD_DIR: &str = ".grove";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CliArgs {
    print_hello: bool,
    event_log_path: Option<PathBuf>,
    debug_record: bool,
    replay_trace_path: Option<PathBuf>,
    replay_snapshot_path: Option<PathBuf>,
    replay_emit_test_name: Option<String>,
    replay_invariant_only: bool,
}

fn parse_cli_args(args: impl IntoIterator<Item = String>) -> std::io::Result<CliArgs> {
    let mut cli = CliArgs::default();
    let mut args = args.into_iter();

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
                let Some(path) = args.next() else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "replay requires a trace path",
                    ));
                };
                cli.replay_trace_path = Some(PathBuf::from(path));
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
            _ => {}
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

    Ok(cli)
}

fn debug_record_path(app_start_ts: u64) -> std::io::Result<PathBuf> {
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

fn resolve_event_log_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }

    let grove_dir = Path::new(DEBUG_RECORD_DIR);
    if path.starts_with(grove_dir) {
        return path;
    }

    grove_dir.join(path)
}

fn ensure_event_log_parent_directory(path: &Path) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }

    fs::create_dir_all(parent)
}

fn main() -> std::io::Result<()> {
    let cli = parse_cli_args(std::env::args().skip(1))?;

    if let Some(trace_path) = cli.replay_trace_path.as_ref() {
        if let Some(name) = cli.replay_emit_test_name.as_deref() {
            let fixture_path = grove::ui::tui::emit_replay_fixture(trace_path, name)?;
            println!("replay fixture written: {}", fixture_path.display());
        }

        let options = grove::ui::tui::ReplayOptions {
            invariant_only: cli.replay_invariant_only,
            snapshot_path: cli.replay_snapshot_path.clone(),
        };
        let outcome = grove::ui::tui::replay_debug_record(trace_path, &options)?;
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
            let _ = grove::infrastructure::event_log::FileEventLogger::open(event_log_path)?;
        }
        println!("Hello from grove.");
        return Ok(());
    }

    if cli.debug_record
        && let Some(path) = event_log_path
    {
        return grove::ui::tui::run_with_debug_record(path, app_start_ts);
    }

    grove::ui::tui::run_with_event_log(event_log_path)
}

#[cfg(test)]
mod main_tests;

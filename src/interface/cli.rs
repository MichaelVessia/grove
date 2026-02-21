use std::fs;
use std::path::{Path, PathBuf};

use crate::infrastructure::event_log::{FileEventLogger, now_millis};

const DEBUG_RECORD_DIR: &str = ".grove";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CliArgs {
    print_hello: bool,
    event_log_path: Option<PathBuf>,
    debug_record: bool,
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
            _ => {}
        }
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

pub fn run(args: impl IntoIterator<Item = String>) -> std::io::Result<()> {
    let cli = parse_cli_args(args)?;
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
            let _ = FileEventLogger::open(event_log_path)?;
        }
        println!("Hello from grove.");
        return Ok(());
    }

    if cli.debug_record
        && let Some(path) = event_log_path
    {
        return crate::interface::tui::run_with_debug_record(path, app_start_ts);
    }

    crate::interface::tui::run_with_event_log(event_log_path)
}

#[cfg(test)]
mod tests {
    use super::{
        CliArgs, debug_record_path, ensure_event_log_parent_directory, parse_cli_args,
        resolve_event_log_path,
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
            }
        );
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

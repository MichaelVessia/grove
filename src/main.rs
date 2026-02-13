use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

fn now_millis() -> u64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
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

fn main() -> std::io::Result<()> {
    let cli = parse_cli_args(std::env::args().skip(1))?;
    let app_start_ts = now_millis();
    let debug_record_path = if cli.debug_record {
        Some(debug_record_path(app_start_ts)?)
    } else {
        None
    };
    if let Some(path) = debug_record_path.as_ref() {
        eprintln!("grove debug record: {}", path.display());
    }
    let event_log_path = debug_record_path.or(cli.event_log_path);

    if cli.print_hello {
        if let Some(event_log_path) = event_log_path.as_ref() {
            let _ = grove::event_log::FileEventLogger::open(event_log_path)?;
        }
        println!("{}", grove::hello_message("grove"));
        return Ok(());
    }

    if cli.debug_record {
        if let Some(path) = event_log_path {
            return grove::run_tui_with_debug_record(path, app_start_ts);
        }
    }

    grove::run_tui_with_event_log(event_log_path)
}

#[cfg(test)]
mod tests {
    use super::{CliArgs, debug_record_path, parse_cli_args};
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
        let parsed = parse_cli_args(vec!["--debug-record".to_string()])
            .expect("debug flag should parse");
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
}

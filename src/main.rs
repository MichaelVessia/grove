use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct CliArgs {
    print_hello: bool,
    event_log_path: Option<PathBuf>,
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
            _ => {}
        }
    }

    Ok(cli)
}

fn main() -> std::io::Result<()> {
    let cli = parse_cli_args(std::env::args().skip(1))?;

    if cli.print_hello {
        if let Some(event_log_path) = cli.event_log_path.as_ref() {
            let _ = grove::event_log::FileEventLogger::open(event_log_path)?;
        }
        println!("{}", grove::hello_message("grove"));
        return Ok(());
    }

    grove::run_tui_with_event_log(cli.event_log_path)
}

#[cfg(test)]
mod tests {
    use super::{CliArgs, parse_cli_args};
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
            }
        );
    }

    #[test]
    fn cli_parser_requires_event_log_path() {
        let error = parse_cli_args(vec!["--event-log".to_string()])
            .expect_err("missing event log path should fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }
}

use std::path::PathBuf;

pub use crate::ui::tui::RuntimeObservabilityPaths;

pub fn run_with_event_log(
    event_log_path: Option<PathBuf>,
    daemon_socket_path: Option<PathBuf>,
    observability_paths: RuntimeObservabilityPaths,
) -> std::io::Result<()> {
    crate::ui::tui::run_with_event_log(event_log_path, daemon_socket_path, observability_paths)
}

pub fn run_with_debug_record(
    event_log_path: PathBuf,
    app_start_ts: u64,
    daemon_socket_path: Option<PathBuf>,
    observability_paths: RuntimeObservabilityPaths,
) -> std::io::Result<()> {
    crate::ui::tui::run_with_debug_record(
        event_log_path,
        app_start_ts,
        daemon_socket_path,
        observability_paths,
    )
}

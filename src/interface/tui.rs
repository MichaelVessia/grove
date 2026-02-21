use std::path::PathBuf;

pub fn run_with_event_log(event_log_path: Option<PathBuf>) -> std::io::Result<()> {
    crate::ui::tui::run_with_event_log(event_log_path)
}

pub fn run_with_debug_record(event_log_path: PathBuf, app_start_ts: u64) -> std::io::Result<()> {
    crate::ui::tui::run_with_debug_record(event_log_path, app_start_ts)
}

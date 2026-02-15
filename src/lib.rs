pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod ui;

pub fn hello_message(app_name: &str) -> String {
    format!("Hello from {app_name}.")
}

pub fn run_tui_with_event_log(event_log_path: Option<std::path::PathBuf>) -> std::io::Result<()> {
    ui::tui::run_with_event_log(event_log_path)
}

pub fn run_tui_with_debug_record(
    event_log_path: std::path::PathBuf,
    app_start_ts: u64,
) -> std::io::Result<()> {
    ui::tui::run_with_debug_record(event_log_path, app_start_ts)
}

#[cfg(test)]
mod lib_tests;

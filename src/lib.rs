pub mod adapters;
pub mod agent_runtime;
pub mod config;
pub mod domain;
pub mod event_log;
pub mod hardening;
pub mod interactive;
pub mod mouse;
pub mod preview;
pub mod state;
pub mod tui;
pub mod ui;
pub mod workspace_lifecycle;
pub mod zellij_emulator;

pub fn hello_message(app_name: &str) -> String {
    format!("Hello from {app_name}.")
}

pub fn run_tui_with_event_log(event_log_path: Option<std::path::PathBuf>) -> std::io::Result<()> {
    tui::run_with_event_log(event_log_path)
}

pub fn run_tui_with_debug_record(
    event_log_path: std::path::PathBuf,
    app_start_ts: u64,
) -> std::io::Result<()> {
    tui::run_with_debug_record(event_log_path, app_start_ts)
}

#[cfg(test)]
mod tests {
    use super::hello_message;

    #[test]
    fn hello_message_includes_app_name() {
        assert_eq!(hello_message("grove"), "Hello from grove.");
    }
}

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use ftui::render::budget::FrameBudgetConfig;
use ftui::runtime::{DiffStrategyConfig, WidgetRefreshConfig};
use ftui::{Program, ProgramConfig};
use serde_json::Value;

use crate::infrastructure::event_log::{
    Event as LogEvent, EventLogger, FileEventLogger, NullEventLogger,
};

use super::GroveApp;

pub fn run_with_event_log(
    event_log_path: Option<PathBuf>,
    daemon_socket_path: Option<PathBuf>,
) -> std::io::Result<()> {
    run_with_logger(event_log_path, None, daemon_socket_path)
}

pub fn run_with_debug_record(
    event_log_path: PathBuf,
    app_start_ts: u64,
    daemon_socket_path: Option<PathBuf>,
) -> std::io::Result<()> {
    run_with_logger(Some(event_log_path), Some(app_start_ts), daemon_socket_path)
}

fn run_with_logger(
    event_log_path: Option<PathBuf>,
    debug_record_start_ts: Option<u64>,
    daemon_socket_path: Option<PathBuf>,
) -> std::io::Result<()> {
    ensure_tmux_extended_keys();

    let event_log: Box<dyn EventLogger> = if let Some(path) = event_log_path {
        Box::new(FileEventLogger::open(&path)?)
    } else {
        Box::new(NullEventLogger)
    };

    if let Some(app_start_ts) = debug_record_start_ts {
        event_log.log(
            LogEvent::new("debug_record", "started")
                .with_data("app_start_ts", Value::from(app_start_ts)),
        );
    }

    let app = GroveApp::new(event_log, debug_record_start_ts, daemon_socket_path);

    let config = program_config();
    Program::with_config(app, config)?.run()
}

fn program_config() -> ProgramConfig {
    let mut config = ProgramConfig::fullscreen()
        .with_mouse()
        .with_diff_config(ftui::RuntimeDiffConfig::default().with_strategy_config(
            DiffStrategyConfig {
                c_scan: 1_000_000.0,
                uncertainty_guard_variance: 1_000_000.0,
                hysteresis_ratio: 0.0,
                ..DiffStrategyConfig::default()
            },
        ))
        .with_budget(FrameBudgetConfig::strict(Duration::from_millis(250)))
        .with_widget_refresh(WidgetRefreshConfig {
            enabled: false,
            ..WidgetRefreshConfig::default()
        });
    config.kitty_keyboard = true;
    config
}

fn ensure_tmux_extended_keys() {
    if std::env::var_os("TMUX").is_none() {
        return;
    }

    let Ok(output) = Command::new("tmux")
        .args(["show-options", "-sv", "extended-keys"])
        .output()
    else {
        return;
    };
    if !output.status.success() {
        return;
    }

    let mode = String::from_utf8_lossy(&output.stdout);
    if !tmux_extended_keys_needs_enable(mode.as_ref()) {
        return;
    }

    let _ = Command::new("tmux")
        .args(["set-option", "-sq", "extended-keys", "on"])
        .output();
}

fn tmux_extended_keys_needs_enable(current_mode: &str) -> bool {
    let normalized = current_mode.trim().to_ascii_lowercase();
    !(normalized == "on" || normalized == "always")
}

#[cfg(test)]
mod tests {
    use super::{program_config, tmux_extended_keys_needs_enable};

    #[test]
    fn program_config_enables_kitty_keyboard() {
        assert!(program_config().kitty_keyboard);
    }

    #[test]
    fn tmux_extended_keys_needs_enable_only_when_off() {
        assert!(tmux_extended_keys_needs_enable("off"));
        assert!(tmux_extended_keys_needs_enable(""));
        assert!(!tmux_extended_keys_needs_enable("on"));
        assert!(!tmux_extended_keys_needs_enable("always"));
    }
}

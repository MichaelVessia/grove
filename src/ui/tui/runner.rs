use std::path::PathBuf;
use std::time::Duration;

use ftui::render::budget::FrameBudgetConfig;
use ftui::runtime::WidgetRefreshConfig;
use ftui::{App, ScreenMode};
use serde_json::Value;

use crate::infrastructure::event_log::{
    Event as LogEvent, EventLogger, FileEventLogger, NullEventLogger,
};

use super::GroveApp;

pub fn run() -> std::io::Result<()> {
    run_with_event_log(None)
}

pub fn run_with_event_log(event_log_path: Option<PathBuf>) -> std::io::Result<()> {
    run_with_logger(event_log_path, None)
}

pub fn run_with_debug_record(event_log_path: PathBuf, app_start_ts: u64) -> std::io::Result<()> {
    run_with_logger(Some(event_log_path), Some(app_start_ts))
}

fn run_with_logger(
    event_log_path: Option<PathBuf>,
    debug_record_start_ts: Option<u64>,
) -> std::io::Result<()> {
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

    let app = if let Some(app_start_ts) = debug_record_start_ts {
        GroveApp::new_with_debug_recorder(event_log, app_start_ts)
    } else {
        GroveApp::new_with_event_logger(event_log)
    };

    App::new(app)
        .screen_mode(ScreenMode::AltScreen)
        .with_mouse()
        .with_budget(FrameBudgetConfig::strict(Duration::from_millis(250)))
        .with_widget_refresh(WidgetRefreshConfig {
            enabled: false,
            ..WidgetRefreshConfig::default()
        })
        .run()
}

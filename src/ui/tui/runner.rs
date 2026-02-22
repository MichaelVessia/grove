use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs::File, io::BufWriter, io::Write};

use ftui::render::budget::FrameBudgetConfig;
use ftui::runtime::WidgetRefreshConfig;
use ftui::runtime::{
    EvidenceSinkConfig, FrameTiming, FrameTimingConfig, FrameTimingSink, RenderTraceConfig,
};
use ftui::{Program, ProgramConfig};
use serde_json::{Value, json};

use crate::infrastructure::event_log::{
    Event as LogEvent, EventLogger, FileEventLogger, NullEventLogger,
};

use super::GroveApp;

#[derive(Debug, Clone, Default)]
pub struct RuntimeObservabilityPaths {
    pub evidence_log_path: Option<PathBuf>,
    pub render_trace_path: Option<PathBuf>,
    pub frame_timing_log_path: Option<PathBuf>,
}

pub fn run_with_event_log(
    event_log_path: Option<PathBuf>,
    daemon_socket_path: Option<PathBuf>,
    observability_paths: RuntimeObservabilityPaths,
) -> std::io::Result<()> {
    run_with_logger(
        event_log_path,
        None,
        daemon_socket_path,
        observability_paths,
    )
}

pub fn run_with_debug_record(
    event_log_path: PathBuf,
    app_start_ts: u64,
    daemon_socket_path: Option<PathBuf>,
    observability_paths: RuntimeObservabilityPaths,
) -> std::io::Result<()> {
    run_with_logger(
        Some(event_log_path),
        Some(app_start_ts),
        daemon_socket_path,
        observability_paths,
    )
}

fn run_with_logger(
    event_log_path: Option<PathBuf>,
    debug_record_start_ts: Option<u64>,
    daemon_socket_path: Option<PathBuf>,
    observability_paths: RuntimeObservabilityPaths,
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

    let config = program_config(&observability_paths)?;
    Program::with_config(app, config)?.run()
}

fn program_config(
    observability_paths: &RuntimeObservabilityPaths,
) -> std::io::Result<ProgramConfig> {
    let mut config = ProgramConfig::fullscreen()
        .with_mouse()
        .with_budget(FrameBudgetConfig::strict(Duration::from_millis(500)))
        .with_widget_refresh(WidgetRefreshConfig {
            enabled: false,
            ..WidgetRefreshConfig::default()
        });
    if let Some(path) = observability_paths.evidence_log_path.as_ref() {
        config = config.with_evidence_sink(EvidenceSinkConfig::enabled_file(path));
    }
    if let Some(path) = observability_paths.render_trace_path.as_ref() {
        config = config.with_render_trace(RenderTraceConfig::enabled_file(path));
    }
    if let Some(path) = observability_paths.frame_timing_log_path.as_ref() {
        let sink = FrameTimingJsonlSink::new(path.clone())?;
        config = config.with_frame_timing(FrameTimingConfig::new(Arc::new(sink)));
    }
    config.kitty_keyboard = true;
    Ok(config)
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

#[derive(Debug)]
struct FrameTimingJsonlSink {
    writer: Mutex<BufWriter<File>>,
    started_at: std::time::Instant,
}

impl FrameTimingJsonlSink {
    fn new(path: PathBuf) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
            started_at: std::time::Instant::now(),
        })
    }
}

impl FrameTimingSink for FrameTimingJsonlSink {
    fn record_frame(&self, timing: &FrameTiming) {
        let mut writer = match self.writer.lock() {
            Ok(writer) => writer,
            Err(_) => return,
        };
        let mono_ms = u64::try_from(self.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        let line = json!({
            "event": "frame_runtime",
            "kind": "timing",
            "mono_ms": mono_ms,
            "frame_idx": timing.frame_idx,
            "update_us": timing.update_us,
            "render_us": timing.render_us,
            "diff_us": timing.diff_us,
            "present_us": timing.present_us,
            "total_us": timing.total_us
        });
        let Ok(text) = serde_json::to_string(&line) else {
            return;
        };
        if writer.write_all(text.as_bytes()).is_err() {
            return;
        }
        if writer.write_all(b"\n").is_err() {
            return;
        }
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::{program_config, tmux_extended_keys_needs_enable};

    #[test]
    fn program_config_enables_kitty_keyboard() {
        assert!(
            program_config(&super::RuntimeObservabilityPaths::default())
                .expect("program config should build")
                .kitty_keyboard
        );
    }

    #[test]
    fn tmux_extended_keys_needs_enable_only_when_off() {
        assert!(tmux_extended_keys_needs_enable("off"));
        assert!(tmux_extended_keys_needs_enable(""));
        assert!(!tmux_extended_keys_needs_enable("on"));
        assert!(!tmux_extended_keys_needs_enable("always"));
    }
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::BufRead;
use std::io::Write;
use std::process::ChildStdin;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use ftui::runtime::{StopSignal, SubId, Subscription};
use serde_json::Value;

use crate::infrastructure::event_log::Event as LogEvent;
use crate::ui::tui::{
    CommandTmuxInput, GroveApp, LIVE_PREVIEW_FULL_SCROLLBACK_LINES, Msg, PreviewSessionGeometry,
    PreviewStreamConnected, PreviewStreamDisconnected, PreviewStreamEvent, PreviewStreamOutput,
};

const TMUX_CONTROL_MODE_POLL_MS: u64 = 50;
const TMUX_CONTROL_MODE_PAUSE_AFTER_SECONDS: &str = "0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::tui) enum PreviewStreamSource {
    Connecting,
    Stream,
    Fallback,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui::tui) struct PreviewStreamState {
    pub(in crate::ui::tui) target_session: Option<String>,
    pub(in crate::ui::tui) connected_session: Option<String>,
    pub(in crate::ui::tui) generation: u64,
    pub(in crate::ui::tui) bootstrap_completed: bool,
    pub(in crate::ui::tui) reconciliation_pending: bool,
    pub(in crate::ui::tui) last_chunk_bytes: usize,
    pub(in crate::ui::tui) source: PreviewStreamSource,
    pub(in crate::ui::tui) buffer: String,
}

impl Default for PreviewStreamState {
    fn default() -> Self {
        Self {
            target_session: None,
            connected_session: None,
            generation: 0,
            bootstrap_completed: false,
            reconciliation_pending: false,
            last_chunk_bytes: 0,
            source: PreviewStreamSource::Disconnected,
            buffer: String::new(),
        }
    }
}

impl GroveApp {
    fn latest_live_preview_raw_output_for_session(&self, session_name: &str) -> Option<String> {
        if self.polling.last_live_preview_session.as_deref() != Some(session_name) {
            return None;
        }
        self.preview
            .recent_captures
            .back()
            .map(|capture| capture.raw_output.clone())
    }

    fn desired_preview_stream_session(&self) -> Option<String> {
        self.selected_live_preview_session_if_ready()
    }

    pub(in crate::ui::tui) fn preview_stream_subscription(
        &self,
    ) -> Option<Box<dyn Subscription<Msg>>> {
        let session = self.polling.preview_stream.target_session.clone()?;
        Some(Box::new(SelectedPreviewStreamSubscription::new(
            session,
            self.polling.preview_stream.generation,
        )))
    }

    pub(in crate::ui::tui) fn preview_stream_is_healthy_for_session(
        &self,
        session_name: &str,
    ) -> bool {
        self.polling.preview_stream.source == PreviewStreamSource::Stream
            && self.polling.preview_stream.target_session.as_deref() == Some(session_name)
            && self.polling.preview_stream.connected_session.as_deref() == Some(session_name)
    }

    pub(in crate::ui::tui) fn preview_stream_blocks_selected_poll(
        &self,
        session_name: &str,
    ) -> bool {
        if self.interactive_target_session().as_deref() == Some(session_name) {
            return false;
        }
        self.preview_stream_is_healthy_for_session(session_name)
            && self.polling.preview_stream.bootstrap_completed
            && !self.polling.preview_stream.reconciliation_pending
    }

    pub(in crate::ui::tui) fn sync_preview_stream_target(&mut self) {
        let desired = self.desired_preview_stream_session();
        let previous = self.polling.preview_stream.target_session.clone();
        if desired == self.polling.preview_stream.target_session {
            if desired.is_none() {
                self.polling.preview_stream.connected_session = None;
                self.polling.preview_stream.bootstrap_completed = false;
                self.polling.preview_stream.reconciliation_pending = false;
                self.polling.preview_stream.source = PreviewStreamSource::Disconnected;
                self.polling.preview_stream.buffer.clear();
            }
            return;
        }

        self.polling.preview_stream.generation =
            self.polling.preview_stream.generation.saturating_add(1);
        self.polling.preview_stream.target_session = desired.clone();
        self.polling.preview_stream.connected_session = None;
        self.polling.preview_stream.bootstrap_completed = false;
        self.polling.preview_stream.reconciliation_pending = false;
        self.polling.preview_stream.last_chunk_bytes = 0;
        self.polling.preview_session_geometry = None;
        let preserve_interactive_terminal = self.interactive_target_session().is_some()
            && self.preview.selected_terminal().is_some();
        if !preserve_interactive_terminal {
            if previous.is_some() {
                self.preview.reset_selected_session_state();
            } else {
                self.preview.clear_selected_terminal();
            }
        }
        self.polling.preview_stream.source = if desired.is_some() {
            PreviewStreamSource::Connecting
        } else {
            PreviewStreamSource::Disconnected
        };
        self.polling.preview_stream.buffer.clear();
    }

    fn preview_stream_matches(&self, session_name: &str, generation: u64) -> bool {
        self.polling.preview_stream.target_session.as_deref() == Some(session_name)
            && self.polling.preview_stream.generation == generation
    }

    fn mark_preview_stream_connected(&mut self, session_name: &str, generation: u64) -> bool {
        if !self.preview_stream_matches(session_name, generation) {
            self.telemetry.event_log.log(
                LogEvent::new("preview_stream", "stale_event_dropped")
                    .with_data("event", Value::from("connected"))
                    .with_data("session", Value::from(session_name.to_string()))
                    .with_data("generation", Value::from(generation))
                    .with_data(
                        "latest_generation",
                        Value::from(self.polling.preview_stream.generation),
                    ),
            );
            return false;
        }

        let was_connected = self.polling.preview_stream.connected_session.as_deref()
            == Some(session_name)
            && self.polling.preview_stream.source == PreviewStreamSource::Stream;
        self.polling.preview_stream.connected_session = Some(session_name.to_string());
        self.polling.preview_stream.source = PreviewStreamSource::Stream;
        if !was_connected {
            self.telemetry.event_log.log(
                LogEvent::new("preview_stream", "connected")
                    .with_data("session", Value::from(session_name.to_string()))
                    .with_data("generation", Value::from(generation)),
            );
        }
        true
    }

    fn handle_preview_stream_output(&mut self, output: PreviewStreamOutput) {
        if !self.mark_preview_stream_connected(output.session.as_str(), output.generation) {
            return;
        }

        self.polling.preview_stream.last_chunk_bytes = output.chunk.len();
        self.polling
            .preview_stream
            .buffer
            .push_str(output.chunk.as_str());

        let geometry = self
            .polling
            .preview_session_geometry
            .as_ref()
            .filter(|geometry| geometry.session == output.session)
            .cloned()
            .or_else(|| {
                self.preview_output_dimensions()
                    .map(|(width, height)| PreviewSessionGeometry {
                        session: output.session.clone(),
                        width,
                        height,
                    })
            });

        if let Some(geometry) = geometry {
            if self.polling.preview_stream.bootstrap_completed {
                if self.preview.selected_terminal().is_some() {
                    self.preview
                        .apply_selected_terminal_chunk(output.chunk.as_str());
                } else {
                    self.preview.bootstrap_selected_terminal_from_stream(
                        self.polling.preview_stream.buffer.as_str(),
                        geometry.width,
                        geometry.height,
                    );
                }
            } else if let Some(raw_output) =
                self.latest_live_preview_raw_output_for_session(output.session.as_str())
            {
                self.preview.bootstrap_selected_terminal_from_stream(
                    raw_output.as_str(),
                    geometry.width,
                    geometry.height,
                );
                self.preview
                    .apply_selected_terminal_chunk(output.chunk.as_str());
                self.polling.preview_stream.reconciliation_pending = true;
            } else {
                self.polling.preview_stream.reconciliation_pending = true;
            }
        } else {
            self.preview.clear_selected_terminal();
        }

        if !self.polling.preview_stream.bootstrap_completed
            || self.polling.preview_stream.reconciliation_pending
        {
            self.poll_preview_prioritized();
        }
    }

    fn handle_preview_stream_disconnect(&mut self, disconnect: PreviewStreamDisconnected) {
        if !self.preview_stream_matches(disconnect.session.as_str(), disconnect.generation) {
            self.telemetry.event_log.log(
                LogEvent::new("preview_stream", "stale_event_dropped")
                    .with_data("event", Value::from("disconnected"))
                    .with_data("session", Value::from(disconnect.session.clone()))
                    .with_data("generation", Value::from(disconnect.generation))
                    .with_data(
                        "latest_generation",
                        Value::from(self.polling.preview_stream.generation),
                    ),
            );
            return;
        }

        self.polling.preview_stream.connected_session = None;
        self.polling.preview_stream.bootstrap_completed = true;
        self.polling.preview_stream.reconciliation_pending = false;
        self.polling.preview_stream.last_chunk_bytes = 0;
        self.polling.preview_stream.source = PreviewStreamSource::Fallback;
        self.preview.clear_selected_terminal();
        self.polling.preview_stream.buffer.clear();
        self.session.last_tmux_error = disconnect.error.clone();
        let mut event = LogEvent::new("preview_stream", "disconnected")
            .with_data("session", Value::from(disconnect.session.clone()))
            .with_data("generation", Value::from(disconnect.generation));
        if let Some(error) = disconnect.error.clone() {
            event = event.with_data("error", Value::from(error));
        }
        self.telemetry.event_log.log(event);
    }

    pub(in crate::ui::tui) fn handle_preview_stream_event(&mut self, event: PreviewStreamEvent) {
        match event {
            PreviewStreamEvent::Connected(connected) => {
                self.mark_preview_stream_connected(
                    connected.session.as_str(),
                    connected.generation,
                );
            }
            PreviewStreamEvent::Output(output) => self.handle_preview_stream_output(output),
            PreviewStreamEvent::Disconnected(disconnect) => {
                self.handle_preview_stream_disconnect(disconnect);
            }
        }
    }

    pub(in crate::ui::tui) fn mark_preview_stream_bootstrap_completed(
        &mut self,
        session_name: &str,
    ) {
        if self.polling.preview_stream.target_session.as_deref() != Some(session_name) {
            return;
        }
        self.polling.preview_stream.buffer.clear();
        self.polling.preview_stream.bootstrap_completed = true;
        self.polling.preview_stream.reconciliation_pending = false;
        if self.polling.preview_stream.source == PreviewStreamSource::Connecting {
            self.telemetry.event_log.log(
                LogEvent::new("preview_stream", "bootstrap_completed")
                    .with_data("session", Value::from(session_name.to_string()))
                    .with_data(
                        "generation",
                        Value::from(self.polling.preview_stream.generation),
                    ),
            );
        }
    }
}

struct SelectedPreviewStreamSubscription {
    session: String,
    generation: u64,
    id: SubId,
}

impl SelectedPreviewStreamSubscription {
    fn new(session: String, generation: u64) -> Self {
        let mut hasher = DefaultHasher::new();
        "selected_preview_stream".hash(&mut hasher);
        session.hash(&mut hasher);
        generation.hash(&mut hasher);
        Self {
            session,
            generation,
            id: hasher.finish(),
        }
    }

    fn process_event_to_msg(
        session_name: &str,
        generation: u64,
        event: StreamProcessEvent,
    ) -> Option<Msg> {
        match event {
            StreamProcessEvent::Stdout(line) => map_control_mode_line_to_msg(
                session_name,
                generation,
                line.as_str(),
                |session_name| {
                    CommandTmuxInput::capture_session_output(
                        session_name,
                        LIVE_PREVIEW_FULL_SCROLLBACK_LINES,
                        true,
                    )
                },
            ),
            StreamProcessEvent::Exited { error } => Some(Msg::PreviewStreamEvent(
                PreviewStreamEvent::Disconnected(PreviewStreamDisconnected {
                    session: session_name.to_string(),
                    generation,
                    error,
                }),
            )),
        }
    }
}

impl Subscription<Msg> for SelectedPreviewStreamSubscription {
    fn id(&self) -> SubId {
        self.id
    }

    fn run(&self, sender: mpsc::Sender<Msg>, stop: StopSignal) {
        let pane_id = match active_pane_id_for_session(self.session.as_str()) {
            Ok(pane_id) => pane_id,
            Err(error) => {
                let _ = sender.send(Msg::PreviewStreamEvent(PreviewStreamEvent::Disconnected(
                    PreviewStreamDisconnected {
                        session: self.session.clone(),
                        generation: self.generation,
                        error: Some(format!("tmux active pane lookup failed: {error}")),
                    },
                )));
                return;
            }
        };
        let mut command = Command::new("tmux");
        command
            .args([
                "-C",
                "attach-session",
                "-t",
                self.session.as_str(),
                "-f",
                &format!(
                    "read-only,ignore-size,pause-after={TMUX_CONTROL_MODE_PAUSE_AFTER_SECONDS}"
                ),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let _ = sender.send(Msg::PreviewStreamEvent(PreviewStreamEvent::Disconnected(
                    PreviewStreamDisconnected {
                        session: self.session.clone(),
                        generation: self.generation,
                        error: Some(format!("tmux control stream spawn failed: {error}")),
                    },
                )));
                return;
            }
        };

        let mut control_stdin = match child.stdin.take() {
            Some(stdin) => match write_control_mode_startup_commands(stdin, pane_id.as_str()) {
                Ok(stdin) => Some(stdin),
                Err(error) => {
                    let _ = sender.send(Msg::PreviewStreamEvent(PreviewStreamEvent::Disconnected(
                        PreviewStreamDisconnected {
                            session: self.session.clone(),
                            generation: self.generation,
                            error: Some(format!("tmux control stream init failed: {error}")),
                        },
                    )));
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                }
            },
            None => None,
        };

        if control_stdin.is_none() {
            let _ = sender.send(Msg::PreviewStreamEvent(PreviewStreamEvent::Disconnected(
                PreviewStreamDisconnected {
                    session: self.session.clone(),
                    generation: self.generation,
                    error: Some("tmux control stream stdin unavailable".to_string()),
                },
            )));
            let _ = child.kill();
            let _ = child.wait();
            return;
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let stderr_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let session = self.session.clone();
        let generation = self.generation;
        std::thread::scope(|scope| {
            let _control_stdin = control_stdin.take();
            let stdout_handle = stdout.map(|stdout| {
                let sender = sender.clone();
                let session = session.clone();
                scope.spawn(move || {
                    let reader = std::io::BufReader::new(stdout);
                    for line in reader.lines() {
                        let Ok(line) = line else {
                            break;
                        };
                        let Some(msg) = Self::process_event_to_msg(
                            session.as_str(),
                            generation,
                            StreamProcessEvent::Stdout(line),
                        ) else {
                            continue;
                        };
                        if sender.send(msg).is_err() {
                            break;
                        }
                    }
                })
            });
            let stderr_handle = stderr.map(|stderr| {
                let stderr_lines = Arc::clone(&stderr_lines);
                scope.spawn(move || {
                    let reader = std::io::BufReader::new(stderr);
                    for line in reader.lines() {
                        let Ok(line) = line else {
                            break;
                        };
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        let Ok(mut lines) = stderr_lines.lock() else {
                            return;
                        };
                        lines.push(trimmed.to_string());
                    }
                })
            });

            let final_event = loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let error = if status.success() {
                            None
                        } else {
                            Some(exit_error_message(status.code(), &stderr_lines))
                        };
                        break StreamProcessEvent::Exited { error };
                    }
                    Ok(None) => {}
                    Err(error) => {
                        break StreamProcessEvent::Exited {
                            error: Some(format!("tmux control stream wait failed: {error}")),
                        };
                    }
                }

                if stop.is_stopped() {
                    let _ = child.kill();
                    let _ = child.wait();
                    break StreamProcessEvent::Exited { error: None };
                }
                std::thread::sleep(Duration::from_millis(TMUX_CONTROL_MODE_POLL_MS));
            };

            if let Some(handle) = stdout_handle {
                let _ = handle.join();
            }
            if let Some(handle) = stderr_handle {
                let _ = handle.join();
            }
            if let Some(msg) = Self::process_event_to_msg(session.as_str(), generation, final_event)
            {
                let _ = sender.send(msg);
            }
        });
    }
}

enum StreamProcessEvent {
    Stdout(String),
    Exited { error: Option<String> },
}

enum ParsedControlModeLine {
    Connected,
    Output(String),
    Disconnected(Option<String>),
}

fn exit_error_message(code: Option<i32>, stderr_lines: &Arc<Mutex<Vec<String>>>) -> String {
    let status = match code {
        Some(code) => format!("tmux control stream exited with status {code}"),
        None => "tmux control stream exited".to_string(),
    };
    let Ok(lines) = stderr_lines.lock() else {
        return status;
    };
    let stderr = lines.join(" ");
    if stderr.is_empty() {
        return status;
    }
    format!("{status}: {stderr}")
}

fn map_control_mode_line_to_msg(
    session_name: &str,
    generation: u64,
    line: &str,
    _capture_snapshot: impl Fn(&str) -> std::io::Result<String>,
) -> Option<Msg> {
    let parsed = parse_control_mode_line(session_name, line)?;
    let event = match parsed {
        ParsedControlModeLine::Connected => PreviewStreamEvent::Connected(PreviewStreamConnected {
            session: session_name.to_string(),
            generation,
        }),
        ParsedControlModeLine::Output(chunk) => PreviewStreamEvent::Output(PreviewStreamOutput {
            session: session_name.to_string(),
            generation,
            chunk,
        }),
        ParsedControlModeLine::Disconnected(error) => {
            PreviewStreamEvent::Disconnected(PreviewStreamDisconnected {
                session: session_name.to_string(),
                generation,
                error,
            })
        }
    };
    Some(Msg::PreviewStreamEvent(event))
}

fn parse_control_mode_line(session_name: &str, line: &str) -> Option<ParsedControlModeLine> {
    if let Some(value) = line.strip_prefix("%output ") {
        let (_, payload) = value.split_once(' ')?;
        let chunk = decode_control_mode_text(payload);
        if chunk.is_empty() {
            return None;
        }
        return Some(ParsedControlModeLine::Output(chunk));
    }

    if let Some(value) = line.strip_prefix("%extended-output ") {
        let (_, payload) = value.split_once(" : ")?;
        let chunk = decode_control_mode_text(payload);
        if chunk.is_empty() {
            return None;
        }
        return Some(ParsedControlModeLine::Output(chunk));
    }

    if let Some(value) = line.strip_prefix("%client-session-changed ") {
        let (_, name) = value.rsplit_once(' ')?;
        if name == session_name {
            return Some(ParsedControlModeLine::Connected);
        }
    }

    if let Some(value) = line.strip_prefix("%session-changed ") {
        let (_, name) = value.rsplit_once(' ')?;
        if name == session_name {
            return Some(ParsedControlModeLine::Connected);
        }
    }

    if let Some(reason) = line.strip_prefix("%exit") {
        return Some(ParsedControlModeLine::Disconnected(trimmed_control_reason(
            reason,
        )));
    }

    None
}

fn trimmed_control_reason(reason: &str) -> Option<String> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn active_pane_id_for_session(session_name: &str) -> std::io::Result<String> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "-t", session_name, "#{pane_id}"])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(std::io::Error::other(format!(
            "tmux display-message failed for '{session_name}': {stderr}"
        )));
    }

    let pane_id = String::from_utf8(output.stdout)
        .map_err(|error| {
            std::io::Error::other(format!("tmux pane id utf8 decode failed: {error}"))
        })?
        .trim()
        .to_string();
    if pane_id.is_empty() {
        return Err(std::io::Error::other(format!(
            "tmux display-message returned empty pane id for '{session_name}'"
        )));
    }
    Ok(pane_id)
}

fn build_control_mode_startup_commands(pane_id: &str) -> String {
    format!("refresh-client -A {pane_id}:on\n")
}

fn write_control_mode_startup_commands(
    mut stdin: ChildStdin,
    pane_id: &str,
) -> std::io::Result<ChildStdin> {
    let startup_commands = build_control_mode_startup_commands(pane_id);
    stdin.write_all(startup_commands.as_bytes())?;
    stdin.flush()?;
    Ok(stdin)
}

fn decode_control_mode_text(value: &str) -> String {
    let mut decoded = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }

        let Some(next) = chars.peek().copied() else {
            decoded.push('\\');
            break;
        };

        if next.is_ascii_digit() {
            let mut octal = String::new();
            for _ in 0..3 {
                let Some(digit) = chars.peek().copied() else {
                    break;
                };
                if !digit.is_ascii_digit() {
                    break;
                }
                octal.push(digit);
                let _ = chars.next();
            }
            if octal.len() == 3
                && let Ok(value) = u8::from_str_radix(octal.as_str(), 8)
            {
                decoded.push(char::from(value));
                continue;
            }
            decoded.push('\\');
            decoded.push_str(octal.as_str());
            continue;
        }

        let escape = chars.next().unwrap_or(next);
        let decoded_escape = match escape {
            'e' => '\u{1b}',
            'r' => '\r',
            'n' => '\n',
            't' => '\t',
            other => other,
        };
        decoded.push(decoded_escape);
    }
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn control_mode_startup_commands_enable_pane_output() {
        assert_eq!(
            build_control_mode_startup_commands("%180"),
            "refresh-client -A %180:on\n"
        );
    }

    #[test]
    fn control_mode_output_line_decodes_incremental_chunk_without_snapshot() {
        let msg = map_control_mode_line_to_msg(
            "grove-wt-grove-grove-agent-1",
            7,
            "%extended-output %180 0 : \\033[51;1H\\033[1mhi",
            |_| Err(std::io::Error::other("snapshot should not be called")),
        )
        .expect("output line should map to a message");

        assert_eq!(
            msg,
            Msg::PreviewStreamEvent(PreviewStreamEvent::Output(PreviewStreamOutput {
                session: "grove-wt-grove-grove-agent-1".to_string(),
                generation: 7,
                chunk: "\u{1b}[51;1H\u{1b}[1mhi".to_string(),
            }))
        );
    }

    #[test]
    fn startup_writer_keeps_stdin_open_for_long_lived_client() {
        let mut child = Command::new("sh")
            .args(["-c", "read line; while read next; do :; done"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("child should spawn");

        let stdin = child.stdin.take().expect("stdin should exist");
        let stdin = write_control_mode_startup_commands(stdin, "%180")
            .expect("startup commands should write");

        thread::sleep(StdDuration::from_millis(50));
        assert!(child.try_wait().expect("wait should succeed").is_none());

        drop(stdin);
        let _ = child.wait();
    }
}

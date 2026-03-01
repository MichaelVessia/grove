#[derive(Debug, Clone)]
struct ReplayTrace {
    bootstrap: ReplayBootstrapSnapshot,
    messages: Vec<ReplayTraceMessage>,
    states: HashMap<u64, ReplayStateSnapshot>,
    frame_hashes: HashMap<u64, VecDeque<u64>>,
}

#[derive(Debug, Clone)]
struct ReplayTraceMessage {
    seq: u64,
    msg: ReplayMsg,
}

#[derive(Debug, Deserialize)]
struct LoggedLine {
    event: String,
    kind: String,
    data: Value,
}

struct ReplayTmuxInput;

impl TmuxInput for ReplayTmuxInput {
    fn execute(&self, _command: &[String]) -> std::io::Result<()> {
        Ok(())
    }

    fn capture_output(
        &self,
        _target_session: &str,
        _scrollback_lines: usize,
        _include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        Ok(String::new())
    }

    fn capture_cursor_metadata(&self, _target_session: &str) -> std::io::Result<String> {
        Ok("0 0 0 0 0".to_string())
    }

    fn resize_session(
        &self,
        _target_session: &str,
        _target_width: u16,
        _target_height: u16,
    ) -> std::io::Result<()> {
        Ok(())
    }

    fn paste_buffer(&self, _target_session: &str, _text: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn supports_background_send(&self) -> bool {
        true
    }

    fn supports_background_poll(&self) -> bool {
        true
    }

    fn supports_background_launch(&self) -> bool {
        true
    }
}

#[derive(Default)]
struct ReplayClipboard {
    text: String,
}

impl ClipboardAccess for ReplayClipboard {
    fn read_text(&mut self) -> Result<String, String> {
        if self.text.is_empty() {
            return Err("clipboard empty".to_string());
        }

        Ok(self.text.clone())
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        self.text = text.to_string();
        Ok(())
    }
}

impl GroveApp {
    pub(super) fn replay_enabled(&self) -> bool {
        self.telemetry.debug_record_start_ts.is_some()
    }

    pub(super) fn record_replay_bootstrap(&self) {
        if !self.replay_enabled() {
            return;
        }

        let data = serde_json::to_value(ReplayBootstrapSnapshot::from_app(self));
        let Ok(snapshot) = data else {
            return;
        };

        self.telemetry.event_log.log(
            LogEvent::new("replay", "bootstrap")
                .with_data("schema_version", Value::from(REPLAY_SCHEMA_VERSION))
                .with_data("bootstrap", snapshot),
        );
    }

    pub(super) fn record_replay_msg_received(&mut self, msg: &Msg) -> u64 {
        if !self.replay_enabled() {
            return 0;
        }

        self.telemetry.replay_msg_seq_counter = self.telemetry.replay_msg_seq_counter.saturating_add(1);
        let seq = self.telemetry.replay_msg_seq_counter;

        let replay_msg = ReplayMsg::from_msg(msg);
        let Ok(encoded) = serde_json::to_value(replay_msg) else {
            return seq;
        };

        self.telemetry.event_log.log(
            LogEvent::new("replay", "msg_received")
                .with_data("schema_version", Value::from(REPLAY_SCHEMA_VERSION))
                .with_data("seq", Value::from(seq))
                .with_data("msg", encoded),
        );

        seq
    }

    pub(super) fn record_replay_state_after_update(&self, seq: u64) {
        if !self.replay_enabled() || seq == 0 {
            return;
        }

        let Ok(snapshot) = serde_json::to_value(ReplayStateSnapshot::from_app(self)) else {
            return;
        };

        self.telemetry.event_log.log(
            LogEvent::new("replay", "state_after_update")
                .with_data("schema_version", Value::from(REPLAY_SCHEMA_VERSION))
                .with_data("seq", Value::from(seq))
                .with_data("state", snapshot),
        );
    }
}

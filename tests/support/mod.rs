use std::path::PathBuf;
use std::time::{Duration, Instant};

#[allow(dead_code)]
pub mod render;

#[allow(dead_code)]
pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[allow(dead_code)]
pub fn read_fixture(name: &str) -> std::io::Result<String> {
    std::fs::read_to_string(fixture_path(name))
}

#[allow(dead_code)]
pub struct EventLogReader {
    path: PathBuf,
}

#[allow(dead_code)]
impl EventLogReader {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn read_events(&self) -> std::io::Result<Vec<serde_json::Value>> {
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return Ok(Vec::new());
        };

        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line).map_err(|error| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid event log line: {error}"),
                    )
                })
            })
            .collect()
    }

    pub fn wait_for(
        &self,
        event_kind: &str,
        timeout: Duration,
    ) -> std::io::Result<serde_json::Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let events = self.read_events()?;
            if let Some(found) = events.into_iter().find(|event| {
                event.get("kind").and_then(serde_json::Value::as_str) == Some(event_kind)
            }) {
                return Ok(found);
            }

            if Instant::now() >= deadline {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("event kind not found before timeout: {event_kind}"),
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn assert_sequence(&self, expected_kinds: &[&str]) -> std::io::Result<()> {
        let events = self.read_events()?;
        let mut expected_index = 0usize;

        for event in events {
            let kind = event
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if expected_index < expected_kinds.len() && kind == expected_kinds[expected_index] {
                expected_index = expected_index.saturating_add(1);
            }
        }

        if expected_index != expected_kinds.len() {
            return Err(std::io::Error::other(format!(
                "event sequence not found: expected {:?}",
                expected_kinds
            )));
        }

        Ok(())
    }
}

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub ts: u64,
    pub event: String,
    pub kind: String,
    pub data: Value,
}

impl Event {
    pub fn new(event: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            ts: now_millis(),
            event: event.into(),
            kind: kind.into(),
            data: Value::Object(Map::new()),
        }
    }

    pub fn with_data(mut self, key: impl Into<String>, value: Value) -> Self {
        if let Value::Object(data) = &mut self.data {
            data.insert(key.into(), value);
        }
        self
    }

    pub fn with_data_fields(mut self, fields: impl IntoIterator<Item = (String, Value)>) -> Self {
        if let Value::Object(data) = &mut self.data {
            for (key, value) in fields {
                data.insert(key, value);
            }
        }
        self
    }

    pub fn to_json_value(&self) -> Value {
        let mut object = Map::new();
        object.insert("ts".to_string(), Value::from(self.ts));
        object.insert("event".to_string(), Value::from(self.event.clone()));
        object.insert("kind".to_string(), Value::from(self.kind.clone()));
        object.insert("data".to_string(), self.data.clone());
        Value::Object(object)
    }
}

fn now_millis() -> u64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

pub trait EventLogger: Send {
    fn log(&self, event: Event);
}

pub struct NullEventLogger;

impl EventLogger for NullEventLogger {
    fn log(&self, _event: Event) {}
}

pub struct FileEventLogger {
    writer: Mutex<BufWriter<File>>,
}

impl FileEventLogger {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
        })
    }
}

impl EventLogger for FileEventLogger {
    fn log(&self, event: Event) {
        let Ok(mut writer) = self.writer.lock() else {
            return;
        };

        let Ok(line) = serde_json::to_string(&event.to_json_value()) else {
            return;
        };

        if writer.write_all(line.as_bytes()).is_err() {
            return;
        }
        if writer.write_all(b"\n").is_err() {
            return;
        }
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests;

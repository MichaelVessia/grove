use std::sync::{Arc, Mutex};

use crate::infrastructure::event_log::{Event as LoggedEvent, EventLogger};

pub(in crate::ui::tui::tests) type RecordedEvents = Arc<Mutex<Vec<LoggedEvent>>>;

pub(in crate::ui::tui::tests) struct RecordingEventLogger {
    pub(in crate::ui::tui::tests) events: RecordedEvents,
}

impl EventLogger for RecordingEventLogger {
    fn log(&self, event: LoggedEvent) {
        let Ok(mut events) = self.events.lock() else {
            return;
        };
        events.push(event);
    }
}

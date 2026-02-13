use ftui::core::event::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
use ftui::core::geometry::Rect;
use ftui::layout::{Constraint, Flex};
use ftui::render::frame::Frame;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::{App, Cmd, Model, ScreenMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Msg {
    Quit,
    Noop,
}

impl From<Event> for Msg {
    fn from(event: Event) -> Self {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                kind: KeyEventKind::Press,
                ..
            }) => Self::Quit,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }) if modifiers.contains(Modifiers::CTRL) => Self::Quit,
            _ => Self::Noop,
        }
    }
}

struct HelloApp;

impl HelloApp {
    const fn new() -> Self {
        Self
    }

    fn first_frame_lines(&self) -> [&'static str; 3] {
        [
            "Grove",
            "Phase 0.5 hello world, FrankenTUI booted.",
            "Press q or Ctrl+C to quit.",
        ]
    }
}

impl Model for HelloApp {
    type Message = Msg;

    fn update(&mut self, msg: Msg) -> Cmd<Self::Message> {
        match msg {
            Msg::Quit => Cmd::Quit,
            Msg::Noop => Cmd::None,
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Fixed(1),
                Constraint::Fixed(1),
            ])
            .split(area);

        let [title, subtitle, help] = self.first_frame_lines();
        Paragraph::new(title).render(rows[0], frame);
        Paragraph::new(subtitle).render(rows[1], frame);
        Paragraph::new(help).render(rows[2], frame);
    }
}

pub fn run() -> std::io::Result<()> {
    App::new(HelloApp::new())
        .screen_mode(ScreenMode::AltScreen)
        .run()
}

#[cfg(test)]
mod tests {
    use super::{HelloApp, Msg};
    use ftui::Cmd;
    use ftui::core::event::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};

    #[test]
    fn key_q_maps_to_quit() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press));
        assert_eq!(Msg::from(event), Msg::Quit);
    }

    #[test]
    fn ctrl_c_maps_to_quit() {
        let event = Event::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );
        assert_eq!(Msg::from(event), Msg::Quit);
    }

    #[test]
    fn non_quit_key_maps_to_noop() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press));
        assert_eq!(Msg::from(event), Msg::Noop);
    }

    #[test]
    fn quit_message_returns_quit_command() {
        let mut app = HelloApp::new();
        let cmd = ftui::Model::update(&mut app, Msg::Quit);
        assert!(matches!(cmd, Cmd::Quit));
    }

    #[test]
    fn first_frame_contains_quit_hint() {
        let app = HelloApp::new();
        let lines = app.first_frame_lines();
        assert_eq!(lines[0], "Grove");
        assert_eq!(lines[2], "Press q or Ctrl+C to quit.");
    }
}

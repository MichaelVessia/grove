use std::time::Instant;

use ftui::core::event::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::widgets::Widget;
use ftui::widgets::paragraph::Paragraph;
use ftui::{App, Cmd, Model, ScreenMode};

use crate::adapters::{
    BootstrapData, CommandGitAdapter, CommandSystemAdapter, CommandTmuxAdapter, DiscoveryState,
    bootstrap_data,
};
use crate::preview::PreviewState;
use crate::state::{Action, AppState, PaneFocus, UiMode, reduce};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Msg {
    Quit,
    Key(KeyEvent),
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
            Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                kind: KeyEventKind::Press,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Down,
                kind: KeyEventKind::Press,
                ..
            }) => Self::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
            Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                kind: KeyEventKind::Press,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Up,
                kind: KeyEventKind::Press,
                ..
            }) => Self::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
            Event::Key(key_event) => Self::Key(key_event),
            _ => Self::Noop,
        }
    }
}

struct GroveApp {
    repo_name: String,
    state: AppState,
    discovery_state: DiscoveryState,
    preview: PreviewState,
}

impl GroveApp {
    fn new() -> Self {
        let bootstrap = bootstrap_data(
            &CommandGitAdapter,
            &CommandTmuxAdapter,
            &CommandSystemAdapter,
        );
        Self::from_bootstrap(bootstrap)
    }

    fn from_bootstrap(bootstrap: BootstrapData) -> Self {
        let mut app = Self {
            repo_name: bootstrap.repo_name,
            state: AppState::new(bootstrap.workspaces),
            discovery_state: bootstrap.discovery_state,
            preview: PreviewState::new(),
        };
        app.refresh_preview();
        app
    }

    fn mode_label(&self) -> &'static str {
        match self.state.mode {
            UiMode::List => "List",
            UiMode::Preview => "Preview",
        }
    }

    fn focus_label(&self) -> &'static str {
        match self.state.focus {
            PaneFocus::WorkspaceList => "WorkspaceList",
            PaneFocus::Preview => "Preview",
        }
    }

    fn selected_status_hint(&self) -> &'static str {
        match self
            .state
            .selected_workspace()
            .map(|workspace| workspace.status)
        {
            Some(crate::domain::WorkspaceStatus::Main) => "main worktree",
            Some(crate::domain::WorkspaceStatus::Idle) => "idle",
            Some(crate::domain::WorkspaceStatus::Active) => "active",
            Some(crate::domain::WorkspaceStatus::Thinking) => "thinking",
            Some(crate::domain::WorkspaceStatus::Waiting) => "waiting",
            Some(crate::domain::WorkspaceStatus::Done) => "done",
            Some(crate::domain::WorkspaceStatus::Error) => "error",
            Some(crate::domain::WorkspaceStatus::Unsupported) => "unsupported",
            Some(crate::domain::WorkspaceStatus::Unknown) => "unknown",
            None => "none",
        }
    }

    fn status_bar_line(&self) -> String {
        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                format!("Status: discovery error ({message}) [q]quit")
            }
            DiscoveryState::Empty => "Status: no worktrees found [q]quit".to_string(),
            DiscoveryState::Ready => match self.state.mode {
                UiMode::List => format!(
                    "Status: [j/k]move [Tab]focus [Enter]preview [q]quit | selected={}",
                    self.selected_status_hint()
                ),
                UiMode::Preview => format!(
                    "Status: [j/k]scroll [PgUp/PgDn]scroll [G]bottom [Esc]list [Tab]focus [q]quit | autoscroll={} offset={}",
                    if self.preview.auto_scroll {
                        "on"
                    } else {
                        "off"
                    },
                    self.preview.offset
                ),
            },
        }
    }

    fn refresh_preview(&mut self) {
        let content = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "Workspace: {}\nBranch: {}\nPath: {}\nAgent: {}\nStatus: {}\nOrphaned session: {}",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display(),
                    workspace.agent.label(),
                    self.selected_status_hint(),
                    if workspace.is_orphaned { "yes" } else { "no" }
                )
            })
            .unwrap_or_else(|| "No workspace selected".to_string());

        self.preview.apply_capture(&content);
    }

    fn selected_preview_height(&self, total_height: usize) -> usize {
        let workspace_rows = match self.discovery_state {
            DiscoveryState::Ready => self.state.workspaces.len(),
            _ => 1,
        };
        let reserved = 3 + workspace_rows + 2 + 2;
        total_height.saturating_sub(reserved).max(1)
    }

    fn scroll_preview(&mut self, delta: i32) {
        let _ = self.preview.scroll(delta, Instant::now());
    }

    fn move_selection(&mut self, action: Action) {
        let before = self.state.selected_index;
        reduce(&mut self.state, action);
        if self.state.selected_index != before {
            self.preview.reset_for_selection_change();
            self.refresh_preview();
        }
    }

    fn handle_key(&mut self, key_event: KeyEvent) {
        if key_event.kind != KeyEventKind::Press {
            return;
        }

        match key_event.code {
            KeyCode::Tab => reduce(&mut self.state, Action::ToggleFocus),
            KeyCode::Enter => {
                reduce(&mut self.state, Action::EnterPreviewMode);
                self.refresh_preview();
            }
            KeyCode::Escape => reduce(&mut self.state, Action::EnterListMode),
            KeyCode::PageUp => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(-5);
                }
            }
            KeyCode::PageDown => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(5);
                }
            }
            KeyCode::Char('G') => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.preview.jump_to_bottom();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(1);
                } else {
                    self.move_selection(Action::MoveSelectionDown);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview {
                    self.scroll_preview(-1);
                } else {
                    self.move_selection(Action::MoveSelectionUp);
                }
            }
            _ => {}
        }
    }

    fn shell_lines(&self, preview_height: usize) -> Vec<String> {
        let mut lines = vec![
            format!("Grove Shell | Repo: {}", self.repo_name),
            format!(
                "Mode: {} | Focus: {}",
                self.mode_label(),
                self.focus_label()
            ),
            "Workspaces (j/k, arrows, Tab focus, Enter preview, Esc list)".to_string(),
        ];

        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                lines.push(format!("! discovery failed: {message}"));
            }
            DiscoveryState::Empty => {
                lines.push("No workspaces discovered".to_string());
            }
            DiscoveryState::Ready => {
                for (idx, workspace) in self.state.workspaces.iter().enumerate() {
                    let selected = if idx == self.state.selected_index {
                        ">"
                    } else {
                        " "
                    };
                    lines.push(format!(
                        "{} {} {} | {} | {}{}",
                        selected,
                        workspace.status.icon(),
                        workspace.name,
                        workspace.branch,
                        workspace.path.display(),
                        if workspace.is_orphaned {
                            " | session ended"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "{} ({}, {})",
                    workspace.name,
                    workspace.branch,
                    workspace.path.display()
                )
            })
            .unwrap_or_else(|| "none".to_string());

        lines.push(String::new());
        lines.push("Preview Pane".to_string());
        lines.push(format!("Selected workspace: {}", selected_workspace));
        let visible_lines = self.preview.visible_lines(preview_height);
        if visible_lines.is_empty() {
            lines.push("(no preview output)".to_string());
        } else {
            lines.extend(visible_lines);
        }
        lines.push(self.status_bar_line());

        lines
    }
}

impl Model for GroveApp {
    type Message = Msg;

    fn update(&mut self, msg: Msg) -> Cmd<Self::Message> {
        match msg {
            Msg::Quit => Cmd::Quit,
            Msg::Key(key_event) => {
                self.handle_key(key_event);
                Cmd::None
            }
            Msg::Noop => Cmd::None,
        }
    }

    fn view(&self, frame: &mut Frame) {
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let preview_height = self.selected_preview_height(usize::from(frame.buffer.height()));
        let content = self.shell_lines(preview_height).join("\n");
        Paragraph::new(content).render(area, frame);
    }
}

pub fn run() -> std::io::Result<()> {
    App::new(GroveApp::new())
        .screen_mode(ScreenMode::AltScreen)
        .run()
}

#[cfg(test)]
mod tests {
    use super::{GroveApp, Msg};
    use crate::adapters::{BootstrapData, DiscoveryState};
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};
    use ftui::Cmd;
    use ftui::core::event::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
    use std::path::PathBuf;

    fn fixture_app() -> GroveApp {
        GroveApp::from_bootstrap(BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: vec![
                Workspace::try_new(
                    "grove".to_string(),
                    PathBuf::from("/repos/grove"),
                    "main".to_string(),
                    Some(1_700_000_200),
                    AgentType::Claude,
                    WorkspaceStatus::Main,
                    true,
                )
                .expect("workspace should be valid"),
                Workspace::try_new(
                    "feature-a".to_string(),
                    PathBuf::from("/repos/grove-feature-a"),
                    "feature-a".to_string(),
                    Some(1_700_000_100),
                    AgentType::Codex,
                    WorkspaceStatus::Idle,
                    false,
                )
                .expect("workspace should be valid"),
            ],
            discovery_state: DiscoveryState::Ready,
            orphaned_sessions: Vec::new(),
        })
    }

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
    fn key_j_maps_to_key_message() {
        let event = Event::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press));
        assert_eq!(
            Msg::from(event),
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press))
        );
    }

    #[test]
    fn tab_maps_to_key_message() {
        let event = Event::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press));
        assert_eq!(
            Msg::from(event),
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press))
        );
    }

    #[test]
    fn key_message_updates_model_state() {
        let mut app = fixture_app();
        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
        );
        assert!(matches!(cmd, Cmd::None));
        assert_eq!(app.state.selected_index, 1);
    }

    #[test]
    fn shell_contains_list_preview_and_status_placeholders() {
        let app = fixture_app();
        let lines = app.shell_lines(8);
        let content = lines.join("\n");

        assert!(content.contains("Workspaces"));
        assert!(content.contains("Preview Pane"));
        assert!(content.contains("Status:"));
        assert!(content.contains("feature-a | feature-a | /repos/grove-feature-a"));
        assert!(content.contains("Workspace: grove"));
    }

    #[test]
    fn shell_renders_discovery_error_state() {
        let app = GroveApp::from_bootstrap(BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Error("fatal: not a git repository".to_string()),
            orphaned_sessions: Vec::new(),
        });
        let lines = app.shell_lines(8);
        let content = lines.join("\n");

        assert!(content.contains("discovery failed"));
        assert!(content.contains("discovery error"));
    }

    #[test]
    fn preview_mode_keys_scroll_and_jump_to_bottom() {
        let mut app = fixture_app();
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.state.mode, crate::state::UiMode::Preview);

        let was_auto_scroll = app.preview.auto_scroll;
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(was_auto_scroll, true);
        assert!(!app.preview.auto_scroll);
        assert!(app.preview.offset > 0);

        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('G')).with_kind(KeyEventKind::Press)),
        );
        assert_eq!(app.preview.offset, 0);
        assert!(app.preview.auto_scroll);
    }
}

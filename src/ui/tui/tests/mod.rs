mod render_support {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/render.rs"
    ));
}

use self::render_support::{
    assert_row_bg, assert_row_fg, find_cell_with_char, find_row_containing, row_text,
};
use super::{
    AppDependencies, AppPaths, ClipboardAccess, CommandZellijInput, CreateDialogField,
    CreateWorkspaceCompletion, CursorCapture, DeleteDialogField, GroveApp, HIT_ID_HEADER,
    HIT_ID_PREVIEW, HIT_ID_STATUS, HIT_ID_WORKSPACE_LIST, HIT_ID_WORKSPACE_ROW, LaunchDialogField,
    LaunchDialogState, LivePreviewCapture, Msg, PALETTE_CMD_FOCUS_LIST,
    PALETTE_CMD_MOVE_SELECTION_DOWN, PALETTE_CMD_OPEN_PREVIEW, PALETTE_CMD_SCROLL_DOWN,
    PALETTE_CMD_START_AGENT, PREVIEW_METADATA_ROWS, PendingResizeVerification,
    PreviewPollCompletion, PreviewTab, StartAgentCompletion, StopAgentCompletion,
    TextSelectionPoint, TmuxInput, WORKSPACE_ITEM_HEIGHT, WorkspaceStatusCapture, ansi_16_color,
    ansi_line_to_styled_line, parse_cursor_metadata, ui_theme,
};
use crate::adapters::{BootstrapData, DiscoveryState};
use crate::agent_runtime::workspace_status_targets_for_polling_with_live_preview;
use crate::config::{MultiplexerKind, ProjectConfig};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::event_log::{Event as LoggedEvent, EventLogger, NullEventLogger};
use crate::interactive::InteractiveState;
use crate::state::{PaneFocus, UiMode};
use crate::workspace_lifecycle::{BranchMode, CreateWorkspaceRequest, CreateWorkspaceResult};
use ftui::core::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
    PasteEvent,
};
use ftui::render::frame::HitId;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::toast::ToastStyle;
use ftui::{Cmd, Frame, GraphemePool};
use proptest::prelude::*;
use serde_json::Value;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type RecordedCommands = Rc<RefCell<Vec<Vec<String>>>>;
type RecordedCaptures = Rc<RefCell<Vec<Result<String, String>>>>;
type RecordedCalls = Rc<RefCell<Vec<String>>>;
type RecordedEvents = Arc<Mutex<Vec<LoggedEvent>>>;
type FixtureApp = (
    GroveApp,
    RecordedCommands,
    RecordedCaptures,
    RecordedCaptures,
);
type FixtureAppWithCalls = (
    GroveApp,
    RecordedCommands,
    RecordedCaptures,
    RecordedCaptures,
    RecordedCalls,
);
type FixtureAppWithEvents = (
    GroveApp,
    RecordedCommands,
    RecordedCaptures,
    RecordedCaptures,
    RecordedEvents,
);

struct RecordingEventLogger {
    events: RecordedEvents,
}

impl EventLogger for RecordingEventLogger {
    fn log(&self, event: LoggedEvent) {
        let Ok(mut events) = self.events.lock() else {
            return;
        };
        events.push(event);
    }
}

#[derive(Clone)]
struct RecordingTmuxInput {
    commands: RecordedCommands,
    captures: RecordedCaptures,
    cursor_captures: RecordedCaptures,
    calls: RecordedCalls,
}

#[derive(Clone, Default)]
struct RecordingClipboard {
    text: Rc<RefCell<String>>,
}

impl ClipboardAccess for RecordingClipboard {
    fn read_text(&mut self) -> Result<String, String> {
        Ok(self.text.borrow().clone())
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        self.text.replace(text.to_string());
        Ok(())
    }
}

fn test_clipboard() -> Box<dyn ClipboardAccess> {
    Box::new(RecordingClipboard::default())
}

impl TmuxInput for RecordingTmuxInput {
    fn execute(&self, command: &[String]) -> std::io::Result<()> {
        self.commands.borrow_mut().push(command.to_vec());
        self.calls
            .borrow_mut()
            .push(format!("exec:{}", command.join(" ")));
        Ok(())
    }

    fn capture_output(
        &self,
        target_session: &str,
        scrollback_lines: usize,
        include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        self.calls.borrow_mut().push(format!(
            "capture:{target_session}:{scrollback_lines}:{include_escape_sequences}"
        ));
        let mut captures = self.captures.borrow_mut();
        if captures.is_empty() {
            return Ok(String::new());
        }

        let next = captures.remove(0);
        match next {
            Ok(output) => Ok(output),
            Err(error) => Err(std::io::Error::other(error)),
        }
    }

    fn capture_cursor_metadata(&self, target_session: &str) -> std::io::Result<String> {
        self.calls
            .borrow_mut()
            .push(format!("cursor:{target_session}"));
        let mut captures = self.cursor_captures.borrow_mut();
        if captures.is_empty() {
            return Ok("1 0 0 120 40".to_string());
        }

        let next = captures.remove(0);
        match next {
            Ok(output) => Ok(output),
            Err(error) => Err(std::io::Error::other(error)),
        }
    }

    fn resize_session(
        &self,
        target_session: &str,
        target_width: u16,
        target_height: u16,
    ) -> std::io::Result<()> {
        self.calls.borrow_mut().push(format!(
            "resize:{target_session}:{target_width}:{target_height}"
        ));
        Ok(())
    }

    fn paste_buffer(&self, target_session: &str, text: &str) -> std::io::Result<()> {
        self.calls.borrow_mut().push(format!(
            "paste-buffer:{target_session}:{}",
            text.chars().count()
        ));
        self.commands.borrow_mut().push(vec![
            "tmux".to_string(),
            "paste-buffer".to_string(),
            "-t".to_string(),
            target_session.to_string(),
            text.to_string(),
        ]);
        Ok(())
    }
}

#[derive(Clone)]
struct BackgroundOnlyTmuxInput;

impl TmuxInput for BackgroundOnlyTmuxInput {
    fn execute(&self, _command: &[String]) -> std::io::Result<()> {
        Ok(())
    }

    fn capture_output(
        &self,
        _target_session: &str,
        _scrollback_lines: usize,
        _include_escape_sequences: bool,
    ) -> std::io::Result<String> {
        panic!("sync preview capture should not run when background mode is enabled")
    }

    fn capture_cursor_metadata(&self, _target_session: &str) -> std::io::Result<String> {
        panic!("sync cursor capture should not run when background mode is enabled")
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
}

fn fixture_bootstrap(status: WorkspaceStatus) -> BootstrapData {
    let mut main_workspace = Workspace::try_new(
        "grove".to_string(),
        PathBuf::from("/repos/grove"),
        "main".to_string(),
        Some(1_700_000_200),
        AgentType::Claude,
        WorkspaceStatus::Main,
        true,
    )
    .expect("workspace should be valid");
    main_workspace.project_path = Some(PathBuf::from("/repos/grove"));

    let mut feature_workspace = Workspace::try_new(
        "feature-a".to_string(),
        PathBuf::from("/repos/grove-feature-a"),
        "feature-a".to_string(),
        Some(1_700_000_100),
        AgentType::Codex,
        status,
        false,
    )
    .expect("workspace should be valid");
    feature_workspace.project_path = Some(PathBuf::from("/repos/grove"));

    BootstrapData {
        repo_name: "grove".to_string(),
        workspaces: vec![main_workspace, feature_workspace],
        discovery_state: DiscoveryState::Ready,
        orphaned_sessions: Vec::new(),
    }
}

fn fixture_projects() -> Vec<ProjectConfig> {
    vec![ProjectConfig {
        name: "grove".to_string(),
        path: PathBuf::from("/repos/grove"),
    }]
}

fn fixture_app() -> GroveApp {
    let sidebar_ratio_path = unique_sidebar_ratio_path("fixture");
    let config_path = unique_config_path("fixture");
    GroveApp::from_parts_with_clipboard_and_projects(
        fixture_bootstrap(WorkspaceStatus::Idle),
        fixture_projects(),
        AppDependencies {
            tmux_input: Box::new(RecordingTmuxInput {
                commands: Rc::new(RefCell::new(Vec::new())),
                captures: Rc::new(RefCell::new(Vec::new())),
                cursor_captures: Rc::new(RefCell::new(Vec::new())),
                calls: Rc::new(RefCell::new(Vec::new())),
            }),
            clipboard: test_clipboard(),
            paths: AppPaths::new(sidebar_ratio_path, config_path),
            multiplexer: MultiplexerKind::Tmux,
            event_log: Box::new(NullEventLogger),
            debug_record_start_ts: None,
        },
    )
}

fn event_kinds(events: &RecordedEvents) -> Vec<String> {
    let Ok(events) = events.lock() else {
        return Vec::new();
    };
    events.iter().map(|event| event.kind.clone()).collect()
}

fn recorded_events(events: &RecordedEvents) -> Vec<LoggedEvent> {
    let Ok(events) = events.lock() else {
        return Vec::new();
    };
    events.clone()
}

fn clear_recorded_events(events: &RecordedEvents) {
    let Ok(mut events) = events.lock() else {
        return;
    };
    events.clear();
}

fn assert_kind_subsequence(actual: &[String], expected: &[&str]) {
    let mut expected_index = 0usize;
    for kind in actual {
        if expected_index < expected.len() && kind == expected[expected_index] {
            expected_index = expected_index.saturating_add(1);
        }
    }
    assert_eq!(
        expected_index,
        expected.len(),
        "expected subsequence {:?} in {:?}",
        expected,
        actual
    );
}

fn key_press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code).with_kind(KeyEventKind::Press)
}

fn focus_agent_preview_tab(app: &mut GroveApp) {
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Agent;
}

fn force_tick_due(app: &mut GroveApp) {
    let now = Instant::now();
    app.next_tick_due_at = Some(now);
    app.next_poll_due_at = Some(now);
}

fn cmd_contains_task(cmd: &Cmd<Msg>) -> bool {
    match cmd {
        Cmd::Task(_, _) => true,
        Cmd::Batch(commands) | Cmd::Sequence(commands) => commands.iter().any(cmd_contains_task),
        _ => false,
    }
}

fn arb_key_event() -> impl Strategy<Value = KeyEvent> {
    proptest::prop_oneof![
        Just(key_press(KeyCode::Char('j'))),
        Just(key_press(KeyCode::Char('k'))),
        Just(key_press(KeyCode::Char('s'))),
        Just(key_press(KeyCode::Char('x'))),
        Just(key_press(KeyCode::Char('n'))),
        Just(key_press(KeyCode::Char('!'))),
        Just(key_press(KeyCode::Char('q'))),
        Just(key_press(KeyCode::Char('G'))),
        Just(key_press(KeyCode::Tab)),
        Just(key_press(KeyCode::Enter)),
        Just(key_press(KeyCode::Escape)),
        Just(key_press(KeyCode::Up)),
        Just(key_press(KeyCode::Down)),
        Just(key_press(KeyCode::PageUp)),
        Just(key_press(KeyCode::PageDown)),
        proptest::char::range('a', 'z').prop_map(|ch| key_press(KeyCode::Char(ch))),
    ]
}

fn arb_msg() -> impl Strategy<Value = Msg> {
    proptest::prop_oneof![
        arb_key_event().prop_map(Msg::Key),
        Just(Msg::Tick),
        Just(Msg::Noop),
        (1u16..200, 1u16..60).prop_map(|(width, height)| Msg::Resize { width, height }),
    ]
}

fn unique_sidebar_ratio_path(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-sidebar-width-{label}-{}-{timestamp}.txt",
        std::process::id()
    ))
}

fn unique_config_path(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-config-{label}-{}-{timestamp}.toml",
        std::process::id()
    ))
}

fn unique_temp_workspace_dir(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "grove-test-workspace-{label}-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temp workspace directory should exist");
    path
}

fn fixture_app_with_tmux(
    status: WorkspaceStatus,
    captures: Vec<Result<String, String>>,
) -> FixtureApp {
    fixture_app_with_tmux_and_sidebar_path(
        status,
        captures,
        Vec::new(),
        unique_sidebar_ratio_path("fixture-with-tmux"),
    )
}

fn fixture_app_with_tmux_and_sidebar_path(
    status: WorkspaceStatus,
    captures: Vec<Result<String, String>>,
    cursor_captures: Vec<Result<String, String>>,
    sidebar_ratio_path: PathBuf,
) -> FixtureApp {
    let commands = Rc::new(RefCell::new(Vec::new()));
    let captures = Rc::new(RefCell::new(captures));
    let cursor_captures = Rc::new(RefCell::new(cursor_captures));
    let tmux = RecordingTmuxInput {
        commands: commands.clone(),
        captures: captures.clone(),
        cursor_captures: cursor_captures.clone(),
        calls: Rc::new(RefCell::new(Vec::new())),
    };
    (
        GroveApp::from_parts_with_clipboard_and_projects(
            fixture_bootstrap(status),
            fixture_projects(),
            AppDependencies {
                tmux_input: Box::new(tmux),
                clipboard: test_clipboard(),
                paths: AppPaths::new(sidebar_ratio_path, unique_config_path("fixture-with-tmux")),
                multiplexer: MultiplexerKind::Tmux,
                event_log: Box::new(NullEventLogger),
                debug_record_start_ts: None,
            },
        ),
        commands,
        captures,
        cursor_captures,
    )
}

fn fixture_app_with_tmux_and_calls(
    status: WorkspaceStatus,
    captures: Vec<Result<String, String>>,
    cursor_captures: Vec<Result<String, String>>,
) -> FixtureAppWithCalls {
    let sidebar_ratio_path = unique_sidebar_ratio_path("fixture-with-calls");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let captures = Rc::new(RefCell::new(captures));
    let cursor_captures = Rc::new(RefCell::new(cursor_captures));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let tmux = RecordingTmuxInput {
        commands: commands.clone(),
        captures: captures.clone(),
        cursor_captures: cursor_captures.clone(),
        calls: calls.clone(),
    };

    (
        GroveApp::from_parts_with_clipboard_and_projects(
            fixture_bootstrap(status),
            fixture_projects(),
            AppDependencies {
                tmux_input: Box::new(tmux),
                clipboard: test_clipboard(),
                paths: AppPaths::new(sidebar_ratio_path, unique_config_path("fixture-with-calls")),
                multiplexer: MultiplexerKind::Tmux,
                event_log: Box::new(NullEventLogger),
                debug_record_start_ts: None,
            },
        ),
        commands,
        captures,
        cursor_captures,
        calls,
    )
}

fn fixture_app_with_tmux_and_events(
    status: WorkspaceStatus,
    captures: Vec<Result<String, String>>,
    cursor_captures: Vec<Result<String, String>>,
) -> FixtureAppWithEvents {
    let sidebar_ratio_path = unique_sidebar_ratio_path("fixture-with-events");
    let commands = Rc::new(RefCell::new(Vec::new()));
    let captures = Rc::new(RefCell::new(captures));
    let cursor_captures = Rc::new(RefCell::new(cursor_captures));
    let events = Arc::new(Mutex::new(Vec::new()));
    let tmux = RecordingTmuxInput {
        commands: commands.clone(),
        captures: captures.clone(),
        cursor_captures: cursor_captures.clone(),
        calls: Rc::new(RefCell::new(Vec::new())),
    };
    let event_log = RecordingEventLogger {
        events: events.clone(),
    };

    (
        GroveApp::from_parts_with_clipboard_and_projects(
            fixture_bootstrap(status),
            fixture_projects(),
            AppDependencies {
                tmux_input: Box::new(tmux),
                clipboard: test_clipboard(),
                paths: AppPaths::new(
                    sidebar_ratio_path,
                    unique_config_path("fixture-with-events"),
                ),
                multiplexer: MultiplexerKind::Tmux,
                event_log: Box::new(event_log),
                debug_record_start_ts: None,
            },
        ),
        commands,
        captures,
        cursor_captures,
        events,
    )
}

fn fixture_background_app(status: WorkspaceStatus) -> GroveApp {
    GroveApp::from_parts_with_clipboard_and_projects(
        fixture_bootstrap(status),
        fixture_projects(),
        AppDependencies {
            tmux_input: Box::new(BackgroundOnlyTmuxInput),
            clipboard: test_clipboard(),
            paths: AppPaths::new(
                unique_sidebar_ratio_path("background"),
                unique_config_path("background"),
            ),
            multiplexer: MultiplexerKind::Tmux,
            event_log: Box::new(NullEventLogger),
            debug_record_start_ts: None,
        },
    )
}

fn with_rendered_frame(app: &GroveApp, width: u16, height: u16, assert_frame: impl FnOnce(&Frame)) {
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(width, height, &mut pool);
    ftui::Model::view(app, &mut frame);
    assert_frame(&frame);
}

proptest::proptest! {
    #[test]
    fn no_panic_on_random_messages(msgs in prop::collection::vec(arb_msg(), 1..200)) {
        let mut app = fixture_app();
        for msg in msgs {
            let _ = ftui::Model::update(&mut app, msg);
        }
    }

    #[test]
    fn selection_always_in_bounds(msgs in prop::collection::vec(arb_msg(), 1..200)) {
        let mut app = fixture_app();
        for msg in msgs {
            let _ = ftui::Model::update(&mut app, msg);
            if !app.state.workspaces.is_empty() {
                prop_assert!(app.state.selected_index < app.state.workspaces.len());
            }
        }
    }

    #[test]
    fn modal_exclusivity(msgs in prop::collection::vec(arb_msg(), 1..200)) {
        let mut app = fixture_app();
        for msg in msgs {
            let _ = ftui::Model::update(&mut app, msg);
            let active_modals = [
                app.launch_dialog.is_some(),
                app.create_dialog.is_some(),
                app.delete_dialog.is_some(),
                app.keybind_help_open,
                app.command_palette.is_visible(),
                app.interactive.is_some(),
            ]
                .iter()
                .filter(|is_active| **is_active)
                .count();
            prop_assert!(active_modals <= 1);
        }
    }

    #[test]
    fn scroll_offset_in_bounds(msgs in prop::collection::vec(arb_msg(), 1..200)) {
        let mut app = fixture_app();
        for msg in msgs {
            let _ = ftui::Model::update(&mut app, msg);
            prop_assert!(app.preview.offset <= app.preview.lines.len());
        }
    }

    #[test]
    fn view_never_panics(
        msgs in prop::collection::vec(arb_msg(), 0..100),
        width in 20u16..200,
        height in 5u16..60,
    ) {
        let mut app = fixture_app();
        for msg in msgs {
            let _ = ftui::Model::update(&mut app, msg);
        }

        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(width, height, &mut pool);
        ftui::Model::view(&app, &mut frame);
    }

    #[test]
    fn view_fills_status_bar_row(msgs in prop::collection::vec(arb_msg(), 0..50)) {
        let mut app = fixture_app();
        for msg in msgs {
            let _ = ftui::Model::update(&mut app, msg);
        }

        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(80, 24, &mut pool);
        ftui::Model::view(&app, &mut frame);

        let status_row = frame.height().saturating_sub(1);
        let status = row_text(&frame, status_row, 0, frame.width());
        prop_assert!(!status.is_empty(), "status bar should not be blank");
    }
}

#[test]
fn sidebar_shows_workspace_names() {
    let app = fixture_app();
    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        assert!(find_row_containing(frame, "grove", x_start, x_end).is_some());
        assert!(find_row_containing(frame, "feature-a", x_start, x_end).is_some());
    });
}

#[test]
fn workspace_age_renders_in_preview_header_not_sidebar_row() {
    let mut app = fixture_app();
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_secs();
    let last_activity =
        i64::try_from(now_secs.saturating_sub(17 * 60)).expect("timestamp should fit i64");
    app.state.workspaces[0].last_activity_unix_secs = Some(last_activity);
    app.state.selected_index = 0;
    let expected_age = app.relative_age_label(app.state.workspaces[0].last_activity_unix_secs);

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let sidebar_x_start = layout.sidebar.x.saturating_add(1);
    let sidebar_x_end = layout.sidebar.right().saturating_sub(1);
    let preview_x_start = layout.preview.x.saturating_add(1);
    let preview_x_end = layout.preview.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(sidebar_row) =
            find_row_containing(frame, "▸ base", sidebar_x_start, sidebar_x_end)
        else {
            panic!("sidebar workspace row should be rendered");
        };
        let sidebar_text = row_text(frame, sidebar_row, sidebar_x_start, sidebar_x_end);
        assert!(
            !sidebar_text.contains(expected_age.as_str()),
            "sidebar row should not include age label, got: {sidebar_text}"
        );

        let Some(preview_row) = find_row_containing(
            frame,
            "base · main · Claude",
            preview_x_start,
            preview_x_end,
        ) else {
            panic!("preview header row should be rendered");
        };
        let preview_text = row_text(frame, preview_row, preview_x_start, preview_x_end);
        assert!(
            preview_text.contains(expected_age.as_str()),
            "preview header should include age label, got: {preview_text}"
        );
    });
}

#[test]
fn selected_workspace_row_has_selection_marker() {
    let mut app = fixture_app();
    app.state.selected_index = 1;

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let rendered_row = row_text(frame, selected_row, x_start, x_end);
        assert!(
            rendered_row.starts_with("▸ "),
            "selected row should start with selection marker, got: {rendered_row}"
        );
    });
}

#[test]
fn sidebar_row_omits_duplicate_workspace_and_branch_text() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("feature row should be rendered");
        };
        let row_text = row_text(frame, row, x_start, x_end);
        assert!(
            !row_text.contains("feature-a · feature-a"),
            "row should not duplicate workspace and branch when they match, got: {row_text}"
        );
        assert!(
            row_text.contains("feature-a · Codex"),
            "row should include workspace and agent labels, got: {row_text}"
        );
    });
}

#[test]
fn shell_lines_show_workspace_and_agent_labels_without_status_badges() {
    let app = fixture_app();
    let lines = app.shell_lines(12);
    let Some(base_line) = lines.iter().find(|line| line.contains("base | main")) else {
        panic!("base workspace shell line should be present");
    };
    let Some(feature_line) = lines
        .iter()
        .find(|line| line.contains("feature-a | feature-a"))
    else {
        panic!("feature workspace shell line should be present");
    };
    assert!(
        !base_line.contains("["),
        "base workspace should not show status badge, got: {base_line}"
    );
    assert!(
        !feature_line.contains("["),
        "feature workspace should not show status badge, got: {feature_line}"
    );
    assert!(
        base_line.contains("Claude"),
        "base workspace should include Claude label, got: {base_line}"
    );
    assert!(
        feature_line.contains("Codex"),
        "feature workspace should include Codex label, got: {feature_line}"
    );
}

#[test]
fn active_workspace_without_recent_activity_uses_static_indicators() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.output_changing = false;
    app.agent_output_changing = false;
    assert!(!app.status_is_visually_working(
        Some(app.state.workspaces[1].path.as_path()),
        WorkspaceStatus::Active,
        true
    ));

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(
            !sidebar_row_text.contains("["),
            "active workspace should not show status badge when not changing, got: {sidebar_row_text}"
        );
        assert!(!sidebar_row_text.contains("run."));

        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(!status_text.contains("run."));
    });
}

#[test]
fn active_workspace_with_recent_activity_window_animates_indicators() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.output_changing = false;
    app.agent_output_changing = false;
    app.push_agent_activity_frame(true);
    assert!(app.status_is_visually_working(
        Some(app.state.workspaces[1].path.as_path()),
        WorkspaceStatus::Active,
        true
    ));

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(!sidebar_row_text.contains("run."));
    });
}

#[test]
fn active_workspace_with_recent_activity_animates_indicators() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.output_changing = true;
    app.agent_output_changing = true;
    assert!(app.status_is_visually_working(
        Some(app.state.workspaces[1].path.as_path()),
        WorkspaceStatus::Active,
        true
    ));

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(!sidebar_row_text.contains("run."));
    });
}

#[test]
fn active_workspace_activity_window_expires_after_inactive_frames() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.output_changing = false;
    app.agent_output_changing = false;
    app.push_agent_activity_frame(true);
    for _ in 0..super::AGENT_ACTIVITY_WINDOW_FRAMES {
        app.push_agent_activity_frame(false);
    }
    assert!(!app.status_is_visually_working(
        Some(app.state.workspaces[1].path.as_path()),
        WorkspaceStatus::Active,
        true
    ));

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(!sidebar_row_text.contains("run."));
    });
}

#[test]
fn waiting_workspace_row_has_no_status_badge_or_input_banner() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Waiting, Vec::new());
    app.state.selected_index = 1;
    app.sidebar_width_pct = 70;

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(
            !sidebar_row_text.contains("["),
            "waiting workspace should not show status badge, got: {sidebar_row_text}"
        );
    });
}

#[test]
fn activity_spinner_does_not_shift_header_or_status_layout() {
    let (mut idle_app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    idle_app.state.selected_index = 1;
    idle_app.output_changing = false;
    idle_app.agent_output_changing = false;

    let (mut active_app, _commands2, _captures2, _cursor_captures2) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    active_app.state.selected_index = 1;
    active_app.output_changing = true;
    active_app.agent_output_changing = true;

    with_rendered_frame(&idle_app, 80, 24, |idle_frame| {
        with_rendered_frame(&active_app, 80, 24, |active_frame| {
            let idle_header = row_text(idle_frame, 0, 0, idle_frame.width());
            let active_header = row_text(active_frame, 0, 0, active_frame.width());
            assert_eq!(
                idle_header, active_header,
                "header layout should remain stable when spinner state changes"
            );

            let idle_status_row = idle_frame.height().saturating_sub(1);
            let active_status_row = active_frame.height().saturating_sub(1);
            let idle_status = row_text(idle_frame, idle_status_row, 0, idle_frame.width());
            let active_status = row_text(active_frame, active_status_row, 0, active_frame.width());
            assert_eq!(
                idle_status, active_status,
                "status keybind hints should remain stable when spinner state changes"
            );
        });
    });
}

#[test]
fn interactive_input_echo_does_not_trigger_activity_spinner() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.output_changing = true;
    app.agent_output_changing = false;
    assert!(!app.status_is_visually_working(
        Some(app.state.workspaces[1].path.as_path()),
        WorkspaceStatus::Active,
        true
    ));

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(!sidebar_row_text.contains("run."));

        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(
            status_text.contains("j/k move, h/l pane, Enter open"),
            "status row should show keybind hints, got: {status_text}"
        );
    });
}

#[test]
fn modal_dialog_renders_over_sidebar() {
    let mut app = fixture_app();
    app.launch_dialog = Some(LaunchDialogState {
        prompt: String::new(),
        pre_launch_command: String::new(),
        skip_permissions: false,
        focused_field: LaunchDialogField::Prompt,
    });

    with_rendered_frame(&app, 80, 24, |frame| {
        assert!(find_row_containing(frame, "Start Agent", 0, frame.width()).is_some());
    });
}

#[test]
fn launch_dialog_uses_opaque_background_fill() {
    let mut app = fixture_app();
    app.launch_dialog = Some(LaunchDialogState {
        prompt: String::new(),
        pre_launch_command: String::new(),
        skip_permissions: false,
        focused_field: LaunchDialogField::Prompt,
    });

    with_rendered_frame(&app, 80, 24, |frame| {
        let dialog_width = frame.width().saturating_sub(8).min(100);
        let dialog_height = 11u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let probe_x = dialog_x.saturating_add(dialog_width.saturating_sub(3));
        let probe_y = dialog_y.saturating_add(4);
        let Some(cell) = frame.buffer.get(probe_x, probe_y) else {
            panic!("expected dialog probe cell at ({probe_x},{probe_y})");
        };
        assert_eq!(cell.bg, ui_theme().base);
    });
}

#[test]
fn create_dialog_uses_opaque_background_fill() {
    let mut app = fixture_app();
    app.open_create_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let dialog_width = frame.width().saturating_sub(8).min(90);
        let dialog_height = 14u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let probe_x = dialog_x.saturating_add(dialog_width.saturating_sub(3));
        let probe_y = dialog_y.saturating_add(4);
        let Some(cell) = frame.buffer.get(probe_x, probe_y) else {
            panic!("expected dialog probe cell at ({probe_x},{probe_y})");
        };
        assert_eq!(cell.bg, ui_theme().base);
    });
}

#[test]
fn create_dialog_selected_agent_row_uses_highlight_background() {
    let mut app = fixture_app();
    app.open_create_dialog();
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );

    with_rendered_frame(&app, 80, 24, |frame| {
        let dialog_width = frame.width().saturating_sub(8).min(90);
        let dialog_height = 14u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let x_start = dialog_x.saturating_add(1);
        let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
        let y_start = dialog_y.saturating_add(1);
        let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));
        let find_dialog_row = |needle: &str| {
            (y_start..y_end).find(|&row| row_text(frame, row, x_start, x_end).contains(needle))
        };

        let Some(selected_row) = find_dialog_row("Claude") else {
            panic!("selected agent row should be rendered");
        };
        assert_row_bg(frame, selected_row, x_start, x_end, ui_theme().surface1);

        let Some(unselected_row) = find_dialog_row("Codex") else {
            panic!("unselected agent row should be rendered");
        };
        assert_row_bg(frame, unselected_row, x_start, x_end, ui_theme().base);

        let Some(cell) = frame.buffer.get(x_start, dialog_y.saturating_add(1)) else {
            panic!(
                "expected dialog cell at ({x_start},{})",
                dialog_y.saturating_add(1)
            );
        };
        assert_eq!(cell.bg, ui_theme().base);
    });
}

#[test]
fn create_dialog_unfocused_agent_row_uses_base_background() {
    let mut app = fixture_app();
    app.open_create_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let dialog_width = frame.width().saturating_sub(8).min(90);
        let dialog_height = 14u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let x_start = dialog_x.saturating_add(1);
        let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
        let y_start = dialog_y.saturating_add(1);
        let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));
        let find_dialog_row = |needle: &str| {
            (y_start..y_end).find(|&row| row_text(frame, row, x_start, x_end).contains(needle))
        };

        let Some(name_row) = find_dialog_row("[Name]") else {
            panic!("name row should be rendered");
        };
        assert_row_bg(frame, name_row, x_start, x_end, ui_theme().surface1);

        let Some(selected_agent_row) = find_dialog_row("Claude") else {
            panic!("selected agent row should be rendered");
        };
        assert_row_bg(frame, selected_agent_row, x_start, x_end, ui_theme().base);
    });
}

#[test]
fn create_dialog_renders_action_buttons() {
    let mut app = fixture_app();
    app.open_create_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let dialog_width = frame.width().saturating_sub(8).min(90);
        let dialog_height = 14u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let x_start = dialog_x.saturating_add(1);
        let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
        let y_start = dialog_y.saturating_add(1);
        let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));

        let has_buttons = (y_start..y_end).any(|row| {
            let text = row_text(frame, row, x_start, x_end);
            text.contains("Create") && text.contains("Cancel")
        });
        assert!(
            has_buttons,
            "create dialog action buttons should be visible"
        );
    });
}

#[test]
fn status_row_shows_keybind_hints_not_toast_state() {
    let mut app = fixture_app();
    app.show_toast("Agent started", false);

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(!status_text.contains("Agent started"));
        assert!(status_text.contains("j/k move, h/l pane, Enter open"));
    });
}

#[test]
fn status_row_shows_start_hint_in_preview_mode() {
    let mut app = fixture_app();
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Agent;

    with_rendered_frame(&app, 180, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("s start"));
        assert!(status_text.contains("x stop"));
        assert!(status_text.contains("D delete"));
    });
}

#[test]
fn status_row_hides_agent_hints_in_git_tab() {
    let mut app = fixture_app();
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Git;

    with_rendered_frame(&app, 180, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(!status_text.contains("s start"));
        assert!(!status_text.contains("x stop"));
        assert!(!status_text.contains("j/k scroll"));
        assert!(status_text.contains("Enter attach lazygit"));
    });
}

#[test]
fn question_key_opens_keybind_help_modal() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('?')).with_kind(KeyEventKind::Press));

    assert!(app.keybind_help_open);
}

#[test]
fn keybind_help_modal_closes_on_escape() {
    let mut app = fixture_app();
    app.keybind_help_open = true;

    let _ = app.handle_key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press));

    assert!(!app.keybind_help_open);
}

#[test]
fn keybind_help_modal_blocks_navigation_keys() {
    let mut app = fixture_app();
    app.keybind_help_open = true;
    let selected_before = app.state.selected_index;

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press));

    assert_eq!(app.state.selected_index, selected_before);
}

#[test]
fn ctrl_k_opens_command_palette() {
    let mut app = fixture_app();

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('k'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );

    assert!(app.command_palette.is_visible());
}

#[test]
fn ctrl_k_is_blocked_while_modal_is_open() {
    let mut app = fixture_app();
    app.open_create_dialog();
    assert!(app.create_dialog.is_some());

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('k'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );

    assert!(app.create_dialog.is_some());
    assert!(!app.command_palette.is_visible());
}

#[test]
fn ctrl_k_is_blocked_in_interactive_mode() {
    let mut app = fixture_app();
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        24,
        80,
    ));

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('k'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );

    assert!(app.interactive.is_some());
    assert!(!app.command_palette.is_visible());
}

#[test]
fn command_palette_blocks_background_navigation_keys() {
    let mut app = fixture_app();
    let selected_before = app.state.selected_index;

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('k'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );
    assert!(app.command_palette.is_visible());

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press));

    assert_eq!(app.state.selected_index, selected_before);
    assert_eq!(app.command_palette.query(), "j");
}

#[test]
fn command_palette_enter_executes_selected_action() {
    let mut app = fixture_app();

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('k'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );
    assert!(app.command_palette.is_visible());

    for character in ['n', 'e', 'w'] {
        let _ =
            app.handle_key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press));
    }
    let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

    assert!(!app.command_palette.is_visible());
    assert!(app.create_dialog.is_some());
}

#[test]
fn command_palette_action_set_scopes_to_focus_and_mode() {
    let mut app = fixture_app();
    let list_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(
        list_ids
            .iter()
            .any(|id| id == PALETTE_CMD_MOVE_SELECTION_DOWN)
    );
    assert!(list_ids.iter().any(|id| id == PALETTE_CMD_OPEN_PREVIEW));
    assert!(!list_ids.iter().any(|id| id == PALETTE_CMD_SCROLL_DOWN));
    assert!(!list_ids.iter().any(|id| id == PALETTE_CMD_START_AGENT));

    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Agent;
    let preview_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(preview_ids.iter().any(|id| id == PALETTE_CMD_SCROLL_DOWN));
    assert!(preview_ids.iter().any(|id| id == PALETTE_CMD_FOCUS_LIST));
    assert!(preview_ids.iter().any(|id| id == PALETTE_CMD_START_AGENT));
    assert!(
        !preview_ids
            .iter()
            .any(|id| id == PALETTE_CMD_MOVE_SELECTION_DOWN)
    );

    app.preview_tab = PreviewTab::Git;
    let git_preview_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(
        !git_preview_ids
            .iter()
            .any(|id| id == PALETTE_CMD_SCROLL_DOWN)
    );
    assert!(
        !git_preview_ids
            .iter()
            .any(|id| id == PALETTE_CMD_START_AGENT)
    );
}

#[test]
fn uppercase_s_opens_settings_dialog() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));

    assert!(app.settings_dialog.is_some());
}

#[test]
fn settings_dialog_save_switches_multiplexer_and_persists_config() {
    let mut app = fixture_app();
    assert_eq!(app.multiplexer, MultiplexerKind::Tmux);

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
    assert!(app.settings_dialog.is_some());

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press));
    let _ = app.handle_key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press));
    let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

    assert!(app.settings_dialog.is_none());
    assert_eq!(app.multiplexer, MultiplexerKind::Zellij);
    let loaded = crate::config::load_from_path(&app.config_path).expect("config should load");
    assert_eq!(loaded.multiplexer, MultiplexerKind::Zellij);
}

#[test]
fn settings_dialog_multiplexer_cycles_with_h_and_l() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
    assert!(app.settings_dialog.is_some());

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('h')).with_kind(KeyEventKind::Press));
    assert_eq!(
        app.settings_dialog
            .as_ref()
            .map(|dialog| dialog.multiplexer),
        Some(MultiplexerKind::Zellij)
    );

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('h')).with_kind(KeyEventKind::Press));
    assert_eq!(
        app.settings_dialog
            .as_ref()
            .map(|dialog| dialog.multiplexer),
        Some(MultiplexerKind::Tmux)
    );

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press));
    assert_eq!(
        app.settings_dialog
            .as_ref()
            .map(|dialog| dialog.multiplexer),
        Some(MultiplexerKind::Zellij)
    );

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press));
    assert_eq!(
        app.settings_dialog
            .as_ref()
            .map(|dialog| dialog.multiplexer),
        Some(MultiplexerKind::Tmux)
    );
}

#[test]
fn settings_dialog_blocks_switch_when_workspace_running() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    assert_eq!(app.multiplexer, MultiplexerKind::Tmux);

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
    assert!(app.settings_dialog.is_some());

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press));
    let _ = app.handle_key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press));
    let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

    assert!(app.settings_dialog.is_some());
    assert_eq!(app.multiplexer, MultiplexerKind::Tmux);
    assert!(app.status_bar_line().contains("restart running workspaces"));
}

#[test]
fn zellij_capture_session_output_emulates_ansi_from_session_log() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    let session_name = format!(
        "grove-ws-zellij-emulator-test-{}-{timestamp}",
        std::process::id()
    );
    let log_path = crate::agent_runtime::zellij_capture_log_path(&session_name);
    let log_dir = log_path
        .parent()
        .expect("capture log path should have parent")
        .to_path_buf();
    fs::create_dir_all(&log_dir).expect("capture log directory should exist");
    fs::write(
        &log_path,
        concat!(
            "Script started on 2026-02-14 21:24:17-05:00 [COMMAND=\"codex\"]\n",
            "\0line one\n",
            "\u{1b}[31mline two red\u{1b}[0m\n",
            "\u{1b}[32mline three green\u{1b}[0m\n",
            "Script done on 2026-02-14 21:25:06-05:00 [COMMAND_EXIT_CODE=\"0\"]\n"
        ),
    )
    .expect("capture log should be written");
    let input = CommandZellijInput::default();

    let captured = input
        .capture_session_output(&session_name, 4)
        .expect("capture should load from log file");

    assert!(captured.contains("line one"));
    assert!(captured.contains("line two red"));
    assert!(captured.contains("line three green"));
    assert!(captured.contains("exited with code 0"));
    assert!(captured.contains("\u{1b}["));
    assert!(!captured.contains("Script started on "));
    assert!(!captured.contains("Script done on "));

    let _ = fs::remove_file(log_path);
}

#[test]
fn zellij_capture_session_output_returns_empty_when_log_missing() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    let session_name = format!(
        "grove-ws-zellij-missing-log-{}-{timestamp}",
        std::process::id()
    );
    let input = CommandZellijInput::default();

    let captured = input
        .capture_session_output(&session_name, 50)
        .expect("missing log should return empty output");
    assert!(captured.is_empty());
}

#[test]
fn status_row_shows_help_close_hint_when_help_modal_open() {
    let mut app = fixture_app();
    app.keybind_help_open = true;

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Esc/? close help"));
    });
}

#[test]
fn status_row_shows_palette_hints_when_palette_open() {
    let mut app = fixture_app();
    app.open_command_palette();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Type to search"));
        assert!(status_text.contains("Enter run"));
    });
}

#[test]
fn toast_overlay_renders_message() {
    let mut app = fixture_app();
    app.show_toast("Copied 2 line(s)", false);

    with_rendered_frame(&app, 80, 24, |frame| {
        let found = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("Copied 2 line(s)"));
        assert!(found, "toast message should render in frame");
    });
}

#[test]
fn interactive_copy_sets_success_toast_message() {
    let mut app = fixture_app();
    app.preview.lines = vec!["alpha".to_string()];
    app.preview.render_lines = app.preview.lines.clone();

    app.copy_interactive_selection_or_visible();

    let Some(toast) = app.notifications.visible().last() else {
        panic!("copy should set toast message");
    };
    assert!(matches!(toast.config.style_variant, ToastStyle::Success));
    assert_eq!(toast.content.message, "Copied 1 line(s)");
}

#[test]
fn status_row_shows_create_dialog_keybind_hints_when_modal_open() {
    let mut app = fixture_app();
    app.open_create_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Tab/S-Tab field"));
        assert!(status_text.contains("Enter select/create"));
    });
}

#[test]
fn status_row_shows_edit_dialog_keybind_hints_when_modal_open() {
    let mut app = fixture_app();
    app.open_edit_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Space toggle agent"));
        assert!(status_text.contains("Enter save/select"));
    });
}

#[test]
fn status_row_shows_launch_dialog_keybind_hints_when_modal_open() {
    let mut app = fixture_app();
    app.launch_dialog = Some(LaunchDialogState {
        prompt: String::new(),
        pre_launch_command: String::new(),
        skip_permissions: false,
        focused_field: LaunchDialogField::Prompt,
    });

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Tab/S-Tab field"));
        assert!(status_text.contains("Enter select/start"));
    });
}

#[test]
fn status_row_shows_delete_dialog_keybind_hints_when_modal_open() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.open_delete_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Tab/S-Tab field"));
        assert!(status_text.contains("Space toggle"));
    });
}

#[test]
fn view_hides_terminal_cursor_without_focused_input_widget() {
    let app = fixture_app();

    with_rendered_frame(&app, 80, 24, |frame| {
        assert!(frame.cursor_position.is_none());
        assert!(!frame.cursor_visible);
    });
}

#[test]
fn preview_pane_renders_ansi_colors() {
    let mut app = fixture_app();
    app.preview.lines = vec!["Success: all tests passed".to_string()];
    app.preview.render_lines = vec!["\u{1b}[32mSuccess\u{1b}[0m: all tests passed".to_string()];

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.preview.x.saturating_add(1);
    let x_end = layout.preview.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(row) = find_row_containing(frame, "Success", x_start, x_end) else {
            panic!("success row should be present in preview pane");
        };
        let Some(s_col) = find_cell_with_char(frame, row, x_start, x_end, 'S') else {
            panic!("success row should include first character column");
        };

        assert_row_fg(frame, row, s_col, s_col.saturating_add(7), ansi_16_color(2));
    });
}

#[test]
fn codex_interactive_preview_keeps_ansi_colors() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.interactive = Some(InteractiveState::new(
        "%1".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
    ));
    app.preview.lines = vec!["Success: all tests passed".to_string()];
    app.preview.render_lines = vec!["\u{1b}[32mSuccess\u{1b}[0m: all tests passed".to_string()];

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.preview.x.saturating_add(1);
    let x_end = layout.preview.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(row) = find_row_containing(frame, "Success", x_start, x_end) else {
            panic!("success row should be present in preview pane");
        };
        let Some(s_col) = find_cell_with_char(frame, row, x_start, x_end, 'S') else {
            panic!("success row should include first character column");
        };

        assert_row_fg(frame, row, s_col, s_col.saturating_add(7), ansi_16_color(2));
    });
}

#[test]
fn view_registers_hit_regions_for_panes_and_workspace_rows() {
    let app = fixture_app();
    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);

    with_rendered_frame(&app, 80, 24, |frame| {
        assert_eq!(
            frame
                .hit_test(layout.header.x, layout.header.y)
                .map(|hit| hit.0),
            Some(HitId::new(HIT_ID_HEADER))
        );
        assert_eq!(
            frame
                .hit_test(layout.preview.x, layout.preview.y)
                .map(|hit| hit.0),
            Some(HitId::new(HIT_ID_PREVIEW))
        );
        assert_eq!(
            frame
                .hit_test(layout.status.x, layout.status.y)
                .map(|hit| hit.0),
            Some(HitId::new(HIT_ID_STATUS))
        );
        assert_eq!(
            frame
                .hit_test(sidebar_inner.x, sidebar_inner.y)
                .map(|hit| hit.0),
            Some(HitId::new(HIT_ID_WORKSPACE_LIST))
        );
        assert_eq!(
            frame
                .hit_test(sidebar_inner.x, sidebar_inner.y.saturating_add(1))
                .map(|hit| hit.0),
            Some(HitId::new(HIT_ID_WORKSPACE_ROW))
        );
        assert_eq!(
            frame
                .hit_test(sidebar_inner.x, sidebar_inner.y.saturating_add(1))
                .map(|hit| hit.2),
            Some(0)
        );
    });
}

#[test]
fn mouse_workspace_selection_uses_row_hit_data_after_render() {
    let mut app = fixture_app();
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
    let second_row_y = sidebar_inner
        .y
        .saturating_add(1)
        .saturating_add(WORKSPACE_ITEM_HEIGHT);

    with_rendered_frame(&app, 100, 40, |_frame| {});

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            sidebar_inner.x,
            second_row_y,
        )),
    );

    assert_eq!(app.state.selected_index, 1);
}

#[test]
fn start_agent_emits_dialog_and_lifecycle_events() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Idle, Vec::new(), Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    let kinds = event_kinds(&events);
    assert_kind_subsequence(
        &kinds,
        &["dialog_opened", "dialog_confirmed", "agent_started"],
    );
    assert!(kinds.iter().any(|kind| kind == "toast_shown"));
}

#[test]
fn preview_poll_change_emits_output_changed_event() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(
            WorkspaceStatus::Active,
            vec![Ok("line one\nline two\n".to_string())],
            Vec::new(),
        );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);

    let kinds = event_kinds(&events);
    assert!(kinds.iter().any(|kind| kind == "output_changed"));
}

#[test]
fn tick_queues_async_preview_poll_with_background_io() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("background-poll");
    let mut app = GroveApp::from_parts(
        fixture_bootstrap(WorkspaceStatus::Active),
        Box::new(BackgroundOnlyTmuxInput),
        AppPaths::new(sidebar_ratio_path, unique_config_path("background-poll")),
        MultiplexerKind::Tmux,
        Box::new(NullEventLogger),
        None,
    );
    app.state.selected_index = 1;
    force_tick_due(&mut app);

    let cmd = ftui::Model::update(&mut app, Msg::Tick);
    assert!(cmd_contains_task(&cmd));
}

#[test]
fn tick_queues_async_poll_for_background_workspace_statuses_only() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("background-status-only");
    let mut app = GroveApp::from_parts(
        fixture_bootstrap(WorkspaceStatus::Idle),
        Box::new(BackgroundOnlyTmuxInput),
        AppPaths::new(
            sidebar_ratio_path,
            unique_config_path("background-status-only"),
        ),
        MultiplexerKind::Tmux,
        Box::new(NullEventLogger),
        None,
    );
    app.state.selected_index = 0;
    force_tick_due(&mut app);

    let cmd = ftui::Model::update(&mut app, Msg::Tick);
    assert!(!cmd_contains_task(&cmd));
}

#[test]
fn async_preview_capture_failure_sets_toast_message() {
    let mut app = fixture_app();
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: false,
                capture_ms: 2,
                total_ms: 2,
                result: Err("capture failed".to_string()),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );

    assert!(app.status_bar_line().contains("preview capture failed"));
}

#[test]
fn stale_preview_poll_result_is_dropped_by_generation() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());
    app.state.selected_index = 1;
    app.preview.lines = vec!["initial".to_string()];
    app.preview.render_lines = vec!["initial".to_string()];
    app.poll_generation = 2;

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: false,
                capture_ms: 1,
                total_ms: 1,
                result: Ok("stale-output\n".to_string()),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );
    assert_eq!(app.preview.lines, vec!["initial".to_string()]);
    assert!(
        event_kinds(&events)
            .iter()
            .any(|kind| kind == "stale_result_dropped")
    );

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 2,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: false,
                capture_ms: 1,
                total_ms: 1,
                result: Ok("fresh-output\n".to_string()),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );
    assert_eq!(app.preview.lines, vec!["fresh-output".to_string()]);
}

#[test]
fn preview_poll_uses_cleaned_change_for_status_lane() {
    let mut app = fixture_app();
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: true,
                capture_ms: 1,
                total_ms: 1,
                result: Ok("hello\u{1b}[?1000h\u{1b}[<35;192;47M".to_string()),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );
    assert!(app.output_changing);

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 2,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: true,
                capture_ms: 1,
                total_ms: 1,
                result: Ok("hello\u{1b}[?1000l".to_string()),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );

    assert!(!app.output_changing);
    let capture = app
        .preview
        .recent_captures
        .back()
        .expect("capture record should exist");
    assert!(capture.changed_raw);
    assert!(!capture.changed_cleaned);
}

#[test]
fn preview_poll_waiting_prompt_sets_waiting_status() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    if let Some(workspace) = app.state.selected_workspace_mut() {
        workspace.status = WorkspaceStatus::Active;
    }

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: true,
                capture_ms: 1,
                total_ms: 1,
                result: Ok("Approve command? [y/n]".to_string()),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );

    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Waiting)
    );
}

#[test]
fn preview_poll_updates_non_selected_workspace_status_from_background_capture() {
    let mut app = fixture_app();
    app.state.selected_index = 0;

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: None,
            cursor_capture: None,
            workspace_status_captures: vec![WorkspaceStatusCapture {
                workspace_name: "feature-a".to_string(),
                workspace_path: PathBuf::from("/repos/grove-feature-a"),
                session_name: "grove-ws-feature-a".to_string(),
                supported_agent: true,
                capture_ms: 1,
                result: Ok("> Implement {feature}\n? for shortcuts\n".to_string()),
            }],
        }),
    );

    assert_eq!(app.state.workspaces[1].status, WorkspaceStatus::Waiting);
    assert!(!app.state.workspaces[1].is_orphaned);
}

#[test]
fn zellij_workspace_status_poll_targets_include_idle_workspaces() {
    let mut app = fixture_app();
    app.multiplexer = MultiplexerKind::Zellij;
    app.state.selected_index = 0;
    app.state.workspaces[1].status = WorkspaceStatus::Idle;

    let targets = workspace_status_targets_for_polling_with_live_preview(
        &app.state.workspaces,
        app.multiplexer,
        None,
    );
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].workspace_name, "feature-a");
    assert_eq!(targets[0].session_name, "grove-ws-feature-a");
}

#[test]
fn tmux_workspace_status_poll_targets_skip_idle_workspaces() {
    let mut app = fixture_app();
    app.multiplexer = MultiplexerKind::Tmux;
    app.state.selected_index = 0;
    app.state.workspaces[1].status = WorkspaceStatus::Idle;

    let targets = workspace_status_targets_for_polling_with_live_preview(
        &app.state.workspaces,
        app.multiplexer,
        None,
    );
    assert!(targets.is_empty());
}

#[test]
fn preview_poll_non_selected_missing_session_marks_orphaned_idle() {
    let mut app = fixture_app();
    app.state.selected_index = 0;
    app.state.workspaces[1].status = WorkspaceStatus::Active;
    app.state.workspaces[1].is_orphaned = false;

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: None,
            cursor_capture: None,
            workspace_status_captures: vec![WorkspaceStatusCapture {
                workspace_name: "feature-a".to_string(),
                workspace_path: PathBuf::from("/repos/grove-feature-a"),
                session_name: "grove-ws-feature-a".to_string(),
                supported_agent: true,
                capture_ms: 1,
                result: Err(
                    "tmux capture-pane failed for 'grove-ws-feature-a': can't find pane"
                        .to_string(),
                ),
            }],
        }),
    );

    assert_eq!(app.state.workspaces[1].status, WorkspaceStatus::Idle);
    assert!(app.state.workspaces[1].is_orphaned);
}

#[test]
fn preview_poll_missing_session_marks_workspace_orphaned_idle() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.interactive = Some(InteractiveState::new(
        "%1".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        20,
        80,
    ));
    if let Some(workspace) = app.state.selected_workspace_mut() {
        workspace.status = WorkspaceStatus::Active;
        workspace.is_orphaned = false;
    }

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: true,
                capture_ms: 1,
                total_ms: 1,
                result: Err(
                    "tmux capture-pane failed for 'grove-ws-feature-a': can't find pane"
                        .to_string(),
                ),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );

    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Idle)
    );
    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.is_orphaned),
        Some(true)
    );
    assert!(app.interactive.is_none());
}

#[test]
fn preview_scroll_emits_scrolled_and_autoscroll_events() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Idle, Vec::new(), Vec::new());
    app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
    app.preview.offset = 0;
    app.preview.auto_scroll = true;

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollUp, 90, 10)),
    );

    let kinds = event_kinds(&events);
    assert!(kinds.iter().any(|kind| kind == "scrolled"));
    assert!(kinds.iter().any(|kind| kind == "autoscroll_toggled"));
}

#[test]
fn create_dialog_confirmed_event_includes_branch_payload() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Idle, Vec::new(), Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    for character in ['f', 'o', 'o'] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
    for _ in 0..3 {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    let dialog_confirmed = recorded_events(&events)
        .into_iter()
        .find(|event| event.kind == "dialog_confirmed" && event.event == "dialog")
        .expect("dialog_confirmed event should be logged");
    assert_eq!(
        dialog_confirmed
            .data
            .get("branch_mode")
            .and_then(Value::as_str),
        Some("new")
    );
    assert_eq!(
        dialog_confirmed
            .data
            .get("workspace_name")
            .and_then(Value::as_str),
        Some("foo")
    );
}

#[test]
fn project_add_dialog_accepts_shift_modified_uppercase_path_characters() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('A'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('/')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('U'))
                .with_modifiers(Modifiers::SHIFT)
                .with_kind(KeyEventKind::Press),
        ),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('S'))
                .with_modifiers(Modifiers::SHIFT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert_eq!(
        app.project_dialog
            .as_ref()
            .and_then(|dialog| dialog.add_dialog.as_ref())
            .map(|dialog| dialog.path.clone()),
        Some("/US".to_string())
    );
}

#[test]
fn project_dialog_filter_accepts_shift_modified_characters() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('G'))
                .with_modifiers(Modifiers::SHIFT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert_eq!(
        app.project_dialog
            .as_ref()
            .map(|dialog| dialog.filter.clone()),
        Some("G".to_string())
    );
}

#[test]
fn project_dialog_j_and_k_are_treated_as_filter_input() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        app.project_dialog
            .as_ref()
            .map(|dialog| dialog.filter.clone()),
        Some("jk".to_string())
    );
}

#[test]
fn project_dialog_tab_and_shift_tab_navigate_selection() {
    let mut app = fixture_app();
    app.projects.push(ProjectConfig {
        name: "site".to_string(),
        path: PathBuf::from("/repos/site"),
    });

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        app.project_dialog
            .as_ref()
            .map(|dialog| dialog.selected_filtered_index),
        Some(0)
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(
        app.project_dialog
            .as_ref()
            .map(|dialog| dialog.selected_filtered_index),
        Some(1)
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::BackTab).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(
        app.project_dialog
            .as_ref()
            .map(|dialog| dialog.selected_filtered_index),
        Some(0)
    );
}

#[test]
fn create_workspace_completed_success_queues_refresh_task_in_background_mode() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    let request = CreateWorkspaceRequest {
        workspace_name: "feature-x".to_string(),
        branch_mode: BranchMode::NewBranch {
            base_branch: "main".to_string(),
        },
        agent: AgentType::Claude,
    };
    let result = CreateWorkspaceResult {
        workspace_path: PathBuf::from("/repos/grove-feature-x"),
        branch: "feature-x".to_string(),
        warnings: Vec::new(),
    };

    let cmd = ftui::Model::update(
        &mut app,
        Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion {
            request,
            result: Ok(result),
        }),
    );

    assert!(cmd_contains_task(&cmd));
    assert!(app.refresh_in_flight);
}

#[test]
fn interactive_enter_and_exit_emit_mode_events() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );

    let kinds = event_kinds(&events);
    assert_kind_subsequence(&kinds, &["interactive_entered", "interactive_exited"]);
}

#[test]
fn key_q_maps_to_key_message() {
    let event = Event::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press));
    assert_eq!(
        Msg::from(event),
        Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press))
    );
}

#[test]
fn ctrl_c_maps_to_key_message() {
    let event = Event::Key(
        KeyEvent::new(KeyCode::Char('c'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );
    assert_eq!(
        Msg::from(event),
        Msg::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press)
        )
    );
}

#[test]
fn tmux_runtime_paths_avoid_status_calls_in_tui_module() {
    let source = include_str!("mod.rs");
    let status_call_pattern = ['.', 's', 't', 'a', 't', 'u', 's', '(']
        .into_iter()
        .collect::<String>();
    assert!(
        !source.contains(&status_call_pattern),
        "runtime tmux paths should avoid status command calls to preserve one-writer discipline"
    );
}

#[test]
fn tick_maps_to_tick_message() {
    assert_eq!(Msg::from(Event::Tick), Msg::Tick);
}

#[test]
fn key_message_updates_model_state() {
    let mut app = fixture_app();
    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    assert!(matches!(cmd, Cmd::Tick(_)));
    assert_eq!(app.state.selected_index, 1);
}

#[test]
fn q_quits_when_not_interactive() {
    let mut app = fixture_app();
    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
    );
    assert!(matches!(cmd, Cmd::Quit));
}

#[test]
fn ctrl_q_quits_via_action_mapper() {
    let mut app = fixture_app();
    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('q'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );
    assert!(matches!(cmd, Cmd::Quit));
}

#[test]
fn ctrl_d_quits_when_idle_via_action_mapper() {
    let mut app = fixture_app();
    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('d'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );
    assert!(matches!(cmd, Cmd::Quit));
}

#[test]
fn ctrl_c_dismisses_modal_via_action_mapper() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    focus_agent_preview_tab(&mut app);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    assert!(app.launch_dialog.is_some());

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(app.launch_dialog.is_none());
}

#[test]
fn ctrl_c_dismisses_delete_modal_via_action_mapper() {
    let mut app = fixture_app();
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
    );
    assert!(app.delete_dialog.is_some());

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(app.delete_dialog.is_none());
}

#[test]
fn ctrl_c_with_task_running_does_not_quit() {
    let mut app = fixture_app();
    app.start_in_flight = true;

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(!matches!(cmd, Cmd::Quit));
    assert!(
        app.status_bar_line()
            .contains("cannot cancel running lifecycle task")
    );
}

#[test]
fn ctrl_d_with_task_running_does_not_quit() {
    let mut app = fixture_app();
    app.start_in_flight = true;

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('d'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(!matches!(cmd, Cmd::Quit));
}

#[test]
fn start_key_launches_selected_workspace_agent() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    assert!(app.launch_dialog.is_some());
    assert!(commands.borrow().is_empty());
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().as_slice(),
        &[
            vec![
                "tmux".to_string(),
                "new-session".to_string(),
                "-d".to_string(),
                "-s".to_string(),
                "grove-ws-feature-a".to_string(),
                "-c".to_string(),
                "/repos/grove-feature-a".to_string(),
            ],
            vec![
                "tmux".to_string(),
                "set-option".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "history-limit".to_string(),
                "10000".to_string(),
            ],
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "codex".to_string(),
                "Enter".to_string(),
            ],
        ]
    );
    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Active)
    );
}

#[test]
fn h_and_l_switch_focus_between_workspace_and_preview_when_not_interactive() {
    let mut app = fixture_app();
    app.state.mode = UiMode::List;
    app.state.focus = PaneFocus::WorkspaceList;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(app.state.mode, UiMode::Preview);
    assert_eq!(app.state.focus, PaneFocus::Preview);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('h')).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(app.state.mode, UiMode::List);
    assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
}

#[test]
fn background_start_confirm_queues_lifecycle_task() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.state.selected_index = 1;
    focus_agent_preview_tab(&mut app);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(cmd_contains_task(&cmd));
}

#[test]
fn start_agent_completed_updates_workspace_status() {
    let mut app = fixture_app();
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::StartAgentCompleted(StartAgentCompletion {
            workspace_name: "feature-a".to_string(),
            workspace_path: PathBuf::from("/repos/grove-feature-a"),
            session_name: "grove-ws-feature-a".to_string(),
            result: Ok(()),
        }),
    );

    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Active)
    );
}

#[test]
fn unsafe_toggle_changes_launch_command_flags() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('!')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().last(),
        Some(&vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "codex --dangerously-bypass-approvals-and-sandbox".to_string(),
            "Enter".to_string(),
        ])
    );
    assert!(app.launch_skip_permissions);
}

#[test]
fn start_key_uses_workspace_prompt_file_launcher_script() {
    let workspace_dir = unique_temp_workspace_dir("prompt");
    let prompt_path = workspace_dir.join(".grove-prompt");
    fs::write(&prompt_path, "fix bug\nand add tests").expect("prompt file should be writable");

    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.workspaces[1].path = workspace_dir.clone();
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().last(),
        Some(&vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            format!("bash {}/.grove-start.sh", workspace_dir.display()),
            "Enter".to_string(),
        ])
    );

    let launcher_path = workspace_dir.join(".grove-start.sh");
    let launcher_script =
        fs::read_to_string(&launcher_path).expect("launcher script should be written");
    assert!(launcher_script.contains("fix bug"));
    assert!(launcher_script.contains("and add tests"));
    assert!(launcher_script.contains("codex"));

    let _ = fs::remove_dir_all(workspace_dir);
}

#[test]
fn start_dialog_pre_launch_command_runs_before_agent() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );

    for character in ['d', 'i', 'r', 'e', 'n', 'v', ' ', 'a', 'l', 'l', 'o', 'w'] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().last(),
        Some(&vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "direnv allow && codex".to_string(),
            "Enter".to_string(),
        ])
    );
}

#[test]
fn start_dialog_field_navigation_can_toggle_unsafe_for_launch() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char(' ')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().last(),
        Some(&vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "codex --dangerously-bypass-approvals-and-sandbox".to_string(),
            "Enter".to_string(),
        ])
    );
}

#[test]
fn start_dialog_blocks_background_navigation_and_escape_cancels() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;

    assert_eq!(app.state.selected_index, 1);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(app.state.selected_index, 1);
    assert_eq!(
        app.launch_dialog
            .as_ref()
            .map(|dialog| dialog.prompt.clone()),
        Some("k".to_string())
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );

    assert!(app.launch_dialog.is_none());
    assert!(commands.borrow().is_empty());
}

#[test]
fn new_workspace_key_opens_create_dialog() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        app.create_dialog.as_ref().map(|dialog| dialog.agent),
        Some(AgentType::Claude)
    );
    assert_eq!(
        app.create_dialog
            .as_ref()
            .map(|dialog| dialog.base_branch.clone()),
        Some("main".to_string())
    );
}

#[test]
fn edit_workspace_key_opens_edit_dialog() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('e')).with_kind(KeyEventKind::Press)),
    );

    let Some(dialog) = app.edit_dialog.as_ref() else {
        panic!("edit dialog should be open");
    };
    assert_eq!(dialog.workspace_name, "grove");
    assert_eq!(dialog.branch, "main");
    assert_eq!(dialog.agent, AgentType::Claude);
}

#[test]
fn edit_dialog_save_updates_workspace_agent_and_marker() {
    let mut app = fixture_app();
    let workspace_dir = unique_temp_workspace_dir("edit-save");
    app.state.workspaces[0].path = workspace_dir.clone();
    app.state.workspaces[0].agent = AgentType::Claude;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('e')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char(' ')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(app.edit_dialog.is_none());
    assert_eq!(app.state.workspaces[0].agent, AgentType::Codex);
    assert_eq!(
        fs::read_to_string(workspace_dir.join(".grove-agent"))
            .expect("agent marker should be readable")
            .trim(),
        "codex"
    );
    assert!(app.status_bar_line().contains("workspace updated"));
}

#[test]
fn delete_key_opens_delete_dialog_for_selected_workspace() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
    );

    let Some(dialog) = app.delete_dialog.as_ref() else {
        panic!("delete dialog should be open");
    };
    assert_eq!(dialog.workspace_name, "feature-a");
    assert_eq!(dialog.branch, "feature-a");
    assert_eq!(dialog.focused_field, DeleteDialogField::DeleteLocalBranch);
}

#[test]
fn delete_key_on_main_workspace_shows_guard_toast() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.delete_dialog.is_none());
    assert!(
        app.status_bar_line()
            .contains("cannot delete base workspace")
    );
}

#[test]
fn delete_dialog_blocks_navigation_and_escape_cancels() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.open_delete_dialog();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(app.state.selected_index, 1);
    assert_eq!(
        app.delete_dialog
            .as_ref()
            .map(|dialog| dialog.focused_field),
        Some(DeleteDialogField::DeleteButton)
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );
    assert!(app.delete_dialog.is_none());
}

#[test]
fn delete_dialog_confirm_queues_background_task() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
    );
    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
    );

    assert!(cmd_contains_task(&cmd));
    assert!(app.delete_dialog.is_none());
    assert!(app.delete_in_flight);
}

#[test]
fn create_dialog_tab_cycles_focus_field() {
    let mut app = fixture_app();
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        app.create_dialog
            .as_ref()
            .map(|dialog| dialog.focused_field),
        Some(CreateDialogField::Project)
    );
}

#[test]
fn create_dialog_j_and_k_on_agent_field_toggle_agent() {
    let mut app = fixture_app();
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        app.create_dialog.as_ref().map(|dialog| dialog.agent),
        Some(AgentType::Codex)
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(
        app.create_dialog.as_ref().map(|dialog| dialog.agent),
        Some(AgentType::Claude)
    );
}

#[test]
fn create_dialog_branch_field_edits_base_branch_in_new_mode() {
    let mut app = fixture_app();
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );

    for _ in 0..4 {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
        );
    }
    for character in ['d', 'e', 'v'] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }

    assert_eq!(
        app.create_dialog
            .as_ref()
            .map(|dialog| dialog.base_branch.clone()),
        Some("dev".to_string())
    );
}

#[test]
fn create_dialog_ctrl_n_and_ctrl_p_toggle_agent() {
    let mut app = fixture_app();
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    for _ in 0..3 {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('n'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );
    assert_eq!(
        app.create_dialog.as_ref().map(|dialog| dialog.agent),
        Some(AgentType::Codex)
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('p'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );
    assert_eq!(
        app.create_dialog.as_ref().map(|dialog| dialog.agent),
        Some(AgentType::Claude)
    );
}

#[test]
fn create_dialog_ctrl_n_and_ctrl_p_move_base_branch_dropdown() {
    let mut app = fixture_app();
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    app.create_branch_all = vec![
        "main".to_string(),
        "develop".to_string(),
        "release".to_string(),
    ];
    if let Some(dialog) = app.create_dialog.as_mut() {
        dialog.base_branch.clear();
    }
    app.refresh_create_branch_filtered();

    for _ in 0..2 {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('n'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );
    assert_eq!(app.create_branch_index, 1);
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('p'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );
    assert_eq!(app.create_branch_index, 0);
}

#[test]
fn create_dialog_base_branch_dropdown_selects_with_enter() {
    let mut app = fixture_app();
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );

    app.create_branch_all = vec![
        "main".to_string(),
        "develop".to_string(),
        "release".to_string(),
    ];
    app.refresh_create_branch_filtered();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    for _ in 0..4 {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
        );
    }
    for character in ['d', 'e'] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        app.create_dialog
            .as_ref()
            .map(|dialog| dialog.base_branch.clone()),
        Some("develop".to_string())
    );
    assert_eq!(
        app.create_dialog
            .as_ref()
            .map(|dialog| dialog.focused_field),
        Some(CreateDialogField::Agent)
    );
}

#[test]
fn create_dialog_blocks_navigation_and_escape_cancels() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(app.state.selected_index, 0);
    assert_eq!(
        app.create_dialog
            .as_ref()
            .map(|dialog| dialog.workspace_name.clone()),
        Some("j".to_string())
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );
    assert!(app.create_dialog.is_none());
}

#[test]
fn create_dialog_enter_without_name_shows_validation_toast() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    for _ in 0..4 {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(app.create_dialog.is_some());
    assert!(app.status_bar_line().contains("workspace name is required"));
}

#[test]
fn create_dialog_enter_on_cancel_closes_modal() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
    );
    for _ in 0..5 {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(app.create_dialog.is_none());
}

#[test]
fn stop_key_stops_selected_workspace_agent() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().as_slice(),
        &[
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
                "C-c".to_string(),
            ],
            vec![
                "tmux".to_string(),
                "kill-session".to_string(),
                "-t".to_string(),
                "grove-ws-feature-a".to_string(),
            ],
        ]
    );
    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Idle)
    );
}

#[test]
fn background_stop_key_queues_lifecycle_task() {
    let mut app = fixture_background_app(WorkspaceStatus::Active);
    app.state.selected_index = 1;
    focus_agent_preview_tab(&mut app);

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );

    assert!(cmd_contains_task(&cmd));
}

#[test]
fn stop_agent_completed_updates_workspace_status_and_exits_interactive() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
    ));

    ftui::Model::update(
        &mut app,
        Msg::StopAgentCompleted(StopAgentCompletion {
            workspace_name: "feature-a".to_string(),
            workspace_path: PathBuf::from("/repos/grove-feature-a"),
            session_name: "grove-ws-feature-a".to_string(),
            result: Ok(()),
        }),
    );

    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Idle)
    );
    assert!(app.interactive.is_none());
}

#[test]
fn start_key_opens_dialog_for_main_workspace() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );

    assert!(commands.borrow().is_empty());
    assert!(app.launch_dialog.is_some());
    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Main)
    );
}

#[test]
fn start_key_on_running_workspace_shows_toast_and_no_dialog() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.launch_dialog.is_none());
    assert!(commands.borrow().is_empty());
    assert!(app.status_bar_line().contains("agent already running"));
}

#[test]
fn start_key_noop_when_agent_tab_not_focused() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.launch_dialog.is_none());
    assert!(commands.borrow().is_empty());
}

#[test]
fn stop_key_without_running_agent_shows_toast() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    focus_agent_preview_tab(&mut app);
    app.state.selected_index = 1;
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );

    assert!(commands.borrow().is_empty());
    assert!(app.status_bar_line().contains("no agent running"));
}

#[test]
fn stop_key_noop_in_git_tab() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Git;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );

    assert!(commands.borrow().is_empty());
}

#[test]
fn stop_key_on_active_main_workspace_stops_agent() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    app.state.workspaces[0].status = WorkspaceStatus::Active;
    focus_agent_preview_tab(&mut app);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().as_slice(),
        &[
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-grove".to_string(),
                "C-c".to_string(),
            ],
            vec![
                "tmux".to_string(),
                "kill-session".to_string(),
                "-t".to_string(),
                "grove-ws-grove".to_string(),
            ],
        ]
    );
    assert_eq!(
        app.state
            .selected_workspace()
            .map(|workspace| workspace.status),
        Some(WorkspaceStatus::Main)
    );
}

#[test]
fn enter_on_active_workspace_starts_interactive_mode() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(app.interactive.is_some());
    assert_eq!(app.mode_label(), "Interactive");
}

#[test]
fn enter_on_active_main_workspace_starts_interactive_mode() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    app.state.workspaces[0].status = WorkspaceStatus::Active;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(app.interactive.is_some());
    assert_eq!(
        app.interactive
            .as_ref()
            .map(|state| state.target_session.as_str()),
        Some("grove-ws-grove")
    );
    assert_eq!(app.mode_label(), "Interactive");
}

#[test]
fn enter_on_active_workspace_resizes_tmux_session_to_preview_dimensions() {
    let (mut app, _commands, _captures, _cursor_captures, calls) =
        fixture_app_with_tmux_and_calls(WorkspaceStatus::Active, Vec::new(), Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(
        calls
            .borrow()
            .iter()
            .any(|call| call == "resize:grove-ws-feature-a:78:34")
    );
}

#[test]
fn enter_interactive_immediately_polls_preview_and_cursor() {
    let (mut app, _commands, _captures, _cursor_captures, calls) = fixture_app_with_tmux_and_calls(
        WorkspaceStatus::Active,
        vec![Ok("entered\n".to_string())],
        vec![Ok("1 0 0 78 34".to_string())],
    );
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(
        calls
            .borrow()
            .iter()
            .any(|call| call == "capture:grove-ws-feature-a:600:true")
    );
    assert!(
        calls
            .borrow()
            .iter()
            .any(|call| call == "cursor:grove-ws-feature-a")
    );
}

#[test]
fn resize_in_interactive_mode_immediately_resizes_and_polls() {
    let (mut app, _commands, _captures, _cursor_captures, calls) = fixture_app_with_tmux_and_calls(
        WorkspaceStatus::Active,
        vec![Ok("entered\n".to_string()), Ok("resized\n".to_string())],
        vec![Ok("1 0 0 78 34".to_string()), Ok("1 0 0 58 34".to_string())],
    );
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    calls.borrow_mut().clear();

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 80,
            height: 40,
        },
    );

    assert!(
        calls
            .borrow()
            .iter()
            .any(|call| call.starts_with("resize:grove-ws-feature-a:"))
    );
    assert!(
        calls
            .borrow()
            .iter()
            .any(|call| call == "capture:grove-ws-feature-a:600:true")
    );
}

#[test]
fn resize_verify_retries_once_then_stops() {
    let (mut app, _commands, _captures, _cursor_captures, calls) = fixture_app_with_tmux_and_calls(
        WorkspaceStatus::Active,
        vec![Ok("after-retry\n".to_string())],
        vec![Ok("1 0 0 70 20".to_string())],
    );
    app.state.selected_index = 1;
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
    ));
    app.pending_resize_verification = Some(PendingResizeVerification {
        session: "grove-ws-feature-a".to_string(),
        expected_width: 78,
        expected_height: 34,
        retried: false,
    });

    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: None,
            cursor_capture: Some(CursorCapture {
                session: "grove-ws-feature-a".to_string(),
                capture_ms: 1,
                result: Ok("1 0 0 70 20".to_string()),
            }),
            workspace_status_captures: Vec::new(),
        }),
    );

    let resize_retries = calls
        .borrow()
        .iter()
        .filter(|call| *call == "resize:grove-ws-feature-a:78:34")
        .count();
    assert_eq!(resize_retries, 1);
    assert!(app.pending_resize_verification.is_none());
}

#[test]
fn interactive_keys_forward_to_tmux_session() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
    );

    assert!(matches!(cmd, Cmd::Tick(_)));
    assert_eq!(
        commands.borrow().as_slice(),
        &[vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "q".to_string(),
        ]]
    );
}

#[test]
fn interactive_filters_split_mouse_bracket_fragment() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    let Some(state) = app.interactive.as_mut() else {
        panic!("interactive state should be active");
    };
    state.note_mouse_event(Instant::now());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('[')).with_kind(KeyEventKind::Press)),
    );

    assert!(commands.borrow().is_empty());
}

#[test]
fn interactive_filters_split_mouse_fragment_without_opening_bracket() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    let Some(state) = app.interactive.as_mut() else {
        panic!("interactive state should be active");
    };
    state.note_mouse_event(Instant::now());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('<')).with_kind(KeyEventKind::Press)),
    );

    assert!(commands.borrow().is_empty());
}

#[test]
fn interactive_filters_boundary_marker_before_split_mouse_fragment() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    let Some(state) = app.interactive.as_mut() else {
        panic!("interactive state should be active");
    };
    state.note_mouse_event(Instant::now());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('M')).with_kind(KeyEventKind::Press)),
    );

    assert!(commands.borrow().is_empty());
}

#[test]
fn interactive_still_forwards_bracket_when_not_mouse_fragment() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('[')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        commands.borrow().as_slice(),
        &[vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "[".to_string(),
        ]]
    );
}

#[test]
fn double_escape_exits_interactive_mode() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );

    assert!(app.interactive.is_none());
    assert_eq!(
        commands.borrow().as_slice(),
        &[vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "Escape".to_string(),
        ]]
    );
}

#[test]
fn ctrl_backslash_exits_interactive_mode() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('\\'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(app.interactive.is_none());
    assert!(commands.borrow().is_empty());
}

#[test]
fn ctrl_backslash_control_character_exits_interactive_mode() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('\u{1c}')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.interactive.is_none());
    assert!(commands.borrow().is_empty());
}

#[test]
fn ctrl_four_exits_interactive_mode() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('4'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(app.interactive.is_none());
    assert!(commands.borrow().is_empty());
}

#[test]
fn interactive_key_schedules_debounced_poll_interval() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );

    match cmd {
        Cmd::Tick(interval) => {
            assert!(
                interval <= Duration::from_millis(20) && interval >= Duration::from_millis(15),
                "expected debounced interactive interval near 20ms, got {interval:?}"
            );
        }
        _ => panic!("expected Cmd::Tick from interactive key update"),
    }
}

#[test]
fn interactive_key_does_not_postpone_existing_due_tick() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    let first_cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );
    assert!(matches!(first_cmd, Cmd::Tick(_)));
    let first_due = app
        .next_tick_due_at
        .expect("first key should schedule a due tick");

    std::thread::sleep(Duration::from_millis(1));

    let second_cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('y')).with_kind(KeyEventKind::Press)),
    );
    let second_due = app
        .next_tick_due_at
        .expect("second key should retain an existing due tick");

    assert!(
        second_due <= first_due,
        "second key should not postpone existing due tick"
    );
    assert!(
        matches!(second_cmd, Cmd::None),
        "when a sooner tick is already pending, no new timer should be scheduled"
    );
}

#[test]
fn interactive_update_flow_sequences_tick_copy_paste_and_exit() {
    let (mut app, _commands, _captures, _cursor_captures, calls) = fixture_app_with_tmux_and_calls(
        WorkspaceStatus::Active,
        vec![
            Ok("initial-preview".to_string()),
            Ok("preview-output".to_string()),
            Ok("copied-text".to_string()),
        ],
        vec![Ok("1 0 0 78 34".to_string())],
    );
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    calls.borrow_mut().clear();

    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('v'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(
        calls.borrow().as_slice(),
        &[
            "capture:grove-ws-feature-a:600:true".to_string(),
            "cursor:grove-ws-feature-a".to_string(),
            "exec:tmux send-keys -l -t grove-ws-feature-a x".to_string(),
            "paste-buffer:grove-ws-feature-a:14".to_string(),
            "exec:tmux send-keys -t grove-ws-feature-a Escape".to_string(),
        ]
    );
    assert!(app.interactive.is_none());
}

#[test]
fn interactive_input_latency_correlates_forwarded_key_with_preview_update() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(
            WorkspaceStatus::Active,
            vec![
                Ok("initial-preview".to_string()),
                Ok("initial-preview\nx".to_string()),
            ],
            vec![Ok("1 0 0 120 40".to_string())],
        );
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    clear_recorded_events(&events);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );
    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);

    let recorded = recorded_events(&events);
    let forwarded = recorded
        .iter()
        .find(|event| event.event == "input" && event.kind == "interactive_forwarded")
        .expect("forwarded input event should be logged");
    let seq = forwarded
        .data
        .get("seq")
        .and_then(Value::as_u64)
        .expect("forwarded input should include seq");

    let latency = recorded
        .iter()
        .find(|event| event.event == "input" && event.kind == "interactive_input_to_preview")
        .expect("input latency event should be logged");
    assert_eq!(latency.data.get("seq").and_then(Value::as_u64), Some(seq));
    assert!(
        latency
            .data
            .get("input_to_preview_ms")
            .and_then(Value::as_u64)
            .is_some()
    );
    assert!(
        latency
            .data
            .get("tmux_to_preview_ms")
            .and_then(Value::as_u64)
            .is_some()
    );

    let output_changed = recorded
        .iter()
        .find(|event| event.event == "preview_update" && event.kind == "output_changed")
        .expect("preview update event should be logged");
    assert_eq!(
        output_changed.data.get("input_seq").and_then(Value::as_u64),
        Some(seq)
    );
}

#[test]
fn preview_update_logs_coalesced_input_range() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(
            WorkspaceStatus::Active,
            vec![
                Ok("initial-preview".to_string()),
                Ok("initial-preview\nab".to_string()),
            ],
            vec![Ok("1 0 0 120 40".to_string())],
        );
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    clear_recorded_events(&events);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('a')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('b')).with_kind(KeyEventKind::Press)),
    );
    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);

    let recorded = recorded_events(&events);
    let output_changed = recorded
        .iter()
        .find(|event| event.event == "preview_update" && event.kind == "output_changed")
        .expect("preview update event should be logged");
    assert_eq!(
        output_changed
            .data
            .get("consumed_input_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        output_changed
            .data
            .get("consumed_input_seq_first")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        output_changed
            .data
            .get("consumed_input_seq_last")
            .and_then(Value::as_u64),
        Some(2)
    );

    let coalesced = recorded
        .iter()
        .find(|event| event.event == "input" && event.kind == "interactive_inputs_coalesced")
        .expect("coalesced input event should be logged");
    assert_eq!(
        coalesced
            .data
            .get("consumed_input_count")
            .and_then(Value::as_u64),
        Some(2)
    );
}

#[test]
fn tick_logs_skip_reason_when_not_due() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());
    clear_recorded_events(&events);

    app.next_tick_due_at = Some(Instant::now() + Duration::from_secs(10));
    app.next_tick_interval_ms = Some(10_000);
    ftui::Model::update(&mut app, Msg::Tick);

    let recorded = recorded_events(&events);
    let skipped = recorded
        .iter()
        .find(|event| event.event == "tick" && event.kind == "skipped")
        .expect("tick skip event should be logged");
    assert_eq!(
        skipped.data.get("reason").and_then(Value::as_str),
        Some("not_due")
    );
    assert_eq!(
        skipped.data.get("interval_ms").and_then(Value::as_u64),
        Some(10_000)
    );
}

#[test]
fn interactive_exit_clears_pending_input_traces() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    clear_recorded_events(&events);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('\\'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        ),
    );

    let recorded = recorded_events(&events);
    let cleared = recorded
        .iter()
        .find(|event| event.event == "input" && event.kind == "pending_inputs_cleared")
        .expect("pending traces should be cleared when interactive exits");
    assert_eq!(
        cleared.data.get("session").and_then(Value::as_str),
        Some("grove-ws-feature-a")
    );
    assert!(
        cleared
            .data
            .get("cleared")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0)
    );
}

#[test]
fn codex_live_preview_capture_keeps_tmux_escape_output() {
    let (mut app, _commands, _captures, _cursor_captures, calls) = fixture_app_with_tmux_and_calls(
        WorkspaceStatus::Active,
        vec![
            Ok("line one\nline two\n".to_string()),
            Ok("line one\nline two\n".to_string()),
        ],
        Vec::new(),
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);

    assert!(
        calls
            .borrow()
            .iter()
            .any(|call| call == "capture:grove-ws-feature-a:600:true")
    );
}

#[test]
fn claude_live_preview_capture_keeps_tmux_escape_output() {
    let (mut app, _commands, _captures, _cursor_captures, calls) = fixture_app_with_tmux_and_calls(
        WorkspaceStatus::Active,
        vec![
            Ok("line one\nline two\n".to_string()),
            Ok("line one\nline two\n".to_string()),
        ],
        Vec::new(),
    );
    app.state.workspaces[1].agent = AgentType::Claude;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);

    assert!(
        calls
            .borrow()
            .iter()
            .any(|call| call == "capture:grove-ws-feature-a:600:true")
    );
}

#[test]
fn tick_polls_live_tmux_output_into_preview() {
    let (mut app, _commands, _captures, _cursor_captures) = fixture_app_with_tmux(
        WorkspaceStatus::Active,
        vec![
            Ok("line one\nline two\n".to_string()),
            Ok("line one\nline two\n".to_string()),
        ],
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);

    assert_eq!(
        app.preview.lines,
        vec!["line one".to_string(), "line two".to_string()]
    );
}

#[test]
fn stale_tick_before_due_is_ignored() {
    let (mut app, _commands, _captures, _cursor_captures, calls) = fixture_app_with_tmux_and_calls(
        WorkspaceStatus::Active,
        vec![Ok("line".to_string())],
        Vec::new(),
    );

    app.state.selected_index = 1;
    app.next_tick_due_at = Some(Instant::now() + Duration::from_secs(5));

    let cmd = ftui::Model::update(&mut app, Msg::Tick);

    assert!(matches!(cmd, Cmd::None));
    assert!(calls.borrow().is_empty());
}

#[test]
fn parse_cursor_metadata_requires_five_fields() {
    assert_eq!(
        parse_cursor_metadata("1 4 2 120 40"),
        Some(super::CursorMetadata {
            cursor_visible: true,
            cursor_col: 4,
            cursor_row: 2,
            pane_width: 120,
            pane_height: 40,
        })
    );
    assert!(parse_cursor_metadata("1 4 2 120").is_none());
    assert!(parse_cursor_metadata("invalid").is_none());
}

#[test]
fn ansi_line_parser_preserves_text_and_styles() {
    let line = ansi_line_to_styled_line("a\u{1b}[31mb\u{1b}[0mc");
    assert_eq!(line.to_plain_text(), "abc");
    assert_eq!(line.spans().len(), 3);
    assert_eq!(line.spans()[1].as_str(), "b");
    assert_eq!(
        line.spans()[1].style.and_then(|style| style.fg),
        Some(ansi_16_color(1))
    );
}

#[test]
fn tick_polls_cursor_metadata_and_renders_overlay() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("cursor-overlay");
    let (mut app, _commands, _captures, _cursor_captures) = fixture_app_with_tmux_and_sidebar_path(
        WorkspaceStatus::Active,
        vec![
            Ok("first\nsecond\nthird\n".to_string()),
            Ok("first\nsecond\nthird\n".to_string()),
        ],
        vec![Ok("1 1 1 78 34".to_string()), Ok("1 1 1 78 34".to_string())],
        sidebar_ratio_path,
    );
    app.state.workspaces[1].agent = AgentType::Claude;
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    force_tick_due(&mut app);
    ftui::Model::update(&mut app, Msg::Tick);

    let rendered = app.shell_lines(8).join("\n");
    assert_eq!(
        app.interactive.as_ref().map(|state| (
            state.cursor_row,
            state.cursor_col,
            state.pane_height
        )),
        Some((1, 1, 34))
    );
    assert!(rendered.contains("s|econd"), "{rendered}");
}

#[test]
fn divider_ratio_persists_across_app_instances() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("persist");
    let (mut app, _commands, _captures, _cursor_captures) = fixture_app_with_tmux_and_sidebar_path(
        WorkspaceStatus::Idle,
        Vec::new(),
        Vec::new(),
        sidebar_ratio_path.clone(),
    );

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            33,
            8,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            52,
            8,
        )),
    );

    assert_eq!(app.sidebar_width_pct, 52);
    assert_eq!(
        fs::read_to_string(&sidebar_ratio_path).expect("ratio file should be written"),
        "52"
    );

    let (app_reloaded, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux_and_sidebar_path(
            WorkspaceStatus::Idle,
            Vec::new(),
            Vec::new(),
            sidebar_ratio_path.clone(),
        );

    assert_eq!(app_reloaded.sidebar_width_pct, 52);
    let _ = fs::remove_file(sidebar_ratio_path);
}

#[test]
fn mouse_click_on_list_selects_workspace() {
    let mut app = fixture_app();
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
    let second_row_y = sidebar_inner
        .y
        .saturating_add(1)
        .saturating_add(WORKSPACE_ITEM_HEIGHT);

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            sidebar_inner.x.saturating_add(1),
            second_row_y,
        )),
    );

    assert_eq!(app.state.selected_index, 1);
}

#[test]
fn mouse_drag_on_divider_updates_sidebar_ratio() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            33,
            8,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            55,
            8,
        )),
    );

    assert_eq!(app.sidebar_width_pct, 55);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            55,
            8,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            20,
            8,
        )),
    );

    assert_eq!(app.sidebar_width_pct, 55);
}

#[test]
fn mouse_drag_near_divider_still_updates_sidebar_ratio() {
    let mut app = fixture_app();

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            32,
            8,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            50,
            8,
        )),
    );

    assert_eq!(app.sidebar_width_pct, 50);
}

#[test]
fn mouse_scroll_in_preview_scrolls_output() {
    let mut app = fixture_app();
    app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
    app.preview.offset = 0;
    app.preview.auto_scroll = true;

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollUp, 90, 10)),
    );

    assert!(app.preview.offset > 0);
    assert!(!app.preview.auto_scroll);
}

#[test]
fn mouse_drag_in_interactive_preview_highlights_selected_text() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = vec!["alpha beta".to_string()];
    app.preview.render_lines = vec!["\u{1b}[32malpha beta\u{1b}[0m".to_string()];

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            preview_inner.x,
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );

    let x_start = layout.preview.x.saturating_add(1);
    let x_end = layout.preview.right().saturating_sub(1);
    with_rendered_frame(&app, 100, 40, |frame| {
        let Some(output_row) = find_row_containing(frame, "alpha beta", x_start, x_end) else {
            panic!("output row should be rendered");
        };
        let Some(first_col) = find_cell_with_char(frame, output_row, x_start, x_end, 'a') else {
            panic!("selected output row should include first char");
        };

        assert_row_bg(
            frame,
            output_row,
            first_col,
            first_col.saturating_add(5),
            ui_theme().surface1,
        );
        assert_row_fg(
            frame,
            output_row,
            first_col,
            first_col.saturating_add(5),
            ansi_16_color(2),
        );
    });
}

#[test]
fn interactive_mouse_drag_logs_click_mapping_and_selected_preview() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = vec!["alpha beta".to_string()];
    app.preview.render_lines = app.preview.lines.clone();

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    clear_recorded_events(&events);

    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            preview_inner.x,
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );

    let recorded = recorded_events(&events);
    let started = recorded
        .iter()
        .find(|event| event.event == "selection" && event.kind == "preview_drag_started")
        .expect("drag start event should be logged");
    assert_eq!(started.data.get("mapped"), Some(&Value::from(true)));
    assert_eq!(started.data.get("line"), Some(&Value::from(0)));
    assert_eq!(started.data.get("col"), Some(&Value::from(0)));
    assert_eq!(
        started.data.get("line_clean_preview"),
        Some(&Value::from("alpha beta"))
    );
    assert_eq!(started.data.get("grapheme"), Some(&Value::from("a")));

    let finished = recorded
        .iter()
        .find(|event| event.event == "selection" && event.kind == "preview_drag_finished")
        .expect("drag finish event should be logged");
    assert_eq!(finished.data.get("has_selection"), Some(&Value::from(true)));
    assert_eq!(finished.data.get("start_line"), Some(&Value::from(0)));
    assert_eq!(finished.data.get("start_col"), Some(&Value::from(0)));
    assert_eq!(finished.data.get("end_line"), Some(&Value::from(0)));
    assert_eq!(finished.data.get("end_col"), Some(&Value::from(4)));
    assert_eq!(
        finished.data.get("selected_preview"),
        Some(&Value::from("alpha"))
    );
    assert_eq!(
        finished.data.get("release_grapheme"),
        Some(&Value::from("a"))
    );
    assert_eq!(finished.data.get("end_grapheme"), Some(&Value::from("a")));

    let mouse_event = recorded
        .iter()
        .find(|event| {
            event.event == "mouse"
                && event.kind == "event"
                && event.data.get("kind") == Some(&Value::from("Down(Left)"))
        })
        .expect("mouse event telemetry should be logged");
    assert_eq!(
        mouse_event.data.get("region"),
        Some(&Value::from("preview"))
    );
    assert_eq!(mouse_event.data.get("mapped_line"), Some(&Value::from(0)));
    assert_eq!(mouse_event.data.get("mapped_col"), Some(&Value::from(0)));
    assert_eq!(
        mouse_event.data.get("mapped_grapheme"),
        Some(&Value::from("a"))
    );
}

#[test]
fn interactive_drag_mapping_prefers_render_line_when_clean_line_empty() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = vec![String::new()];
    app.preview.render_lines = vec!["hello".to_string()];

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    clear_recorded_events(&events);

    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            preview_inner.x,
            select_y,
        )),
    );

    let recorded = recorded_events(&events);
    let started = recorded
        .iter()
        .find(|event| event.event == "selection" && event.kind == "preview_drag_started")
        .expect("drag start event should be logged");
    assert_eq!(
        started.data.get("line_preview"),
        Some(&Value::from("hello"))
    );
    assert_eq!(
        started.data.get("line_clean_preview"),
        Some(&Value::from("hello"))
    );
}

#[test]
fn interactive_drag_mapping_uses_rendered_frame_size_without_resize_message() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = (0..120).map(|index| format!("line-{index:03}")).collect();
    app.preview.render_lines = app.preview.lines.clone();

    with_rendered_frame(&app, 100, 50, |_| {});
    clear_recorded_events(&events);

    let layout = GroveApp::view_layout_for_size(100, 50, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let select_x = preview_inner.x;
    let select_y = preview_inner
        .y
        .saturating_add(PREVIEW_METADATA_ROWS)
        .saturating_add(40);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            select_x,
            select_y,
        )),
    );

    let recorded = recorded_events(&events);
    let started = recorded
        .iter()
        .find(|event| event.event == "selection" && event.kind == "preview_drag_started")
        .expect("drag start event should be logged");
    assert_eq!(started.data.get("mapped"), Some(&Value::from(true)));

    let output_height = usize::from(preview_inner.height.saturating_sub(PREVIEW_METADATA_ROWS));
    let output_row =
        usize::from(select_y.saturating_sub(preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS)));
    let expected_visible_start = app.preview.lines.len().saturating_sub(output_height);
    let expected_line = expected_visible_start.saturating_add(output_row);
    assert_eq!(
        started.data.get("line"),
        Some(&Value::from(
            u64::try_from(expected_line).unwrap_or(u64::MAX)
        ))
    );
}

#[test]
fn mouse_move_then_release_highlights_selected_text_without_drag_event() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = vec!["alpha beta".to_string()];
    app.preview.render_lines = vec!["\u{1b}[32malpha beta\u{1b}[0m".to_string()];

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            preview_inner.x,
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Moved,
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );

    let x_start = layout.preview.x.saturating_add(1);
    let x_end = layout.preview.right().saturating_sub(1);
    with_rendered_frame(&app, 100, 40, |frame| {
        let Some(output_row) = find_row_containing(frame, "alpha beta", x_start, x_end) else {
            panic!("output row should be rendered");
        };
        let Some(first_col) = find_cell_with_char(frame, output_row, x_start, x_end, 'a') else {
            panic!("selected output row should include first char");
        };

        assert_row_bg(
            frame,
            output_row,
            first_col,
            first_col.saturating_add(5),
            ui_theme().surface1,
        );
        assert_row_fg(
            frame,
            output_row,
            first_col,
            first_col.saturating_add(5),
            ansi_16_color(2),
        );
    });
}

#[test]
fn mouse_drag_selection_overrides_existing_ansi_background_sequences() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = vec!["abc".to_string()];
    app.preview.render_lines = vec![
        "\u{1b}[48;2;30;35;50ma\u{1b}[48;2;30;35;50mb\u{1b}[48;2;30;35;50mc\u{1b}[0m".to_string(),
    ];

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            preview_inner.x,
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            preview_inner.x.saturating_add(2),
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            preview_inner.x.saturating_add(2),
            select_y,
        )),
    );

    let x_start = layout.preview.x.saturating_add(1);
    let x_end = layout.preview.right().saturating_sub(1);
    with_rendered_frame(&app, 100, 40, |frame| {
        let Some(output_row) = find_row_containing(frame, "abc", x_start, x_end) else {
            panic!("output row should be rendered");
        };
        let Some(first_col) = find_cell_with_char(frame, output_row, x_start, x_end, 'a') else {
            panic!("selected output row should include first char");
        };

        assert_row_bg(
            frame,
            output_row,
            first_col,
            first_col.saturating_add(3),
            ui_theme().surface1,
        );
    });
}

#[test]
fn selected_preview_text_lines_use_visual_columns() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.preview.lines = vec!["A😀B".to_string()];
    app.preview.render_lines = app.preview.lines.clone();
    app.preview_selection
        .prepare_drag(TextSelectionPoint { line: 0, col: 0 });
    app.preview_selection
        .handle_drag(TextSelectionPoint { line: 0, col: 2 });
    app.preview_selection.finish_drag();

    assert_eq!(
        app.selected_preview_text_lines(),
        Some(vec!["A😀".to_string()])
    );
}

#[test]
fn preview_render_lines_align_with_plain_visible_range_when_lengths_differ() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.preview.lines = (0..40).map(|index| format!("p{index}")).collect();
    app.preview.render_lines = (0..42).map(|index| format!("r{index}")).collect();

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );

    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let output_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);
    let x_start = layout.preview.x.saturating_add(1);
    let x_end = layout.preview.right().saturating_sub(1);
    with_rendered_frame(&app, 100, 40, |frame| {
        let rendered = row_text(frame, output_y, x_start, x_end);
        assert!(
            rendered.contains("r6"),
            "expected first visible rendered row to start from aligned render index, got: {rendered}"
        );
    });
}

#[test]
fn alt_copy_then_alt_paste_uses_mouse_selected_preview_text() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, vec![Ok(String::new())]);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = vec!["alpha beta".to_string()];
    app.preview.render_lines = app.preview.lines.clone();

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let select_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            preview_inner.x,
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Drag(MouseButton::Left),
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Up(MouseButton::Left),
            preview_inner.x.saturating_add(4),
            select_y,
        )),
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('v'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(!app.preview_selection.has_selection());
    assert_eq!(
        commands.borrow().last(),
        Some(&vec![
            "tmux".to_string(),
            "paste-buffer".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "alpha".to_string(),
        ])
    );
}

#[test]
fn bracketed_paste_event_forwards_wrapped_literal() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    ftui::Model::update(&mut app, Msg::Paste(PasteEvent::bracketed("hello\nworld")));

    assert_eq!(
        commands.borrow().last(),
        Some(&vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-l".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "\u{1b}[200~hello\nworld\u{1b}[201~".to_string(),
        ])
    );
}

#[test]
fn alt_copy_then_alt_paste_uses_visible_preview_text_when_no_selection() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("alt-copy-paste");
    let (mut app, commands, captures, _cursor_captures) = fixture_app_with_tmux_and_sidebar_path(
        WorkspaceStatus::Active,
        vec![Ok(String::new())],
        vec![Ok("1 0 0 78 34".to_string())],
        sidebar_ratio_path,
    );
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    app.preview.lines = vec!["copy me".to_string()];
    app.preview.render_lines = app.preview.lines.clone();

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('c'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('v'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(captures.borrow().is_empty());
    assert_eq!(
        commands.borrow().last(),
        Some(&vec![
            "tmux".to_string(),
            "paste-buffer".to_string(),
            "-t".to_string(),
            "grove-ws-feature-a".to_string(),
            "copy me".to_string(),
        ])
    );
}

#[test]
fn shell_contains_list_preview_and_status_placeholders() {
    let app = fixture_app();
    let lines = app.shell_lines(8);
    let content = lines.join("\n");

    assert!(content.contains("Workspaces"));
    assert!(content.contains("Preview Pane"));
    assert!(content.contains("Status:"));
    assert!(content.contains("feature-a | feature-a | Codex | /repos/grove-feature-a"));
    assert!(content.contains("Press 'n' to create a workspace"));
}

#[test]
fn shell_renders_discovery_error_state() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("error-state");
    let app = GroveApp::from_parts(
        BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Error("fatal: not a git repository".to_string()),
            orphaned_sessions: Vec::new(),
        },
        Box::new(RecordingTmuxInput {
            commands: Rc::new(RefCell::new(Vec::new())),
            captures: Rc::new(RefCell::new(Vec::new())),
            cursor_captures: Rc::new(RefCell::new(Vec::new())),
            calls: Rc::new(RefCell::new(Vec::new())),
        }),
        AppPaths::new(sidebar_ratio_path, unique_config_path("error-state")),
        MultiplexerKind::Tmux,
        Box::new(NullEventLogger),
        None,
    );
    let lines = app.shell_lines(8);
    let content = lines.join("\n");

    assert!(content.contains("discovery failed"));
    assert!(content.contains("discovery error"));
}

#[test]
fn preview_mode_keys_scroll_and_jump_to_bottom() {
    let mut app = fixture_app();
    app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
    app.preview.render_lines = app.preview.lines.clone();
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
    assert!(was_auto_scroll);
    assert!(!app.preview.auto_scroll);
    assert!(app.preview.offset > 0);

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('G')).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(app.preview.offset, 0);
    assert!(app.preview.auto_scroll);
}

#[test]
fn preview_mode_bracket_keys_cycle_tabs() {
    let mut app = fixture_app();
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
    assert_eq!(app.preview_tab, PreviewTab::Agent);

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
    assert_eq!(app.preview_tab, PreviewTab::Git);

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
    assert_eq!(app.preview_tab, PreviewTab::Agent);

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('['))));
    assert_eq!(app.preview_tab, PreviewTab::Git);
}

#[test]
fn preview_mode_scroll_keys_noop_in_git_tab() {
    let mut app = fixture_app();
    app.preview.lines = (1..=120).map(|value| value.to_string()).collect();
    app.preview.render_lines = app.preview.lines.clone();
    app.preview.offset = 0;
    app.preview.auto_scroll = true;
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
    assert_eq!(app.preview_tab, PreviewTab::Git);

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::PageDown)));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('G'))));

    assert_eq!(app.preview.offset, 0);
    assert!(app.preview.auto_scroll);
}

#[test]
fn git_tab_renders_lazygit_placeholder_and_launches_session() {
    let mut app = fixture_app();
    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));

    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let output_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);
    let x_start = preview_inner.x;
    let x_end = preview_inner.right();

    with_rendered_frame(&app, 100, 40, |frame| {
        let tabs_line = row_text(frame, preview_inner.y.saturating_add(1), x_start, x_end);
        let output_line = row_text(frame, output_y, x_start, x_end);

        assert!(tabs_line.contains("Agent"));
        assert!(tabs_line.contains("Git"));
        assert!(output_line.contains("lazygit"));
    });
}

#[test]
fn git_tab_launches_lazygit_with_dedicated_tmux_session() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));

    assert_eq!(
        commands.borrow().as_slice(),
        &[
            vec![
                "tmux".to_string(),
                "new-session".to_string(),
                "-d".to_string(),
                "-s".to_string(),
                "grove-ws-grove-git".to_string(),
                "-c".to_string(),
                "/repos/grove".to_string(),
            ],
            vec![
                "tmux".to_string(),
                "set-option".to_string(),
                "-t".to_string(),
                "grove-ws-grove-git".to_string(),
                "history-limit".to_string(),
                "10000".to_string(),
            ],
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-ws-grove-git".to_string(),
                "lazygit".to_string(),
                "Enter".to_string(),
            ],
        ]
    );
}

#[test]
fn git_tab_launches_lazygit_with_zellij_session_plan() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    app.multiplexer = MultiplexerKind::Zellij;

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));

    let command_lines: Vec<String> = commands
        .borrow()
        .iter()
        .map(|command| command.join(" "))
        .collect();

    assert!(
        command_lines
            .iter()
            .any(|line| line.contains("kill-session 'grove-ws-grove-git'"))
    );
    assert!(
        command_lines
            .iter()
            .any(|line| line.contains("--session grove-ws-grove-git run"))
    );
    assert!(
        command_lines
            .iter()
            .any(|line| line.contains("script -qefc 'lazygit'"))
    );
}

#[test]
fn enter_on_git_tab_attaches_to_lazygit_session() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));

    assert_eq!(
        app.interactive
            .as_ref()
            .map(|state| state.target_session.as_str()),
        Some("grove-ws-grove-git")
    );
    assert_eq!(app.mode_label(), "Interactive");
}

#[test]
fn preview_mode_scroll_keys_noop_when_content_fits_viewport() {
    let mut app = fixture_app();
    app.preview.lines = (1..=4).map(|value| value.to_string()).collect();
    app.preview.render_lines = app.preview.lines.clone();
    app.preview.offset = 0;
    app.preview.auto_scroll = true;

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
    );

    assert_eq!(app.preview.offset, 0);
    assert!(app.preview.auto_scroll);
}

#[test]
fn frame_debug_record_logs_every_view() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("frame-log");
    let events = Arc::new(Mutex::new(Vec::new()));
    let event_log = RecordingEventLogger {
        events: events.clone(),
    };
    let app = GroveApp::from_parts(
        fixture_bootstrap(WorkspaceStatus::Idle),
        Box::new(RecordingTmuxInput {
            commands: Rc::new(RefCell::new(Vec::new())),
            captures: Rc::new(RefCell::new(Vec::new())),
            cursor_captures: Rc::new(RefCell::new(Vec::new())),
            calls: Rc::new(RefCell::new(Vec::new())),
        }),
        AppPaths::new(sidebar_ratio_path, unique_config_path("frame-log")),
        MultiplexerKind::Tmux,
        Box::new(event_log),
        Some(1_771_023_000_000),
    );

    with_rendered_frame(&app, 100, 40, |_frame| {});
    with_rendered_frame(&app, 100, 40, |_frame| {});

    let recorded = recorded_events(&events);
    let frame_events: Vec<LoggedEvent> = recorded
        .into_iter()
        .filter(|event| event.event == "frame" && event.kind == "rendered")
        .collect();
    assert_eq!(frame_events.len(), 2);
    assert_eq!(
        frame_events[0].data.get("seq").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        frame_events[1].data.get("seq").and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        frame_events[0]
            .data
            .get("app_start_ts")
            .and_then(Value::as_u64),
        Some(1_771_023_000_000)
    );
}

#[test]
fn frame_debug_record_includes_frame_lines() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("frame-lines");
    let events = Arc::new(Mutex::new(Vec::new()));
    let event_log = RecordingEventLogger {
        events: events.clone(),
    };
    let mut app = GroveApp::from_parts(
        fixture_bootstrap(WorkspaceStatus::Idle),
        Box::new(RecordingTmuxInput {
            commands: Rc::new(RefCell::new(Vec::new())),
            captures: Rc::new(RefCell::new(Vec::new())),
            cursor_captures: Rc::new(RefCell::new(Vec::new())),
            calls: Rc::new(RefCell::new(Vec::new())),
        }),
        AppPaths::new(sidebar_ratio_path, unique_config_path("frame-lines")),
        MultiplexerKind::Tmux,
        Box::new(event_log),
        Some(1_771_023_000_123),
    );
    app.preview.lines = vec!["render-check 🧪".to_string()];
    app.preview.render_lines = app.preview.lines.clone();

    with_rendered_frame(&app, 80, 24, |_frame| {});

    let frame_event = recorded_events(&events)
        .into_iter()
        .find(|event| event.event == "frame" && event.kind == "rendered")
        .expect("frame event should be present");

    let lines = frame_event
        .data
        .get("frame_lines")
        .and_then(Value::as_array)
        .expect("frame_lines should be array");
    assert!(lines.iter().any(|line| {
        line.as_str()
            .is_some_and(|text| text.contains("render-check 🧪"))
    }));
    assert!(frame_event.data.get("frame_hash").is_some());
    assert_eq!(
        frame_event.data.get("degradation").and_then(Value::as_str),
        Some("Full")
    );
    assert!(
        frame_event
            .data
            .get("non_empty_line_count")
            .and_then(Value::as_u64)
            .is_some_and(|count| count > 0)
    );
    assert_eq!(
        frame_event
            .data
            .get("frame_cursor_visible")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        frame_event
            .data
            .get("frame_cursor_has_position")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn frame_debug_record_includes_interactive_cursor_snapshot() {
    let sidebar_ratio_path = unique_sidebar_ratio_path("frame-cursor-snapshot");
    let events = Arc::new(Mutex::new(Vec::new()));
    let event_log = RecordingEventLogger {
        events: events.clone(),
    };
    let mut app = GroveApp::from_parts(
        fixture_bootstrap(WorkspaceStatus::Idle),
        Box::new(RecordingTmuxInput {
            commands: Rc::new(RefCell::new(Vec::new())),
            captures: Rc::new(RefCell::new(Vec::new())),
            cursor_captures: Rc::new(RefCell::new(Vec::new())),
            calls: Rc::new(RefCell::new(Vec::new())),
        }),
        AppPaths::new(
            sidebar_ratio_path,
            unique_config_path("frame-cursor-snapshot"),
        ),
        MultiplexerKind::Tmux,
        Box::new(event_log),
        Some(1_771_023_000_124),
    );
    app.interactive = Some(InteractiveState::new(
        "%1".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        3,
        80,
    ));
    if let Some(state) = app.interactive.as_mut() {
        state.update_cursor(1, 2, true, 3, 80);
    }
    app.preview.lines = vec![
        "line-0".to_string(),
        "line-1".to_string(),
        "line-2".to_string(),
    ];
    app.preview.render_lines = app.preview.lines.clone();

    with_rendered_frame(&app, 80, 24, |_frame| {});

    let frame_event = recorded_events(&events)
        .into_iter()
        .find(|event| event.event == "frame" && event.kind == "rendered")
        .expect("frame event should be present");
    assert_eq!(
        frame_event
            .data
            .get("interactive_cursor_row")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        frame_event
            .data
            .get("interactive_cursor_col")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        frame_event
            .data
            .get("interactive_cursor_in_viewport")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        frame_event
            .data
            .get("interactive_cursor_visible_index")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        frame_event
            .data
            .get("interactive_cursor_target_col")
            .and_then(Value::as_u64),
        Some(2)
    );
}

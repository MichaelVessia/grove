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
    AppDependencies, AppPaths, ClipboardAccess, CommandTmuxInput, CreateDialogField,
    CreateWorkspaceCompletion, CursorCapture, DeleteDialogField, DeleteProjectCompletion,
    DeleteWorkspaceCompletion, EditDialogField, GroveApp, HIT_ID_HEADER, HIT_ID_PREVIEW,
    HIT_ID_STATUS, HIT_ID_WORKSPACE_LIST, HIT_ID_WORKSPACE_ROW, LaunchDialogField,
    LaunchDialogState, LazygitLaunchCompletion, LivePreviewCapture, MergeDialogField, Msg,
    PREVIEW_METADATA_ROWS, PendingResizeVerification, PreviewPollCompletion, PreviewTab,
    StartAgentCompletion, StopAgentCompletion, TextSelectionPoint, TmuxInput, UiCommand,
    UpdateFromBaseDialogField, WORKSPACE_ITEM_HEIGHT, WorkspaceStatusCapture, ansi_16_color,
    ansi_line_to_styled_line, parse_cursor_metadata, ui_theme,
};
use crate::application::agent_runtime::workspace_status_targets_for_polling_with_live_preview;
use crate::application::interactive::InteractiveState;
use crate::application::workspace_lifecycle::{
    BranchMode, CreateWorkspaceRequest, CreateWorkspaceResult,
};
use crate::domain::{AgentType, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::{BootstrapData, DiscoveryState};
use crate::infrastructure::config::{MultiplexerKind, ProjectConfig};
use crate::infrastructure::event_log::{Event as LoggedEvent, EventLogger, NullEventLogger};
use crate::ui::state::{PaneFocus, UiMode};
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

    fn supports_background_poll(&self) -> bool {
        true
    }

    fn supports_background_launch(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct BackgroundLaunchTmuxInput;

impl TmuxInput for BackgroundLaunchTmuxInput {
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
        Ok("1 0 0 120 40".to_string())
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

    fn supports_background_launch(&self) -> bool {
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
    feature_workspace.base_branch = Some("main".to_string());

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
                app.merge_dialog.is_some(),
                app.update_from_base_dialog.is_some(),
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
fn sidebar_row_shows_deleting_indicator_for_in_flight_delete() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.state.selected_index = 1;
    app.delete_in_flight = true;
    app.delete_in_flight_workspace = Some(app.state.workspaces[1].path.clone());

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(feature_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("feature row should be rendered");
        };
        let feature_row_text = row_text(frame, feature_row, x_start, x_end);
        assert!(
            feature_row_text.contains(" · De"),
            "feature row should include deleting indicator, got: {feature_row_text}"
        );

        let Some(base_row) = find_row_containing(frame, "base", x_start, x_end) else {
            panic!("base row should be rendered");
        };
        let base_row_text = row_text(frame, base_row, x_start, x_end);
        assert!(
            !base_row_text.contains(" · De"),
            "base row should not include deleting indicator, got: {base_row_text}"
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
fn backslash_toggles_sidebar_visibility() {
    let mut app = fixture_app();
    assert!(!app.sidebar_hidden);

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('\\')).with_kind(KeyEventKind::Press));

    assert!(app.sidebar_hidden);
    let hidden_layout = GroveApp::view_layout_for_size_with_sidebar(120, 40, 33, true);
    assert_eq!(hidden_layout.sidebar.width, 0);
    assert_eq!(hidden_layout.divider.width, 0);
    assert_eq!(hidden_layout.preview.width, 120);

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('\\')).with_kind(KeyEventKind::Press));
    assert!(!app.sidebar_hidden);
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
    let palette_id = |command: UiCommand| -> String {
        command
            .palette_spec()
            .map(|spec| spec.id.to_string())
            .expect("command should be palette discoverable")
    };

    let mut app = fixture_app();
    app.state.selected_index = 1;
    let list_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::MoveSelectionDown))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::OpenPreview))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::MergeWorkspace))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::UpdateFromBase))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::OpenProjects))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::DeleteProject))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ToggleSidebar))
    );
    assert!(
        !list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ScrollDown))
    );
    assert!(
        !list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::StartAgent))
    );
    assert!(
        !list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::PreviousTab))
    );
    assert!(
        !list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::NextTab))
    );

    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Agent;
    let preview_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ScrollDown))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::FocusList))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::StartAgent))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::PreviousTab))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::NextTab))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ToggleSidebar))
    );
    assert!(
        !preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::MoveSelectionDown))
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
            .any(|id| id == &palette_id(UiCommand::ScrollDown))
    );
    assert!(
        !git_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::StartAgent))
    );
    assert!(
        git_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::PreviousTab))
    );
    assert!(
        git_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::NextTab))
    );
    assert!(
        git_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ToggleSidebar))
    );
}

#[test]
fn ui_command_palette_ids_are_unique_and_roundtrip() {
    let mut ids = std::collections::HashSet::new();
    for command in UiCommand::all() {
        let Some(spec) = command.palette_spec() else {
            continue;
        };
        assert!(
            ids.insert(spec.id),
            "duplicate command palette id: {}",
            spec.id
        );
        assert_eq!(UiCommand::from_palette_id(spec.id), Some(*command));
    }
}

#[test]
fn uppercase_s_opens_settings_dialog() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));

    assert!(app.settings_dialog.is_some());
}

#[test]
fn settings_dialog_save_persists_tmux_config() {
    let mut app = fixture_app();
    assert_eq!(app.multiplexer, MultiplexerKind::Tmux);

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
    assert!(app.settings_dialog.is_some());

    let _ = app.handle_key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press));
    let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

    assert!(app.settings_dialog.is_none());
    assert_eq!(app.multiplexer, MultiplexerKind::Tmux);
    let loaded = crate::infrastructure::config::load_from_path(&app.config_path)
        .expect("config should load");
    assert_eq!(loaded.multiplexer, MultiplexerKind::Tmux);
}

#[test]
fn settings_dialog_multiplexer_keys_keep_tmux_selection() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
    assert!(app.settings_dialog.is_some());

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('h')).with_kind(KeyEventKind::Press));
    assert_eq!(
        app.settings_dialog
            .as_ref()
            .map(|dialog| dialog.multiplexer),
        Some(MultiplexerKind::Tmux)
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
        Some(MultiplexerKind::Tmux)
    );
}

#[test]
fn command_tmux_input_uses_background_send_mode() {
    let input = CommandTmuxInput;
    assert!(input.supports_background_send());
}

#[test]
fn command_tmux_input_uses_background_poll_mode() {
    let input = CommandTmuxInput;
    assert!(input.supports_background_poll());
}

#[test]
fn command_tmux_input_uses_background_launch_mode() {
    let input = CommandTmuxInput;
    assert!(input.supports_background_launch());
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
        assert!(status_text.contains("type/backspace base branch"));
        assert!(status_text.contains("h/l buttons"));
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
fn status_row_shows_merge_dialog_keybind_hints_when_modal_open() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.open_merge_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Tab/S-Tab field"));
        assert!(status_text.contains("Space toggle cleanup"));
    });
}

#[test]
fn status_row_shows_update_from_base_dialog_hints_when_modal_open() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.open_update_from_base_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Tab/S-Tab field"));
        assert!(status_text.contains("Enter select/update"));
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

mod runtime_flow;

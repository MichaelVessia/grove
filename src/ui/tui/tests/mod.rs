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
    AppDependencies, ClipboardAccess, CommandTmuxInput, CreateDialogField, CreateDialogTab,
    CreateWorkspaceCompletion, CursorCapture, DeleteDialogField, DeleteProjectCompletion,
    DeleteWorkspaceCompletion, EditDialogField, GroveApp, HIT_ID_HEADER, HIT_ID_PREVIEW,
    HIT_ID_STATUS, HIT_ID_WORKSPACE_LIST, HIT_ID_WORKSPACE_PR_LINK, HIT_ID_WORKSPACE_ROW,
    HelpHintContext, LaunchDialogField, LaunchDialogState, LazygitLaunchCompletion,
    LivePreviewCapture, MergeDialogField, MergeWorkspaceCompletion, Msg, PREVIEW_METADATA_ROWS,
    PendingAutoStartWorkspace, PendingResizeVerification, PreviewPollCompletion, PreviewTab,
    ProjectAddDialogField, ProjectDefaultsDialogField, RefreshWorkspacesCompletion,
    SettingsDialogField, StartAgentCompletion, StartAgentConfigField, StartAgentConfigState,
    StopAgentCompletion, StopDialogField, TextSelectionPoint, TmuxInput, UiCommand,
    UpdateFromBaseDialogField, WORKSPACE_ITEM_HEIGHT, WorkspaceAttention,
    WorkspaceShellLaunchCompletion, WorkspaceStatusCapture, ansi_16_color,
    ansi_lines_to_styled_lines, decode_workspace_pr_hit_data, parse_cursor_metadata, ui_theme,
    usize_to_u64,
};
use crate::application::agent_runtime::workspace_status_targets_for_polling_with_live_preview;
use crate::application::interactive::InteractiveState;
use crate::application::workspace_lifecycle::{
    BranchMode, CreateWorkspaceRequest, CreateWorkspaceResult,
};
use crate::domain::{AgentType, PullRequest, PullRequestStatus, Workspace, WorkspaceStatus};
use crate::infrastructure::adapters::{BootstrapData, DiscoveryState};
use crate::infrastructure::config::ProjectConfig;
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
    }
}

fn fixture_projects() -> Vec<ProjectConfig> {
    vec![ProjectConfig {
        name: "grove".to_string(),
        path: PathBuf::from("/repos/grove"),
        defaults: Default::default(),
    }]
}

fn fixture_app() -> GroveApp {
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
            config_path,

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

fn cmd_contains_mouse_capture_toggle(cmd: &Cmd<Msg>, enabled: bool) -> bool {
    match cmd {
        Cmd::SetMouseCapture(state) => *state == enabled,
        Cmd::Batch(commands) | Cmd::Sequence(commands) => commands
            .iter()
            .any(|command| cmd_contains_mouse_capture_toggle(command, enabled)),
        _ => false,
    }
}

fn arb_key_event() -> impl Strategy<Value = KeyEvent> {
    proptest::prop_oneof![
        Just(key_press(KeyCode::Char('j'))),
        Just(key_press(KeyCode::Char('k'))),
        Just(key_press(KeyCode::Char('s'))),
        Just(key_press(KeyCode::Char('x'))),
        Just(key_press(KeyCode::Char('r'))),
        Just(key_press(KeyCode::Char('R'))),
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
    fixture_app_with_tmux_and_config_path(
        status,
        captures,
        Vec::new(),
        unique_config_path("fixture-with-tmux"),
    )
}

fn fixture_app_with_tmux_and_config_path(
    status: WorkspaceStatus,
    captures: Vec<Result<String, String>>,
    cursor_captures: Vec<Result<String, String>>,
    config_path: PathBuf,
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
                config_path,

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
    let config_path = unique_config_path("fixture-with-calls");
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
                config_path,

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
    let config_path = unique_config_path("fixture-with-events");
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
                config_path,

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
            config_path: unique_config_path("background"),

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
                app.launch_dialog().is_some(),
                app.create_dialog().is_some(),
                app.delete_dialog().is_some(),
                app.merge_dialog().is_some(),
                app.update_from_base_dialog().is_some(),
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
    let layout = GroveApp::view_layout_for_size(160, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 160, 24, |frame| {
        assert!(find_row_containing(frame, "grove", x_start, x_end).is_some());
        assert!(find_row_containing(frame, "feature-a", x_start, x_end).is_some());
    });
}

#[test]
fn sidebar_keeps_first_project_header_visible_after_tiny_initial_render() {
    let mut app = fixture_app();
    for index in 0..24usize {
        let mut workspace = Workspace::try_new(
            format!("extra-{index}"),
            PathBuf::from(format!("/repos/grove-extra-{index}")),
            format!("extra-{index}"),
            None,
            AgentType::Codex,
            WorkspaceStatus::Idle,
            false,
        )
        .expect("workspace should be valid");
        workspace.project_path = Some(PathBuf::from("/repos/grove"));
        app.state.workspaces.push(workspace);
    }

    with_rendered_frame(&app, 80, 3, |_frame| {});

    let layout = GroveApp::view_layout_for_size(80, 16, app.sidebar_width_pct, false);
    let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 16, |frame| {
        let Some(project_row) = find_row_containing(frame, "▾ grove", x_start, x_end) else {
            panic!("first project header should be visible");
        };
        assert_eq!(
            project_row, sidebar_inner.y,
            "first project header should remain at top of sidebar"
        );
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
    let expected_age_prefix = expected_age.chars().take(2).collect::<String>();

    let layout = GroveApp::view_layout_for_size(160, 24, app.sidebar_width_pct, false);
    let sidebar_x_start = layout.sidebar.x.saturating_add(1);
    let sidebar_x_end = layout.sidebar.right().saturating_sub(1);
    let preview_x_start = layout.preview.x.saturating_add(1);
    let preview_x_end = layout.preview.right().saturating_sub(1);

    with_rendered_frame(&app, 160, 24, |frame| {
        let Some(sidebar_row) =
            find_row_containing(frame, "grove · main", sidebar_x_start, sidebar_x_end)
        else {
            panic!("sidebar workspace row should be rendered");
        };
        let sidebar_text = row_text(frame, sidebar_row, sidebar_x_start, sidebar_x_end);
        assert!(
            sidebar_text.starts_with("│ "),
            "sidebar row should render inside a selected workspace block, got: {sidebar_text}"
        );
        assert!(
            !sidebar_text.contains(expected_age.as_str()),
            "sidebar row should not include age label, got: {sidebar_text}"
        );

        let Some(preview_row) = find_row_containing(
            frame,
            "grove · main · Claude",
            preview_x_start,
            preview_x_end,
        ) else {
            panic!("preview header row should be rendered");
        };
        let preview_text = row_text(frame, preview_row, preview_x_start, preview_x_end);
        assert!(
            preview_text.contains(expected_age_prefix.as_str()),
            "preview header should include age label, got: {preview_text}"
        );
    });
}

#[test]
fn selected_workspace_row_has_selection_marker() {
    let mut app = fixture_app();
    app.state.selected_index = 1;

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let rendered_row = row_text(frame, selected_row, x_start, x_end);
        assert!(
            rendered_row.starts_with("│ ") && rendered_row.ends_with("│"),
            "selected row should render inside a clear selected block, got: {rendered_row}"
        );
    });
}

#[test]
fn sidebar_row_omits_duplicate_workspace_and_branch_text() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("feature row should be rendered");
        };
        let rendered_row_text = row_text(frame, row, x_start, x_end);
        let metadata_row_text = row_text(frame, row.saturating_add(1), x_start, x_end);
        assert!(
            !rendered_row_text.contains("feature-a · feature-a"),
            "row should not duplicate workspace and branch when they match, got: {rendered_row_text}"
        );
        assert!(
            metadata_row_text.contains("Codex"),
            "metadata row should include agent label, got: {metadata_row_text}"
        );
    });
}

#[test]
fn sidebar_row_shows_deleting_indicator_for_in_flight_delete() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.state.selected_index = 1;
    app.sidebar_width_pct = 60;
    app.delete_in_flight = true;
    app.delete_in_flight_workspace = Some(app.state.workspaces[1].path.clone());
    app.delete_requested_workspaces
        .insert(app.state.workspaces[1].path.clone());

    let layout = GroveApp::view_layout_for_size(160, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 160, 24, |frame| {
        let Some(feature_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("feature row should be rendered");
        };
        let feature_row_text = row_text(frame, feature_row.saturating_add(1), x_start, x_end);
        assert!(
            feature_row_text.contains(" · De"),
            "feature metadata row should include deleting indicator, got: {feature_row_text}"
        );

        let Some(base_row) = find_row_containing(frame, "grove", x_start, x_end) else {
            panic!("base row should be rendered");
        };
        let base_row_text = row_text(frame, base_row.saturating_add(1), x_start, x_end);
        assert!(
            !base_row_text.contains(" · De"),
            "base metadata row should not include deleting indicator, got: {base_row_text}"
        );
    });
}

#[test]
fn sidebar_second_line_shows_pull_request_status_icons() {
    let mut app = fixture_app();
    app.state.workspaces[1].pull_requests = vec![
        PullRequest {
            number: 101,
            url: "https://github.com/acme/grove/pull/101".to_string(),
            status: PullRequestStatus::Open,
        },
        PullRequest {
            number: 102,
            url: "https://github.com/acme/grove/pull/102".to_string(),
            status: PullRequestStatus::Merged,
        },
        PullRequest {
            number: 103,
            url: "https://github.com/acme/grove/pull/103".to_string(),
            status: PullRequestStatus::Closed,
        },
    ];

    let layout = GroveApp::view_layout_for_size(140, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);
    let theme = ui_theme();

    with_rendered_frame(&app, 140, 24, |frame| {
        let Some(row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("feature row should be rendered");
        };
        let metadata_row = row.saturating_add(1);
        let metadata_text = row_text(frame, metadata_row, x_start, x_end);
        assert!(
            metadata_text.contains("PRs:"),
            "metadata row should include PR label, got: {metadata_text}"
        );
        assert!(
            metadata_text.contains(" #101")
                && metadata_text.contains(" #102")
                && metadata_text.contains(" #103"),
            "metadata row should include PR ids with status icons, got: {metadata_text}"
        );

        let Some(open_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '') else {
            panic!("open PR icon should render");
        };
        assert_row_fg(
            frame,
            metadata_row,
            open_col,
            open_col.saturating_add(1),
            theme.teal,
        );

        let Some(merged_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '')
        else {
            panic!("merged PR icon should render");
        };
        assert_row_fg(
            frame,
            metadata_row,
            merged_col,
            merged_col.saturating_add(1),
            theme.mauve,
        );

        let Some(closed_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '')
        else {
            panic!("closed PR icon should render");
        };
        assert_row_fg(
            frame,
            metadata_row,
            closed_col,
            closed_col.saturating_add(1),
            theme.red,
        );
    });
}

#[test]
fn workspace_pr_token_registers_link_hit_data() {
    let mut app = fixture_app();
    app.state.workspaces[1].pull_requests = vec![PullRequest {
        number: 321,
        url: "https://github.com/acme/grove/pull/321".to_string(),
        status: PullRequestStatus::Open,
    }];

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("feature row should be rendered");
        };
        let metadata_row = row.saturating_add(1);
        let Some(icon_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '') else {
            panic!("PR icon should render");
        };
        let Some((hit_id, _region, hit_data)) = frame.hit_test(icon_col, metadata_row) else {
            panic!("PR icon should have hit target");
        };
        assert_eq!(hit_id, HitId::new(HIT_ID_WORKSPACE_PR_LINK));
        assert_eq!(decode_workspace_pr_hit_data(hit_data), Some((1, 0)));
    });
}

#[test]
fn base_workspace_hides_pull_request_list() {
    let mut app = fixture_app();
    app.state.workspaces[0].pull_requests = vec![PullRequest {
        number: 777,
        url: "https://github.com/acme/grove/pull/777".to_string(),
        status: PullRequestStatus::Open,
    }];

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(base_row) = find_row_containing(frame, "grove", x_start, x_end) else {
            panic!("base row should render");
        };
        let metadata_text = row_text(frame, base_row.saturating_add(1), x_start, x_end);
        assert!(
            !metadata_text.contains("PRs:"),
            "base workspace should hide PR list, got: {metadata_text}"
        );
    });
}

#[test]
fn shell_lines_show_workspace_and_agent_labels_without_status_badges() {
    let app = fixture_app();
    let lines = app.shell_lines(12);
    let Some(base_line) = lines.iter().find(|line| line.contains("grove | main")) else {
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

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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
fn live_preview_scrollback_lines_uses_idle_window_when_inactive() {
    let (app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());

    assert_eq!(
        app.live_preview_scrollback_lines(),
        super::LIVE_PREVIEW_IDLE_SCROLLBACK_LINES
    );
}

#[test]
fn live_preview_scrollback_lines_uses_full_window_with_recent_activity() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.output_changing = true;

    assert_eq!(
        app.live_preview_scrollback_lines(),
        super::LIVE_PREVIEW_SCROLLBACK_LINES
    );
}

#[test]
fn live_preview_scrollback_lines_uses_full_window_in_interactive_mode() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
    ));

    assert_eq!(
        app.live_preview_scrollback_lines(),
        super::LIVE_PREVIEW_SCROLLBACK_LINES
    );
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

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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
    app.state.selected_index = 0;
    app.sidebar_width_pct = 70;
    app.workspace_attention.insert(
        PathBuf::from("/repos/grove-feature-a"),
        WorkspaceAttention::NeedsAttention,
    );

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
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
        assert!(
            sidebar_row_text.contains(" ! "),
            "waiting workspace should show attention indicator, got: {sidebar_row_text}"
        );
    });
}

#[test]
fn working_workspace_row_shows_attention_indicator_when_present() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.output_changing = true;
    app.agent_output_changing = true;
    app.workspace_attention.insert(
        PathBuf::from("/repos/grove-feature-a"),
        WorkspaceAttention::NeedsAttention,
    );

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(
            sidebar_row_text.contains(" ! "),
            "working workspace should show attention indicator, got: {sidebar_row_text}"
        );
    });
}

#[test]
fn done_workspace_row_shows_done_attention_indicator() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Done, Vec::new());
    app.state.selected_index = 0;
    app.workspace_attention.insert(
        PathBuf::from("/repos/grove-feature-a"),
        WorkspaceAttention::NeedsAttention,
    );

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(
            sidebar_row_text.contains(" ! "),
            "done workspace should show done attention indicator, got: {sidebar_row_text}"
        );
    });
}

#[test]
fn error_workspace_row_shows_error_attention_indicator() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Error, Vec::new());
    app.state.selected_index = 0;
    app.workspace_attention.insert(
        PathBuf::from("/repos/grove-feature-a"),
        WorkspaceAttention::NeedsAttention,
    );

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(selected_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("selected workspace row should be rendered");
        };
        let sidebar_row_text = row_text(frame, selected_row, x_start, x_end);
        assert!(
            sidebar_row_text.contains(" ! "),
            "error workspace should show error attention indicator, got: {sidebar_row_text}"
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
                "status footer should remain stable when spinner state changes"
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

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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
            status_text.contains("project: grove")
                && status_text.contains("workspace: feature-a")
                && status_text.contains("? help")
                && status_text.contains("Ctrl+K palette"),
            "status row should show compact context + key hints, got: {status_text}"
        );
    });
}

#[test]
fn modal_dialog_renders_over_sidebar() {
    let mut app = fixture_app();
    app.set_launch_dialog(LaunchDialogState {
        start_config: StartAgentConfigState::new(String::new(), String::new(), false),
        focused_field: LaunchDialogField::StartConfig(StartAgentConfigField::Prompt),
    });

    with_rendered_frame(&app, 80, 24, |frame| {
        assert!(find_row_containing(frame, "Start Agent", 0, frame.width()).is_some());
    });
}

#[test]
fn launch_dialog_uses_opaque_background_fill() {
    let mut app = fixture_app();
    app.set_launch_dialog(LaunchDialogState {
        start_config: StartAgentConfigState::new(String::new(), String::new(), false),
        focused_field: LaunchDialogField::StartConfig(StartAgentConfigField::Prompt),
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
        let dialog_height = 25u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let probe_x = dialog_x.saturating_add(dialog_width.saturating_sub(3));
        let probe_y = dialog_y.saturating_add(1);
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
        let dialog_height = 25u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let x_start = dialog_x.saturating_add(1);
        let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
        let y_start = dialog_y.saturating_add(1);
        let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));
        let find_dialog_row = |needle: &str| {
            (y_start..y_end).find(|&row| row_text(frame, row, x_start, x_end).contains(needle))
        };

        let Some(selected_row) = find_dialog_row("[Agent] Claude") else {
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
        let dialog_height = 25u16;
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

        let Some(selected_agent_row) = find_dialog_row("[Agent] Claude") else {
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
        let dialog_height = 25u16;
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
fn create_dialog_alt_brackets_switch_between_manual_and_pr_tabs() {
    let mut app = fixture_app();
    app.open_create_dialog();

    assert_eq!(
        app.create_dialog().map(|dialog| dialog.tab),
        Some(CreateDialogTab::Manual)
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::WorkspaceName)
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char(']'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert_eq!(
        app.create_dialog().map(|dialog| dialog.tab),
        Some(CreateDialogTab::PullRequest)
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::Project)
    );

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('['))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert_eq!(
        app.create_dialog().map(|dialog| dialog.tab),
        Some(CreateDialogTab::Manual)
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::WorkspaceName)
    );
}

#[test]
fn create_dialog_mode_tabs_are_mouse_clickable() {
    let mut app = fixture_app();
    app.open_create_dialog();
    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 80,
            height: 24,
        },
    );

    let width = 80u16;
    let height = 24u16;
    let dialog_width = width.saturating_sub(8).min(90);
    let dialog_height = 25u16;
    let dialog_x = width.saturating_sub(dialog_width) / 2;
    let dialog_y = height.saturating_sub(dialog_height) / 2;
    let inner_x = dialog_x.saturating_add(1);
    let tab_row_y = dialog_y.saturating_add(1).saturating_add(2);

    let manual_tab_width =
        u16::try_from("Manual".len().saturating_add(2)).expect("manual tab width should fit");
    let pull_tab_x = inner_x.saturating_add(manual_tab_width).saturating_add(1);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            pull_tab_x.saturating_add(1),
            tab_row_y,
        )),
    );

    assert_eq!(
        app.create_dialog().map(|dialog| dialog.tab),
        Some(CreateDialogTab::PullRequest)
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::Project)
    );

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            inner_x.saturating_add(1),
            tab_row_y,
        )),
    );

    assert_eq!(
        app.create_dialog().map(|dialog| dialog.tab),
        Some(CreateDialogTab::Manual)
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::WorkspaceName)
    );
}

#[test]
fn create_dialog_allows_paste_into_pr_url_field() {
    let mut app = fixture_app();
    app.open_create_dialog();

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char(']'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::PullRequestUrl)
    );

    ftui::Model::update(
        &mut app,
        Msg::Paste(PasteEvent::bracketed(
            "https://github.com/flocasts/web-monorepo/pull/3484",
        )),
    );

    assert_eq!(
        app.create_dialog().map(|dialog| dialog.pr_url.clone()),
        Some("https://github.com/flocasts/web-monorepo/pull/3484".to_string())
    );
}

#[test]
fn status_row_ignores_toast_and_shows_compact_footer() {
    let mut app = fixture_app();
    app.show_success_toast("Agent started");

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(!status_text.contains("Agent started"));
        assert!(status_text.contains("project: grove"));
        assert!(status_text.contains("workspace: grove"));
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_preview_mode() {
    let mut app = fixture_app();
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Agent;

    with_rendered_frame(&app, 220, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("project: grove"));
        assert!(status_text.contains("workspace: grove"));
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_git_tab() {
    let mut app = fixture_app();
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Git;

    with_rendered_frame(&app, 180, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(!status_text.contains("s start"));
        assert!(!status_text.contains("x stop"));
        assert!(!status_text.contains("r restart"));
        assert!(!status_text.contains("Enter attach lazygit"));
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_shell_tab() {
    let mut app = fixture_app();
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.preview_tab = PreviewTab::Shell;

    with_rendered_frame(&app, 180, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(!status_text.contains("Enter attach shell"));
        assert!(!status_text.contains("j/k scroll"));
        assert!(!status_text.contains("s start"));
        assert!(!status_text.contains("x stop"));
        assert!(!status_text.contains("r restart"));
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
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
    let hidden_layout = GroveApp::view_layout_for_size(120, 40, 33, true);
    assert_eq!(hidden_layout.sidebar.width, 0);
    assert_eq!(hidden_layout.divider.width, 0);
    assert_eq!(hidden_layout.preview.width, 120);

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('\\')).with_kind(KeyEventKind::Press));
    assert!(!app.sidebar_hidden);
}

#[test]
fn uppercase_m_toggles_mouse_capture_and_emits_runtime_command() {
    let mut app = fixture_app();
    assert!(app.mouse_capture_enabled);

    let disable_cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('M')).with_kind(KeyEventKind::Press)),
    );

    assert!(!app.mouse_capture_enabled);
    assert!(cmd_contains_mouse_capture_toggle(&disable_cmd, false));

    let enable_cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('M')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.mouse_capture_enabled);
    assert!(cmd_contains_mouse_capture_toggle(&enable_cmd, true));
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
fn ctrl_k_control_character_opens_command_palette() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('\u{0b}')).with_kind(KeyEventKind::Press));

    assert!(app.command_palette.is_visible());
}

#[test]
fn ctrl_k_is_blocked_while_modal_is_open() {
    let mut app = fixture_app();
    app.open_create_dialog();
    assert!(app.create_dialog().is_some());

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('k'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );

    assert!(app.create_dialog().is_some());
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
    assert!(app.create_dialog().is_some());
}

#[test]
fn command_palette_ctrl_n_moves_selection_down() {
    let mut app = fixture_app();
    app.open_command_palette();
    assert!(app.command_palette.is_visible());
    assert!(app.command_palette.result_count() > 1);
    assert_eq!(app.command_palette.selected_index(), 0);

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('n'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );

    assert_eq!(app.command_palette.query(), "");
    assert_eq!(app.command_palette.selected_index(), 1);
}

#[test]
fn command_palette_repeat_down_arrow_moves_selection_down() {
    let mut app = fixture_app();
    app.open_command_palette();
    assert!(app.command_palette.is_visible());
    assert!(app.command_palette.result_count() > 1);
    assert_eq!(app.command_palette.selected_index(), 0);

    let _ = app.handle_key(KeyEvent::new(KeyCode::Down).with_kind(KeyEventKind::Repeat));

    assert_eq!(app.command_palette.query(), "");
    assert_eq!(app.command_palette.selected_index(), 1);
}

#[test]
fn command_palette_repeat_ctrl_n_moves_selection_down() {
    let mut app = fixture_app();
    app.open_command_palette();
    assert!(app.command_palette.is_visible());
    assert!(app.command_palette.result_count() > 1);
    assert_eq!(app.command_palette.selected_index(), 0);

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('n'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Repeat),
    );

    assert_eq!(app.command_palette.query(), "");
    assert_eq!(app.command_palette.selected_index(), 1);
}

#[test]
fn command_palette_ctrl_p_moves_selection_up() {
    let mut app = fixture_app();
    app.open_command_palette();
    assert!(app.command_palette.is_visible());
    assert!(app.command_palette.result_count() > 2);

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('n'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );
    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('n'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );
    assert_eq!(app.command_palette.selected_index(), 2);

    let _ = app.handle_key(
        KeyEvent::new(KeyCode::Char('p'))
            .with_modifiers(Modifiers::CTRL)
            .with_kind(KeyEventKind::Press),
    );

    assert_eq!(app.command_palette.query(), "");
    assert_eq!(app.command_palette.selected_index(), 1);
}

#[test]
fn command_palette_max_visible_scales_with_viewport_height() {
    assert_eq!(GroveApp::command_palette_max_visible_for_height(16), 11);
    assert_eq!(GroveApp::command_palette_max_visible_for_height(24), 17);
    assert_eq!(GroveApp::command_palette_max_visible_for_height(40), 30);
    assert_eq!(GroveApp::command_palette_max_visible_for_height(200), 30);
}

#[test]
fn command_palette_max_visible_prevents_overlay_clipping() {
    for viewport_height in 5u16..=120 {
        let max_visible = GroveApp::command_palette_max_visible_for_height(viewport_height);
        let top_offset = viewport_height / 6;
        let requested_rows = u16::try_from(max_visible).unwrap_or(u16::MAX);
        let palette_height = requested_rows
            .saturating_add(3)
            .max(5)
            .min(viewport_height.saturating_sub(2));
        assert!(
            top_offset.saturating_add(palette_height) <= viewport_height,
            "palette should fit viewport, height={viewport_height}, top_offset={top_offset}, max_visible={max_visible}, palette_height={palette_height}"
        );
    }
}

#[test]
fn open_command_palette_sizes_page_navigation_to_viewport_height() {
    let mut app = fixture_app();
    app.viewport_height = 16;
    app.open_command_palette();
    assert!(app.command_palette.is_visible());
    let expected_jump = GroveApp::command_palette_max_visible_for_height(app.viewport_height);
    assert!(app.command_palette.result_count() > expected_jump);

    let _ = app.handle_key(KeyEvent::new(KeyCode::PageDown).with_kind(KeyEventKind::Press));

    assert_eq!(app.command_palette.selected_index(), expected_jump);
}

#[test]
fn command_palette_overlay_uses_near_full_width() {
    let mut app = fixture_app();
    app.open_command_palette();

    with_rendered_frame(&app, 80, 24, |frame| {
        let title_row = find_row_containing(frame, "Command Palette", 0, frame.width())
            .expect("command palette title row should exist");
        let left_border = find_cell_with_char(frame, title_row, 0, frame.width(), '┌')
            .expect("left border should exist");
        let right_border = find_cell_with_char(frame, title_row, 0, frame.width(), '┐')
            .expect("right border should exist");
        assert!(
            left_border <= 4,
            "palette should use near-full width, left border x={left_border}"
        );
        assert!(
            right_border >= frame.width().saturating_sub(5),
            "palette should use near-full width, right border x={right_border}"
        );
    });
}

#[test]
fn command_palette_keeps_keybind_visible_on_narrow_width() {
    let mut app = fixture_app();
    app.open_command_palette();
    app.command_palette.set_query("toggle pane focus");

    with_rendered_frame(&app, 60, 24, |frame| {
        let has_keybind = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("[Tab/h/l]"));
        assert!(
            has_keybind,
            "expected keybind hint to remain visible in command palette rows"
        );
    });
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
            .any(|id| id == &palette_id(UiCommand::RefreshWorkspaces))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ToggleMouseCapture))
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
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ResizeSidebarNarrower))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ResizeSidebarWider))
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
            .any(|id| id == &palette_id(UiCommand::RestartAgent))
    );
    assert!(
        list_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::PreviousTab))
    );
    assert!(
        list_ids
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
        !preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::RestartAgent))
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
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ResizeSidebarNarrower))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ResizeSidebarWider))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::MoveSelectionDown))
    );
    assert!(
        preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ToggleMouseCapture))
    );
    assert!(
        !preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::EnterInteractive))
    );

    app.state.workspaces[1].status = WorkspaceStatus::Active;
    let running_preview_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(
        running_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::RestartAgent))
    );
    assert!(
        !running_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::StartAgent))
    );

    app.shell_sessions
        .ready
        .insert("grove-ws-feature-a-shell".to_string());
    let preview_ids_with_shell: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(
        preview_ids_with_shell
            .iter()
            .any(|id| id == &palette_id(UiCommand::EnterInteractive))
    );

    app.preview_tab = PreviewTab::Shell;
    let shell_preview_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    assert!(
        shell_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::ScrollDown))
    );
    assert!(
        shell_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::EnterInteractive))
    );
    assert!(
        !shell_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::StartAgent))
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
    assert!(
        git_preview_ids
            .iter()
            .any(|id| id == &palette_id(UiCommand::MoveSelectionDown))
    );
}

#[test]
fn command_palette_exposes_update_action_for_main_workspace() {
    let mut app = fixture_app();
    app.state.selected_index = 0;
    let list_ids: Vec<String> = app
        .build_command_palette_actions()
        .into_iter()
        .map(|action| action.id)
        .collect();
    let update_id = UiCommand::UpdateFromBase
        .palette_spec()
        .map(|spec| spec.id)
        .expect("command should be palette discoverable");
    assert!(list_ids.iter().any(|id| id == update_id));
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
fn ui_command_help_hint_labels_match_context_command_lists() {
    let contexts = [
        HelpHintContext::Global,
        HelpHintContext::Workspace,
        HelpHintContext::List,
        HelpHintContext::PreviewAgent,
        HelpHintContext::PreviewShell,
        HelpHintContext::PreviewGit,
    ];

    for context in contexts {
        let listed = UiCommand::help_hints_for(context);
        for command in UiCommand::all() {
            let is_listed = listed
                .iter()
                .any(|listed_command| listed_command == command);
            let has_label = command.help_hint_label(context).is_some();
            assert_eq!(
                is_listed, has_label,
                "context {:?} command {:?} should have list/label parity",
                context, command
            );
        }
    }
}

#[test]
fn uppercase_s_opens_settings_dialog() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));

    assert!(app.settings_dialog().is_some());
}

#[test]
fn uppercase_r_refreshes_workspaces_from_list_mode() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.refresh_in_flight);
    assert!(cmd_contains_task(&cmd));
    assert!(app.status_bar_line().contains("refreshing workspaces"));
}

#[test]
fn uppercase_r_is_debounced_after_recent_manual_refresh() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);

    let _ = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
    );
    assert!(app.refresh_in_flight);

    app.apply_refresh_workspaces_completion(RefreshWorkspacesCompletion {
        preferred_workspace_path: None,
        bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
    });
    assert!(!app.refresh_in_flight);

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
    );

    assert!(!app.refresh_in_flight);
    assert!(!cmd_contains_task(&cmd));
    assert!(app.status_bar_line().contains("refresh throttled"));

    app.last_manual_refresh_requested_at = Some(Instant::now() - Duration::from_secs(11));
    let cmd_after_cooldown = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.refresh_in_flight);
    assert!(cmd_contains_task(&cmd_after_cooldown));
}

#[test]
fn manual_refresh_completion_shows_success_toast() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);

    let _ = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
    );
    assert!(app.refresh_in_flight);

    app.apply_refresh_workspaces_completion(RefreshWorkspacesCompletion {
        preferred_workspace_path: None,
        bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
    });

    assert!(!app.refresh_in_flight);
    assert!(app.status_bar_line().contains("workspace refresh complete"));
}

#[test]
fn manual_refresh_completion_shows_error_toast() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);

    let _ = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
    );
    assert!(app.refresh_in_flight);

    app.apply_refresh_workspaces_completion(RefreshWorkspacesCompletion {
        preferred_workspace_path: None,
        bootstrap: BootstrapData {
            repo_name: "grove".to_string(),
            workspaces: Vec::new(),
            discovery_state: DiscoveryState::Error("github unavailable".to_string()),
        },
    });

    assert!(!app.refresh_in_flight);
    assert!(app.status_bar_line().contains("workspace refresh failed"));
}

#[test]
fn settings_dialog_save_does_not_write_global_config() {
    let mut app = fixture_app();

    let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
    assert!(app.settings_dialog().is_some());

    let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

    assert!(app.settings_dialog().is_none());
    assert!(!app.config_path.exists());
    assert!(
        app.status_bar_line()
            .contains("settings are read-only, edit ~/.config/grove/config.toml")
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
fn keybind_help_overlay_renders_when_help_modal_open() {
    let mut app = fixture_app();
    app.keybind_help_open = true;

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_text = (0..frame.height())
            .map(|row| row_text(frame, row, 0, frame.width()))
            .collect::<Vec<String>>()
            .join("\n");
        assert!(
            status_text.contains("Keybind Help"),
            "help overlay missing title: {status_text}"
        );
        assert!(
            status_text.contains("[Global]"),
            "help overlay missing global section: {status_text}"
        );
    });
}

#[test]
fn keybind_help_lists_interactive_reserved_keys() {
    let mut app = fixture_app();
    app.keybind_help_open = true;

    with_rendered_frame(&app, 160, 28, |frame| {
        let has_shift_tab = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("Shift+Tab"));
        let has_shift_enter = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("Shift+Enter"));
        let has_reserved = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("Ctrl+K palette"));
        let has_palette_nav = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("[Palette] Type search"));

        assert!(has_shift_tab);
        assert!(has_shift_enter);
        assert!(has_reserved);
        assert!(has_palette_nav);
    });
}

#[test]
fn keybind_help_lists_mouse_capture_toggle() {
    let mut app = fixture_app();
    app.keybind_help_open = true;

    with_rendered_frame(&app, 160, 28, |frame| {
        let has_mouse_toggle = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("M toggle mouse capture"));
        assert!(has_mouse_toggle);
    });
}

#[test]
fn keybind_help_lists_projects_modal_shortcuts_without_truncation() {
    let mut app = fixture_app();
    app.keybind_help_open = true;

    with_rendered_frame(&app, 160, 32, |frame| {
        let has_projects_remove = (0..frame.height())
            .any(|row| row_text(frame, row, 0, frame.width()).contains("Ctrl+X/Del remove"));
        assert!(has_projects_remove);
    });
}

#[test]
fn keybind_help_uses_available_height_to_show_footer() {
    let mut app = fixture_app();
    app.keybind_help_open = true;

    with_rendered_frame(&app, 160, 32, |frame| {
        let has_close_hint = (0..frame.height()).any(|row| {
            row_text(frame, row, 0, frame.width()).contains("Close help: Esc, Enter, or ?")
        });
        assert!(has_close_hint);
    });
}

#[test]
fn status_row_shows_palette_hints_when_palette_open() {
    let mut app = fixture_app();
    app.open_command_palette();

    with_rendered_frame(&app, 120, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Palette"));
        assert!(
            !status_text.trim().is_empty(),
            "status row should remain visible with palette open, got: {status_text}"
        );
    });
}

#[test]
fn status_row_uses_ui_mode_as_state_chip() {
    let app = fixture_app();

    with_rendered_frame(&app, 120, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains(" List "));
        assert!(!status_text.contains(" Context "));
    });
}

#[test]
fn project_dialog_keeps_compact_footer() {
    let mut app = fixture_app();
    app.open_project_dialog();

    with_rendered_frame(&app, 60, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn launch_dialog_keeps_compact_footer() {
    let mut app = fixture_app();
    app.set_launch_dialog(LaunchDialogState {
        start_config: StartAgentConfigState::new(String::new(), String::new(), false),
        focused_field: LaunchDialogField::StartConfig(StartAgentConfigField::Prompt),
    });

    with_rendered_frame(&app, 60, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn create_dialog_wrapped_hint_rows_keep_hint_style() {
    let mut app = fixture_app();
    app.open_create_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let dialog_width = frame.width().saturating_sub(8).min(90);
        let dialog_height = 25u16;
        let dialog_x = frame.width().saturating_sub(dialog_width) / 2;
        let dialog_y = frame.height().saturating_sub(dialog_height) / 2;
        let x_start = dialog_x.saturating_add(1);
        let x_end = dialog_x.saturating_add(dialog_width.saturating_sub(1));
        let y_start = dialog_y.saturating_add(1);
        let y_end = dialog_y.saturating_add(dialog_height.saturating_sub(1));

        let Some(first_row) = (y_start..y_end)
            .find(|&row| row_text(frame, row, x_start, x_end).contains("Tab/C-n next"))
        else {
            panic!("create dialog wrapped hint first row should be rendered");
        };
        let Some(second_row) = ((first_row.saturating_add(1))..y_end)
            .find(|&row| row_text(frame, row, x_start, x_end).contains("unsafe, Enter create"))
        else {
            panic!("create dialog wrapped hint second row should be rendered");
        };

        assert_row_fg(frame, first_row, x_start, x_end, ui_theme().overlay0);
        assert_row_fg(frame, second_row, x_start, x_end, ui_theme().overlay0);
    });
}

#[test]
fn status_row_keeps_compact_footer_in_interactive_mode() {
    let mut app = fixture_app();
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
    ));

    with_rendered_frame(&app, 160, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Interactive"));
        assert!(status_text.contains("project: grove"));
        assert!(status_text.contains("workspace: grove"));
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
        assert!(!status_text.contains("Esc Esc/Ctrl+\\ exit"));
        assert!(!status_text.contains("Alt+C copy"));
    });
}

#[test]
fn toast_overlay_renders_message() {
    let mut app = fixture_app();
    app.show_success_toast("Copied 2 line(s)");

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
fn info_toast_uses_info_style_and_duration() {
    let mut app = fixture_app();
    app.show_info_toast("mouse capture enabled");

    let Some(toast) = app.notifications.visible().last() else {
        panic!("info toast should be visible");
    };
    assert!(matches!(toast.config.style_variant, ToastStyle::Info));
    assert_eq!(toast.config.duration, Some(Duration::from_secs(6)));
    assert_eq!(toast.content.title.as_deref(), Some("Info"));
    assert_eq!(
        toast.content.icon,
        Some(ftui::widgets::toast::ToastIcon::Info)
    );

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(_message_row) =
            find_row_containing(frame, "mouse capture enabled", 0, frame.width())
        else {
            panic!("info toast message row should render");
        };
    });
}

#[test]
fn error_toast_uses_error_style_and_long_duration() {
    let mut app = fixture_app();
    app.show_error_toast("agent start failed");

    let Some(toast) = app.notifications.visible().last() else {
        panic!("error toast should be visible");
    };
    assert!(matches!(toast.config.style_variant, ToastStyle::Error));
    assert_eq!(toast.config.duration, Some(Duration::from_secs(12)));
    assert_eq!(toast.content.title.as_deref(), Some("Error"));
    assert_eq!(
        toast.content.icon,
        Some(ftui::widgets::toast::ToastIcon::Error)
    );

    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(_message_row) = find_row_containing(frame, "agent start failed", 0, frame.width())
        else {
            panic!("error toast message row should render");
        };
    });
}

#[test]
fn toast_messages_are_sanitized_and_truncated_to_fit_width() {
    let mut app = fixture_app();
    let long_message = format!("first line\nsecond line {}", "x".repeat(280));
    app.show_error_toast(long_message);

    let Some(toast) = app.notifications.visible().last() else {
        panic!("toast should be visible");
    };
    assert!(!toast.content.message.contains('\n'));
    assert!(!toast.content.message.contains('\r'));
    assert!(toast.content.message.ends_with('…'));
    let max_message_width = usize::from(toast.config.max_width)
        .saturating_sub(8)
        .max(16);
    assert!(ftui::text::display_width(toast.content.message.as_str()) <= max_message_width);
}

#[test]
fn status_row_keeps_compact_footer_in_create_dialog() {
    let mut app = fixture_app();
    app.open_create_dialog();

    with_rendered_frame(&app, 140, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("Dialog: Create"));
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_edit_dialog() {
    let mut app = fixture_app();
    app.open_edit_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_launch_dialog() {
    let mut app = fixture_app();
    app.set_launch_dialog(LaunchDialogState {
        start_config: StartAgentConfigState::new(String::new(), String::new(), false),
        focused_field: LaunchDialogField::StartConfig(StartAgentConfigField::Prompt),
    });

    with_rendered_frame(&app, 140, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_stop_dialog() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.state.workspaces[1].status = WorkspaceStatus::Active;
    app.open_stop_dialog();

    with_rendered_frame(&app, 90, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_delete_dialog() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.open_delete_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_merge_dialog() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.open_merge_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
    });
}

#[test]
fn status_row_keeps_compact_footer_in_update_from_base_dialog() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.open_update_from_base_dialog();

    with_rendered_frame(&app, 80, 24, |frame| {
        let status_row = frame.height().saturating_sub(1);
        let status_text = row_text(frame, status_row, 0, frame.width());
        assert!(status_text.contains("? help"));
        assert!(status_text.contains("Ctrl+K palette"));
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

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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
fn preview_pane_border_is_blue_when_preview_focused_without_interactive_input() {
    let mut app = fixture_app();
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(corner_cell) = frame.buffer.get(layout.preview.x, layout.preview.y) else {
            panic!("preview border corner should exist");
        };
        assert_eq!(corner_cell.fg, ui_theme().blue);

        let title_text = row_text(
            frame,
            layout.preview.y,
            layout.preview.x.saturating_add(1),
            layout.preview.right().saturating_sub(1),
        );
        assert!(title_text.contains("Preview"));
        assert!(!title_text.contains("INSERT"));
    });
}

#[test]
fn interactive_preview_border_is_teal_and_shows_insert_label() {
    let mut app = fixture_app();
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.interactive = Some(InteractiveState::new(
        "%1".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
    ));

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
    with_rendered_frame(&app, 80, 24, |frame| {
        let Some(corner_cell) = frame.buffer.get(layout.preview.x, layout.preview.y) else {
            panic!("preview border corner should exist");
        };
        assert_eq!(corner_cell.fg, ui_theme().teal);

        let title_text = row_text(
            frame,
            layout.preview.y,
            layout.preview.x.saturating_add(1),
            layout.preview.right().saturating_sub(1),
        );
        assert!(title_text.contains("Preview"));
        assert!(title_text.contains("INSERT"));
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

    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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
    let layout = GroveApp::view_layout_for_size(80, 24, app.sidebar_width_pct, false);
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
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
fn mouse_click_on_workspace_pr_link_selects_workspace() {
    let mut app = fixture_app();
    app.state.selected_index = 0;
    app.state.workspaces[1].pull_requests = vec![PullRequest {
        number: 500,
        url: "https://github.com/acme/grove/pull/500".to_string(),
        status: PullRequestStatus::Open,
    }];

    let layout = GroveApp::view_layout_for_size(120, 24, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);
    let mut target = None;
    with_rendered_frame(&app, 120, 24, |frame| {
        let Some(feature_row) = find_row_containing(frame, "feature-a", x_start, x_end) else {
            panic!("feature row should be rendered");
        };
        let metadata_row = feature_row.saturating_add(1);
        let Some(icon_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '') else {
            panic!("PR icon should be rendered");
        };
        target = Some((icon_col, metadata_row));
    });
    let Some((target_x, target_y)) = target else {
        panic!("PR target should be captured");
    };

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            target_x,
            target_y,
        )),
    );

    assert_eq!(app.state.selected_index, 1);
}

#[test]
fn sidebar_keeps_selected_workspace_visible_with_many_rows() {
    let mut app = fixture_app();
    for index in 0..24usize {
        let mut workspace = Workspace::try_new(
            format!("extra-{index}"),
            PathBuf::from(format!("/repos/grove-extra-{index}")),
            format!("extra-{index}"),
            None,
            AgentType::Codex,
            WorkspaceStatus::Idle,
            false,
        )
        .expect("workspace should be valid");
        workspace.project_path = Some(PathBuf::from("/repos/grove"));
        app.state.workspaces.push(workspace);
    }

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 80,
            height: 16,
        },
    );
    for _ in 0..18 {
        ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('j'))));
    }

    let selected_name = app.state.workspaces[app.state.selected_index].name.clone();
    let layout = GroveApp::view_layout_for_size(80, 16, app.sidebar_width_pct, false);
    let x_start = layout.sidebar.x.saturating_add(1);
    let x_end = layout.sidebar.right().saturating_sub(1);
    with_rendered_frame(&app, 80, 16, |frame| {
        assert!(
            find_row_containing(frame, selected_name.as_str(), x_start, x_end).is_some(),
            "selected workspace should stay visible"
        );
    });
}

#[test]
fn mouse_wheel_on_sidebar_moves_workspace_selection() {
    let mut app = fixture_app();
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
    let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);

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
            MouseEventKind::ScrollDown,
            sidebar_inner.x.saturating_add(1),
            sidebar_inner.y.saturating_add(1),
        )),
    );
    assert_eq!(app.state.selected_index, 1);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::ScrollUp,
            sidebar_inner.x.saturating_add(1),
            sidebar_inner.y.saturating_add(1),
        )),
    );
    assert_eq!(app.state.selected_index, 0);
}

#[test]
fn sidebar_mouse_wheel_burst_same_direction_is_debounced() {
    let mut app = fixture_app();
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
    let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
    let x = sidebar_inner.x.saturating_add(1);
    let y = sidebar_inner.y.saturating_add(1);

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollDown, x, y)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollDown, x, y)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollDown, x, y)),
    );

    assert_eq!(app.state.selected_index, 1);
}

#[test]
fn sidebar_mouse_wheel_allows_fast_direction_change() {
    let mut app = fixture_app();
    let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
    let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
    let x = sidebar_inner.x.saturating_add(1);
    let y = sidebar_inner.y.saturating_add(1);

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollDown, x, y)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(MouseEventKind::ScrollUp, x, y)),
    );

    assert_eq!(app.state.selected_index, 0);
}

mod runtime_flow;

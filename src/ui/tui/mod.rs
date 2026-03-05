mod ansi;
#[path = "bootstrap/bootstrap_app.rs"]
mod bootstrap_app;
#[path = "bootstrap/bootstrap_config.rs"]
mod bootstrap_config;
#[path = "bootstrap/bootstrap_discovery.rs"]
mod bootstrap_discovery;
mod terminal;
#[macro_use]
mod shared;
#[path = "app/mod.rs"]
mod app;
#[path = "commands/catalog.rs"]
mod commands;
#[path = "commands/help.rs"]
mod commands_hints;
#[path = "commands/meta.rs"]
mod commands_meta;
#[path = "commands/palette.rs"]
mod commands_palette;
#[path = "dialogs/dialogs.rs"]
mod dialogs;
#[path = "dialogs/dialogs_confirm.rs"]
mod dialogs_confirm;
#[path = "dialogs/dialogs_create_key.rs"]
mod dialogs_create_key;
#[path = "dialogs/dialogs_create_setup.rs"]
mod dialogs_create_setup;
#[path = "dialogs/dialogs_delete.rs"]
mod dialogs_delete;
#[path = "dialogs/dialogs_edit.rs"]
mod dialogs_edit;
#[path = "dialogs/dialogs_launch.rs"]
mod dialogs_launch;
#[path = "dialogs/dialogs_merge.rs"]
mod dialogs_merge;
#[path = "dialogs/dialogs_projects_crud.rs"]
mod dialogs_projects_crud;
#[path = "dialogs/dialogs_projects_defaults.rs"]
mod dialogs_projects_defaults;
#[path = "dialogs/dialogs_projects_key.rs"]
mod dialogs_projects_key;
#[path = "dialogs/dialogs_projects_reorder.rs"]
mod dialogs_projects_reorder;
#[path = "dialogs/dialogs_projects_state.rs"]
mod dialogs_projects_state;
#[path = "dialogs/dialogs_session_cleanup.rs"]
mod dialogs_session_cleanup;
#[path = "dialogs/dialogs_settings.rs"]
mod dialogs_settings;
#[path = "dialogs/state.rs"]
mod dialogs_state;
#[path = "dialogs/dialogs_stop.rs"]
mod dialogs_stop;
#[path = "dialogs/dialogs_update_from_base.rs"]
mod dialogs_update_from_base;
#[path = "logging/logging_frame.rs"]
mod logging_frame;
#[path = "logging/logging_input.rs"]
mod logging_input;
#[path = "logging/logging_state.rs"]
mod logging_state;
mod msg;
mod runner;
mod selection;
pub use runner::{run_with_debug_record, run_with_event_log};
mod replay;
pub use replay::{ReplayOptions, emit_replay_fixture, replay_debug_record};
mod text;
#[path = "update/update.rs"]
mod update;
#[path = "update/update_core.rs"]
mod update_core;
#[path = "update/update_input_interactive.rs"]
mod update_input_interactive;
#[path = "update/update_input_interactive_clipboard.rs"]
mod update_input_interactive_clipboard;
#[path = "update/update_input_interactive_send.rs"]
mod update_input_interactive_send;
#[path = "update/update_input_key_events.rs"]
mod update_input_key_events;
#[path = "update/update_input_keybinding.rs"]
mod update_input_keybinding;
#[path = "update/update_input_mouse.rs"]
mod update_input_mouse;
#[path = "update/update_lifecycle_create.rs"]
mod update_lifecycle_create;
#[path = "update/update_lifecycle_start.rs"]
mod update_lifecycle_start;
#[path = "update/update_lifecycle_stop.rs"]
mod update_lifecycle_stop;
#[path = "update/update_lifecycle_workspace_completion.rs"]
mod update_lifecycle_workspace_completion;
#[path = "update/update_lifecycle_workspace_refresh.rs"]
mod update_lifecycle_workspace_refresh;
#[path = "update/update_navigation_commands.rs"]
mod update_navigation_commands;
#[path = "update/update_navigation_palette.rs"]
mod update_navigation_palette;
#[path = "update/update_navigation_preview.rs"]
mod update_navigation_preview;
#[path = "update/update_navigation_tabs.rs"]
mod update_navigation_tabs;
#[path = "update/update_polling_capture_cursor.rs"]
mod update_polling_capture_cursor;
#[path = "update/update_polling_capture_dispatch.rs"]
mod update_polling_capture_dispatch;
#[path = "update/update_polling_capture_live.rs"]
mod update_polling_capture_live;
#[path = "update/update_polling_capture_task.rs"]
mod update_polling_capture_task;
#[path = "update/update_polling_capture_workspace.rs"]
mod update_polling_capture_workspace;
#[path = "update/update_polling_state.rs"]
mod update_polling_state;
#[path = "update/prelude.rs"]
mod update_prelude;
#[path = "update/update_tick.rs"]
mod update_tick;
#[path = "view/view.rs"]
mod view;
#[path = "view/view_chrome_divider.rs"]
mod view_chrome_divider;
#[path = "view/view_chrome_header.rs"]
mod view_chrome_header;
#[path = "view/view_chrome_shared.rs"]
mod view_chrome_shared;
#[path = "view/view_chrome_sidebar.rs"]
mod view_chrome_sidebar;
#[path = "view/view_layout.rs"]
mod view_layout;
#[path = "view/view_overlays_confirm.rs"]
mod view_overlays_confirm;
#[path = "view/view_overlays_create.rs"]
mod view_overlays_create;
#[path = "view/view_overlays_edit.rs"]
mod view_overlays_edit;
#[path = "view/view_overlays_help.rs"]
mod view_overlays_help;
#[path = "view/view_overlays_projects.rs"]
mod view_overlays_projects;
#[path = "view/view_overlays_session_cleanup.rs"]
mod view_overlays_session_cleanup;
#[path = "view/view_overlays_settings.rs"]
mod view_overlays_settings;
#[path = "view/view_overlays_workspace_delete.rs"]
mod view_overlays_workspace_delete;
#[path = "view/view_overlays_workspace_launch.rs"]
mod view_overlays_workspace_launch;
#[path = "view/view_overlays_workspace_merge.rs"]
mod view_overlays_workspace_merge;
#[path = "view/view_overlays_workspace_stop.rs"]
mod view_overlays_workspace_stop;
#[path = "view/view_overlays_workspace_update.rs"]
mod view_overlays_workspace_update;
#[path = "view/prelude.rs"]
mod view_prelude;
#[path = "view/view_preview.rs"]
mod view_preview;
#[path = "view/view_preview_content.rs"]
mod view_preview_content;
#[path = "view/view_preview_shell.rs"]
mod view_preview_shell;
#[path = "view/view_selection_interaction.rs"]
mod view_selection_interaction;
#[path = "view/view_selection_logging.rs"]
mod view_selection_logging;
#[path = "view/view_selection_mapping.rs"]
mod view_selection_mapping;
#[path = "view/view_status.rs"]
mod view_status;

include!("model.rs");

#[cfg(test)]
mod tests {
    mod render_support {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/render.rs"
        ));
    }
    mod support {
        pub(super) mod logging {
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
        }
    }

    use self::render_support::{
        assert_row_bg, assert_row_fg, find_cell_with_char, find_row_containing, row_text,
    };
    use self::support::logging::{RecordedEvents, RecordingEventLogger};
    use super::{
        AppDependencies, ClipboardAccess, CommandTmuxInput, CreateDialogField, CreateDialogTab,
        CreateWorkspaceCompletion, CursorCapture, DeleteDialogField, DeleteProjectCompletion,
        DeleteWorkspaceCompletion, EditDialogField, GroveApp, HIT_ID_CREATE_DIALOG_TAB,
        HIT_ID_HEADER, HIT_ID_PREVIEW, HIT_ID_STATUS, HIT_ID_WORKSPACE_LIST,
        HIT_ID_WORKSPACE_PR_LINK, HIT_ID_WORKSPACE_ROW, HelpHintContext, LaunchDialogField,
        LaunchDialogState, LazygitLaunchCompletion, LivePreviewCapture, MergeDialogField,
        MergeWorkspaceCompletion, Msg, PREVIEW_METADATA_ROWS, PendingResizeVerification,
        PreviewPollCompletion, PreviewTab, ProjectAddDialogField, ProjectDefaultsDialogField,
        RefreshWorkspacesCompletion, SettingsDialogField, StartAgentCompletion,
        StartAgentConfigField, StartAgentConfigState, StopAgentCompletion, StopDialogField,
        TextSelectionPoint, TmuxInput, UiCommand, UpdateFromBaseDialogField, WORKSPACE_ITEM_HEIGHT,
        WorkspaceAttention, WorkspaceShellLaunchCompletion, WorkspaceStatusCapture,
        WorkspaceTabKind, ansi_16_color, ansi_lines_to_styled_lines,
        ansi_lines_to_styled_lines_for_theme, decode_create_dialog_tab_hit_data,
        decode_workspace_pr_hit_data, parse_cursor_metadata, ui_theme, ui_theme_for, usize_to_u64,
    };
    use crate::application::interactive::InteractiveState;
    use crate::application::services::runtime_service::workspace_status_targets_for_polling_with_live_preview;
    use crate::application::workspace_lifecycle::{
        BranchMode, CreateWorkspaceRequest, CreateWorkspaceResult,
    };
    use crate::domain::{AgentType, PullRequest, PullRequestStatus, Workspace, WorkspaceStatus};
    use crate::infrastructure::adapters::{BootstrapData, DiscoveryState};
    use crate::infrastructure::config::{ProjectConfig, ThemeName};
    use crate::infrastructure::event_log::{Event as LoggedEvent, NullEventLogger};
    use crate::ui::state::{PaneFocus, UiMode};
    use ftui::core::event::{
        Event, KeyCode, KeyEvent, KeyEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
        PasteEvent,
    };
    use ftui::render::frame::HitId;
    use ftui::widgets::block::Block;
    use ftui::widgets::borders::Borders;
    use ftui::widgets::toast::ToastStyle;
    use ftui::{Cmd, Frame, GraphemePool, PackedRgba};
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

    #[derive(Clone)]
    struct RestoreMetadataTmuxInput {
        rows: String,
    }

    impl TmuxInput for RestoreMetadataTmuxInput {
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

        fn list_sessions_with_tab_metadata(&self) -> std::io::Result<String> {
            Ok(self.rows.clone())
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
        app.polling.next_tick_due_at = Some(now);
        app.polling.next_poll_due_at = Some(now);
    }

    fn cmd_contains_task(cmd: &Cmd<Msg>) -> bool {
        match cmd {
            Cmd::Task(_, _) => true,
            Cmd::Batch(commands) | Cmd::Sequence(commands) => {
                commands.iter().any(cmd_contains_task)
            }
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

    #[test]
    fn startup_restores_workspace_tabs_from_tmux_metadata() {
        let rows = "\
grove-ws-feature-a-agent-1\t/repos/grove-feature-a\tagent\tCodex 1\tcodex\t9\n\
grove-ws-feature-a-shell-1\t/repos/grove-feature-a\tshell\tShell 1\t\t10\n\
grove-ws-feature-a-git\t/repos/grove-feature-a\tgit\tGit\t\t11\n";
        let app = GroveApp::from_parts_with_clipboard_and_projects(
            fixture_bootstrap(WorkspaceStatus::Idle),
            fixture_projects(),
            AppDependencies {
                tmux_input: Box::new(RestoreMetadataTmuxInput {
                    rows: rows.to_string(),
                }),
                clipboard: test_clipboard(),
                config_path: unique_config_path("restore-tabs"),
                event_log: Box::new(NullEventLogger),
                debug_record_start_ts: None,
            },
        );

        let workspace_path = PathBuf::from("/repos/grove-feature-a");
        let tabs = app
            .workspace_tabs
            .get(workspace_path.as_path())
            .expect("feature workspace tabs should exist");
        assert_eq!(
            tabs.tabs
                .iter()
                .map(|tab| tab.kind)
                .collect::<Vec<WorkspaceTabKind>>(),
            vec![
                WorkspaceTabKind::Home,
                WorkspaceTabKind::Agent,
                WorkspaceTabKind::Shell,
                WorkspaceTabKind::Git,
            ],
        );
        assert_eq!(
            tabs.tabs
                .iter()
                .filter_map(|tab| tab.session_name.clone())
                .collect::<Vec<String>>(),
            vec![
                "grove-ws-feature-a-agent-1".to_string(),
                "grove-ws-feature-a-shell-1".to_string(),
                "grove-ws-feature-a-git".to_string(),
            ],
        );
    }

    fn with_rendered_frame(
        app: &GroveApp,
        width: u16,
        height: u16,
        assert_frame: impl FnOnce(&Frame),
    ) {
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
                    app.dialogs.keybind_help_open,
                    app.dialogs.command_palette.is_visible(),
                    app.session.interactive.is_some(),
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
        app.dialogs.delete_in_flight = true;
        app.dialogs.delete_in_flight_workspace = Some(app.state.workspaces[1].path.clone());
        app.dialogs
            .delete_requested_workspaces
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

            let Some(open_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '')
            else {
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
            let Some(icon_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '')
            else {
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
        app.polling.output_changing = false;
        app.polling.agent_output_changing = false;
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
        app.polling.output_changing = true;

        assert_eq!(
            app.live_preview_scrollback_lines(),
            super::LIVE_PREVIEW_SCROLLBACK_LINES
        );
    }

    #[test]
    fn live_preview_scrollback_lines_uses_full_window_in_interactive_mode() {
        let (mut app, _commands, _captures, _cursor_captures) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        app.session.interactive = Some(InteractiveState::new(
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
        app.polling.output_changing = false;
        app.polling.agent_output_changing = false;
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
        app.polling.output_changing = true;
        app.polling.agent_output_changing = true;
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
        app.polling.output_changing = false;
        app.polling.agent_output_changing = false;
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
        app.polling.output_changing = true;
        app.polling.agent_output_changing = true;
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
        idle_app.polling.output_changing = false;
        idle_app.polling.agent_output_changing = false;

        let (mut active_app, _commands2, _captures2, _cursor_captures2) =
            fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
        active_app.state.selected_index = 1;
        active_app.polling.output_changing = true;
        active_app.polling.agent_output_changing = true;

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
                let active_status =
                    row_text(active_frame, active_status_row, 0, active_frame.width());
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
        app.polling.output_changing = true;
        app.polling.agent_output_changing = false;
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
            agent: AgentType::Claude,
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
            agent: AgentType::Claude,
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
    fn create_dialog_selected_project_row_uses_highlight_background() {
        let mut app = fixture_app();
        app.open_create_dialog();
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

            let Some(selected_row) = find_dialog_row("[Project] grove") else {
                panic!("selected project row should be rendered");
            };
            assert_row_bg(frame, selected_row, x_start, x_end, ui_theme().surface1);

            let Some(unselected_row) = find_dialog_row("[Name]") else {
                panic!("unselected name row should be rendered");
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
    fn create_dialog_unfocused_project_row_uses_base_background() {
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

            let Some(project_row) = find_dialog_row("[Project] grove") else {
                panic!("project row should be rendered");
            };
            assert_row_bg(frame, project_row, x_start, x_end, ui_theme().base);
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

        let mut manual_click: Option<(u16, u16)> = None;
        let mut pull_request_click: Option<(u16, u16)> = None;
        with_rendered_frame(&app, 80, 24, |frame| {
            for y in 0..frame.height() {
                for x in 0..frame.width() {
                    let Some((hit_id, _region, hit_data)) = frame.hit_test(x, y) else {
                        continue;
                    };
                    if hit_id != HitId::new(HIT_ID_CREATE_DIALOG_TAB) {
                        continue;
                    }
                    match decode_create_dialog_tab_hit_data(hit_data) {
                        Some(CreateDialogTab::Manual) => {
                            if manual_click.is_none() {
                                manual_click = Some((x, y));
                            }
                        }
                        Some(CreateDialogTab::PullRequest) => {
                            if pull_request_click.is_none() {
                                pull_request_click = Some((x, y));
                            }
                        }
                        None => {}
                    }
                }
            }
        });

        let Some((manual_x, manual_y)) = manual_click else {
            panic!("manual mode tab hit target should be present");
        };
        let Some((pull_request_x, pull_request_y)) = pull_request_click else {
            panic!("pull-request mode tab hit target should be present");
        };

        ftui::Model::update(
            &mut app,
            Msg::Mouse(MouseEvent::new(
                MouseEventKind::Down(MouseButton::Left),
                pull_request_x,
                pull_request_y,
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
                manual_x,
                manual_y,
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

        assert!(app.dialogs.keybind_help_open);
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
        app.dialogs.keybind_help_open = true;

        let _ = app.handle_key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press));

        assert!(!app.dialogs.keybind_help_open);
    }

    #[test]
    fn keybind_help_modal_blocks_navigation_keys() {
        let mut app = fixture_app();
        app.dialogs.keybind_help_open = true;
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

        assert!(app.dialogs.command_palette.is_visible());
    }

    #[test]
    fn ctrl_k_control_character_opens_command_palette() {
        let mut app = fixture_app();

        let _ =
            app.handle_key(KeyEvent::new(KeyCode::Char('\u{0b}')).with_kind(KeyEventKind::Press));

        assert!(app.dialogs.command_palette.is_visible());
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
        assert!(!app.dialogs.command_palette.is_visible());
    }

    #[test]
    fn ctrl_k_is_blocked_in_interactive_mode() {
        let mut app = fixture_app();
        app.session.interactive = Some(InteractiveState::new(
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

        assert!(app.session.interactive.is_some());
        assert!(!app.dialogs.command_palette.is_visible());
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
        assert!(app.dialogs.command_palette.is_visible());

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press));

        assert_eq!(app.state.selected_index, selected_before);
        assert_eq!(app.dialogs.command_palette.query(), "j");
    }

    #[test]
    fn command_palette_enter_executes_selected_action() {
        let mut app = fixture_app();

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('k'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );
        assert!(app.dialogs.command_palette.is_visible());

        for character in ['n', 'e', 'w'] {
            let _ = app
                .handle_key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press));
        }
        let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

        assert!(!app.dialogs.command_palette.is_visible());
        assert!(app.create_dialog().is_some());
    }

    #[test]
    fn command_palette_ctrl_n_moves_selection_down() {
        let mut app = fixture_app();
        app.open_command_palette();
        assert!(app.dialogs.command_palette.is_visible());
        assert!(app.dialogs.command_palette.result_count() > 1);
        assert_eq!(app.dialogs.command_palette.selected_index(), 0);

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('n'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );

        assert_eq!(app.dialogs.command_palette.query(), "");
        assert_eq!(app.dialogs.command_palette.selected_index(), 1);
    }

    #[test]
    fn command_palette_repeat_down_arrow_moves_selection_down() {
        let mut app = fixture_app();
        app.open_command_palette();
        assert!(app.dialogs.command_palette.is_visible());
        assert!(app.dialogs.command_palette.result_count() > 1);
        assert_eq!(app.dialogs.command_palette.selected_index(), 0);

        let _ = app.handle_key(KeyEvent::new(KeyCode::Down).with_kind(KeyEventKind::Repeat));

        assert_eq!(app.dialogs.command_palette.query(), "");
        assert_eq!(app.dialogs.command_palette.selected_index(), 1);
    }

    #[test]
    fn command_palette_repeat_ctrl_n_moves_selection_down() {
        let mut app = fixture_app();
        app.open_command_palette();
        assert!(app.dialogs.command_palette.is_visible());
        assert!(app.dialogs.command_palette.result_count() > 1);
        assert_eq!(app.dialogs.command_palette.selected_index(), 0);

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('n'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Repeat),
        );

        assert_eq!(app.dialogs.command_palette.query(), "");
        assert_eq!(app.dialogs.command_palette.selected_index(), 1);
    }

    #[test]
    fn command_palette_ctrl_p_moves_selection_up() {
        let mut app = fixture_app();
        app.open_command_palette();
        assert!(app.dialogs.command_palette.is_visible());
        assert!(app.dialogs.command_palette.result_count() > 2);

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
        assert_eq!(app.dialogs.command_palette.selected_index(), 2);

        let _ = app.handle_key(
            KeyEvent::new(KeyCode::Char('p'))
                .with_modifiers(Modifiers::CTRL)
                .with_kind(KeyEventKind::Press),
        );

        assert_eq!(app.dialogs.command_palette.query(), "");
        assert_eq!(app.dialogs.command_palette.selected_index(), 1);
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
        assert!(app.dialogs.command_palette.is_visible());
        let expected_jump = GroveApp::command_palette_max_visible_for_height(app.viewport_height);
        assert!(app.dialogs.command_palette.result_count() > expected_jump);

        let _ = app.handle_key(KeyEvent::new(KeyCode::PageDown).with_kind(KeyEventKind::Press));

        assert_eq!(app.dialogs.command_palette.selected_index(), expected_jump);
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
        app.dialogs.command_palette.set_query("toggle pane focus");

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

        app.session
            .shell_sessions
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
    fn command_palette_exposes_cleanup_sessions_action() {
        let app = fixture_app();
        let list_ids: Vec<String> = app
            .build_command_palette_actions()
            .into_iter()
            .map(|action| action.id)
            .collect();
        let cleanup_id = UiCommand::CleanupSessions
            .palette_spec()
            .map(|spec| spec.id)
            .expect("cleanup command should be palette discoverable");
        assert!(list_ids.iter().any(|id| id == cleanup_id));
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
    fn ui_command_registry_covers_every_command() {
        let mut meta_count = 0;
        for command in UiCommand::all() {
            let _ = command.meta();
            meta_count += 1;
        }
        assert_eq!(meta_count, UiCommand::all().len());
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
    fn ui_command_help_hints_have_unique_contexts_per_command() {
        for command in UiCommand::all() {
            let help_hints = command.meta().help_hints;
            for (index, left) in help_hints.iter().enumerate() {
                for right in help_hints.iter().skip(index + 1) {
                    assert_ne!(
                        left.context, right.context,
                        "duplicate help hint context {:?} for command {:?}",
                        left.context, command
                    );
                }
            }
        }
    }

    #[test]
    fn ui_command_metadata_counts_match_expected_snapshot() {
        assert_eq!(
            UiCommand::all()
                .iter()
                .filter(|command| command.meta().palette.is_some())
                .count(),
            34
        );
        assert_eq!(UiCommand::help_hints_for(HelpHintContext::Global).len(), 13);
        assert_eq!(
            UiCommand::help_hints_for(HelpHintContext::Workspace).len(),
            10
        );
        assert_eq!(UiCommand::help_hints_for(HelpHintContext::List).len(), 1);
        assert_eq!(
            UiCommand::help_hints_for(HelpHintContext::PreviewAgent).len(),
            9
        );
        assert_eq!(
            UiCommand::help_hints_for(HelpHintContext::PreviewShell).len(),
            6
        );
        assert_eq!(
            UiCommand::help_hints_for(HelpHintContext::PreviewGit).len(),
            3
        );
    }

    #[test]
    fn ui_command_keybound_commands_are_discoverable() {
        let contexts = [
            HelpHintContext::Global,
            HelpHintContext::Workspace,
            HelpHintContext::List,
            HelpHintContext::PreviewAgent,
            HelpHintContext::PreviewShell,
            HelpHintContext::PreviewGit,
        ];
        for command in UiCommand::all() {
            if command.keybindings().is_empty() {
                continue;
            }
            let has_help_hint = contexts
                .iter()
                .any(|context| command.help_hint_label(*context).is_some());
            assert!(
                has_help_hint || command.palette_spec().is_some(),
                "keybound command {:?} must be discoverable in help and/or palette",
                command
            );
        }
    }

    #[test]
    fn ui_command_keybinding_specs_have_no_duplicates_per_command() {
        for command in UiCommand::all() {
            let keybindings = command.keybindings();
            for (index, left) in keybindings.iter().enumerate() {
                for right in keybindings.iter().skip(index + 1) {
                    assert_ne!(
                        left, right,
                        "duplicate keybinding spec for command {:?}",
                        command
                    );
                }
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

        assert!(app.dialogs.refresh_in_flight);
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
        assert!(app.dialogs.refresh_in_flight);

        app.apply_refresh_workspaces_completion(RefreshWorkspacesCompletion {
            preferred_workspace_path: None,
            bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
        });
        assert!(!app.dialogs.refresh_in_flight);

        let cmd = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
        );

        assert!(!app.dialogs.refresh_in_flight);
        assert!(!cmd_contains_task(&cmd));
        assert!(app.status_bar_line().contains("refresh throttled"));

        app.dialogs.last_manual_refresh_requested_at =
            Some(Instant::now() - Duration::from_secs(11));
        let cmd_after_cooldown = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
        );

        assert!(app.dialogs.refresh_in_flight);
        assert!(cmd_contains_task(&cmd_after_cooldown));
    }

    #[test]
    fn manual_refresh_completion_shows_success_toast() {
        let mut app = fixture_background_app(WorkspaceStatus::Idle);

        let _ = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
        );
        assert!(app.dialogs.refresh_in_flight);

        app.apply_refresh_workspaces_completion(RefreshWorkspacesCompletion {
            preferred_workspace_path: None,
            bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
        });

        assert!(!app.dialogs.refresh_in_flight);
        assert!(app.status_bar_line().contains("workspace refresh complete"));
    }

    #[test]
    fn manual_refresh_completion_shows_error_toast() {
        let mut app = fixture_background_app(WorkspaceStatus::Idle);

        let _ = ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char('R')).with_kind(KeyEventKind::Press)),
        );
        assert!(app.dialogs.refresh_in_flight);

        app.apply_refresh_workspaces_completion(RefreshWorkspacesCompletion {
            preferred_workspace_path: None,
            bootstrap: BootstrapData {
                repo_name: "grove".to_string(),
                workspaces: Vec::new(),
                discovery_state: DiscoveryState::Error("github unavailable".to_string()),
            },
        });

        assert!(!app.dialogs.refresh_in_flight);
        assert!(app.status_bar_line().contains("workspace refresh failed"));
    }

    #[test]
    fn settings_dialog_save_persists_selected_theme() {
        let mut app = fixture_app();
        assert_eq!(app.theme_name, ThemeName::CatppuccinMocha);

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
        assert!(app.settings_dialog().is_some());
        assert_eq!(
            app.settings_dialog().map(|dialog| dialog.focused_field),
            Some(SettingsDialogField::Theme)
        );

        let _ = app.handle_key(KeyEvent::new(KeyCode::Right).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press));

        assert!(app.settings_dialog().is_none());
        assert_eq!(app.theme_name, ThemeName::RosePine);
        assert!(app.status_bar_line().contains("theme saved: rose-pine"));

        let loaded = crate::infrastructure::config::load_from_path(&app.config_path)
            .expect("config should load");
        assert_eq!(loaded.theme, ThemeName::RosePine);
    }

    #[test]
    fn settings_dialog_theme_preview_applies_while_cycling() {
        let mut app = fixture_app();
        assert_eq!(app.theme_name, ThemeName::CatppuccinMocha);

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Right).with_kind(KeyEventKind::Press));

        assert_eq!(
            app.settings_dialog().map(|dialog| dialog.theme),
            Some(ThemeName::RosePine)
        );
        assert_eq!(app.theme_name, ThemeName::RosePine);
    }

    #[test]
    fn settings_dialog_cancel_restores_theme_after_preview() {
        let mut app = fixture_app();
        assert_eq!(app.theme_name, ThemeName::CatppuccinMocha);

        let _ = app.handle_key(KeyEvent::new(KeyCode::Char('S')).with_kind(KeyEventKind::Press));
        let _ = app.handle_key(KeyEvent::new(KeyCode::Right).with_kind(KeyEventKind::Press));
        assert_eq!(app.theme_name, ThemeName::RosePine);

        let _ = app.handle_key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press));
        assert!(app.settings_dialog().is_none());
        assert_eq!(app.theme_name, ThemeName::CatppuccinMocha);

        let loaded = crate::infrastructure::config::load_from_path(&app.config_path)
            .expect("config should load");
        assert_eq!(loaded.theme, ThemeName::CatppuccinMocha);
    }

    #[test]
    fn render_uses_selected_theme_palette() {
        let mut monokai_app = fixture_app();
        monokai_app.theme_name = ThemeName::Monokai;

        let mut latte_app = fixture_app();
        latte_app.theme_name = ThemeName::CatppuccinLatte;

        with_rendered_frame(&monokai_app, 80, 24, |frame| {
            let header_bg = frame.buffer.get(0, 0).expect("header cell should exist").bg;
            assert_eq!(header_bg, ui_theme_for(ThemeName::Monokai).crust);
        });

        with_rendered_frame(&latte_app, 80, 24, |frame| {
            let header_bg = frame.buffer.get(0, 0).expect("header cell should exist").bg;
            assert_eq!(header_bg, ui_theme_for(ThemeName::CatppuccinLatte).crust);
            assert_ne!(header_bg, ui_theme_for(ThemeName::Monokai).crust);
        });
    }

    #[test]
    fn agent_preview_unstyled_cells_use_theme_background() {
        let mut app = fixture_app();
        app.theme_name = ThemeName::CatppuccinLatte;
        app.preview_tab = PreviewTab::Agent;
        app.preview.lines = vec!["unstyled output".to_string()];
        app.preview.render_lines = app.preview.lines.clone();
        app.preview.offset = 0;

        let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
        let content_x = layout.preview.x.saturating_add(1);
        let content_y = layout
            .preview
            .y
            .saturating_add(1)
            .saturating_add(PREVIEW_METADATA_ROWS);

        with_rendered_frame(&app, 100, 40, |frame| {
            let cell = frame
                .buffer
                .get(content_x, content_y)
                .expect("preview content cell should exist");
            assert_eq!(cell.bg, ui_theme_for(ThemeName::CatppuccinLatte).base);
            assert_eq!(cell.fg, ui_theme_for(ThemeName::CatppuccinLatte).text);
        });
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
        app.dialogs.keybind_help_open = true;

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
        app.dialogs.keybind_help_open = true;

        with_rendered_frame(&app, 160, 28, |frame| {
            let has_shift_tab = (0..frame.height())
                .any(|row| row_text(frame, row, 0, frame.width()).contains("Shift+Tab"));
            let has_shift_enter = (0..frame.height())
                .any(|row| row_text(frame, row, 0, frame.width()).contains("Shift+Enter"));
            let has_reserved = (0..frame.height())
                .any(|row| row_text(frame, row, 0, frame.width()).contains("Ctrl+K palette"));
            let has_palette_nav = (0..frame.height()).any(|row| {
                row_text(frame, row, 0, frame.width()).contains("[Palette] Type search")
            });

            assert!(has_shift_tab);
            assert!(has_shift_enter);
            assert!(has_reserved);
            assert!(has_palette_nav);
        });
    }

    #[test]
    fn keybind_help_lists_mouse_capture_toggle() {
        let mut app = fixture_app();
        app.dialogs.keybind_help_open = true;

        with_rendered_frame(&app, 160, 28, |frame| {
            let has_mouse_toggle = (0..frame.height()).any(|row| {
                row_text(frame, row, 0, frame.width()).contains("M toggle mouse capture")
            });
            assert!(has_mouse_toggle);
        });
    }

    #[test]
    fn keybind_help_lists_projects_modal_shortcuts_without_truncation() {
        let mut app = fixture_app();
        app.dialogs.keybind_help_open = true;

        with_rendered_frame(&app, 160, 32, |frame| {
            let has_projects_remove = (0..frame.height())
                .any(|row| row_text(frame, row, 0, frame.width()).contains("Ctrl+X/Del remove"));
            assert!(has_projects_remove);
        });
    }

    #[test]
    fn keybind_help_uses_available_height_to_show_footer() {
        let mut app = fixture_app();
        app.dialogs.keybind_help_open = true;

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
            agent: AgentType::Claude,
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
            let Some(second_row) = ((first_row.saturating_add(1))..y_end).find(|&row| {
                row_text(frame, row, x_start, x_end).contains("Enter create, Esc cancel")
            }) else {
                panic!("create dialog wrapped hint second row should be rendered");
            };

            assert_row_fg(frame, first_row, x_start, x_end, ui_theme().overlay0);
            assert_row_fg(frame, second_row, x_start, x_end, ui_theme().overlay0);
        });
    }

    #[test]
    fn status_row_keeps_compact_footer_in_interactive_mode() {
        let mut app = fixture_app();
        app.session.interactive = Some(InteractiveState::new(
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
            let Some(_message_row) =
                find_row_containing(frame, "agent start failed", 0, frame.width())
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
            agent: AgentType::Claude,
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
        app.session.interactive = Some(InteractiveState::new(
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
        app.session.interactive = Some(InteractiveState::new(
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
            let Some(icon_col) = find_cell_with_char(frame, metadata_row, x_start, x_end, '')
            else {
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

    mod runtime_flow {
        use super::*;

        mod interactive_runtime {
            use super::*;

            #[test]
            fn stop_key_opens_stop_dialog_for_selected_workspace() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.workspaces[1].status = WorkspaceStatus::Active;
                focus_agent_preview_tab(&mut app);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
                );

                let Some(dialog) = app.stop_dialog() else {
                    panic!("stop dialog should be open");
                };
                assert_eq!(dialog.workspace.name, "feature-a");
                assert_eq!(dialog.session_name, "grove-ws-feature-a");
                assert_eq!(dialog.focused_field, StopDialogField::StopButton);
            }

            #[test]
            fn x_opens_stop_dialog_from_agent_preview_when_list_is_focused() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.workspaces[1].status = WorkspaceStatus::Active;
                app.state.mode = UiMode::Preview;
                app.state.focus = PaneFocus::WorkspaceList;
                app.preview_tab = PreviewTab::Agent;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.stop_dialog().is_some());
            }

            #[test]
            fn l_then_x_opens_stop_dialog_from_agent_preview() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.workspaces[1].status = WorkspaceStatus::Active;
                app.state.mode = UiMode::List;
                app.state.focus = PaneFocus::WorkspaceList;
                app.preview_tab = PreviewTab::Agent;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(app.state.mode, UiMode::Preview);
                assert_eq!(app.state.focus, PaneFocus::Preview);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.stop_dialog().is_some());
            }

            #[test]
            fn x_opens_stop_dialog_from_agent_preview_when_preview_is_focused_in_list_mode() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.workspaces[1].status = WorkspaceStatus::Active;
                app.state.mode = UiMode::List;
                app.state.focus = PaneFocus::Preview;
                app.preview_tab = PreviewTab::Agent;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.stop_dialog().is_some());
            }

            #[test]
            fn alt_x_noop_in_noninteractive_shell_preview() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.workspaces[1].status = WorkspaceStatus::Active;
                app.state.mode = UiMode::Preview;
                app.state.focus = PaneFocus::Preview;
                app.preview_tab = PreviewTab::Shell;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('x'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.stop_dialog().is_none());
            }

            #[test]
            fn alt_x_does_not_exit_interactive_or_open_stop_dialog() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                app.session.interactive = Some(InteractiveState::new(
                    "%0".to_string(),
                    "grove-ws-feature-a".to_string(),
                    Instant::now(),
                    34,
                    78,
                ));

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('x'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.session.interactive.is_some());
                assert!(app.stop_dialog().is_none());
                assert_eq!(
                    commands.borrow().as_slice(),
                    &[vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-l".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "x".to_string(),
                    ]]
                );
            }

            #[test]
            fn interactive_send_clears_attention_for_agent_workspace() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                app.session.interactive = Some(InteractiveState::new(
                    "%0".to_string(),
                    "grove-ws-feature-a".to_string(),
                    Instant::now(),
                    34,
                    78,
                ));
                app.workspace_attention.insert(
                    PathBuf::from("/repos/grove-feature-a"),
                    WorkspaceAttention::NeedsAttention,
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(
                    commands.borrow().as_slice(),
                    &[vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-l".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "x".to_string(),
                    ]]
                );
                assert!(
                    !app.workspace_attention
                        .contains_key(&PathBuf::from("/repos/grove-feature-a"))
                );
            }

            #[test]
            fn stop_dialog_blocks_navigation_and_escape_cancels() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.workspaces[1].status = WorkspaceStatus::Active;
                app.open_stop_dialog();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(app.state.selected_index, 1);
                assert_eq!(
                    app.stop_dialog().map(|dialog| dialog.focused_field),
                    Some(StopDialogField::CancelButton)
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );
                assert!(app.stop_dialog().is_none());
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
                assert!(app.stop_dialog().is_some());
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
                        vec![
                            "tmux".to_string(),
                            "new-session".to_string(),
                            "-d".to_string(),
                            "-s".to_string(),
                            "grove-ws-feature-a-shell".to_string(),
                            "-c".to_string(),
                            "/repos/grove-feature-a".to_string(),
                        ],
                        vec![
                            "tmux".to_string(),
                            "set-option".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a-shell".to_string(),
                            "history-limit".to_string(),
                            "10000".to_string(),
                        ],
                        vec![
                            "tmux".to_string(),
                            "resize-window".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a-shell".to_string(),
                            "-x".to_string(),
                            "80".to_string(),
                            "-y".to_string(),
                            "36".to_string(),
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
            fn restart_key_restarts_selected_workspace_agent() {
                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok(
                        "To continue this session, run codex resume run-1234".to_string()
                    )],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.confirm_dialog().is_some());
                assert!(commands.borrow().is_empty());

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(app.confirm_dialog().is_none());
                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "C-c".to_string(),
                        ]
                }));
                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "codex resume run-1234".to_string(),
                            "Enter".to_string(),
                        ]
                }));
                assert!(!commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "kill-session".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                        ]
                }));
                assert_eq!(
                    app.state
                        .selected_workspace()
                        .map(|workspace| workspace.status),
                    Some(WorkspaceStatus::Active)
                );
            }

            #[test]
            fn restart_key_reuses_skip_permissions_mode_for_codex_resume() {
                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok(
                        "To continue this session, run codex resume run-1234".to_string()
                    )],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.launch_skip_permissions = true;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "codex --dangerously-bypass-approvals-and-sandbox resume run-1234"
                                .to_string(),
                            "Enter".to_string(),
                        ]
                }));
            }

            #[test]
            fn restart_key_uses_workspace_skip_permissions_marker_for_codex_resume() {
                let workspace_dir = unique_temp_workspace_dir("restart-skip-marker");
                fs::create_dir_all(workspace_dir.join(".grove"))
                    .expect(".grove dir should be writable");
                fs::write(workspace_dir.join(".grove/skip_permissions"), "true\n")
                    .expect("skip marker should be writable");

                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok(
                        "To continue this session, run codex resume run-1234".to_string()
                    )],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.state.workspaces[1].path = workspace_dir.clone();
                app.launch_skip_permissions = false;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "codex --dangerously-bypass-approvals-and-sandbox resume run-1234"
                                .to_string(),
                            "Enter".to_string(),
                        ]
                }));

                let _ = fs::remove_dir_all(workspace_dir);
            }

            #[test]
            fn restart_key_uses_workspace_skip_permissions_marker_for_main_codex_workspace() {
                let workspace_dir = unique_temp_workspace_dir("restart-main-skip-marker");
                fs::create_dir_all(workspace_dir.join(".grove"))
                    .expect(".grove dir should be writable");
                fs::write(workspace_dir.join(".grove/skip_permissions"), "true\n")
                    .expect("skip marker should be writable");

                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok(
                        "To continue this session, run codex resume run-main-1234".to_string(),
                    )],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 0;
                app.state.workspaces[0].path = workspace_dir.clone();
                app.state.workspaces[0].agent = AgentType::Codex;
                app.state.workspaces[0].status = WorkspaceStatus::Active;
                app.launch_skip_permissions = false;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-grove".to_string(),
                            "codex --dangerously-bypass-approvals-and-sandbox resume run-main-1234"
                                .to_string(),
                            "Enter".to_string(),
                        ]
                }));

                let _ = fs::remove_dir_all(workspace_dir);
            }

            #[test]
            fn restart_key_restarts_claude_agent_in_same_tmux_session() {
                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok("Restart with: claude --resume sess-1234".to_string())],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.state.workspaces[1].agent = AgentType::Claude;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                let recorded = commands.borrow();
                assert!(recorded.iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-l".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "/exit".to_string(),
                        ]
                }));
                assert!(recorded.iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "claude --resume sess-1234".to_string(),
                            "Enter".to_string(),
                        ]
                }));
                assert!(!recorded.iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "kill-session".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                        ]
                }));
                assert!(!recorded.iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "new-session".to_string(),
                            "-d".to_string(),
                            "-s".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "-c".to_string(),
                            "/repos/grove-feature-a".to_string(),
                        ]
                }));
            }

            #[test]
            fn restart_key_applies_project_agent_env_defaults_before_resume() {
                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok("run codex resume run-1234".to_string())],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.state.workspaces[1].agent = AgentType::Codex;
                app.projects[0].defaults.agent_env.codex = vec![
                    "CODEX_CONFIG_DIR=~/.codex-work".to_string(),
                    "OPENAI_API_BASE=https://api.example.com/v1".to_string(),
                ];

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "export CODEX_CONFIG_DIR='~/.codex-work' OPENAI_API_BASE='https://api.example.com/v1'"
                                .to_string(),
                            "Enter".to_string(),
                        ]
                }));
                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "codex resume run-1234".to_string(),
                            "Enter".to_string(),
                        ]
                }));
            }

            #[test]
            fn restart_key_rejects_invalid_project_agent_env_defaults() {
                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok("run codex resume run-1234".to_string())],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.state.workspaces[1].agent = AgentType::Codex;
                app.projects[0].defaults.agent_env.codex = vec!["INVALID-KEY=value".to_string()];

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().is_empty());
                assert!(
                    app.status_bar_line()
                        .contains("invalid project agent env: invalid env key 'INVALID-KEY'")
                );
            }

            #[test]
            fn restart_key_restarts_opencode_in_same_tmux_session() {
                let (mut app, commands, _captures, _cursor_captures) = fixture_app_with_tmux(
                    WorkspaceStatus::Active,
                    vec![Ok(
                        "resume with opencode -s ses_36d243142ffeYteys2MXS86Nnt".to_string()
                    )],
                );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.state.workspaces[1].agent = AgentType::OpenCode;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "C-c".to_string(),
                        ]
                }));
                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "opencode -s ses_36d243142ffeYteys2MXS86Nnt".to_string(),
                            "Enter".to_string(),
                        ]
                }));
                assert!(!commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "kill-session".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                        ]
                }));
            }

            #[test]
            fn background_restart_key_queues_lifecycle_task() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                focus_agent_preview_tab(&mut app);

                let open_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                assert!(!cmd_contains_task(&open_cmd));
                assert!(app.confirm_dialog().is_some());

                let confirm_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(cmd_contains_task(&confirm_cmd));
            }

            #[test]
            fn escape_cancels_restart_dialog() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
                );
                assert!(app.confirm_dialog().is_some());

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );

                assert!(app.confirm_dialog().is_none());
                assert!(commands.borrow().is_empty());
            }

            #[test]
            fn background_stop_key_queues_lifecycle_task() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                focus_agent_preview_tab(&mut app);

                let open_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
                );
                assert!(!cmd_contains_task(&open_cmd));
                assert!(app.stop_dialog().is_some());

                let confirm_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(cmd_contains_task(&confirm_cmd));
            }

            #[test]
            fn stop_agent_completed_updates_workspace_status_and_exits_interactive() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.mode = UiMode::Preview;
                app.state.focus = PaneFocus::Preview;
                app.preview.lines = vec!["stale-preview".to_string()];
                app.preview.render_lines = app.preview.lines.clone();
                app.session.interactive = Some(InteractiveState::new(
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
                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
                let preview_text = app.preview.lines.join("\n");
                assert!(!preview_text.contains("stale-preview"));
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
                assert!(app.launch_dialog().is_some());
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

                assert!(app.launch_dialog().is_none());
                assert!(commands.borrow().is_empty());
                assert!(app.status_bar_line().contains("agent already running"));
            }

            #[test]
            fn start_key_opens_dialog_when_agent_tab_not_focused() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.launch_dialog().is_some());
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
            fn restart_key_without_running_agent_shows_toast() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
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
            fn restart_key_noop_in_git_tab() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                app.state.mode = UiMode::Preview;
                app.state.focus = PaneFocus::Preview;
                app.preview_tab = PreviewTab::Git;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('r')).with_kind(KeyEventKind::Press)),
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
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
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

                assert!(app.session.interactive.is_some());
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

                assert!(app.session.interactive.is_some());
                assert_eq!(
                    app.session
                        .interactive
                        .as_ref()
                        .map(|state| state.target_session.as_str()),
                    Some("grove-ws-grove")
                );
                assert_eq!(app.mode_label(), "Interactive");
            }

            #[test]
            fn enter_on_main_workspace_in_shell_tab_launches_shell_and_enters_interactive_mode() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                app.preview_tab = PreviewTab::Shell;

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
                            "grove-ws-grove-shell".to_string(),
                            "-c".to_string(),
                            "/repos/grove".to_string(),
                        ],
                        vec![
                            "tmux".to_string(),
                            "set-option".to_string(),
                            "-t".to_string(),
                            "grove-ws-grove-shell".to_string(),
                            "history-limit".to_string(),
                            "10000".to_string(),
                        ],
                        vec![
                            "tmux".to_string(),
                            "resize-window".to_string(),
                            "-t".to_string(),
                            "grove-ws-grove-shell".to_string(),
                            "-x".to_string(),
                            "80".to_string(),
                            "-y".to_string(),
                            "36".to_string(),
                        ],
                    ]
                );
                assert_eq!(
                    app.session
                        .interactive
                        .as_ref()
                        .map(|state| state.target_session.as_str()),
                    Some("grove-ws-grove-shell")
                );
                assert_eq!(app.mode_label(), "Interactive");
            }

            #[test]
            fn shell_tab_main_workspace_summary_uses_shell_status_copy() {
                let mut app = fixture_app();
                app.preview_tab = PreviewTab::Shell;
                app.state.selected_index = 0;
                app.state.workspaces[0].status = WorkspaceStatus::Active;

                app.refresh_preview_summary();

                let combined = app.preview.lines.join("\n");
                assert!(!combined.contains("Connecting to main workspace session"));
                assert!(combined.contains("Preparing shell session for grove"));
            }

            #[test]
            fn enter_on_idle_workspace_launches_shell_session_and_enters_interactive_mode() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
                );
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
                            "grove-ws-feature-a-shell".to_string(),
                            "-c".to_string(),
                            "/repos/grove-feature-a".to_string(),
                        ],
                        vec![
                            "tmux".to_string(),
                            "set-option".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a-shell".to_string(),
                            "history-limit".to_string(),
                            "10000".to_string(),
                        ],
                        vec![
                            "tmux".to_string(),
                            "resize-window".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a-shell".to_string(),
                            "-x".to_string(),
                            "80".to_string(),
                            "-y".to_string(),
                            "36".to_string(),
                        ],
                    ]
                );
                assert_eq!(
                    app.session
                        .interactive
                        .as_ref()
                        .map(|state| state.target_session.as_str()),
                    Some("grove-ws-feature-a-shell")
                );
                assert_eq!(app.mode_label(), "Interactive");
            }

            #[test]
            fn enter_on_active_workspace_resizes_tmux_session_to_preview_dimensions() {
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );

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
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
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
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
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
                    calls.borrow().iter().any(|call| {
                        call == &format!("capture:grove-ws-feature-a:{}:true", 600)
                    })
                );
            }

            #[test]
            fn resize_verify_retries_once_then_stops() {
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
                        WorkspaceStatus::Active,
                        vec![Ok("after-retry\n".to_string())],
                        vec![Ok("1 0 0 70 20".to_string())],
                    );
                app.state.selected_index = 1;
                app.session.interactive = Some(InteractiveState::new(
                    "%0".to_string(),
                    "grove-ws-feature-a".to_string(),
                    Instant::now(),
                    34,
                    78,
                ));
                app.session.pending_resize_verification = Some(PendingResizeVerification {
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
                assert!(app.session.pending_resize_verification.is_none());
            }

            #[test]
            fn preview_poll_drops_cursor_capture_for_non_interactive_session() {
                let (mut app, _commands, _captures, _cursor_captures, events) =
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );
                app.state.selected_index = 1;
                app.session.interactive = Some(InteractiveState::new(
                    "%0".to_string(),
                    "grove-ws-feature-a".to_string(),
                    Instant::now(),
                    20,
                    80,
                ));
                if let Some(state) = app.session.interactive.as_mut() {
                    state.update_cursor(3, 4, true, 20, 80);
                }

                ftui::Model::update(
                    &mut app,
                    Msg::PreviewPollCompleted(PreviewPollCompletion {
                        generation: 1,
                        live_capture: None,
                        cursor_capture: Some(CursorCapture {
                            session: "grove-ws-grove".to_string(),
                            capture_ms: 1,
                            result: Ok("1 9 7 88 22".to_string()),
                        }),
                        workspace_status_captures: Vec::new(),
                    }),
                );

                let state = app
                    .session
                    .interactive
                    .as_ref()
                    .expect("interactive state should remain active");
                assert_eq!(state.target_session, "grove-ws-feature-a");
                assert_eq!(state.cursor_row, 3);
                assert_eq!(state.cursor_col, 4);
                assert_eq!(state.pane_height, 20);
                assert_eq!(state.pane_width, 80);
                assert!(
                    event_kinds(&events)
                        .iter()
                        .any(|kind| kind == "cursor_session_mismatch_dropped")
                );
            }

            #[test]
            fn interactive_keys_forward_to_tmux_session() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                assert!(app.enter_interactive(Instant::now()));
                assert!(app.session.interactive.is_some());

                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
                );

                assert!(!matches!(cmd, Cmd::Quit));
                assert!(app.polling.next_tick_due_at.is_some());
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
            fn interactive_shift_tab_forwards_to_tmux_session() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                assert!(app.enter_interactive(Instant::now()));
                assert!(app.session.interactive.is_some());

                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::BackTab).with_kind(KeyEventKind::Press)),
                );

                assert!(!matches!(cmd, Cmd::Quit));
                assert!(app.polling.next_tick_due_at.is_some());
                assert_eq!(
                    commands.borrow().as_slice(),
                    &[vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "BTab".to_string(),
                    ]]
                );
            }

            #[test]
            fn interactive_shift_enter_forwards_to_tmux_session() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                assert!(app.enter_interactive(Instant::now()));
                assert!(app.session.interactive.is_some());

                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Enter)
                            .with_modifiers(Modifiers::SHIFT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(!matches!(cmd, Cmd::Quit));
                assert!(app.polling.next_tick_due_at.is_some());
                assert_eq!(
                    commands.borrow().as_slice(),
                    &[vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-l".to_string(),
                        "-t".to_string(),
                        "grove-ws-feature-a".to_string(),
                        "\u{1b}[13;2u".to_string(),
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

                let Some(state) = app.session.interactive.as_mut() else {
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

                let Some(state) = app.session.interactive.as_mut() else {
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

                let Some(state) = app.session.interactive.as_mut() else {
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

                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
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

                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
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

                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
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

                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
                assert!(commands.borrow().is_empty());
            }

            #[test]
            fn alt_k_exits_interactive_and_selects_previous_workspace() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );
                assert!(app.session.interactive.is_some());

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('k'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
                assert_eq!(app.state.selected_index, 0);
                assert!(commands.borrow().is_empty());
            }

            #[test]
            fn alt_bracket_exits_interactive_and_switches_to_git_tab() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                app.preview_tab = PreviewTab::Agent;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );
                assert!(app.session.interactive.is_some());

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
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char(']'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::Preview);
                assert_eq!(app.state.focus, PaneFocus::Preview);
                assert_eq!(app.preview_tab, PreviewTab::Git);
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
                            interval <= Duration::from_millis(20)
                                && interval >= Duration::from_millis(15),
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
                    .polling
                    .next_tick_due_at
                    .expect("first key should schedule a due tick");

                std::thread::sleep(Duration::from_millis(1));

                let second_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('y')).with_kind(KeyEventKind::Press)),
                );
                let second_due = app
                    .polling
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
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
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
                assert!(app.session.interactive.is_none());
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
                    .find(|event| {
                        event.event == "input" && event.kind == "interactive_input_to_preview"
                    })
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
                    .find(|event| {
                        event.event == "input" && event.kind == "interactive_inputs_coalesced"
                    })
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
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );
                clear_recorded_events(&events);

                app.polling.next_tick_due_at = Some(Instant::now() + Duration::from_secs(10));
                app.polling.next_tick_interval_ms = Some(10_000);
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
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );

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
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
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
                    calls.borrow().iter().any(|call| {
                        call == &format!("capture:grove-ws-feature-a:{}:true", 200)
                    })
                );
            }

            #[test]
            fn claude_live_preview_capture_keeps_tmux_escape_output() {
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
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
                    calls.borrow().iter().any(|call| {
                        call == &format!("capture:grove-ws-feature-a:{}:true", 200)
                    })
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
                let (mut app, _commands, _captures, _cursor_captures, calls) =
                    fixture_app_with_tmux_and_calls(
                        WorkspaceStatus::Active,
                        vec![Ok("line".to_string())],
                        Vec::new(),
                    );

                app.state.selected_index = 1;
                app.polling.next_tick_due_at = Some(Instant::now() + Duration::from_secs(5));

                let cmd = ftui::Model::update(&mut app, Msg::Tick);

                assert!(matches!(cmd, Cmd::None));
                assert!(calls.borrow().is_empty());
            }

            #[test]
            fn in_flight_preview_poll_schedules_short_tick_for_task_results() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                app.polling.preview_poll_in_flight = true;
                app.polling.next_tick_due_at = Some(Instant::now() + Duration::from_secs(5));

                let cmd = app.schedule_next_tick();
                let Cmd::Tick(interval) = cmd else {
                    panic!("expected Cmd::Tick while preview poll is in flight");
                };
                assert!(interval <= Duration::from_millis(20));
                assert!(interval >= Duration::from_millis(15));
            }

            #[test]
            fn parse_cursor_metadata_requires_five_fields() {
                assert_eq!(
                    parse_cursor_metadata("1 4 2 120 40"),
                    Some(crate::ui::tui::CursorMetadata {
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
                let lines = ansi_lines_to_styled_lines(&["a\u{1b}[31mb\u{1b}[0mc".to_string()]);
                let line = &lines[0];
                assert_eq!(line.to_plain_text(), "abc");
                assert_eq!(line.spans().len(), 3);
                assert_eq!(line.spans()[1].as_str(), "b");
                assert_eq!(
                    line.spans()[1].style.and_then(|style| style.fg),
                    Some(ansi_16_color(1))
                );
            }

            #[test]
            fn ansi_parser_uses_readable_latte_white_slots() {
                let styled_lines = ansi_lines_to_styled_lines_for_theme(
                    &[
                        "\u{1b}[37mnormal-white".to_string(),
                        "\u{1b}[97mbright-white".to_string(),
                    ],
                    ThemeName::CatppuccinLatte,
                );
                let latte = ui_theme_for(ThemeName::CatppuccinLatte);
                assert_eq!(
                    styled_lines[0].spans()[0].style.and_then(|style| style.fg),
                    Some(latte.subtext1)
                );
                assert_eq!(
                    styled_lines[1].spans()[0].style.and_then(|style| style.fg),
                    Some(latte.subtext0)
                );
            }

            #[test]
            fn ansi_parser_dim_for_latte_uses_dim_foreground_color() {
                let styled_lines = ansi_lines_to_styled_lines_for_theme(
                    &["a\u{1b}[2mb".to_string()],
                    ThemeName::CatppuccinLatte,
                );
                assert_eq!(
                    styled_lines[0].spans()[1].style.and_then(|style| style.fg),
                    Some(PackedRgba::rgb(140, 143, 161))
                );
            }

            #[test]
            fn ansi_parser_carries_style_across_lines_until_reset() {
                let styled_lines = ansi_lines_to_styled_lines(&[
                    "a\u{1b}[31mb".to_string(),
                    "c".to_string(),
                    "\u{1b}[0md".to_string(),
                ]);
                assert_eq!(styled_lines.len(), 3);
                assert_eq!(styled_lines[0].to_plain_text(), "ab");
                assert_eq!(styled_lines[1].to_plain_text(), "c");
                assert_eq!(styled_lines[2].to_plain_text(), "d");
                assert_eq!(
                    styled_lines[1].spans()[0].style.and_then(|style| style.fg),
                    Some(ansi_16_color(1))
                );
                assert_eq!(
                    styled_lines[2].spans()[0].style.and_then(|style| style.fg),
                    None
                );
            }

            #[test]
            fn tick_polls_cursor_metadata_and_renders_overlay() {
                let config_path = unique_config_path("cursor-overlay");
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux_and_config_path(
                        WorkspaceStatus::Active,
                        vec![
                            Ok("first\nsecond\nthird\n".to_string()),
                            Ok("first\nsecond\nthird\n".to_string()),
                        ],
                        vec![Ok("1 1 1 78 34".to_string()), Ok("1 1 1 78 34".to_string())],
                        config_path,
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
                    app.session.interactive.as_ref().map(|state| (
                        state.cursor_row,
                        state.cursor_col,
                        state.pane_height
                    )),
                    Some((1, 1, 34))
                );
                assert!(rendered.contains("s|econd"), "{rendered}");
            }

            #[test]
            fn divider_ratio_changes_are_session_only() {
                let config_path = unique_config_path("persist");
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux_and_config_path(
                        WorkspaceStatus::Idle,
                        Vec::new(),
                        Vec::new(),
                        config_path.clone(),
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
                assert!(!config_path.exists());

                let (app_reloaded, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux_and_config_path(
                        WorkspaceStatus::Idle,
                        Vec::new(),
                        Vec::new(),
                        config_path.clone(),
                    );

                assert_eq!(app_reloaded.sidebar_width_pct, 33);
                let _ = fs::remove_file(config_path);
            }

            #[test]
            fn mouse_click_on_list_selects_workspace() {
                let mut app = fixture_app();
                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
            fn mouse_workspace_switch_exits_interactive_mode() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                assert!(app.enter_interactive(Instant::now()));

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
                let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
                let first_row_y = sidebar_inner.y.saturating_add(1);

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
                        first_row_y,
                    )),
                );

                assert_eq!(app.state.selected_index, 0);
                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
            }
        }
        mod mouse_preview {
            use super::*;

            fn preview_tab_click_point(app: &GroveApp, tab_kind: WorkspaceTabKind) -> (u16, u16) {
                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
                let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
                let tab_y = preview_inner.y.saturating_add(1);
                let mut tab_x = preview_inner.x;
                let Some(workspace) = app.state.selected_workspace() else {
                    return (preview_inner.x, tab_y);
                };
                let Some(tabs) = app.workspace_tabs.get(workspace.path.as_path()) else {
                    return (preview_inner.x, tab_y);
                };

                for (index, current_tab) in tabs.tabs.iter().enumerate() {
                    if index > 0 {
                        tab_x = tab_x.saturating_add(1);
                    }
                    let title = format!(" {} ", current_tab.title);
                    let Some(tab_width) = u16::try_from(title.len()).ok() else {
                        continue;
                    };
                    if current_tab.kind == tab_kind {
                        return (tab_x, tab_y);
                    }
                    tab_x = tab_x.saturating_add(tab_width);
                }

                (preview_inner.x, tab_y)
            }

            #[test]
            fn mouse_click_preview_tab_switches_tabs() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.open_new_shell_tab();
                let Some(home_id) = app
                    .selected_workspace_tabs_state()
                    .and_then(|tabs| tabs.find_kind(WorkspaceTabKind::Home))
                    .map(|tab| tab.id)
                else {
                    panic!("home tab should exist");
                };
                let _ = app.select_tab_by_id_for_selected_workspace(home_id);
                assert_eq!(app.preview_tab, PreviewTab::Home);

                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );
                let (shell_tab_x, tab_y) = preview_tab_click_point(&app, WorkspaceTabKind::Shell);

                ftui::Model::update(
                    &mut app,
                    Msg::Mouse(MouseEvent::new(
                        MouseEventKind::Down(MouseButton::Left),
                        shell_tab_x,
                        tab_y,
                    )),
                );

                assert_eq!(app.preview_tab, PreviewTab::Shell);
                assert_eq!(app.state.mode, UiMode::Preview);
                assert_eq!(app.state.focus, PaneFocus::Preview);
                assert!(app.session.interactive.is_none());
            }

            #[test]
            fn mouse_click_preview_tab_exits_interactive_and_switches_tabs() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                app.open_new_shell_tab();
                app.open_or_focus_git_tab();
                let Some(shell_id) = app
                    .selected_workspace_tabs_state()
                    .and_then(|tabs| tabs.find_kind(WorkspaceTabKind::Shell))
                    .map(|tab| tab.id)
                else {
                    panic!("shell tab should exist");
                };
                let _ = app.select_tab_by_id_for_selected_workspace(shell_id);
                assert!(app.enter_interactive(Instant::now()));
                assert_eq!(app.preview_tab, PreviewTab::Shell);

                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );
                let (git_tab_x, tab_y) = preview_tab_click_point(&app, WorkspaceTabKind::Git);

                ftui::Model::update(
                    &mut app,
                    Msg::Mouse(MouseEvent::new(
                        MouseEventKind::Down(MouseButton::Left),
                        git_tab_x,
                        tab_y,
                    )),
                );

                assert_eq!(app.preview_tab, PreviewTab::Git);
                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::Preview);
                assert_eq!(app.state.focus, PaneFocus::Preview);
            }

            #[test]
            fn mouse_click_preview_enters_interactive_mode() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );
                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);

                ftui::Model::update(
                    &mut app,
                    Msg::Mouse(MouseEvent::new(
                        MouseEventKind::Down(MouseButton::Left),
                        layout.preview.x.saturating_add(1),
                        layout.preview.y.saturating_add(1),
                    )),
                );

                assert!(app.session.interactive.is_some());
                assert_eq!(app.state.mode, UiMode::Preview);
                assert_eq!(app.state.focus, PaneFocus::Preview);
            }

            #[test]
            fn mouse_workspace_click_exits_interactive_without_selection_change() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 1;
                assert!(app.enter_interactive(Instant::now()));

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
                assert!(app.session.interactive.is_none());
                assert_eq!(app.state.mode, UiMode::List);
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
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

                assert_eq!(app.sidebar_width_pct, 51);
            }

            #[test]
            fn mouse_drag_uses_rendered_width_without_resize_message() {
                let mut app = fixture_app();
                with_rendered_frame(&app, 200, 40, |_| {});

                ftui::Model::update(
                    &mut app,
                    Msg::Mouse(MouseEvent::new(
                        MouseEventKind::Down(MouseButton::Left),
                        66,
                        8,
                    )),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Mouse(MouseEvent::new(
                        MouseEventKind::Drag(MouseButton::Left),
                        100,
                        8,
                    )),
                );

                assert_eq!(app.sidebar_width_pct, 50);
            }

            #[test]
            fn mouse_drag_from_divider_hit_padding_does_not_jump_on_first_drag_event() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );
                assert_eq!(app.sidebar_width_pct, 33);

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
                        32,
                        8,
                    )),
                );

                assert_eq!(app.sidebar_width_pct, 33);
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

                assert!(app.preview.offset >= 3);
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
                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
                    let Some(output_row) = find_row_containing(frame, "alpha beta", x_start, x_end)
                    else {
                        panic!("output row should be rendered");
                    };
                    let Some(first_col) =
                        find_cell_with_char(frame, output_row, x_start, x_end, 'a')
                    else {
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
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );

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

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
                    .find(|event| {
                        event.event == "selection" && event.kind == "preview_drag_started"
                    })
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
                    .find(|event| {
                        event.event == "selection" && event.kind == "preview_drag_finished"
                    })
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
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );

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

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
                    .find(|event| {
                        event.event == "selection" && event.kind == "preview_drag_started"
                    })
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
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );

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

                let layout = GroveApp::view_layout_for_size(100, 50, app.sidebar_width_pct, false);
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
                    .find(|event| {
                        event.event == "selection" && event.kind == "preview_drag_started"
                    })
                    .expect("drag start event should be logged");
                assert_eq!(started.data.get("mapped"), Some(&Value::from(true)));

                let output_height =
                    usize::from(preview_inner.height.saturating_sub(PREVIEW_METADATA_ROWS));
                let output_row = usize::from(
                    select_y.saturating_sub(preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS)),
                );
                let expected_visible_start = app.preview.lines.len().saturating_sub(output_height);
                let expected_line = expected_visible_start.saturating_add(output_row);
                assert_eq!(
                    started.data.get("line"),
                    Some(&Value::from(usize_to_u64(expected_line)))
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
                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
                    let Some(output_row) = find_row_containing(frame, "alpha beta", x_start, x_end)
                    else {
                        panic!("output row should be rendered");
                    };
                    let Some(first_col) =
                        find_cell_with_char(frame, output_row, x_start, x_end, 'a')
                    else {
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
                    "\u{1b}[48;2;30;35;50ma\u{1b}[48;2;30;35;50mb\u{1b}[48;2;30;35;50mc\u{1b}[0m"
                        .to_string(),
                ];

                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );
                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
                    let Some(first_col) =
                        find_cell_with_char(frame, output_row, x_start, x_end, 'a')
                    else {
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

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
            fn preview_output_rows_use_theme_background_for_shell_tab() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 0;
                app.preview_tab = PreviewTab::Shell;

                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
                let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
                let output_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

                with_rendered_frame(&app, 100, 40, |frame| {
                    for x in preview_inner.x..preview_inner.right() {
                        let Some(cell) = frame.buffer.get(x, output_y) else {
                            panic!("output row cell should be rendered");
                        };
                        assert_eq!(
                            cell.bg,
                            ui_theme().base,
                            "expected theme background at ({x},{output_y})",
                        );
                    }
                });
            }

            #[test]
            fn preview_output_rows_use_theme_background_for_agent_tab() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
                app.state.selected_index = 0;
                app.preview_tab = PreviewTab::Agent;
                app.preview.lines.clear();
                app.preview.render_lines.clear();

                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
                let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
                let output_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);

                let far_right_x = preview_inner.right().saturating_sub(1);
                with_rendered_frame(&app, 100, 40, |frame| {
                    let Some(cell) = frame.buffer.get(far_right_x, output_y) else {
                        panic!("output row cell should be rendered");
                    };
                    assert_eq!(
                        cell.bg,
                        ui_theme().base,
                        "agent tab should use theme background at ({far_right_x},{output_y})",
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
                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
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
                let config_path = unique_config_path("alt-copy-paste");
                let (mut app, commands, captures, _cursor_captures) =
                    fixture_app_with_tmux_and_config_path(
                        WorkspaceStatus::Active,
                        vec![Ok(String::new())],
                        vec![Ok("1 0 0 78 34".to_string())],
                        config_path,
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
                assert!(content.contains("Press 'n' to create a workspace, hit 's' to open agent"));
            }

            #[test]
            fn shell_renders_discovery_error_state() {
                let config_path = unique_config_path("error-state");
                let app = GroveApp::from_parts(
                    BootstrapData {
                        repo_name: "grove".to_string(),
                        workspaces: Vec::new(),
                        discovery_state: DiscoveryState::Error(
                            "fatal: not a git repository".to_string(),
                        ),
                    },
                    Box::new(RecordingTmuxInput {
                        commands: Rc::new(RefCell::new(Vec::new())),
                        captures: Rc::new(RefCell::new(Vec::new())),
                        cursor_captures: Rc::new(RefCell::new(Vec::new())),
                        calls: Rc::new(RefCell::new(Vec::new())),
                    }),
                    config_path,
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
                assert_eq!(app.state.mode, crate::ui::state::UiMode::Preview);

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
            fn preview_mode_arrow_page_keys_and_end_control_scrollback() {
                let mut app = fixture_app();
                app.preview.lines = (1..=240).map(|value| value.to_string()).collect();
                app.preview.render_lines = app.preview.lines.clone();
                ftui::Model::update(
                    &mut app,
                    Msg::Resize {
                        width: 100,
                        height: 40,
                    },
                );
                let page_delta = app
                    .preview_output_dimensions()
                    .map_or(1usize, |(_, height)| usize::from(height).saturating_sub(1))
                    .max(1);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
                assert_eq!(app.state.mode, crate::ui::state::UiMode::Preview);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Up)));
                assert_eq!(app.preview.offset, 1);
                assert!(!app.preview.auto_scroll);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Down)));
                assert_eq!(app.preview.offset, 0);
                assert!(app.preview.auto_scroll);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::PageUp)));
                assert_eq!(app.preview.offset, page_delta);
                assert!(!app.preview.auto_scroll);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::PageDown)));
                assert_eq!(app.preview.offset, 0);
                assert!(app.preview.auto_scroll);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::PageUp)));
                assert_eq!(app.preview.offset, page_delta);
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::End)));
                assert_eq!(app.preview.offset, 0);
                assert!(app.preview.auto_scroll);
            }

            #[test]
            fn preview_mode_bracket_keys_cycle_tabs() {
                let mut app = fixture_app();
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
                assert_eq!(app.preview_tab, PreviewTab::Agent);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
                assert_eq!(app.preview_tab, PreviewTab::Shell);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
                assert_eq!(app.preview_tab, PreviewTab::Git);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
                assert_eq!(app.preview_tab, PreviewTab::Agent);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('['))));
                assert_eq!(app.preview_tab, PreviewTab::Git);

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('['))));
                assert_eq!(app.preview_tab, PreviewTab::Shell);
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
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));

                let layout = GroveApp::view_layout_for_size(100, 40, app.sidebar_width_pct, false);
                let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
                let output_y = preview_inner.y.saturating_add(PREVIEW_METADATA_ROWS);
                let x_start = preview_inner.x;
                let x_end = preview_inner.right();

                with_rendered_frame(&app, 100, 40, |frame| {
                    let tabs_line =
                        row_text(frame, preview_inner.y.saturating_add(1), x_start, x_end);
                    let output_line = row_text(frame, output_y, x_start, x_end);

                    assert!(tabs_line.contains("Agent"));
                    assert!(tabs_line.contains("Shell"));
                    assert!(tabs_line.contains("Git"));
                    assert!(output_line.contains("lazygit"));
                });
            }

            #[test]
            fn git_tab_queues_async_lazygit_launch_when_supported() {
                let config_path = unique_config_path("background-lazygit-launch");
                let mut app = GroveApp::from_parts(
                    fixture_bootstrap(WorkspaceStatus::Idle),
                    Box::new(BackgroundLaunchTmuxInput),
                    config_path,
                    Box::new(NullEventLogger),
                    None,
                );

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
                let cmd = ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));

                assert_eq!(app.preview_tab, PreviewTab::Git);
                assert!(cmd_contains_task(&cmd));
                assert!(
                    app.session
                        .lazygit_sessions
                        .in_flight
                        .contains("grove-ws-grove-git")
                );
            }

            #[test]
            fn git_tab_launches_lazygit_with_dedicated_tmux_session() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                let lazygit_command = app.session.lazygit_command.clone();

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));

                let expected_suffix = vec![
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
                        "resize-window".to_string(),
                        "-t".to_string(),
                        "grove-ws-grove-git".to_string(),
                        "-x".to_string(),
                        "80".to_string(),
                        "-y".to_string(),
                        "36".to_string(),
                    ],
                    vec![
                        "tmux".to_string(),
                        "send-keys".to_string(),
                        "-t".to_string(),
                        "grove-ws-grove-git".to_string(),
                        lazygit_command,
                        "Enter".to_string(),
                    ],
                ];
                assert!(commands.borrow().as_slice().ends_with(&expected_suffix));
            }

            #[test]
            fn lazygit_launch_completion_success_marks_session_ready() {
                let mut app = fixture_app();
                app.session
                    .lazygit_sessions
                    .in_flight
                    .insert("grove-ws-grove-git".to_string());

                ftui::Model::update(
                    &mut app,
                    Msg::LazygitLaunchCompleted(LazygitLaunchCompletion {
                        session_name: "grove-ws-grove-git".to_string(),
                        duration_ms: 12,
                        result: Ok(()),
                    }),
                );

                assert!(
                    app.session
                        .lazygit_sessions
                        .ready
                        .contains("grove-ws-grove-git")
                );
                assert!(
                    !app.session
                        .lazygit_sessions
                        .in_flight
                        .contains("grove-ws-grove-git")
                );
                assert!(
                    !app.session
                        .lazygit_sessions
                        .failed
                        .contains("grove-ws-grove-git")
                );
            }

            #[test]
            fn lazygit_launch_completion_failure_marks_session_failed() {
                let mut app = fixture_app();
                app.session
                    .lazygit_sessions
                    .in_flight
                    .insert("grove-ws-grove-git".to_string());

                ftui::Model::update(
                    &mut app,
                    Msg::LazygitLaunchCompleted(LazygitLaunchCompletion {
                        session_name: "grove-ws-grove-git".to_string(),
                        duration_ms: 9,
                        result: Err("spawn failed".to_string()),
                    }),
                );

                assert!(
                    app.session
                        .lazygit_sessions
                        .failed
                        .contains("grove-ws-grove-git")
                );
                assert!(
                    !app.session
                        .lazygit_sessions
                        .in_flight
                        .contains("grove-ws-grove-git")
                );
                assert!(app.status_bar_line().contains("lazygit launch failed"));
            }

            #[test]
            fn lazygit_launch_completion_duplicate_session_marks_session_ready() {
                let mut app = fixture_app();
                app.session
                    .lazygit_sessions
                    .in_flight
                    .insert("grove-ws-grove-git".to_string());

                ftui::Model::update(
                    &mut app,
                    Msg::LazygitLaunchCompleted(LazygitLaunchCompletion {
                        session_name: "grove-ws-grove-git".to_string(),
                        duration_ms: 9,
                        result: Err(
                            "command failed: tmux new-session -d -s grove-ws-grove-git -c /repos/grove; duplicate session: grove-ws-grove-git".to_string(),
                        ),
                    }),
                );

                assert!(
                    app.session
                        .lazygit_sessions
                        .ready
                        .contains("grove-ws-grove-git")
                );
                assert!(
                    !app.session
                        .lazygit_sessions
                        .in_flight
                        .contains("grove-ws-grove-git")
                );
                assert!(
                    !app.session
                        .lazygit_sessions
                        .failed
                        .contains("grove-ws-grove-git")
                );
                assert!(!app.status_bar_line().contains("lazygit launch failed"));
            }

            #[test]
            fn workspace_shell_launch_completion_success_marks_session_ready() {
                let mut app = fixture_app();
                app.session
                    .shell_sessions
                    .in_flight
                    .insert("grove-ws-feature-a-shell".to_string());

                ftui::Model::update(
                    &mut app,
                    Msg::WorkspaceShellLaunchCompleted(WorkspaceShellLaunchCompletion {
                        session_name: "grove-ws-feature-a-shell".to_string(),
                        duration_ms: 14,
                        result: Ok(()),
                    }),
                );

                assert!(
                    app.session
                        .shell_sessions
                        .ready
                        .contains("grove-ws-feature-a-shell")
                );
                assert!(
                    !app.session
                        .shell_sessions
                        .in_flight
                        .contains("grove-ws-feature-a-shell")
                );
                assert!(
                    !app.session
                        .shell_sessions
                        .failed
                        .contains("grove-ws-feature-a-shell")
                );
            }

            #[test]
            fn workspace_shell_launch_completion_success_polls_from_list_mode() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);
                app.state.selected_index = 1;
                app.state.mode = UiMode::List;
                app.state.focus = PaneFocus::WorkspaceList;
                app.preview_tab = PreviewTab::Agent;
                app.session
                    .shell_sessions
                    .in_flight
                    .insert("grove-ws-feature-a-shell".to_string());

                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::WorkspaceShellLaunchCompleted(WorkspaceShellLaunchCompletion {
                        session_name: "grove-ws-feature-a-shell".to_string(),
                        duration_ms: 14,
                        result: Ok(()),
                    }),
                );

                assert!(cmd_contains_task(&cmd));
            }

            #[test]
            fn workspace_shell_launch_completion_duplicate_session_marks_session_ready() {
                let mut app = fixture_app();
                app.session
                    .shell_sessions
                    .in_flight
                    .insert("grove-ws-feature-a-shell".to_string());

                ftui::Model::update(
                    &mut app,
                    Msg::WorkspaceShellLaunchCompleted(WorkspaceShellLaunchCompletion {
                        session_name: "grove-ws-feature-a-shell".to_string(),
                        duration_ms: 14,
                        result: Err(
                            "command failed: tmux new-session -d -s grove-ws-feature-a-shell -c /repos/grove-feature-a; duplicate session: grove-ws-feature-a-shell".to_string(),
                        ),
                    }),
                );

                assert!(
                    app.session
                        .shell_sessions
                        .ready
                        .contains("grove-ws-feature-a-shell")
                );
                assert!(
                    !app.session
                        .shell_sessions
                        .in_flight
                        .contains("grove-ws-feature-a-shell")
                );
                assert!(
                    !app.session
                        .shell_sessions
                        .failed
                        .contains("grove-ws-feature-a-shell")
                );
                assert!(
                    !app.status_bar_line()
                        .contains("workspace shell launch failed")
                );
            }

            #[test]
            fn enter_on_git_tab_attaches_to_lazygit_session() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char(']'))));
                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Enter)));

                assert_eq!(
                    app.session
                        .interactive
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
                let config_path = unique_config_path("frame-log");
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
                    config_path,
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
                let config_path = unique_config_path("frame-lines");
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
                    config_path,
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
                let config_path = unique_config_path("frame-cursor-snapshot");
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
                    config_path,
                    Box::new(event_log),
                    Some(1_771_023_000_124),
                );
                app.session.interactive = Some(InteractiveState::new(
                    "%1".to_string(),
                    "grove-ws-feature-a".to_string(),
                    Instant::now(),
                    3,
                    80,
                ));
                if let Some(state) = app.session.interactive.as_mut() {
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
        }
        mod navigation_and_agent_lifecycle {
            use super::*;

            #[test]
            fn interactive_enter_and_exit_emit_mode_events() {
                let (mut app, _commands, _captures, _cursor_captures, events) =
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );

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
                let event =
                    Event::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press));
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
            fn q_opens_quit_dialog_when_not_interactive() {
                let mut app = fixture_app();
                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
                );
                assert!(!matches!(cmd, Cmd::Quit));
                assert_eq!(
                    app.confirm_dialog().map(|dialog| dialog.focused_field),
                    Some(crate::ui::tui::ConfirmDialogField::CancelButton)
                );
            }

            #[test]
            fn enter_on_default_no_cancels_quit_dialog() {
                let mut app = fixture_app();
                let open_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
                );
                assert!(!matches!(open_cmd, Cmd::Quit));
                assert!(app.confirm_dialog().is_some());

                let confirm_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );
                assert!(!matches!(confirm_cmd, Cmd::Quit));
                assert!(app.confirm_dialog().is_none());
            }

            #[test]
            fn y_confirms_quit_dialog_and_quits() {
                let mut app = fixture_app();
                let open_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
                );
                assert!(!matches!(open_cmd, Cmd::Quit));
                assert!(app.confirm_dialog().is_some());

                let confirm_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('y')).with_kind(KeyEventKind::Press)),
                );
                assert!(matches!(confirm_cmd, Cmd::Quit));
                assert!(app.confirm_dialog().is_none());
            }

            #[test]
            fn escape_cancels_quit_dialog() {
                let mut app = fixture_app();
                let _ = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
                );
                assert!(app.confirm_dialog().is_some());

                let cancel_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );
                assert!(!matches!(cancel_cmd, Cmd::Quit));
                assert!(app.confirm_dialog().is_none());
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
            fn ctrl_c_opens_quit_dialog_when_not_interactive() {
                let mut app = fixture_app();
                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('c'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert!(!matches!(cmd, Cmd::Quit));
                assert_eq!(
                    app.confirm_dialog().map(|dialog| dialog.focused_field),
                    Some(crate::ui::tui::ConfirmDialogField::CancelButton)
                );
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
                assert!(app.launch_dialog().is_some());

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('c'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.launch_dialog().is_none());
            }

            #[test]
            fn ctrl_c_dismisses_delete_modal_via_action_mapper() {
                let mut app = fixture_app();
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                assert!(app.delete_dialog().is_some());

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('c'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.delete_dialog().is_none());
            }

            #[test]
            fn ctrl_c_with_task_running_does_not_quit() {
                let mut app = fixture_app();
                app.dialogs.start_in_flight = true;

                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('c'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(!matches!(cmd, Cmd::Quit));
                assert_eq!(
                    app.confirm_dialog().map(|dialog| dialog.focused_field),
                    Some(crate::ui::tui::ConfirmDialogField::CancelButton)
                );
            }

            #[test]
            fn ctrl_d_with_task_running_does_not_quit() {
                let mut app = fixture_app();
                app.dialogs.start_in_flight = true;

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
                assert!(app.launch_dialog().is_some());
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
                            "resize-window".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "-x".to_string(),
                            "80".to_string(),
                            "-y".to_string(),
                            "36".to_string(),
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
            fn h_and_l_toggle_focus_between_panes_when_not_interactive() {
                let mut app = fixture_app();
                app.state.mode = UiMode::List;
                app.state.focus = PaneFocus::WorkspaceList;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(app.state.focus, PaneFocus::Preview);
                assert_eq!(app.state.mode, UiMode::Preview);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('l')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
                assert_eq!(app.state.mode, UiMode::Preview);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('h')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(app.state.focus, PaneFocus::Preview);
                assert_eq!(app.state.mode, UiMode::Preview);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('h')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
                assert_eq!(app.state.mode, UiMode::Preview);
            }

            #[test]
            fn alt_j_and_alt_k_move_workspace_selection_from_preview_focus() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.mode = UiMode::Preview;
                app.state.focus = PaneFocus::Preview;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('k'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.state.selected_index, 0);
                assert_eq!(app.state.mode, UiMode::Preview);
                assert_eq!(app.state.focus, PaneFocus::Preview);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('j'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.state.selected_index, 1);
            }

            #[test]
            fn alt_brackets_switch_preview_tab_from_list_focus() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.state.mode = UiMode::List;
                app.state.focus = PaneFocus::WorkspaceList;
                app.open_new_shell_tab();
                app.open_or_focus_git_tab();
                let Some(home_id) = app
                    .selected_workspace_tabs_state()
                    .and_then(|tabs| tabs.find_kind(WorkspaceTabKind::Home))
                    .map(|tab| tab.id)
                else {
                    panic!("home tab should exist");
                };
                let _ = app.select_tab_by_id_for_selected_workspace(home_id);
                assert_eq!(app.preview_tab, PreviewTab::Home);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char(']'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.state.mode, UiMode::Preview);
                assert_eq!(app.state.focus, PaneFocus::Preview);
                assert_eq!(app.preview_tab, PreviewTab::Shell);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('['))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.preview_tab, PreviewTab::Home);
            }

            #[test]
            fn alt_arrows_hl_bf_and_alt_with_extra_modifier_resize_sidebar_globally() {
                let mut app = fixture_app();
                app.sidebar_width_pct = 33;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Right)
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.sidebar_width_pct, 35);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Left)
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.sidebar_width_pct, 33);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('l'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.sidebar_width_pct, 35);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('h'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.sidebar_width_pct, 33);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('f'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.sidebar_width_pct, 35);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('b'))
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.sidebar_width_pct, 33);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Right)
                            .with_modifiers(Modifiers::ALT | Modifiers::SHIFT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(app.sidebar_width_pct, 35);
            }

            #[test]
            fn alt_resize_keeps_interactive_mode_active() {
                let mut app = fixture_app();
                app.sidebar_width_pct = 33;
                app.session.interactive = Some(InteractiveState::new(
                    "%0".to_string(),
                    "grove-ws-feature-a-shell".to_string(),
                    Instant::now(),
                    34,
                    78,
                ));

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Right)
                            .with_modifiers(Modifiers::ALT)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.session.interactive.is_some());
                assert_eq!(app.sidebar_width_pct, 35);
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
            fn start_key_applies_project_agent_env_defaults_before_agent_launch() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.projects[0].defaults.agent_env.codex = vec![
                    "CODEX_CONFIG_DIR=~/.codex-work".to_string(),
                    "OPENAI_API_BASE=https://api.example.com/v1".to_string(),
                ];

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "send-keys".to_string(),
                            "-t".to_string(),
                            "grove-ws-feature-a".to_string(),
                            "export CODEX_CONFIG_DIR='~/.codex-work' OPENAI_API_BASE='https://api.example.com/v1'"
                                .to_string(),
                            "Enter".to_string(),
                        ]
                }));
            }

            #[test]
            fn start_key_rejects_invalid_project_agent_env_defaults() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.projects[0].defaults.agent_env.codex = vec!["INVALID-KEY=value".to_string()];

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(commands.borrow().is_empty());
                assert!(
                    app.status_bar_line()
                        .contains("invalid project agent env: invalid env key 'INVALID-KEY'")
                );
            }

            #[test]
            fn unsafe_toggle_updates_launch_skip_permissions_for_session() {
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                focus_agent_preview_tab(&mut app);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('!')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.launch_skip_permissions);
                assert!(!app.config_path.exists());
            }

            #[test]
            fn start_key_persists_workspace_skip_permissions_marker() {
                let workspace_dir = unique_temp_workspace_dir("start-skip-marker");
                let (mut app, _commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                app.state.workspaces[1].path = workspace_dir.clone();
                app.launch_skip_permissions = true;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                let marker = fs::read_to_string(workspace_dir.join(".grove/skip_permissions"))
                    .expect("skip marker should exist after start");
                assert_eq!(marker, "true\n");
                assert!(app.launch_skip_permissions);
                assert!(!app.config_path.exists());

                let _ = fs::remove_dir_all(workspace_dir);
            }

            #[test]
            fn start_key_uses_workspace_prompt_file_launcher_script() {
                let workspace_dir = unique_temp_workspace_dir("prompt");
                let prompt_path = workspace_dir.join(".grove/prompt");
                fs::create_dir_all(workspace_dir.join(".grove"))
                    .expect(".grove dir should be writable");
                fs::write(&prompt_path, "fix bug\nand add tests")
                    .expect("prompt file should be writable");

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
                        format!("bash {}/.grove/start.sh", workspace_dir.display()),
                        "Enter".to_string(),
                    ])
                );

                let launcher_path = workspace_dir.join(".grove/start.sh");
                let launcher_script =
                    fs::read_to_string(&launcher_path).expect("launcher script should be written");
                assert!(launcher_script.contains("fix bug"));
                assert!(launcher_script.contains("and add tests"));
                assert!(launcher_script.contains("codex"));

                let _ = fs::remove_dir_all(workspace_dir);
            }

            #[test]
            fn start_dialog_init_command_runs_before_agent() {
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
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                let last_command = commands
                    .borrow()
                    .last()
                    .expect("last tmux command should exist")
                    .clone();
                assert_eq!(last_command[0], "tmux");
                assert_eq!(last_command[1], "send-keys");
                assert_eq!(last_command[2], "-t");
                assert_eq!(last_command[3], "grove-ws-feature-a");
                assert_eq!(last_command[5], "Enter");
                let launch_command = &last_command[4];
                assert!(launch_command.contains("workspace-init-"));
                assert!(launch_command.contains("direnv allow"));
                assert!(launch_command.contains("codex"));
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
                assert_eq!(
                    app.launch_dialog().map(|dialog| dialog.focused_field),
                    Some(LaunchDialogField::StartConfig(
                        StartAgentConfigField::Unsafe
                    ))
                );
                app.handle_launch_dialog_key(
                    KeyEvent::new(KeyCode::Char(' ')).with_kind(KeyEventKind::Press),
                );
                assert_eq!(
                    app.launch_dialog()
                        .map(|dialog| dialog.start_config.skip_permissions),
                    Some(true)
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
                    app.launch_dialog()
                        .map(|dialog| dialog.start_config.prompt.clone()),
                    Some("k".to_string())
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );

                assert!(app.launch_dialog().is_none());
                assert!(commands.borrow().is_empty());
            }

            #[test]
            fn start_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.open_start_dialog();

                assert_eq!(
                    app.launch_dialog().map(|dialog| dialog.focused_field),
                    Some(LaunchDialogField::StartConfig(
                        StartAgentConfigField::Prompt
                    ))
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.launch_dialog().map(|dialog| dialog.focused_field),
                    Some(LaunchDialogField::StartConfig(
                        StartAgentConfigField::InitCommand
                    ))
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
                    app.launch_dialog().map(|dialog| dialog.focused_field),
                    Some(LaunchDialogField::StartConfig(
                        StartAgentConfigField::Prompt
                    ))
                );
            }

            #[test]
            fn new_workspace_key_opens_create_dialog() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.base_branch.clone()),
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

                let Some(dialog) = app.edit_dialog() else {
                    panic!("edit dialog should be open");
                };
                assert_eq!(dialog.workspace_name, "grove");
                assert!(dialog.is_main);
                assert_eq!(dialog.branch, "main");
                assert_eq!(dialog.base_branch, "main");
                assert_eq!(dialog.focused_field, EditDialogField::BaseBranch);
            }

            #[test]
            fn edit_dialog_save_updates_workspace_base_branch_marker() {
                let mut app = fixture_app();
                let workspace_dir = unique_temp_workspace_dir("edit-save");
                app.state.selected_index = 1;
                app.state.workspaces[1].path = workspace_dir.clone();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('e')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
                );
                for character in ['d', 'e', 'v', 'e', 'l', 'o', 'p'] {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
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

                assert!(app.edit_dialog().is_none());
                assert_eq!(
                    app.state.workspaces[1].base_branch.as_deref(),
                    Some("develop")
                );
                assert_eq!(
                    fs::read_to_string(workspace_dir.join(".grove/base"))
                        .expect("base marker should be readable")
                        .trim(),
                    "develop"
                );
                assert!(app.status_bar_line().contains("workspace updated"));
            }

            #[test]
            fn edit_dialog_save_switches_main_workspace_branch() {
                let mut app = fixture_app();
                let workspace_dir = unique_temp_workspace_dir("edit-main-branch");
                let init_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["init", "-b", "main"])
                    .output()
                    .expect("git init should run");
                assert!(
                    init_output.status.success(),
                    "git init failed: {}",
                    String::from_utf8_lossy(&init_output.stderr)
                );
                let user_name_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["config", "user.name", "Grove Tests"])
                    .output()
                    .expect("git config user.name should run");
                assert!(
                    user_name_output.status.success(),
                    "git config user.name failed: {}",
                    String::from_utf8_lossy(&user_name_output.stderr)
                );
                let user_email_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["config", "user.email", "grove-tests@example.com"])
                    .output()
                    .expect("git config user.email should run");
                assert!(
                    user_email_output.status.success(),
                    "git config user.email failed: {}",
                    String::from_utf8_lossy(&user_email_output.stderr)
                );
                fs::write(workspace_dir.join("README.md"), "initial\n")
                    .expect("write should succeed");
                let add_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["add", "README.md"])
                    .output()
                    .expect("git add should run");
                assert!(
                    add_output.status.success(),
                    "git add failed: {}",
                    String::from_utf8_lossy(&add_output.stderr)
                );
                let commit_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["commit", "-m", "initial"])
                    .output()
                    .expect("git commit should run");
                assert!(
                    commit_output.status.success(),
                    "git commit failed: {}",
                    String::from_utf8_lossy(&commit_output.stderr)
                );
                let switch_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["switch", "-c", "develop"])
                    .output()
                    .expect("git switch -c develop should run");
                assert!(
                    switch_output.status.success(),
                    "git switch -c develop failed: {}",
                    String::from_utf8_lossy(&switch_output.stderr)
                );
                let back_to_main_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["switch", "main"])
                    .output()
                    .expect("git switch main should run");
                assert!(
                    back_to_main_output.status.success(),
                    "git switch main failed: {}",
                    String::from_utf8_lossy(&back_to_main_output.stderr)
                );
                app.state.workspaces[0].path = workspace_dir.clone();
                app.state.workspaces[0].branch = "main".to_string();
                app.state.workspaces[0].base_branch = Some("main".to_string());

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('e')).with_kind(KeyEventKind::Press)),
                );
                for _ in 0..4 {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
                    );
                }
                for character in ['d', 'e', 'v', 'e', 'l', 'o', 'p'] {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
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

                let head_output = std::process::Command::new("git")
                    .current_dir(&workspace_dir)
                    .args(["rev-parse", "--abbrev-ref", "HEAD"])
                    .output()
                    .expect("git rev-parse should run");
                assert!(
                    head_output.status.success(),
                    "git rev-parse failed: {}",
                    String::from_utf8_lossy(&head_output.stderr)
                );
                assert_eq!(
                    String::from_utf8_lossy(&head_output.stdout).trim(),
                    "develop"
                );
                assert_eq!(app.state.workspaces[0].branch, "develop");
                assert_eq!(
                    app.state.workspaces[0].base_branch.as_deref(),
                    Some("develop")
                );
                assert!(
                    app.status_bar_line()
                        .contains("base workspace switched to 'develop'")
                );
            }

            #[test]
            fn edit_dialog_save_rejects_empty_base_branch() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('e')).with_kind(KeyEventKind::Press)),
                );

                for _ in 0..4 {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(KeyEvent::new(KeyCode::Backspace).with_kind(KeyEventKind::Press)),
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

                assert!(app.edit_dialog().is_some());
                assert!(app.status_bar_line().contains("base branch is required"));
            }

            #[test]
            fn edit_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();
                app.open_edit_dialog();

                assert_eq!(
                    app.edit_dialog().map(|dialog| dialog.focused_field),
                    Some(EditDialogField::BaseBranch)
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.edit_dialog().map(|dialog| dialog.focused_field),
                    Some(EditDialogField::SaveButton)
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
                    app.edit_dialog().map(|dialog| dialog.focused_field),
                    Some(EditDialogField::BaseBranch)
                );
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

                let Some(dialog) = app.delete_dialog() else {
                    panic!("delete dialog should be open");
                };
                assert_eq!(dialog.workspace_name, "feature-a");
                assert_eq!(dialog.branch, "feature-a");
                assert_eq!(dialog.focused_field, DeleteDialogField::DeleteLocalBranch);
                assert!(dialog.kill_tmux_sessions);
            }

            #[test]
            fn delete_key_on_main_workspace_shows_guard_toast() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.delete_dialog().is_none());
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
                    app.delete_dialog().map(|dialog| dialog.focused_field),
                    Some(DeleteDialogField::KillTmuxSessions)
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );
                assert!(app.delete_dialog().is_none());
            }

            #[test]
            fn delete_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.open_delete_dialog();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.delete_dialog().map(|dialog| dialog.focused_field),
                    Some(DeleteDialogField::KillTmuxSessions)
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
                    app.delete_dialog().map(|dialog| dialog.focused_field),
                    Some(DeleteDialogField::DeleteLocalBranch)
                );
            }

            #[test]
            fn delete_dialog_space_toggles_kill_tmux_sessions() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.open_delete_dialog();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(
                    app.delete_dialog().map(|dialog| dialog.focused_field),
                    Some(DeleteDialogField::KillTmuxSessions)
                );
                assert!(
                    app.delete_dialog()
                        .is_some_and(|dialog| dialog.kill_tmux_sessions)
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char(' ')).with_kind(KeyEventKind::Press)),
                );
                assert!(
                    app.delete_dialog()
                        .is_some_and(|dialog| !dialog.kill_tmux_sessions)
                );
            }
        }
        mod polling_and_status {
            use super::*;

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
            fn stop_agent_emits_dialog_and_lifecycle_events() {
                let (mut app, _commands, _captures, _cursor_captures, events) =
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );
                focus_agent_preview_tab(&mut app);
                app.state.selected_index = 1;
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('x')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                let kinds = event_kinds(&events);
                assert_kind_subsequence(
                    &kinds,
                    &["dialog_opened", "dialog_confirmed", "agent_stopped"],
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
            fn preview_poll_capture_completed_logs_scrollback_lines() {
                let (mut app, _commands, _captures, _cursor_captures, events) =
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        vec![Ok("line one\nline two\n".to_string())],
                        Vec::new(),
                    );
                app.state.selected_index = 1;

                force_tick_due(&mut app);
                ftui::Model::update(&mut app, Msg::Tick);

                let capture_event = recorded_events(&events)
                    .into_iter()
                    .find(|event| event.kind == "capture_completed")
                    .expect("capture_completed event should exist");
                let Value::Object(data) = capture_event.data else {
                    panic!("capture_completed data should be an object");
                };
                assert_eq!(
                    data.get("scrollback_lines"),
                    Some(&Value::from(usize_to_u64(200)))
                );
            }

            #[test]
            fn tick_queues_async_preview_poll_with_background_io() {
                let config_path = unique_config_path("background-poll");
                let mut app = GroveApp::from_parts(
                    fixture_bootstrap(WorkspaceStatus::Active),
                    Box::new(BackgroundOnlyTmuxInput),
                    config_path,
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
                let config_path = unique_config_path("background-status-only");
                let mut app = GroveApp::from_parts(
                    fixture_bootstrap(WorkspaceStatus::Idle),
                    Box::new(BackgroundOnlyTmuxInput),
                    config_path,
                    Box::new(NullEventLogger),
                    None,
                );
                app.state.selected_index = 0;
                force_tick_due(&mut app);

                let cmd = ftui::Model::update(&mut app, Msg::Tick);
                assert!(!cmd_contains_task(&cmd));
            }

            #[test]
            fn poll_preview_marks_request_when_background_poll_is_in_flight() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                app.polling.preview_poll_in_flight = true;

                app.poll_preview();

                assert!(app.polling.preview_poll_requested);
                assert!(app.telemetry.deferred_cmds.is_empty());
            }

            #[test]
            fn async_preview_still_polls_background_workspace_status_targets_when_live_preview_exists()
             {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                app.state.workspaces[0].status = WorkspaceStatus::Active;

                let live_preview = app.prepare_live_preview_session();
                assert!(live_preview.is_some());

                let status_targets =
                    app.status_poll_targets_for_async_preview(live_preview.as_ref());
                assert_eq!(status_targets.len(), 1);
                assert_eq!(status_targets[0].workspace_name, "grove");
            }

            #[test]
            fn async_preview_polls_workspace_status_targets_when_live_preview_missing() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 0;

                let live_preview = app.prepare_live_preview_session();
                assert!(live_preview.is_none());

                let status_targets =
                    app.status_poll_targets_for_async_preview(live_preview.as_ref());
                assert_eq!(status_targets.len(), 1);
                assert_eq!(status_targets[0].workspace_name, "feature-a");
            }

            #[test]
            fn async_preview_rate_limits_background_workspace_status_targets() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                app.state.workspaces[0].status = WorkspaceStatus::Active;

                let live_preview = app.prepare_live_preview_session();
                assert!(live_preview.is_some());

                let initial_targets =
                    app.status_poll_targets_for_async_preview(live_preview.as_ref());
                assert_eq!(initial_targets.len(), 1);
                app.polling.last_workspace_status_poll_at = Some(Instant::now());

                let throttled_targets =
                    app.status_poll_targets_for_async_preview(live_preview.as_ref());
                assert!(throttled_targets.is_empty());
            }

            #[test]
            fn prepare_live_preview_session_launches_shell_from_list_mode() {
                let (mut app, commands, _captures, _cursor_captures) =
                    fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
                app.state.selected_index = 1;
                app.state.mode = UiMode::List;
                app.state.focus = PaneFocus::WorkspaceList;

                let live_preview = app.prepare_live_preview_session();

                assert_eq!(
                    live_preview
                        .as_ref()
                        .map(|target| target.session_name.as_str()),
                    Some("grove-ws-feature-a-shell")
                );
                assert!(live_preview.is_some_and(|target| target.include_escape_sequences));
                assert!(
                    app.session
                        .shell_sessions
                        .ready
                        .contains("grove-ws-feature-a-shell")
                );
                assert!(commands.borrow().iter().any(|command| {
                    command
                        == &vec![
                            "tmux".to_string(),
                            "new-session".to_string(),
                            "-d".to_string(),
                            "-s".to_string(),
                            "grove-ws-feature-a-shell".to_string(),
                            "-c".to_string(),
                            "/repos/grove-feature-a".to_string(),
                        ]
                }));
            }

            #[test]
            fn preview_poll_completion_runs_deferred_background_poll_request() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                app.polling.poll_generation = 1;
                app.polling.preview_poll_in_flight = true;
                app.polling.preview_poll_requested = true;

                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::PreviewPollCompleted(PreviewPollCompletion {
                        generation: 1,
                        live_capture: None,
                        cursor_capture: None,
                        workspace_status_captures: Vec::new(),
                    }),
                );

                assert!(app.polling.preview_poll_in_flight);
                assert!(!app.polling.preview_poll_requested);
                assert!(cmd_contains_task(&cmd));
            }

            #[test]
            fn switching_workspace_drops_in_flight_capture_for_previous_session() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                app.preview.apply_capture("stale-feature-output\n");
                app.polling.poll_generation = 1;
                app.polling.preview_poll_in_flight = true;

                let switch_cmd =
                    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));

                assert_eq!(app.state.selected_index, 0);
                assert!(cmd_contains_task(&switch_cmd));
                assert!(!app.polling.preview_poll_requested);
                assert_eq!(app.polling.poll_generation, 2);
                assert_ne!(app.preview.lines, vec!["stale-feature-output".to_string()]);

                let stale_cmd = ftui::Model::update(
                    &mut app,
                    Msg::PreviewPollCompleted(PreviewPollCompletion {
                        generation: 1,
                        live_capture: Some(LivePreviewCapture {
                            session: "grove-ws-feature-a".to_string(),
                            scrollback_lines: 600,
                            include_escape_sequences: false,
                            capture_ms: 1,
                            total_ms: 1,
                            result: Ok("stale-output\n".to_string()),
                        }),
                        cursor_capture: None,
                        workspace_status_captures: Vec::new(),
                    }),
                );

                assert!(app.polling.preview_poll_in_flight);
                assert!(!app.polling.preview_poll_requested);
                assert!(!cmd_contains_task(&stale_cmd));
                assert!(
                    app.preview
                        .lines
                        .iter()
                        .all(|line| !line.contains("stale-output"))
                );
                assert_ne!(app.preview.lines, vec!["stale-feature-output".to_string()]);

                ftui::Model::update(
                    &mut app,
                    Msg::PreviewPollCompleted(PreviewPollCompletion {
                        generation: 2,
                        live_capture: Some(LivePreviewCapture {
                            session: "grove-ws-grove".to_string(),
                            scrollback_lines: 600,
                            include_escape_sequences: false,
                            capture_ms: 1,
                            total_ms: 1,
                            result: Ok("fresh-main-output\n".to_string()),
                        }),
                        cursor_capture: None,
                        workspace_status_captures: Vec::new(),
                    }),
                );

                assert!(!app.polling.preview_poll_in_flight);
                assert_eq!(app.preview.lines, vec!["fresh-main-output".to_string()]);
            }

            #[test]
            fn switching_to_active_workspace_keeps_existing_preview_until_fresh_capture() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                if let Some(main_workspace) = app.state.workspaces.get_mut(0) {
                    main_workspace.status = WorkspaceStatus::Active;
                }
                app.state.selected_index = 1;
                app.preview.apply_capture("feature-live-output\n");
                app.polling.poll_generation = 1;
                app.polling.preview_poll_in_flight = true;

                let switch_cmd =
                    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));

                assert_eq!(app.state.selected_index, 0);
                assert!(cmd_contains_task(&switch_cmd));
                assert!(!app.polling.preview_poll_requested);
                assert_eq!(app.polling.poll_generation, 2);
                assert_eq!(app.preview.lines, vec!["feature-live-output".to_string()]);
            }

            #[test]
            fn async_preview_capture_failure_sets_toast_message() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                if let Some(workspace) = app.state.workspaces.get_mut(1) {
                    workspace.status = WorkspaceStatus::Active;
                }

                ftui::Model::update(
                    &mut app,
                    Msg::PreviewPollCompleted(PreviewPollCompletion {
                        generation: 1,
                        live_capture: Some(LivePreviewCapture {
                            session: "grove-ws-feature-a".to_string(),
                            scrollback_lines: 600,
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
                    fixture_app_with_tmux_and_events(
                        WorkspaceStatus::Active,
                        Vec::new(),
                        Vec::new(),
                    );
                app.state.selected_index = 1;
                app.preview.lines = vec!["initial".to_string()];
                app.preview.render_lines = vec!["initial".to_string()];
                app.polling.poll_generation = 2;

                ftui::Model::update(
                    &mut app,
                    Msg::PreviewPollCompleted(PreviewPollCompletion {
                        generation: 1,
                        live_capture: Some(LivePreviewCapture {
                            session: "grove-ws-feature-a".to_string(),
                            scrollback_lines: 600,
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
                            scrollback_lines: 600,
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
                            scrollback_lines: 600,
                            include_escape_sequences: true,
                            capture_ms: 1,
                            total_ms: 1,
                            result: Ok("hello\u{1b}[?1000h\u{1b}[<35;192;47M".to_string()),
                        }),
                        cursor_capture: None,
                        workspace_status_captures: Vec::new(),
                    }),
                );
                assert!(app.polling.output_changing);

                ftui::Model::update(
                    &mut app,
                    Msg::PreviewPollCompleted(PreviewPollCompletion {
                        generation: 2,
                        live_capture: Some(LivePreviewCapture {
                            session: "grove-ws-feature-a".to_string(),
                            scrollback_lines: 600,
                            include_escape_sequences: true,
                            capture_ms: 1,
                            total_ms: 1,
                            result: Ok("hello\u{1b}[?1000l".to_string()),
                        }),
                        cursor_capture: None,
                        workspace_status_captures: Vec::new(),
                    }),
                );

                assert!(!app.polling.output_changing);
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
                            scrollback_lines: 600,
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
                assert!(
                    !app.workspace_attention
                        .contains_key(&PathBuf::from("/repos/grove-feature-a"))
                );
            }

            #[test]
            fn preview_poll_ignores_done_pattern_embedded_in_control_sequence() {
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
                            scrollback_lines: 600,
                            include_escape_sequences: true,
                            capture_ms: 1,
                            total_ms: 1,
                            result: Ok("still working\n\u{1b}]0;task completed\u{7}\n".to_string()),
                        }),
                        cursor_capture: None,
                        workspace_status_captures: Vec::new(),
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
            fn preview_poll_transition_from_done_to_thinking_clears_attention() {
                let mut app = fixture_app();
                app.state.selected_index = 0;
                app.workspace_attention.insert(
                    PathBuf::from("/repos/grove-feature-a"),
                    super::WorkspaceAttention::NeedsAttention,
                );

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
                            result: Ok("thinking...".to_string()),
                        }],
                    }),
                );

                assert_eq!(app.state.workspaces[1].status, WorkspaceStatus::Thinking);
                assert!(
                    !app.workspace_attention
                        .contains_key(&PathBuf::from("/repos/grove-feature-a"))
                );
            }

            #[test]
            fn background_poll_transition_from_waiting_to_active_clears_attention() {
                let mut app = fixture_app();
                app.state.selected_index = 0;
                app.workspace_attention.insert(
                    PathBuf::from("/repos/grove-feature-a"),
                    super::WorkspaceAttention::NeedsAttention,
                );

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
                            result: Ok("still working on it".to_string()),
                        }],
                    }),
                );

                assert_eq!(app.state.workspaces[1].status, WorkspaceStatus::Active);
                assert!(
                    !app.workspace_attention
                        .contains_key(&PathBuf::from("/repos/grove-feature-a"))
                );
            }

            #[test]
            fn selecting_workspace_does_not_clear_attention() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 0;
                app.workspace_attention.insert(
                    PathBuf::from("/repos/grove-feature-a"),
                    super::WorkspaceAttention::NeedsAttention,
                );

                ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('j'))));
                assert_eq!(app.state.selected_index, 1);
                assert!(
                    app.workspace_attention
                        .contains_key(&PathBuf::from("/repos/grove-feature-a"))
                );
            }

            #[test]
            fn entering_interactive_does_not_clear_attention() {
                let mut app = fixture_background_app(WorkspaceStatus::Active);
                app.state.selected_index = 1;
                app.state.mode = UiMode::Preview;
                app.state.focus = PaneFocus::Preview;
                app.state.workspaces[1].status = WorkspaceStatus::Active;
                app.workspace_attention.insert(
                    PathBuf::from("/repos/grove-feature-a"),
                    super::WorkspaceAttention::NeedsAttention,
                );

                assert!(app.enter_interactive(Instant::now()));
                assert!(
                    app.workspace_attention
                        .contains_key(&PathBuf::from("/repos/grove-feature-a"))
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
            fn tmux_workspace_status_poll_targets_skip_idle_workspaces() {
                let mut app = fixture_app();
                app.state.selected_index = 0;
                app.state.workspaces[1].status = WorkspaceStatus::Idle;

                let targets = workspace_status_targets_for_polling_with_live_preview(
                    &app.state.workspaces,
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
                app.session.interactive = Some(InteractiveState::new(
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
                            scrollback_lines: 600,
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
                assert!(app.session.interactive.is_none());
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
        }
        mod projects_and_creation {
            use super::*;

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
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
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
                    app.project_dialog()
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
                    app.project_dialog().map(|dialog| dialog.filter.clone()),
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
                    app.project_dialog().map(|dialog| dialog.filter.clone()),
                    Some("jk".to_string())
                );
            }

            #[test]
            fn project_dialog_tab_and_shift_tab_navigate_selection() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(
                    app.project_dialog()
                        .map(|dialog| dialog.selected_filtered_index),
                    Some(0)
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(
                    app.project_dialog()
                        .map(|dialog| dialog.selected_filtered_index),
                    Some(1)
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::BackTab).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(
                    app.project_dialog()
                        .map(|dialog| dialog.selected_filtered_index),
                    Some(0)
                );
            }

            #[test]
            fn project_dialog_ctrl_n_and_ctrl_p_match_tab_navigation() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
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
                    app.project_dialog()
                        .map(|dialog| dialog.selected_filtered_index),
                    Some(1)
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.project_dialog()
                        .map(|dialog| dialog.selected_filtered_index),
                    Some(0)
                );
            }

            #[test]
            fn project_dialog_ctrl_r_enters_reorder_mode() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('r'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.reorder.as_ref())
                        .is_some()
                );
            }

            #[test]
            fn project_dialog_reorder_j_and_k_move_selection() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('r'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(app.projects[0].name, "site");
                assert_eq!(app.projects[1].name, "grove");

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(app.projects[0].name, "grove");
                assert_eq!(app.projects[1].name, "site");
            }

            #[test]
            fn project_dialog_reorder_enter_saves_project_order() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });
                let mut site_workspace = Workspace::try_new(
                    "site".to_string(),
                    PathBuf::from("/repos/site"),
                    "main".to_string(),
                    Some(1_700_000_300),
                    AgentType::Claude,
                    WorkspaceStatus::Main,
                    true,
                )
                .expect("workspace should be valid");
                site_workspace.project_path = Some(PathBuf::from("/repos/site"));
                app.state.workspaces.push(site_workspace);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('r'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Down).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(app.projects[0].name, "site");
                assert_eq!(app.projects[1].name, "grove");
                assert!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.reorder.as_ref())
                        .is_none()
                );

                let loaded = crate::infrastructure::config::load_from_path(&app.config_path)
                    .expect("config loads");
                assert_eq!(loaded.projects[0].name, "site");
                assert_eq!(loaded.projects[1].name, "grove");
                assert_eq!(
                    app.state
                        .workspaces
                        .iter()
                        .map(|workspace| workspace.name.as_str())
                        .collect::<Vec<_>>(),
                    vec!["site", "grove", "feature-a"]
                );
                assert_eq!(app.state.selected_index, 1);

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Up).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(
                    app.state
                        .selected_workspace()
                        .map(|workspace| workspace.name.as_str()),
                    Some("site")
                );
            }

            #[test]
            fn project_dialog_reorder_escape_restores_original_order() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('r'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Down).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(app.projects[0].name, "grove");
                assert_eq!(app.projects[1].name, "site");
                assert!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.reorder.as_ref())
                        .is_none()
                );
            }

            #[test]
            fn project_add_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('a'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert_eq!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.add_dialog.as_ref())
                        .map(|dialog| dialog.focused_field),
                    Some(ProjectAddDialogField::Name)
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.add_dialog.as_ref())
                        .map(|dialog| dialog.focused_field),
                    Some(ProjectAddDialogField::Path)
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
                    app.project_dialog()
                        .and_then(|dialog| dialog.add_dialog.as_ref())
                        .map(|dialog| dialog.focused_field),
                    Some(ProjectAddDialogField::Name)
                );
            }

            #[test]
            fn project_dialog_ctrl_x_removes_selected_project() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('x'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert_eq!(app.projects.len(), 1);
                assert_eq!(app.projects[0].name, "grove");
                let loaded = crate::infrastructure::config::load_from_path(&app.config_path)
                    .expect("config loads");
                assert_eq!(loaded.projects.len(), 1);
                assert_eq!(loaded.projects[0].name, "grove");
            }

            #[test]
            fn project_dialog_ctrl_x_queues_background_project_delete() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);
                app.projects.push(ProjectConfig {
                    name: "site".to_string(),
                    path: PathBuf::from("/repos/site"),
                    defaults: Default::default(),
                });

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                );
                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('x'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(app.dialogs.project_delete_in_flight);
                assert!(cmd_contains_task(&cmd));
            }

            #[test]
            fn project_delete_completion_clears_in_flight_and_applies_projects() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);
                app.dialogs.project_delete_in_flight = true;
                let kept = ProjectConfig {
                    name: "grove".to_string(),
                    path: PathBuf::from("/repos/grove"),
                    defaults: Default::default(),
                };

                ftui::Model::update(
                    &mut app,
                    Msg::DeleteProjectCompleted(DeleteProjectCompletion {
                        project_name: "site".to_string(),
                        project_path: PathBuf::from("/repos/site"),
                        projects: vec![kept.clone()],
                        result: Ok(()),
                    }),
                );

                assert!(!app.dialogs.project_delete_in_flight);
                assert_eq!(app.projects, vec![kept]);
            }

            #[test]
            fn project_dialog_ctrl_e_opens_project_defaults_dialog() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('e'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.defaults_dialog.as_ref())
                        .is_some()
                );
                assert_eq!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.defaults_dialog.as_ref())
                        .map(|dialog| dialog.workspace_init_command.clone()),
                    Some(String::new())
                );
            }

            #[test]
            fn project_defaults_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('e'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );

                assert_eq!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.defaults_dialog.as_ref())
                        .map(|dialog| dialog.focused_field),
                    Some(ProjectDefaultsDialogField::BaseBranch)
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.project_dialog()
                        .and_then(|dialog| dialog.defaults_dialog.as_ref())
                        .map(|dialog| dialog.focused_field),
                    Some(ProjectDefaultsDialogField::WorkspaceInitCommand)
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
                    app.project_dialog()
                        .and_then(|dialog| dialog.defaults_dialog.as_ref())
                        .map(|dialog| dialog.focused_field),
                    Some(ProjectDefaultsDialogField::BaseBranch)
                );
            }

            #[test]
            fn project_defaults_dialog_save_persists_defaults() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('p')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('e'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                for character in ['d', 'e', 'v'] {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                );
                for character in [
                    'd', 'i', 'r', 'e', 'n', 'v', ' ', 'a', 'l', 'l', 'o', 'w', ';', 'e', 'c', 'h',
                    'o', ' ', 'o', 'k',
                ] {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                );
                for character in [
                    'C', 'L', 'A', 'U', 'D', 'E', '_', 'C', 'O', 'N', 'F', 'I', 'G', '_', 'D', 'I',
                    'R', '=', '~', '/', '.', 'c', 'l', 'a', 'u', 'd', 'e', '-', 'w', 'o', 'r', 'k',
                ] {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                );
                for character in [
                    'C', 'O', 'D', 'E', 'X', '_', 'C', 'O', 'N', 'F', 'I', 'G', '_', 'D', 'I', 'R',
                    '=', '~', '/', '.', 'c', 'o', 'd', 'e', 'x', '-', 'w', 'o', 'r', 'k',
                ] {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                );
                for character in [
                    'O', 'P', 'E', 'N', 'C', 'O', 'D', 'E', '_', 'C', 'O', 'N', 'F', 'I', 'G', '_',
                    'D', 'I', 'R', '=', '~', '/', '.', 'o', 'p', 'e', 'n', 'c', 'o', 'd', 'e', '-',
                    'w', 'o', 'r', 'k',
                ] {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }
                for _ in 0..1 {
                    ftui::Model::update(
                        &mut app,
                        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
                    );
                }
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(app.projects[0].defaults.base_branch, "dev");
                assert_eq!(
                    app.projects[0].defaults.workspace_init_command,
                    "direnv allow;echo ok".to_string()
                );
                assert_eq!(
                    app.projects[0].defaults.agent_env.claude,
                    vec!["CLAUDE_CONFIG_DIR=~/.claude-work".to_string()]
                );
                assert_eq!(
                    app.projects[0].defaults.agent_env.codex,
                    vec!["CODEX_CONFIG_DIR=~/.codex-work".to_string()]
                );
                assert_eq!(
                    app.projects[0].defaults.agent_env.opencode,
                    vec!["OPENCODE_CONFIG_DIR=~/.opencode-work".to_string()]
                );

                let loaded = crate::infrastructure::config::load_from_path(&app.config_path)
                    .expect("config loads");
                assert_eq!(loaded.projects[0].defaults.base_branch, "dev");
                assert_eq!(
                    loaded.projects[0].defaults.workspace_init_command,
                    "direnv allow;echo ok".to_string()
                );
                assert_eq!(
                    loaded.projects[0].defaults.agent_env.claude,
                    vec!["CLAUDE_CONFIG_DIR=~/.claude-work".to_string()]
                );
                assert_eq!(
                    loaded.projects[0].defaults.agent_env.codex,
                    vec!["CODEX_CONFIG_DIR=~/.codex-work".to_string()]
                );
                assert_eq!(
                    loaded.projects[0].defaults.agent_env.opencode,
                    vec!["OPENCODE_CONFIG_DIR=~/.opencode-work".to_string()]
                );
            }

            #[test]
            fn new_workspace_dialog_prefills_from_project_defaults() {
                let mut app = fixture_app();
                app.projects[0].defaults.base_branch = "develop".to_string();
                app.projects[0].defaults.workspace_init_command = "direnv allow".to_string();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.base_branch.clone()),
                    Some("develop".to_string())
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
                assert!(app.dialogs.refresh_in_flight);
            }

            #[test]
            fn refresh_workspace_completion_does_not_auto_launch_sessions_for_new_workspace() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);

                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
                        preferred_workspace_path: Some(PathBuf::from("/repos/grove-feature-a")),
                        bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
                    }),
                );

                assert!(!cmd_contains_task(&cmd));
                assert!(!app.dialogs.start_in_flight);
                assert!(
                    !app.session
                        .shell_sessions
                        .in_flight
                        .contains("grove-ws-feature-a-shell")
                );
            }
        }
        mod workspace_operations {
            use super::*;

            fn fixture_background_app_with_two_feature_workspaces() -> GroveApp {
                let mut bootstrap = fixture_bootstrap(WorkspaceStatus::Idle);
                let mut second_feature_workspace = Workspace::try_new(
                    "feature-b".to_string(),
                    PathBuf::from("/repos/grove-feature-b"),
                    "feature-b".to_string(),
                    Some(1_700_000_050),
                    AgentType::Codex,
                    WorkspaceStatus::Idle,
                    false,
                )
                .expect("workspace should be valid");
                second_feature_workspace.project_path = Some(PathBuf::from("/repos/grove"));
                second_feature_workspace.base_branch = Some("main".to_string());
                bootstrap.workspaces.push(second_feature_workspace);

                GroveApp::from_parts(
                    bootstrap,
                    Box::new(BackgroundOnlyTmuxInput),
                    unique_config_path("delete-queue"),
                    Box::new(NullEventLogger),
                    None,
                )
            }

            #[test]
            fn delete_dialog_confirm_queues_background_task() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);
                app.state.selected_index = 1;
                let deleting_path = app.state.workspaces[1].path.clone();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );

                assert!(cmd_contains_task(&cmd));
                assert!(app.delete_dialog().is_none());
                assert!(app.dialogs.delete_in_flight);
                assert_eq!(app.dialogs.delete_in_flight_workspace, Some(deleting_path));
                assert!(
                    app.dialogs
                        .delete_requested_workspaces
                        .contains(&app.state.workspaces[1].path)
                );
            }

            #[test]
            fn delete_dialog_confirm_queues_additional_delete_request_when_one_is_in_flight() {
                let mut app = fixture_background_app_with_two_feature_workspaces();
                let first_workspace_path = app.state.workspaces[1].path.clone();
                let second_workspace_path = app.state.workspaces[2].path.clone();

                app.state.selected_index = 1;
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                let first_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                assert!(cmd_contains_task(&first_cmd));

                app.state.selected_index = 2;
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                let second_cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );

                assert!(!cmd_contains_task(&second_cmd));
                assert!(app.dialogs.delete_in_flight);
                assert_eq!(
                    app.dialogs.delete_in_flight_workspace,
                    Some(first_workspace_path.clone())
                );
                assert_eq!(app.dialogs.pending_delete_workspaces.len(), 1);
                assert!(
                    app.dialogs
                        .delete_requested_workspaces
                        .contains(&first_workspace_path)
                );
                assert!(
                    app.dialogs
                        .delete_requested_workspaces
                        .contains(&second_workspace_path)
                );
            }

            #[test]
            fn delete_workspace_completion_starts_next_queued_delete_request() {
                let mut app = fixture_background_app_with_two_feature_workspaces();
                let first_workspace_path = app.state.workspaces[1].path.clone();
                let second_workspace_path = app.state.workspaces[2].path.clone();

                app.state.selected_index = 1;
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                app.state.selected_index = 2;
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('D')).with_kind(KeyEventKind::Press)),
                );

                let completion_cmd = ftui::Model::update(
                    &mut app,
                    Msg::DeleteWorkspaceCompleted(DeleteWorkspaceCompletion {
                        workspace_name: "feature-a".to_string(),
                        workspace_path: first_workspace_path.clone(),
                        result: Ok(()),
                        warnings: Vec::new(),
                    }),
                );

                assert!(cmd_contains_task(&completion_cmd));
                assert!(app.dialogs.delete_in_flight);
                assert_eq!(
                    app.dialogs.delete_in_flight_workspace,
                    Some(second_workspace_path.clone())
                );
                assert!(app.dialogs.pending_delete_workspaces.is_empty());
                assert!(
                    !app.dialogs
                        .delete_requested_workspaces
                        .contains(&first_workspace_path)
                );
                assert!(
                    app.dialogs
                        .delete_requested_workspaces
                        .contains(&second_workspace_path)
                );
            }

            #[test]
            fn delete_workspace_completion_clears_in_flight_workspace_marker() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);
                let deleting_path = app.state.workspaces[1].path.clone();
                app.dialogs.delete_in_flight = true;
                app.dialogs.delete_in_flight_workspace = Some(deleting_path.clone());
                app.dialogs
                    .delete_requested_workspaces
                    .insert(deleting_path.clone());

                let _ = ftui::Model::update(
                    &mut app,
                    Msg::DeleteWorkspaceCompleted(DeleteWorkspaceCompletion {
                        workspace_name: "feature-a".to_string(),
                        workspace_path: deleting_path.clone(),
                        result: Ok(()),
                        warnings: Vec::new(),
                    }),
                );

                assert!(!app.dialogs.delete_in_flight);
                assert!(app.dialogs.delete_in_flight_workspace.is_none());
                assert!(
                    !app.dialogs
                        .delete_requested_workspaces
                        .contains(&deleting_path)
                );
            }

            #[test]
            fn merge_key_opens_merge_dialog_for_selected_workspace() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('m')).with_kind(KeyEventKind::Press)),
                );

                let Some(dialog) = app.merge_dialog() else {
                    panic!("merge dialog should be open");
                };
                assert_eq!(dialog.workspace_name, "feature-a");
                assert_eq!(dialog.workspace_branch, "feature-a");
                assert_eq!(dialog.base_branch, "main");
                assert!(dialog.cleanup_workspace);
                assert!(dialog.cleanup_local_branch);
                assert_eq!(dialog.focused_field, MergeDialogField::CleanupWorkspace);
            }

            #[test]
            fn merge_key_on_main_workspace_shows_guard_toast() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('m')).with_kind(KeyEventKind::Press)),
                );

                assert!(app.merge_dialog().is_none());
                assert!(
                    app.status_bar_line()
                        .contains("cannot merge base workspace")
                );
            }

            #[test]
            fn merge_dialog_confirm_queues_background_task() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('m')).with_kind(KeyEventKind::Press)),
                );
                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('m')).with_kind(KeyEventKind::Press)),
                );

                assert!(cmd_contains_task(&cmd));
                assert!(app.merge_dialog().is_none());
                assert!(app.dialogs.merge_in_flight);
            }

            #[test]
            fn merge_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.open_merge_dialog();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.merge_dialog().map(|dialog| dialog.focused_field),
                    Some(MergeDialogField::CleanupLocalBranch)
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
                    app.merge_dialog().map(|dialog| dialog.focused_field),
                    Some(MergeDialogField::CleanupWorkspace)
                );
            }

            #[test]
            fn merge_completion_conflict_error_shows_compact_conflict_summary() {
                let mut app = fixture_app();
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::MergeWorkspaceCompleted(MergeWorkspaceCompletion {
                        workspace_name: "feature-a".to_string(),
                        workspace_path: PathBuf::from("/repos/grove-feature-a"),
                        workspace_branch: "feature-a".to_string(),
                        base_branch: "main".to_string(),
                        result: Err(
                            "git merge --no-ff feature-a: CONFLICT (content): Merge conflict in src/a.rs\nCONFLICT (content): Merge conflict in src/b.rs\nAutomatic merge failed; fix conflicts and then commit the result."
                                .to_string(),
                        ),
                        warnings: Vec::new(),
                    }),
                );

                let status = app.status_bar_line();
                assert!(status.contains("merge conflict"));
                assert!(status.contains("resolve in base worktree"));
            }

            #[test]
            fn update_key_opens_update_from_base_dialog_for_selected_workspace() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('u')).with_kind(KeyEventKind::Press)),
                );

                let Some(dialog) = app.update_from_base_dialog() else {
                    panic!("update dialog should be open");
                };
                assert_eq!(dialog.workspace_name, "feature-a");
                assert_eq!(dialog.workspace_branch, "feature-a");
                assert_eq!(dialog.base_branch, "main");
                assert_eq!(
                    dialog.focused_field,
                    UpdateFromBaseDialogField::UpdateButton
                );
            }

            #[test]
            fn update_key_on_main_workspace_opens_upstream_update_dialog() {
                let mut app = fixture_app();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('u')).with_kind(KeyEventKind::Press)),
                );

                let Some(dialog) = app.update_from_base_dialog() else {
                    panic!("update dialog should be open");
                };
                assert_eq!(dialog.workspace_name, "grove");
                assert_eq!(dialog.workspace_branch, "main");
                assert_eq!(dialog.base_branch, "main");
                assert!(dialog.is_main_workspace);
            }

            #[test]
            fn update_dialog_confirm_queues_background_task() {
                let mut app = fixture_background_app(WorkspaceStatus::Idle);
                app.state.selected_index = 1;

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('u')).with_kind(KeyEventKind::Press)),
                );
                let cmd = ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('u')).with_kind(KeyEventKind::Press)),
                );

                assert!(cmd_contains_task(&cmd));
                assert!(app.update_from_base_dialog().is_none());
                assert!(app.dialogs.update_from_base_in_flight);
            }

            #[test]
            fn update_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();
                app.state.selected_index = 1;
                app.open_update_from_base_dialog();

                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.update_from_base_dialog()
                        .map(|dialog| dialog.focused_field),
                    Some(UpdateFromBaseDialogField::CancelButton)
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
                    app.update_from_base_dialog()
                        .map(|dialog| dialog.focused_field),
                    Some(UpdateFromBaseDialogField::UpdateButton)
                );
            }

            #[test]
            fn settings_dialog_ctrl_n_and_ctrl_p_cycle_fields() {
                let mut app = fixture_app();
                app.open_settings_dialog();

                assert_eq!(
                    app.settings_dialog().map(|dialog| dialog.focused_field),
                    Some(SettingsDialogField::Theme)
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.settings_dialog().map(|dialog| dialog.focused_field),
                    Some(SettingsDialogField::SaveButton)
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(
                        KeyEvent::new(KeyCode::Char('n'))
                            .with_modifiers(Modifiers::CTRL)
                            .with_kind(KeyEventKind::Press),
                    ),
                );
                assert_eq!(
                    app.settings_dialog().map(|dialog| dialog.focused_field),
                    Some(SettingsDialogField::CancelButton)
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
                    app.settings_dialog().map(|dialog| dialog.focused_field),
                    Some(SettingsDialogField::SaveButton)
                );
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
                    app.create_dialog().map(|dialog| dialog.focused_field),
                    Some(CreateDialogField::Project)
                );
            }

            #[test]
            fn create_dialog_j_and_k_on_project_field_switch_project() {
                let mut app = fixture_app();
                app.projects.push(ProjectConfig {
                    name: "other".to_string(),
                    path: PathBuf::from("/repos/other"),
                    defaults: Default::default(),
                });
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
                    Msg::Key(KeyEvent::new(KeyCode::Char('j')).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.project_index),
                    Some(1)
                );
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
                );
                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.project_index),
                    Some(0)
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
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }

                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.base_branch.clone()),
                    Some("dev".to_string())
                );
            }

            #[test]
            fn create_dialog_ctrl_n_and_ctrl_p_follow_tab_navigation() {
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
                    app.create_dialog().map(|dialog| dialog.focused_field),
                    Some(CreateDialogField::CancelButton)
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
                    app.create_dialog().map(|dialog| dialog.focused_field),
                    Some(CreateDialogField::CreateButton)
                );
            }

            #[test]
            fn create_dialog_ctrl_n_and_ctrl_p_move_focus_from_base_branch() {
                let mut app = fixture_app();
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
                );
                app.dialogs.create_branch_all = vec![
                    "main".to_string(),
                    "develop".to_string(),
                    "release".to_string(),
                ];
                if let Some(dialog) = app.create_dialog_mut() {
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
                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.focused_field),
                    Some(CreateDialogField::CreateButton)
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
                    app.create_dialog().map(|dialog| dialog.focused_field),
                    Some(CreateDialogField::BaseBranch)
                );
            }

            #[test]
            fn create_dialog_base_branch_dropdown_selects_with_enter() {
                let mut app = fixture_app();
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Char('n')).with_kind(KeyEventKind::Press)),
                );

                app.dialogs.create_branch_all = vec![
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
                        Msg::Key(
                            KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press),
                        ),
                    );
                }
                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.base_branch.clone()),
                    Some("develop".to_string())
                );
                assert_eq!(
                    app.create_dialog().map(|dialog| dialog.focused_field),
                    Some(CreateDialogField::CreateButton)
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
                    app.create_dialog()
                        .map(|dialog| dialog.workspace_name.clone()),
                    Some("j".to_string())
                );

                ftui::Model::update(
                    &mut app,
                    Msg::Key(KeyEvent::new(KeyCode::Escape).with_kind(KeyEventKind::Press)),
                );
                assert!(app.create_dialog().is_none());
            }

            #[test]
            fn create_dialog_enter_without_name_shows_validation_toast() {
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
                    Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
                );

                assert!(app.create_dialog().is_some());
                assert!(app.status_bar_line().contains("workspace name is required"));
            }

            #[test]
            fn create_dialog_enter_on_cancel_closes_modal() {
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

                assert!(app.create_dialog().is_none());
            }
        }
    }
}

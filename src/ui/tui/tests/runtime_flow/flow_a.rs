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
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());
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
    app.preview_poll_in_flight = true;

    app.poll_preview();

    assert!(app.preview_poll_requested);
    assert!(app.deferred_cmds.is_empty());
}

#[test]
fn async_preview_still_polls_background_workspace_status_targets_when_live_preview_exists() {
    let mut app = fixture_background_app(WorkspaceStatus::Active);
    app.state.selected_index = 1;
    app.state.workspaces[0].status = WorkspaceStatus::Active;

    let live_preview = app.prepare_live_preview_session();
    assert!(live_preview.is_some());

    let status_targets = app.status_poll_targets_for_async_preview(live_preview.as_ref());
    assert_eq!(status_targets.len(), 1);
    assert_eq!(status_targets[0].workspace_name, "grove");
}

#[test]
fn async_preview_polls_workspace_status_targets_when_live_preview_missing() {
    let mut app = fixture_background_app(WorkspaceStatus::Active);
    app.state.selected_index = 0;

    let live_preview = app.prepare_live_preview_session();
    assert!(live_preview.is_none());

    let status_targets = app.status_poll_targets_for_async_preview(live_preview.as_ref());
    assert_eq!(status_targets.len(), 1);
    assert_eq!(status_targets[0].workspace_name, "feature-a");
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
        app.shell_sessions
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
    app.poll_generation = 1;
    app.preview_poll_in_flight = true;
    app.preview_poll_requested = true;

    let cmd = ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 1,
            live_capture: None,
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );

    assert!(app.preview_poll_in_flight);
    assert!(!app.preview_poll_requested);
    assert!(cmd_contains_task(&cmd));
}

#[test]
fn switching_workspace_drops_in_flight_capture_for_previous_session() {
    let mut app = fixture_background_app(WorkspaceStatus::Active);
    app.state.selected_index = 1;
    app.preview.apply_capture("stale-feature-output\n");
    app.poll_generation = 1;
    app.preview_poll_in_flight = true;

    let switch_cmd = ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));

    assert_eq!(app.state.selected_index, 0);
    assert!(cmd_contains_task(&switch_cmd));
    assert!(!app.preview_poll_requested);
    assert_eq!(app.poll_generation, 2);
    assert_ne!(app.preview.lines, vec!["stale-feature-output".to_string()]);

    let stale_cmd = ftui::Model::update(
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

    assert!(app.preview_poll_in_flight);
    assert!(!app.preview_poll_requested);
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
                include_escape_sequences: false,
                capture_ms: 1,
                total_ms: 1,
                result: Ok("fresh-main-output\n".to_string()),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );

    assert!(!app.preview_poll_in_flight);
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
    app.poll_generation = 1;
    app.preview_poll_in_flight = true;

    let switch_cmd = ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));

    assert_eq!(app.state.selected_index, 0);
    assert!(cmd_contains_task(&switch_cmd));
    assert!(!app.preview_poll_requested);
    assert_eq!(app.poll_generation, 2);
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
fn selecting_workspace_clears_attention_for_selected_workspace() {
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
                result: Ok("Approve command? [y/n]".to_string()),
            }],
        }),
    );
    app.workspace_attention.insert(
        PathBuf::from("/repos/grove-feature-a"),
        super::WorkspaceAttention::NeedsAttention,
    );
    assert!(
        app.workspace_attention
            .contains_key(&PathBuf::from("/repos/grove-feature-a"))
    );

    ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('j'))));
    assert_eq!(app.state.selected_index, 1);
    assert!(
        !app.workspace_attention
            .contains_key(&PathBuf::from("/repos/grove-feature-a"))
    );
}

#[test]
fn entering_interactive_clears_attention_for_selected_workspace() {
    let mut app = fixture_app();
    app.state.selected_index = 1;
    app.state.mode = UiMode::Preview;
    app.state.focus = PaneFocus::Preview;
    app.state.workspaces[1].status = WorkspaceStatus::Active;

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
    app.workspace_attention.insert(
        PathBuf::from("/repos/grove-feature-a"),
        super::WorkspaceAttention::NeedsAttention,
    );

    assert!(app.enter_interactive(Instant::now()));
    assert!(
        !app.workspace_attention
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

    let targets =
        workspace_status_targets_for_polling_with_live_preview(&app.state.workspaces, None);
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
    for _ in 0..7 {
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

    let loaded =
        crate::infrastructure::config::load_from_path(&app.config_path).expect("config loads");
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
    let loaded =
        crate::infrastructure::config::load_from_path(&app.config_path).expect("config loads");
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

    assert!(app.project_delete_in_flight);
    assert!(cmd_contains_task(&cmd));
}

#[test]
fn project_delete_completion_clears_in_flight_and_applies_projects() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.project_delete_in_flight = true;
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

    assert!(!app.project_delete_in_flight);
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
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    for character in [
        'd', 'i', 'r', 'e', 'n', 'v', ' ', 'a', 'l', 'l', 'o', 'w', ';', 'e', 'c', 'h', 'o', ' ',
        'o', 'k',
    ] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    for character in [
        'C', 'L', 'A', 'U', 'D', 'E', '_', 'C', 'O', 'N', 'F', 'I', 'G', '_', 'D', 'I', 'R', '=',
        '~', '/', '.', 'c', 'l', 'a', 'u', 'd', 'e', '-', 'w', 'o', 'r', 'k',
    ] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    for character in [
        'C', 'O', 'D', 'E', 'X', '_', 'C', 'O', 'N', 'F', 'I', 'G', '_', 'D', 'I', 'R', '=', '~',
        '/', '.', 'c', 'o', 'd', 'e', 'x', '-', 'w', 'o', 'r', 'k',
    ] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    for character in [
        'O', 'P', 'E', 'N', 'C', 'O', 'D', 'E', '_', 'C', 'O', 'N', 'F', 'I', 'G', '_', 'D', 'I',
        'R', '=', '~', '/', '.', 'o', 'p', 'e', 'n', 'c', 'o', 'd', 'e', '-', 'w', 'o', 'r', 'k',
    ] {
        ftui::Model::update(
            &mut app,
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
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

    let loaded =
        crate::infrastructure::config::load_from_path(&app.config_path).expect("config loads");
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
    assert_eq!(
        app.create_dialog()
            .map(|dialog| dialog.start_config.init_command.clone()),
        Some("direnv allow".to_string())
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
    assert_eq!(
        app.pending_auto_start_workspace
            .as_ref()
            .map(|pending| pending.workspace_path.clone()),
        Some(PathBuf::from("/repos/grove-feature-x"))
    );
    assert_eq!(
        app.pending_auto_start_workspace
            .as_ref()
            .map(|pending| pending.start_config.clone()),
        Some(StartAgentConfigState::new(
            String::new(),
            String::new(),
            false
        ))
    );
    assert_eq!(
        app.pending_auto_launch_shell_workspace_path,
        Some(PathBuf::from("/repos/grove-feature-x"))
    );
}

#[test]
fn refresh_workspace_completion_autostarts_agent_for_new_workspace() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.pending_auto_start_workspace = Some(PendingAutoStartWorkspace {
        workspace_path: PathBuf::from("/repos/grove-feature-a"),
        start_config: StartAgentConfigState::new(String::new(), String::new(), true),
    });

    let cmd = ftui::Model::update(
        &mut app,
        Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
            preferred_workspace_path: Some(PathBuf::from("/repos/grove-feature-a")),
            bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
        }),
    );

    assert!(cmd_contains_task(&cmd));
    assert!(app.start_in_flight);
    assert!(app.pending_auto_start_workspace.is_none());
    assert!(app.launch_skip_permissions);
    assert!(
        !app.shell_sessions
            .in_flight
            .contains("grove-ws-feature-a-shell")
    );
}

#[test]
fn refresh_workspace_completion_auto_launches_shell_for_new_workspace() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.pending_auto_launch_shell_workspace_path = Some(PathBuf::from("/repos/grove-feature-a"));

    let cmd = ftui::Model::update(
        &mut app,
        Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
            preferred_workspace_path: Some(PathBuf::from("/repos/grove-feature-a")),
            bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
        }),
    );

    assert!(cmd_contains_task(&cmd));
    assert!(
        app.shell_sessions
            .in_flight
            .contains("grove-ws-feature-a-shell")
    );
    assert!(app.pending_auto_launch_shell_workspace_path.is_none());
}

#[test]
fn auto_start_pending_workspace_agent_uses_pending_start_config() {
    let workspace_dir = unique_temp_workspace_dir("pending-auto-start");
    fs::create_dir_all(workspace_dir.join(".grove")).expect(".grove dir should be writable");

    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    app.state.selected_index = 1;
    app.state.workspaces[1].path = workspace_dir.clone();
    app.pending_auto_start_workspace = Some(PendingAutoStartWorkspace {
        workspace_path: workspace_dir.clone(),
        start_config: StartAgentConfigState::new(
            "fix flaky test".to_string(),
            "direnv allow".to_string(),
            true,
        ),
    });

    let _ = app.auto_start_pending_workspace_agent();

    assert!(app.pending_auto_start_workspace.is_none());
    assert!(!app.start_in_flight);
    assert!(app.launch_skip_permissions);
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
    assert!(launcher_script.contains("fix flaky test"));
    assert!(launcher_script.contains("direnv allow"));
    assert!(launcher_script.contains("workspace-init-"));
    assert!(launcher_script.contains("codex --dangerously-bypass-approvals-and-sandbox"));

    let _ = fs::remove_dir_all(workspace_dir);
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
    assert_eq!(
        app.confirm_dialog().map(|dialog| dialog.focused_field),
        Some(crate::ui::tui::ConfirmDialogField::CancelButton)
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
    app.preview_tab = PreviewTab::Agent;

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
    assert_eq!(app.preview_tab, PreviewTab::Agent);
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
    app.interactive = Some(InteractiveState::new(
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

    assert!(app.interactive.is_some());
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
    fs::create_dir_all(workspace_dir.join(".grove")).expect(".grove dir should be writable");
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
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
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
    app.handle_launch_dialog_key(KeyEvent::new(KeyCode::Char(' ')).with_kind(KeyEventKind::Press));
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
        app.create_dialog().map(|dialog| dialog.agent),
        Some(AgentType::Claude)
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
    assert_eq!(dialog.agent, AgentType::Claude);
    assert_eq!(dialog.focused_field, EditDialogField::BaseBranch);
}

#[test]
fn edit_dialog_save_updates_workspace_agent_base_branch_and_markers() {
    let mut app = fixture_app();
    let workspace_dir = unique_temp_workspace_dir("edit-save");
    app.state.selected_index = 1;
    app.state.workspaces[1].path = workspace_dir.clone();
    app.state.workspaces[1].agent = AgentType::Codex;

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
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
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
        Msg::Key(KeyEvent::new(KeyCode::Tab).with_kind(KeyEventKind::Press)),
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Enter).with_kind(KeyEventKind::Press)),
    );

    assert!(app.edit_dialog().is_none());
    assert_eq!(app.state.workspaces[1].agent, AgentType::OpenCode);
    assert_eq!(
        app.state.workspaces[1].base_branch.as_deref(),
        Some("develop")
    );
    assert_eq!(
        fs::read_to_string(workspace_dir.join(".grove/agent"))
            .expect("agent marker should be readable")
            .trim(),
        "opencode"
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
    fs::write(workspace_dir.join("README.md"), "initial\n").expect("write should succeed");
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
            Msg::Key(KeyEvent::new(KeyCode::Char(character)).with_kind(KeyEventKind::Press)),
        );
    }
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
        Some(EditDialogField::Agent)
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

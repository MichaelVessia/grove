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
fn async_preview_rate_limits_background_workspace_status_targets() {
    let mut app = fixture_background_app(WorkspaceStatus::Active);
    app.state.selected_index = 1;
    app.state.workspaces[0].status = WorkspaceStatus::Active;

    let live_preview = app.prepare_live_preview_session();
    assert!(live_preview.is_some());

    let initial_targets = app.status_poll_targets_for_async_preview(live_preview.as_ref());
    assert_eq!(initial_targets.len(), 1);
    app.polling.last_workspace_status_poll_at = Some(Instant::now());

    let throttled_targets = app.status_poll_targets_for_async_preview(live_preview.as_ref());
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

    let switch_cmd = ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));

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

    let switch_cmd = ftui::Model::update(&mut app, Msg::Key(key_press(KeyCode::Char('k'))));

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
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());
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

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
                result: Ok(live_preview_capture_output("stale-output\n")),
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
                result: Ok(live_preview_capture_output("fresh-main-output\n")),
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
                result: Ok(live_preview_capture_output("stale-output\n")),
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
                result: Ok(live_preview_capture_output("fresh-output\n")),
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
                result: Ok(live_preview_capture_output(
                    "hello\u{1b}[?1000h\u{1b}[<35;192;47M",
                )),
            }),
            cursor_capture: None,
            workspace_status_captures: Vec::new(),
        }),
    );
    assert!(app.output_changing);

    let prev_digest = app.preview.last_digest().cloned();
    ftui::Model::update(
        &mut app,
        Msg::PreviewPollCompleted(PreviewPollCompletion {
            generation: 2,
            live_capture: Some(LivePreviewCapture {
                session: "grove-ws-feature-a".to_string(),
                include_escape_sequences: true,
                capture_ms: 1,
                total_ms: 1,
                result: Ok(live_preview_capture_output_with(
                    "hello\u{1b}[?1000l",
                    prev_digest.as_ref(),
                )),
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
                result: Ok(live_preview_capture_output("Approve command? [y/n]")),
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
                result: Ok(live_preview_capture_output(
                    "still working\n\u{1b}]0;task completed\u{7}\n",
                )),
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
                result: Ok(workspace_status_capture_output("thinking...", false, true)),
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
                result: Ok(workspace_status_capture_output(
                    "still working on it",
                    false,
                    true,
                )),
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
                result: Ok(workspace_status_capture_output(
                    "Approve command? [y/n]",
                    false,
                    true,
                )),
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
                result: Ok(live_preview_capture_output("Approve command? [y/n]")),
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
                result: Ok(workspace_status_capture_output(
                    "> Implement {feature}\n? for shortcuts\n",
                    false,
                    true,
                )),
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
        None,
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
    for _ in 0..9 {
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
        target: crate::infrastructure::config::ProjectTarget::Local,
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
        target: crate::infrastructure::config::ProjectTarget::Local,
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
        target: crate::infrastructure::config::ProjectTarget::Local,
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
        target: crate::infrastructure::config::ProjectTarget::Local,
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
        target: crate::infrastructure::config::ProjectTarget::Local,
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
            .map(|dialog| dialog.auto_run_setup_commands),
        Some(true)
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
        Some(ProjectDefaultsDialogField::SetupCommands)
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
    for _ in 0..2 {
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
        app.projects[0].defaults.setup_commands,
        vec!["direnv allow".to_string(), "echo ok".to_string()]
    );
    assert!(app.projects[0].defaults.auto_run_setup_commands);

    let loaded =
        crate::infrastructure::config::load_from_path(&app.config_path).expect("config loads");
    assert_eq!(loaded.projects[0].defaults.base_branch, "dev");
    assert_eq!(
        loaded.projects[0].defaults.setup_commands,
        vec!["direnv allow".to_string(), "echo ok".to_string()]
    );
}

#[test]
fn new_workspace_dialog_prefills_from_project_defaults() {
    let mut app = fixture_app();
    app.projects[0].defaults.base_branch = "develop".to_string();
    app.projects[0].defaults.setup_commands = vec!["direnv allow".to_string()];
    app.projects[0].defaults.auto_run_setup_commands = true;

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
            .map(|dialog| dialog.setup_commands.clone()),
        Some("direnv allow".to_string())
    );
    assert_eq!(
        app.create_dialog()
            .map(|dialog| dialog.auto_run_setup_commands),
        Some(true)
    );
}

#[test]
fn create_workspace_completed_success_queues_refresh_task_in_background_mode() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);

    let cmd = ftui::Model::update(
        &mut app,
        Msg::CreateWorkspaceCompleted(CreateWorkspaceCompletion {
            workspace_name: "feature-x".to_string(),
            workspace_path: PathBuf::from("/repos/grove-feature-x"),
            result: Ok(()),
            warnings: Vec::new(),
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
    assert!(
        launcher_script
            .contains("direnv allow && codex --dangerously-bypass-approvals-and-sandbox")
    );

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
        None,
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
            StartAgentConfigField::PreLaunchCommand
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
    assert_eq!(app.state.workspaces[1].agent, AgentType::Claude);
    assert_eq!(
        app.state.workspaces[1].base_branch.as_deref(),
        Some("develop")
    );
    assert_eq!(
        fs::read_to_string(workspace_dir.join(".grove/agent"))
            .expect("agent marker should be readable")
            .trim(),
        "claude"
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
    assert!(app.delete_in_flight);
    assert_eq!(app.delete_in_flight_workspace, Some(deleting_path));
    assert!(
        app.delete_requested_workspaces
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
    assert!(app.delete_in_flight);
    assert_eq!(
        app.delete_in_flight_workspace,
        Some(first_workspace_path.clone())
    );
    assert_eq!(app.pending_delete_workspaces.len(), 1);
    assert!(
        app.delete_requested_workspaces
            .contains(&first_workspace_path)
    );
    assert!(
        app.delete_requested_workspaces
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
    assert!(app.delete_in_flight);
    assert_eq!(
        app.delete_in_flight_workspace,
        Some(second_workspace_path.clone())
    );
    assert!(app.pending_delete_workspaces.is_empty());
    assert!(
        !app.delete_requested_workspaces
            .contains(&first_workspace_path)
    );
    assert!(
        app.delete_requested_workspaces
            .contains(&second_workspace_path)
    );
}

#[test]
fn delete_workspace_completion_clears_in_flight_workspace_marker() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    let deleting_path = app.state.workspaces[1].path.clone();
    app.delete_in_flight = true;
    app.delete_in_flight_workspace = Some(deleting_path.clone());
    app.delete_requested_workspaces
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

    assert!(!app.delete_in_flight);
    assert!(app.delete_in_flight_workspace.is_none());
    assert!(!app.delete_requested_workspaces.contains(&deleting_path));
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
    assert!(app.merge_in_flight);
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
    assert!(app.update_from_base_in_flight);
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
        Some(SettingsDialogField::RemoteName)
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
        Some(SettingsDialogField::RemoteHost)
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
        Some(SettingsDialogField::RemoteName)
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
        app.create_dialog().map(|dialog| dialog.agent),
        Some(AgentType::Codex)
    );
    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('k')).with_kind(KeyEventKind::Press)),
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.agent),
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
    for _ in 0..5 {
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
        Some(CreateDialogField::StartConfig(
            StartAgentConfigField::Prompt
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
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::Agent)
    );
}

#[test]
fn create_dialog_ctrl_n_and_ctrl_p_move_focus_from_base_branch() {
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
        Some(CreateDialogField::SetupCommands)
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
        app.create_dialog().map(|dialog| dialog.base_branch.clone()),
        Some("develop".to_string())
    );
    assert_eq!(
        app.create_dialog().map(|dialog| dialog.focused_field),
        Some(CreateDialogField::SetupCommands)
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
    for _ in 0..9 {
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
    for _ in 0..10 {
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
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
        None,
    ));

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('x'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(app.interactive.is_some());
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
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        34,
        78,
        None,
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
fn start_key_noop_when_agent_tab_not_focused() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    app.state.selected_index = 1;

    ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('s')).with_kind(KeyEventKind::Press)),
    );

    assert!(app.launch_dialog().is_none());
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
        app.interactive
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
        app.interactive
            .as_ref()
            .map(|state| state.target_session.as_str()),
        Some("grove-ws-feature-a-shell")
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
        None,
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
fn preview_poll_drops_cursor_capture_for_non_interactive_session() {
    let (mut app, _commands, _captures, _cursor_captures, events) =
        fixture_app_with_tmux_and_events(WorkspaceStatus::Active, Vec::new(), Vec::new());
    app.state.selected_index = 1;
    app.interactive = Some(InteractiveState::new(
        "%0".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        20,
        80,
        None,
    ));
    if let Some(state) = app.interactive.as_mut() {
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
    assert!(app.interactive.is_some());

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::Char('q')).with_kind(KeyEventKind::Press)),
    );

    assert!(!matches!(cmd, Cmd::Quit));
    assert!(app.next_tick_due_at.is_some());
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
    assert!(app.interactive.is_some());

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(KeyEvent::new(KeyCode::BackTab).with_kind(KeyEventKind::Press)),
    );

    assert!(!matches!(cmd, Cmd::Quit));
    assert!(app.next_tick_due_at.is_some());
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
    assert!(app.interactive.is_some());

    let cmd = ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Enter)
                .with_modifiers(Modifiers::SHIFT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(!matches!(cmd, Cmd::Quit));
    assert!(app.next_tick_due_at.is_some());
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

    assert!(app.interactive.is_none());
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

    assert!(app.interactive.is_none());
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

    assert!(app.interactive.is_none());
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
    assert!(app.interactive.is_some());

    ftui::Model::update(
        &mut app,
        Msg::Key(
            KeyEvent::new(KeyCode::Char('k'))
                .with_modifiers(Modifiers::ALT)
                .with_kind(KeyEventKind::Press),
        ),
    );

    assert!(app.interactive.is_none());
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
    assert!(app.interactive.is_some());

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

    assert!(app.interactive.is_none());
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
fn tick_due_workspace_refresh_queues_inventory_refresh() {
    let mut app = fixture_background_app(WorkspaceStatus::Active);
    app.state.selected_index = 1;
    app.next_workspace_refresh_due_at = Some(Instant::now() - Duration::from_secs(1));
    force_tick_due(&mut app);

    let _ = ftui::Model::update(&mut app, Msg::Tick);

    assert!(app.refresh_in_flight);
    let next_due = app
        .next_workspace_refresh_due_at
        .expect("workspace refresh due timestamp should be set");
    assert!(next_due > Instant::now());
}

#[test]
fn periodic_refresh_completion_applies_external_workspace_changes() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.state.selected_index = 1;
    app.next_workspace_refresh_due_at = Some(Instant::now() - Duration::from_secs(1));
    force_tick_due(&mut app);

    let tick_cmd = ftui::Model::update(&mut app, Msg::Tick);
    assert!(cmd_contains_task(&tick_cmd));
    assert!(app.refresh_in_flight);

    let mut refreshed = fixture_bootstrap(WorkspaceStatus::Idle);
    let mut new_workspace = Workspace::try_new(
        "feature-b".to_string(),
        PathBuf::from("/repos/grove-feature-b"),
        "feature-b".to_string(),
        Some(1_700_000_050),
        AgentType::Codex,
        WorkspaceStatus::Idle,
        false,
    )
    .expect("workspace should be valid");
    new_workspace.project_path = Some(PathBuf::from("/repos/grove"));
    new_workspace.base_branch = Some("main".to_string());
    refreshed.workspaces.push(new_workspace);

    let _ = ftui::Model::update(
        &mut app,
        Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
            preferred_workspace_path: Some(PathBuf::from("/repos/grove-feature-a")),
            bootstrap: refreshed,
        }),
    );

    assert_eq!(app.state.workspaces.len(), 3);
    assert!(
        app.state
            .workspaces
            .iter()
            .any(|workspace| workspace.name == "feature-b")
    );
}

#[test]
fn in_flight_preview_poll_schedules_short_tick_for_task_results() {
    let mut app = fixture_background_app(WorkspaceStatus::Active);
    app.state.selected_index = 1;
    app.preview_poll_in_flight = true;
    app.next_tick_due_at = Some(Instant::now() + Duration::from_secs(5));

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
    let config_path = unique_config_path("cursor-overlay");
    let (mut app, _commands, _captures, _cursor_captures) = fixture_app_with_tmux_and_config_path(
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
    let config_path = unique_config_path("persist");
    let (mut app, _commands, _captures, _cursor_captures) = fixture_app_with_tmux_and_config_path(
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
    let saved = crate::infrastructure::config::load_from_path(&config_path)
        .expect("config should be readable");
    assert_eq!(saved.sidebar_width_pct, 52);

    let (app_reloaded, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux_and_config_path(
            WorkspaceStatus::Idle,
            Vec::new(),
            Vec::new(),
            config_path.clone(),
        );

    assert_eq!(app_reloaded.sidebar_width_pct, 52);
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
    assert!(app.interactive.is_none());
    assert_eq!(app.state.mode, UiMode::List);
    assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
}

fn preview_tab_click_point(sidebar_width_pct: u16, tab: PreviewTab) -> (u16, u16) {
    let layout = GroveApp::view_layout_for_size(100, 40, sidebar_width_pct, false);
    let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
    let tab_y = preview_inner.y.saturating_add(1);
    let mut tab_x = preview_inner.x;

    for (index, current_tab) in [PreviewTab::Agent, PreviewTab::Shell, PreviewTab::Git]
        .iter()
        .copied()
        .enumerate()
    {
        if index > 0 {
            tab_x = tab_x.saturating_add(1);
        }
        let Some(tab_width) = u16::try_from(current_tab.label().len().saturating_add(2)).ok()
        else {
            continue;
        };
        if current_tab == tab {
            return (tab_x, tab_y);
        }
        tab_x = tab_x.saturating_add(tab_width);
    }

    (preview_inner.x, tab_y)
}

#[test]
fn mouse_click_preview_tab_switches_tabs() {
    let mut app = fixture_app();
    assert_eq!(app.preview_tab, PreviewTab::Agent);

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    let (shell_tab_x, tab_y) = preview_tab_click_point(app.sidebar_width_pct, PreviewTab::Shell);

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
    assert!(app.interactive.is_none());
}

#[test]
fn mouse_click_preview_tab_exits_interactive_and_switches_tabs() {
    let (mut app, _commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    assert!(app.enter_interactive(Instant::now()));
    assert_eq!(app.preview_tab, PreviewTab::Agent);

    ftui::Model::update(
        &mut app,
        Msg::Resize {
            width: 100,
            height: 40,
        },
    );
    let (git_tab_x, tab_y) = preview_tab_click_point(app.sidebar_width_pct, PreviewTab::Git);

    ftui::Model::update(
        &mut app,
        Msg::Mouse(MouseEvent::new(
            MouseEventKind::Down(MouseButton::Left),
            git_tab_x,
            tab_y,
        )),
    );

    assert_eq!(app.preview_tab, PreviewTab::Git);
    assert!(app.interactive.is_none());
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

    assert!(app.interactive.is_some());
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
    assert!(app.interactive.is_none());
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
    let (mut app, commands, captures, _cursor_captures) = fixture_app_with_tmux_and_config_path(
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
            discovery_state: DiscoveryState::Error("fatal: not a git repository".to_string()),
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
        let tabs_line = row_text(frame, preview_inner.y.saturating_add(1), x_start, x_end);
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
        app.lazygit_sessions
            .in_flight
            .contains("grove-ws-grove-git")
    );
}

#[test]
fn git_tab_launches_lazygit_with_dedicated_tmux_session() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    let lazygit_command = app.lazygit_command.clone();

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
    app.lazygit_sessions
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

    assert!(app.lazygit_sessions.ready.contains("grove-ws-grove-git"));
    assert!(
        !app.lazygit_sessions
            .in_flight
            .contains("grove-ws-grove-git")
    );
    assert!(!app.lazygit_sessions.failed.contains("grove-ws-grove-git"));
}

#[test]
fn lazygit_launch_completion_failure_marks_session_failed() {
    let mut app = fixture_app();
    app.lazygit_sessions
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

    assert!(app.lazygit_sessions.failed.contains("grove-ws-grove-git"));
    assert!(
        !app.lazygit_sessions
            .in_flight
            .contains("grove-ws-grove-git")
    );
    assert!(app.status_bar_line().contains("lazygit launch failed"));
}

#[test]
fn lazygit_launch_completion_duplicate_session_marks_session_ready() {
    let mut app = fixture_app();
    app.lazygit_sessions
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

    assert!(app.lazygit_sessions.ready.contains("grove-ws-grove-git"));
    assert!(
        !app.lazygit_sessions
            .in_flight
            .contains("grove-ws-grove-git")
    );
    assert!(!app.lazygit_sessions.failed.contains("grove-ws-grove-git"));
    assert!(!app.status_bar_line().contains("lazygit launch failed"));
}

#[test]
fn workspace_shell_launch_completion_success_marks_session_ready() {
    let mut app = fixture_app();
    app.shell_sessions
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
        app.shell_sessions
            .ready
            .contains("grove-ws-feature-a-shell")
    );
    assert!(
        !app.shell_sessions
            .in_flight
            .contains("grove-ws-feature-a-shell")
    );
    assert!(
        !app.shell_sessions
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
    app.shell_sessions
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
    app.shell_sessions
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
        app.shell_sessions
            .ready
            .contains("grove-ws-feature-a-shell")
    );
    assert!(
        !app.shell_sessions
            .in_flight
            .contains("grove-ws-feature-a-shell")
    );
    assert!(
        !app.shell_sessions
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
    app.interactive = Some(InteractiveState::new(
        "%1".to_string(),
        "grove-ws-feature-a".to_string(),
        Instant::now(),
        3,
        80,
        None,
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

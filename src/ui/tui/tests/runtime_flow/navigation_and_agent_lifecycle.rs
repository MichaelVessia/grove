use super::*;

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

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
fn create_dialog_j_and_k_on_agent_field_toggle_agent() {
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
        Some(CreateDialogField::Agent)
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
    for _ in 0..8 {
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
fn interactive_send_clears_attention_for_agent_workspace() {
    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Active, Vec::new());
    app.state.selected_index = 1;
    app.interactive = Some(InteractiveState::new(
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
                "codex --dangerously-bypass-approvals-and-sandbox resume run-1234".to_string(),
                "Enter".to_string(),
            ]
    }));
}

#[test]
fn restart_key_uses_workspace_skip_permissions_marker_for_codex_resume() {
    let workspace_dir = unique_temp_workspace_dir("restart-skip-marker");
    fs::create_dir_all(workspace_dir.join(".grove")).expect(".grove dir should be writable");
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
                "codex --dangerously-bypass-approvals-and-sandbox resume run-1234".to_string(),
                "Enter".to_string(),
            ]
    }));

    let _ = fs::remove_dir_all(workspace_dir);
}

#[test]
fn restart_key_uses_workspace_skip_permissions_marker_for_main_codex_workspace() {
    let workspace_dir = unique_temp_workspace_dir("restart-main-skip-marker");
    fs::create_dir_all(workspace_dir.join(".grove")).expect(".grove dir should be writable");
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
                "codex --dangerously-bypass-approvals-and-sandbox resume run-main-1234".to_string(),
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
fn divider_ratio_changes_are_session_only() {
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
    assert!(app.interactive.is_none());
    assert_eq!(app.state.mode, UiMode::List);
    assert_eq!(app.state.focus, PaneFocus::WorkspaceList);
}

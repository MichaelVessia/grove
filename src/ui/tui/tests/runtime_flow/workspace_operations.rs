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

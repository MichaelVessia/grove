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
    assert!(app.dialogs.refresh_in_flight);
    assert_eq!(
        app.dialogs
            .pending_auto_start_workspace
            .as_ref()
            .map(|pending| pending.workspace_path.clone()),
        Some(PathBuf::from("/repos/grove-feature-x"))
    );
    assert_eq!(
        app.dialogs
            .pending_auto_start_workspace
            .as_ref()
            .map(|pending| pending.start_config.clone()),
        Some(StartAgentConfigState::new(
            String::new(),
            String::new(),
            false
        ))
    );
    assert_eq!(
        app.session.pending_auto_launch_shell_workspace_path,
        Some(PathBuf::from("/repos/grove-feature-x"))
    );
}

#[test]
fn refresh_workspace_completion_autostarts_agent_for_new_workspace() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.dialogs.pending_auto_start_workspace = Some(PendingAutoStartWorkspace {
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
    assert!(app.dialogs.start_in_flight);
    assert!(app.dialogs.pending_auto_start_workspace.is_none());
    assert!(app.launch_skip_permissions);
    assert!(
        !app.session
            .shell_sessions
            .in_flight
            .contains("grove-ws-feature-a-shell")
    );
}

#[test]
fn refresh_workspace_completion_auto_launches_shell_for_new_workspace() {
    let mut app = fixture_background_app(WorkspaceStatus::Idle);
    app.session.pending_auto_launch_shell_workspace_path =
        Some(PathBuf::from("/repos/grove-feature-a"));

    let cmd = ftui::Model::update(
        &mut app,
        Msg::RefreshWorkspacesCompleted(RefreshWorkspacesCompletion {
            preferred_workspace_path: Some(PathBuf::from("/repos/grove-feature-a")),
            bootstrap: fixture_bootstrap(WorkspaceStatus::Idle),
        }),
    );

    assert!(cmd_contains_task(&cmd));
    assert!(
        app.session
            .shell_sessions
            .in_flight
            .contains("grove-ws-feature-a-shell")
    );
    assert!(
        app.session
            .pending_auto_launch_shell_workspace_path
            .is_none()
    );
}

#[test]
fn auto_start_pending_workspace_agent_uses_pending_start_config() {
    let workspace_dir = unique_temp_workspace_dir("pending-auto-start");
    fs::create_dir_all(workspace_dir.join(".grove")).expect(".grove dir should be writable");

    let (mut app, commands, _captures, _cursor_captures) =
        fixture_app_with_tmux(WorkspaceStatus::Idle, Vec::new());
    app.state.selected_index = 1;
    app.state.workspaces[1].path = workspace_dir.clone();
    app.dialogs.pending_auto_start_workspace = Some(PendingAutoStartWorkspace {
        workspace_path: workspace_dir.clone(),
        start_config: StartAgentConfigState::new(
            "fix flaky test".to_string(),
            "direnv allow".to_string(),
            true,
        ),
    });

    let _ = app.auto_start_pending_workspace_agent();

    assert!(app.dialogs.pending_auto_start_workspace.is_none());
    assert!(!app.dialogs.start_in_flight);
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

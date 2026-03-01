use super::*;

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
fn preview_output_rows_do_not_force_theme_background_for_agent_tab() {
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
        assert_ne!(
            cell.bg,
            ui_theme().base,
            "agent tab should not force theme background at ({far_right_x},{output_y})",
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

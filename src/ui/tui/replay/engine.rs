pub fn replay_debug_record(path: &Path, options: &ReplayOptions) -> io::Result<ReplayOutcome> {
    let trace = parse_replay_trace(path)?;
    let mut app = app_from_bootstrap(&trace.bootstrap);
    let mut states_compared = 0usize;
    let mut frames_compared = 0usize;
    let mut snapshot_steps = Vec::new();

    for message in &trace.messages {
        app.replay_msg_seq_counter = message.seq;
        let _ = Model::update(&mut app, message.msg.to_msg());

        if let Err(error) = verify_invariants(&app) {
            return Err(io::Error::other(format!(
                "invariant failed at seq {} ({}): {error}",
                message.seq,
                message.msg.kind_name()
            )));
        }

        let actual_state = ReplayStateSnapshot::from_app(&app);
        if let Some(expected_state) = trace.states.get(&message.seq) {
            if actual_state != *expected_state {
                let expected = serde_json::to_string_pretty(expected_state)
                    .unwrap_or_else(|_| "<encode failed>".to_string());
                let actual = serde_json::to_string_pretty(&actual_state)
                    .unwrap_or_else(|_| "<encode failed>".to_string());
                return Err(io::Error::other(format!(
                    "state mismatch at seq {} ({}):\nexpected: {expected}\nactual: {actual}",
                    message.seq,
                    message.msg.kind_name()
                )));
            }
            states_compared = states_compared.saturating_add(1);
        }

        let actual_frame_hash = render_frame_hash(&app);
        if !options.invariant_only
            && let Some(expected_hashes) = trace.frame_hashes.get(&message.seq)
            && let Some(expected_hash) = expected_hashes.front().copied()
        {
            if actual_frame_hash != expected_hash {
                return Err(io::Error::other(format!(
                    "frame hash mismatch at seq {} ({}): expected {}, got {}",
                    message.seq,
                    message.msg.kind_name(),
                    expected_hash,
                    actual_frame_hash
                )));
            }
            frames_compared = frames_compared.saturating_add(1);
        }

        if options.snapshot_path.is_some() {
            snapshot_steps.push(ReplaySnapshotStep {
                seq: message.seq,
                msg_kind: message.msg.kind_name().to_string(),
                state: actual_state,
                frame_hash: actual_frame_hash,
            });
        }
    }

    if let Some(snapshot_path) = options.snapshot_path.as_ref() {
        let snapshot = ReplaySnapshotFile {
            schema_version: REPLAY_SCHEMA_VERSION,
            trace_path: path.display().to_string(),
            final_state: ReplayStateSnapshot::from_app(&app),
            steps: snapshot_steps,
        };
        let encoded = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| io::Error::other(format!("snapshot encode failed: {error}")))?;
        if let Some(parent) = snapshot_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(snapshot_path, encoded)?;
    }

    Ok(ReplayOutcome {
        trace_path: path.to_path_buf(),
        steps_replayed: trace.messages.len(),
        states_compared,
        frames_compared,
    })
}

fn app_from_bootstrap(snapshot: &ReplayBootstrapSnapshot) -> GroveApp {
    let bootstrap = snapshot.to_bootstrap_data();
    let mut app = GroveApp::from_parts_with_clipboard_and_projects(
        bootstrap,
        snapshot.projects.clone(),
        AppDependencies {
            tmux_input: Box::new(ReplayTmuxInput),
            clipboard: Box::new(ReplayClipboard::default()),
            config_path: replay_config_path(),
            event_log: Box::new(crate::infrastructure::event_log::NullEventLogger),
            debug_record_start_ts: None,
        },
    );

    app.state.selected_index = if app.state.workspaces.is_empty() {
        0
    } else {
        snapshot
            .selected_index
            .min(app.state.workspaces.len().saturating_sub(1))
    };
    app.state.focus = snapshot.focus.to_focus();
    app.state.mode = snapshot.mode.to_mode();
    app.preview_tab = snapshot.preview_tab.to_preview_tab();
    app.viewport_width = snapshot.viewport_width.max(1);
    app.viewport_height = snapshot.viewport_height.max(1);
    app.sidebar_width_pct = snapshot.sidebar_width_pct;
    app.sidebar_hidden = snapshot.sidebar_hidden;
    app.mouse_capture_enabled = snapshot.mouse_capture_enabled;
    app.launch_skip_permissions = snapshot.launch_skip_permissions;
    app.replay_msg_seq_counter = 0;
    app
}

fn verify_invariants(app: &GroveApp) -> Result<(), String> {
    if !app.state.workspaces.is_empty() && app.state.selected_index >= app.state.workspaces.len() {
        return Err(format!(
            "selected_index {} out of bounds for {} workspaces",
            app.state.selected_index,
            app.state.workspaces.len()
        ));
    }

    let active_modal_count = [
        app.active_dialog.is_some(),
        app.keybind_help_open,
        app.command_palette.is_visible(),
        app.interactive.is_some(),
    ]
    .into_iter()
    .filter(|active| *active)
    .count();

    if active_modal_count > 1 {
        return Err(format!(
            "modal exclusivity violated, {} modal states active",
            active_modal_count
        ));
    }

    Ok(())
}

fn render_frame_hash(app: &GroveApp) -> u64 {
    let mut pool = GraphemePool::new();
    let mut frame = Frame::new(
        app.viewport_width.max(1),
        app.viewport_height.max(1),
        &mut pool,
    );
    Model::view(app, &mut frame);

    let lines = frame_lines(&mut frame);
    let mut hasher = DefaultHasher::new();
    lines.hash(&mut hasher);
    hasher.finish()
}

fn frame_lines(frame: &mut Frame) -> Vec<String> {
    let height = frame.buffer.height();
    let mut lines = Vec::with_capacity(usize::from(height));

    for y in 0..height {
        let mut row = String::with_capacity(usize::from(frame.buffer.width()));
        for x in 0..frame.buffer.width() {
            let Some(cell) = frame.buffer.get(x, y).copied() else {
                continue;
            };
            if cell.is_continuation() {
                continue;
            }
            if let Some(value) = cell.content.as_char() {
                row.push(value);
                continue;
            }
            if let Some(grapheme_id) = cell.content.grapheme_id()
                && let Some(grapheme) = frame.pool.get(grapheme_id)
            {
                row.push_str(grapheme);
                continue;
            }
            row.push(' ');
        }
        lines.push(row.trim_end_matches(' ').to_string());
    }

    lines
}

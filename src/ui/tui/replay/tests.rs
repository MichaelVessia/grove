use super::engine::app_from_bootstrap;
use super::fixtures::sanitize_fixture_name;
use super::*;
use serde_json::json;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    std::env::temp_dir().join(format!(
        "grove-replay-{label}-{}-{timestamp}.{extension}",
        std::process::id()
    ))
}

fn minimal_bootstrap() -> ReplayBootstrapSnapshot {
    ReplayBootstrapSnapshot {
        repo_name: "grove".to_string(),
        discovery_state: ReplayDiscoveryState::Ready,
        projects: vec![ProjectConfig {
            name: "grove".to_string(),
            path: PathBuf::from("/tmp/grove"),
            defaults: Default::default(),
        }],
        workspaces: vec![ReplayWorkspace {
            name: "main".to_string(),
            path: PathBuf::from("/tmp/grove"),
            project_name: Some("grove".to_string()),
            project_path: Some(PathBuf::from("/tmp/grove")),
            branch: "main".to_string(),
            base_branch: Some("main".to_string()),
            last_activity_unix_secs: None,
            agent: ReplayAgentType::Claude,
            status: ReplayWorkspaceStatus::Main,
            is_main: true,
            is_orphaned: false,
            supported_agent: true,
            pull_requests: Vec::new(),
        }],
        selected_index: 0,
        focus: ReplayFocus::WorkspaceList,
        mode: ReplayMode::List,
        preview_tab: ReplayPreviewTab::Agent,
        viewport_width: 120,
        viewport_height: 40,
        sidebar_width_pct: 33,
        sidebar_hidden: false,
        mouse_capture_enabled: true,
        launch_skip_permissions: false,
    }
}

#[test]
fn sanitize_fixture_name_strips_invalid_characters() {
    assert_eq!(sanitize_fixture_name("My Fixture#1"), "myfixture1");
    assert_eq!(sanitize_fixture_name("replay-flow_a"), "replay-flow_a");
}

#[test]
fn replay_string_result_round_trip() {
    let ok = ReplayStringResult::from_result(&Ok("output".to_string()));
    assert_eq!(ok.to_result(), Ok("output".to_string()));

    let error = ReplayStringResult::from_result(&Err("boom".to_string()));
    assert_eq!(error.to_result(), Err("boom".to_string()));
}

#[test]
fn replay_unit_result_round_trip() {
    let ok = ReplayUnitResult::from_result(&Ok(()));
    assert_eq!(ok.to_result(), Ok(()));

    let error = ReplayUnitResult::from_result(&Err("boom".to_string()));
    assert_eq!(error.to_result(), Err("boom".to_string()));
}

#[test]
fn replay_key_event_round_trip_preserves_code_modifiers_and_kind() {
    let event = KeyEvent::new(KeyCode::Char('x'))
        .with_modifiers(Modifiers::CTRL | Modifiers::ALT)
        .with_kind(KeyEventKind::Repeat);

    let replay = ReplayKeyEvent::from_key_event(&event);
    let round_trip = replay.to_key_event();
    assert_eq!(round_trip, event);
}

#[test]
fn replay_debug_record_replays_minimal_trace() {
    let bootstrap = minimal_bootstrap();
    let mut app = app_from_bootstrap(&bootstrap);
    app.replay_msg_seq_counter = 1;
    let _ = Model::update(&mut app, Msg::Noop);
    let expected_state = ReplayStateSnapshot::from_app(&app);

    let trace_path = unique_temp_path("minimal-trace", "jsonl");
    let bootstrap_json =
        serde_json::to_value(&bootstrap).expect("bootstrap snapshot should encode");
    let msg_json = serde_json::to_value(ReplayMsg::Noop).expect("message should encode");
    let state_json = serde_json::to_value(&expected_state).expect("state snapshot should encode");

    let lines = [
        json!({
            "ts": 1,
            "event": "replay",
            "kind": "bootstrap",
            "data": {
                "schema_version": REPLAY_SCHEMA_VERSION,
                "bootstrap": bootstrap_json,
            }
        })
        .to_string(),
        json!({
            "ts": 2,
            "event": "replay",
            "kind": "msg_received",
            "data": {
                "schema_version": REPLAY_SCHEMA_VERSION,
                "seq": 1,
                "msg": msg_json,
            }
        })
        .to_string(),
        json!({
            "ts": 3,
            "event": "replay",
            "kind": "state_after_update",
            "data": {
                "schema_version": REPLAY_SCHEMA_VERSION,
                "seq": 1,
                "state": state_json,
            }
        })
        .to_string(),
    ];
    fs::write(&trace_path, format!("{}\n", lines.join("\n"))).expect("trace should write");

    let outcome = replay_debug_record(
        &trace_path,
        &ReplayOptions {
            invariant_only: true,
            snapshot_path: None,
        },
    )
    .expect("replay should succeed");
    assert_eq!(outcome.steps_replayed, 1);
    assert_eq!(outcome.states_compared, 1);
    assert_eq!(outcome.frames_compared, 0);

    let _ = fs::remove_file(trace_path);
}

#[test]
fn replay_debug_record_writes_snapshot_when_requested() {
    let bootstrap = minimal_bootstrap();
    let mut app = app_from_bootstrap(&bootstrap);
    app.replay_msg_seq_counter = 1;
    let _ = Model::update(&mut app, Msg::Noop);
    let expected_state = ReplayStateSnapshot::from_app(&app);

    let trace_path = unique_temp_path("snapshot-trace", "jsonl");
    let snapshot_path = unique_temp_path("snapshot-output", "json");
    let bootstrap_json =
        serde_json::to_value(&bootstrap).expect("bootstrap snapshot should encode");
    let msg_json = serde_json::to_value(ReplayMsg::Noop).expect("message should encode");
    let state_json = serde_json::to_value(&expected_state).expect("state snapshot should encode");

    let lines = [
        json!({
            "ts": 1,
            "event": "replay",
            "kind": "bootstrap",
            "data": {
                "schema_version": REPLAY_SCHEMA_VERSION,
                "bootstrap": bootstrap_json,
            }
        })
        .to_string(),
        json!({
            "ts": 2,
            "event": "replay",
            "kind": "msg_received",
            "data": {
                "schema_version": REPLAY_SCHEMA_VERSION,
                "seq": 1,
                "msg": msg_json,
            }
        })
        .to_string(),
        json!({
            "ts": 3,
            "event": "replay",
            "kind": "state_after_update",
            "data": {
                "schema_version": REPLAY_SCHEMA_VERSION,
                "seq": 1,
                "state": state_json,
            }
        })
        .to_string(),
    ];
    fs::write(&trace_path, format!("{}\n", lines.join("\n"))).expect("trace should write");

    let outcome = replay_debug_record(
        &trace_path,
        &ReplayOptions {
            invariant_only: true,
            snapshot_path: Some(snapshot_path.clone()),
        },
    )
    .expect("replay should succeed");
    assert_eq!(outcome.steps_replayed, 1);
    assert!(snapshot_path.exists(), "snapshot output should be written");

    let snapshot = fs::read_to_string(&snapshot_path).expect("snapshot output should be readable");
    assert!(snapshot.contains("\"schema_version\": 1"));
    assert!(snapshot.contains("\"steps\""));

    let _ = fs::remove_file(trace_path);
    let _ = fs::remove_file(snapshot_path);
}

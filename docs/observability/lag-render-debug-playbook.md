# Lag and Render Debug Playbook

Use this when investigating laggy input, render lag, or render artifacts.

## Capture

Run TUI with full forensic outputs:

```bash
grove tui --debug-record
```

Optional explicit paths:

```bash
grove tui \
  --debug-record \
  --event-log .grove/debug-record-manual.jsonl \
  --evidence-log .grove/evidence-manual.jsonl \
  --render-trace .grove/render-trace-manual.jsonl \
  --frame-timing-log .grove/frame-timing-manual.jsonl
```

Then bundle artifacts:

```bash
grove debug bundle
# or
grove debug bundle --out .grove/debug-bundle-case-001
```

## Artifacts

Primary files:
- `.grove/debug-record-*.jsonl` (core app events)
- `.grove/evidence-*.jsonl` (ftui evidence stream)
- `.grove/render-trace-*.jsonl` (render pipeline trace)
- `.grove/frame-timing-*.jsonl` (frame timing sink)
- `.grove/*_payloads/` (payload dumps)

Bundle output includes `manifest.json` with copied file list.

## Correlation Keys

Most TUI events include:
- `run_id`
- `mono_ms`
- `event_seq`
- `msg_seq`
- `poll_generation`
- `frame_seq`

Use these to join input, poll, and frame timelines.

## Input Lag Triage

Start with:
- `input/interactive_key_received`
- `input/interactive_action_selected`
- `input/interactive_forwarded`
- `input/interactive_input_to_preview`
- `input/interactive_forward_failed`
- `input/pipeline_send_fallback`

High signal fields:
- `seq`, `session`
- `command`, `literal_text`, `text`
- `tmux_send_ms`, `input_to_preview_ms`, `tmux_to_preview_ms`
- `queue_depth`, `consumed_input_count`

## Render Lag Triage

Start with:
- `frame/timing`
- `preview_poll/cycle_started`
- `preview_poll/cycle_completed`
- `preview_poll/capture_completed`
- `preview_update/output_changed`

High signal fields:
- `draw_ms`, `view_ms`, `frame_log_ms`
- `capture_ms`, `apply_capture_ms`, `total_ms`
- `raw_len`, `cleaned_len`, `sanitizer_removed_bytes`
- `raw_hash`, `cleaned_hash`

## Render Artifact Triage

Start with:
- `frame/rendered`
- `preview_poll/capture_completed`
- `preview_update/output_changed`
- `selection/*` when artifact appears during selection/copy

High signal fields:
- `frame_lines`, `frame_hash`
- `raw_output`, `cleaned_output`
- `changed_raw`, `changed_cleaned`
- selection line snapshots (`line_raw_preview`, `line_clean_preview`, `line_render_preview`)

## Workspace Status Drift Triage

Start with:
- `workspace_status/transition`
- `workspace_status/capture_completed`
- `workspace_status/capture_failed`
- `workspace_status/capture_dropped_missing_workspace`

High signal fields:
- `workspace_path`, `session`
- `previous_status`, `next_status`
- `previous_orphaned`, `next_orphaned`
- digest and cleaned output fields

## Daemon Transport Triage

Daemon transport emits structured JSON events:
- client-side events are written to the active TUI event log when
  `--event-log` or `--debug-record` is enabled, or to a custom file via
  `GROVE_DAEMON_CLIENT_LOG_PATH=/path/to/daemon-client.jsonl`:
  `daemon_request/client_completed`, `daemon_request/client_failed`
- daemon (`groved`) stderr events:
  - `daemon_request/server_completed`
  - `daemon_request/server_completed_with_write_error`
  - `daemon_request/server_invalid_dropped`
  - `daemon_request/server_connection_handler_failed`

High signal fields:
- `request`, `response`
- `dispatch_ms`, `write_ms`, `total_ms`
- `coalesced_count`, `fire_and_forget`
- `request_preview`, `error`

## Notes

- Logging is intentionally high-volume and payload-rich for forensic sessions.
- For baseline event catalog and required fields, see `docs/observability/event-schema.json`.

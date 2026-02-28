---
name: logging-best-practices
description: >
  Instrumentation best practices for Grove's NDJSON event logging. Use when
  adding/changing log events, event fields, or logging schema in runtime code.
  Not for replay triage, use `replay-debug` for trace analysis and divergence
  debugging.
  Trigger phrases: "how to log", "add logging", "add event",
  "instrumentation", "telemetry", "event schema".
allowed-tools: Read, Grep, Glob
---

# Grove Logging Best Practices

This skill is for instrumentation design.

If the task is reproducing a bug from an existing trace, use
`.agents/skills/replay-debug/SKILL.md` instead.

## Architecture Overview

Grove uses a structured NDJSON event log toggled by `--debug-record`. When
enabled, every frame render, state change, input, tmux command, and polling
event is written to `.grove/debug-record-{app_start_ts}-{pid}.jsonl`. When
disabled, a `NullEventLogger` discards all events at zero cost.

Key files:
- `src/infrastructure/event_log.rs` -- `Event`, `EventLogger`, `FileEventLogger`, `NullEventLogger`
- `src/ui/tui/logging_state.rs` -- state/dialog/tmux/toast event helpers
- `src/ui/tui/logging_input.rs` -- input pipeline logging helpers
- `src/ui/tui/logging_frame.rs` -- per-frame debug record payload
- `src/ui/tui/update.rs` -- update timing + message handling logging
- `src/main.rs` -- `--debug-record` flag parsing, path generation

## Core Principles

### 1. Wide Events Per Boundary (CRITICAL)

Each log event should be a single, context-rich JSON object capturing
everything relevant to that moment. Don't scatter multiple narrow log lines
across a code path. Instead, build up context and emit one event at the
boundary.

**Good**: one `frame/rendered` event with seq, hash, mode, focus, degradation,
dimensions, input queue state, and the full frame buffer.

**Bad**: separate events for "frame started", "frame mode is X", "frame
focus is Y", "frame done".

### 2. Event Identity: event + kind (CRITICAL)

Every event has two classification fields:

- `event`: the category (e.g., `frame`, `tick`, `input`, `tmux_cmd`, `error`)
- `kind`: the specific occurrence (e.g., `rendered`, `scheduled`, `interactive_key_received`)

This pair is the event's identity. Choose names that read naturally as
`event/kind`: `frame/rendered`, `tick/scheduled`, `input/interactive_forwarded`.

Use `LogEvent::new("event", "kind")` to create events. The timestamp is
captured automatically.

### 3. Rich Data Fields (CRITICAL)

Include high-cardinality fields (session names, frame hashes, input sequence
numbers) and high-dimensionality (many fields per event). The debug record
exists for post-hoc diagnosis of issues you haven't anticipated yet.

Add data with `.with_data(key, value)` or `.with_data_fields(vec![...])`:

```rust
self.event_log.log(
    LogEvent::new("preview_poll", "capture_completed")
        .with_data("session", Value::from(session.as_str()))
        .with_data("capture_ms", Value::from(capture_ms))
        .with_data("changed", Value::from(changed))
        .with_data("output_bytes", Value::from(output_bytes))
        .with_data("total_ms", Value::from(total_ms)),
);
```

### 4. Timing at Boundaries (CRITICAL)

Measure and log durations for operations that touch external systems or do
significant work. Use `Instant::now()` before and after, then log the delta
as `_ms` suffixed fields.

Common patterns:
- `capture_ms` / `total_ms` on preview captures
- `tmux_send_ms` on input forwarding
- `duration_ms` on tmux command completion
- `draw_ms` / `view_ms` / `frame_log_ms` on frame rendering
- `update_ms` on message handling
- `input_to_preview_ms` / `tmux_to_preview_ms` for end-to-end input latency

### 5. Zero Cost When Disabled (HIGH)

All logging goes through `self.event_log.log()` which dispatches to either
`FileEventLogger` (writes to disk) or `NullEventLogger` (no-op). This means:

- Don't gate event construction behind `if self.debug_record_enabled` checks.
  The `NullEventLogger` handles this.
- Don't do expensive computation solely for logging. If you need to compute
  something expensive (like hashing frame lines), and it's only useful for
  logging, consider whether it should be part of the normal frame path.
- Frame line extraction in `log_frame_render()` is an exception, it walks the
  terminal buffer and is only called when debug recording is active.

### 6. Sequence Numbers for Correlation (HIGH)

Use monotonic sequence numbers to correlate events across the pipeline.
Input events use `seq` (from `self.input_seq_counter`) to trace a keystroke
from receipt through action selection, tmux forwarding, and preview update.
Frames use `seq` (from `self.frame_render_seq`) for ordering.

When adding a new pipeline, assign a counter and propagate `seq` through
all related events so they can be joined during analysis.

### 7. Consistent Field Names (HIGH)

Reuse existing field names when logging the same concept:

| Concept | Field name | Type |
|---|---|---|
| Tmux session | `session` | string |
| Workspace name | `workspace` | string |
| Input sequence | `seq` | u64 |
| Duration | `*_ms` suffix | u64 |
| Success/failure | `ok` | bool |
| Error message | `error` or `message` | string |
| Queue state | `pending_depth` | usize |
| Queue age | `oldest_pending_age_ms` | u64 |
| Boolean state | `changed`, `repeat`, `cursor_visible` | bool |

## Event Categories

### Frame events (`frame/*`)

Logged per frame render. Two events per frame:

1. `frame/rendered` -- full frame state (only when `--debug-record` is active):
   `seq`, `frame_lines`, `frame_hash`, `width`, `height`, `line_count`,
   `non_empty_line_count`, `degradation`, `mode`, `focus`, `selected_workspace`,
   `interactive_session`, `sidebar_width_pct`, `preview_offset`,
   `preview_auto_scroll`, `output_changing`, `pending_input_depth`,
   `oldest_pending_input_seq`, `oldest_pending_input_age_ms`, `app_start_ts`

2. `frame/timing` -- performance metrics (always logged):
   `draw_ms`, `view_ms`, `frame_log_ms`, `degradation`, `pending_depth`

### Tick events (`tick/*`)

Polling lifecycle: `scheduled`, `retained`, `processed`, `skipped`,
`interactive_debounce_scheduled`. Always include `pending_depth` and
`oldest_pending_age_ms` on scheduling events.

### Input events (`input/*`)

Full input pipeline: `interactive_key_received`, `interactive_action_selected`,
`interactive_forwarded`, `interactive_input_to_preview`,
`interactive_inputs_coalesced`. Always include `seq` and `session`.

### Tmux events (`tmux_cmd/*`)

Command lifecycle: `execute` (with `command` string), `completed` (with
`command`, `duration_ms`, `ok`, and `error` if failed).

### State/mode events

Low-frequency state transitions: `state_change/selection_changed`,
`state_change/focus_changed`, `mode_change/mode_changed`,
`mode_change/interactive_entered`, `mode_change/interactive_exited`.

### Error events (`error/*`)

Failures that need diagnosis: `error/tmux_error`. Include enough context
to reproduce (command, session, error message).

## Adding a New Event

1. Choose `event` category and `kind` name. Check existing events in
   `src/ui/tui/` to avoid category drift.

2. Build the event at the boundary (completion of the operation, not the start):

```rust
self.event_log.log(
    LogEvent::new("my_category", "my_kind")
        .with_data("relevant_field", Value::from(value))
        .with_data("duration_ms", Value::from(elapsed_ms)),
);
```

3. Include timing if the operation is non-trivial. Capture `Instant::now()`
   before the operation and compute the delta after.

4. Include queue/pipeline state if the event is part of a flow
   (`pending_depth`, `seq`).

5. If the new event changes replay interpretation, also update
   `.agents/skills/replay-debug/SKILL.md`.

## Anti-Patterns

1. **Narrow events**: Don't emit "started" / "finished" pairs when a single
   completion event with `duration_ms` suffices. Exception: operations that
   may never complete (like `tmux_cmd/execute` paired with `completed`).

2. **Missing context**: Don't log an error without the operation that caused
   it. Include the command string, session name, and relevant state.

3. **Unstructured strings**: Never `eprintln!()` for diagnostic info. Use
   `self.event_log.log()` with structured data. The only `eprintln!` in the
   codebase is for the debug record path at startup.

4. **Logging in hot loops without gating**: Frame line extraction is expensive.
   If you add similarly expensive logging, guard it behind
   `self.debug_record_start_ts.is_some()` like `log_frame_render()` does.

5. **Inconsistent field names**: Don't use `elapsed` when the convention is
   `duration_ms`. Don't use `target_session` when the convention is `session`.

6. **Missing sequence numbers**: If the event is part of an input or frame
   pipeline, include `seq` so events can be correlated during analysis.

## Frame Data: What and Why

The `frame/rendered` event captures the full terminal buffer because:

- Visual bugs (flicker, blank frames, layout corruption) can only be diagnosed
  by seeing exactly what was rendered
- Frame hash enables change detection without comparing full content
- `non_empty_line_count` catches blank frame regressions
- `degradation` tracks frame budget health
- `pending_input_depth` + `oldest_pending_input_age_ms` reveal input queue
  pressure at render time

This is intentionally heavy data. It's gated behind `--debug-record` and
only used during active debugging sessions.

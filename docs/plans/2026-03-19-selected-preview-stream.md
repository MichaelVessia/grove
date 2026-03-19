# Selected Preview Stream Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace timer-driven selected-preview capture with a push-driven tmux-backed stream for the foreground session.

**Architecture:** Introduce a dedicated selected-preview stream transport, likely tmux control mode, for one foreground session at a time. Feed incremental output into Grove's existing preview apply path, retain the current polling path as fallback during rollout, and keep background workspace status polling on the existing model until the stream path is proven.

**Tech Stack:** Rust, Grove TUI runtime, tmux control mode or equivalent streaming interface, cargo test, make precommit

---

## Chunk 1: Streaming boundary and lifecycle

### Task 1: Add a preview-stream abstraction with no behavior change

**Files:**
- Create: `src/ui/tui/terminal/preview_stream.rs`
- Modify: `src/ui/tui/terminal.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/bootstrap/bootstrap_app.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing test**

Add a model/bootstrap test proving the app can hold preview-stream state without changing existing polling behavior yet.

Cover:

- stream state starts disconnected
- no selected session means no stream target
- existing preview polling still works when no stream is active

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test preview_stream_state_defaults_to_disconnected -- --nocapture
```

Expected: FAIL because no preview-stream abstraction exists yet.

- [ ] **Step 3: Write minimal implementation**

Add a narrow abstraction:

- preview stream target/session identity
- connection state
- last delivered chunk metadata
- fallback-needed flag

Keep it app-local and avoid leaking transport details across unrelated modules.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test preview_stream_state_defaults_to_disconnected -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tui/terminal/preview_stream.rs src/ui/tui/terminal.rs src/ui/tui/model.rs src/ui/tui/bootstrap/bootstrap_app.rs src/ui/tui/mod.rs
git commit -m "feat: add selected preview stream state"
```

### Task 2: Define stream lifecycle events around selection and focus

**Files:**
- Modify: `src/ui/tui/update/update_navigation_commands.rs`
- Modify: `src/ui/tui/update/update_polling_capture_dispatch.rs`
- Modify: `src/ui/tui/update/update_polling_state.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add tests covering lifecycle transitions:

- selecting a different workspace retargets the stream
- leaving the agent preview tab disconnects the stream
- returning to the selected agent preview reconnects it
- stale stream data for an old session is dropped

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test preview_stream_retargets_on_workspace_change -- --nocapture
cargo test preview_stream_disconnects_when_agent_preview_is_not_visible -- --nocapture
```

Expected: FAIL because no stream lifecycle exists yet.

- [ ] **Step 3: Write minimal implementation**

Add explicit lifecycle helpers that:

- compute whether a selected live preview stream should exist
- reconnect only when the target session changes
- clear stream state when the preview surface no longer qualifies
- reject stale completions using the selected-session identity

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test preview_stream_retargets_on_workspace_change -- --nocapture
cargo test preview_stream_disconnects_when_agent_preview_is_not_visible -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tui/update/update_navigation_commands.rs src/ui/tui/update/update_polling_capture_dispatch.rs src/ui/tui/update/update_polling_state.rs src/ui/tui/mod.rs
git commit -m "feat: add selected preview stream lifecycle"
```

## Chunk 2: tmux transport and preview application

### Task 3: Add a tmux-backed streaming transport behind the abstraction

**Files:**
- Modify: `src/ui/tui/terminal/tmux.rs`
- Modify: `src/ui/tui/terminal/preview_stream.rs`
- Possibly create: `src/ui/tui/terminal/tmux_preview_stream.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add transport-facing tests using a fake stream source that prove:

- a connected stream can deliver incremental output frames
- disconnect errors mark the stream for fallback
- reconnect resets generation/session identity

If tmux process spawning is hard to unit test directly, keep the transport boundary trait-based and test the fake implementation.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test preview_stream_marks_fallback_after_disconnect -- --nocapture
```

Expected: FAIL because no streaming transport exists.

- [ ] **Step 3: Write minimal implementation**

Implement a selected-preview transport using tmux control mode or the simplest equivalent that can stream pane output for one session.

Requirements:

- one foreground stream at a time
- explicit connect/disconnect
- bounded buffering
- incremental event delivery back into the app loop
- no changes to background status polling

Do not remove the polling path yet.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test preview_stream_marks_fallback_after_disconnect -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tui/terminal/tmux.rs src/ui/tui/terminal/preview_stream.rs src/ui/tui/mod.rs
git commit -m "feat: add tmux-backed selected preview stream"
```

### Task 4: Route streamed output through the existing preview apply path

**Files:**
- Modify: `src/ui/tui/update/update_polling_capture_live.rs`
- Modify: `src/ui/tui/update/update_polling_capture_dispatch.rs`
- Modify: `src/application/preview.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add behavior tests proving:

- streamed foreground output updates preview content without waiting for the adaptive poll timer
- session mismatch still drops stale data
- manual preview scrolling remains stable while streamed content arrives

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test selected_preview_stream_updates_without_poll_delay -- --nocapture
```

Expected: FAIL because preview updates still depend on polling completions.

- [ ] **Step 3: Write minimal implementation**

Refactor the existing live-preview application path so both:

- polled full captures
- streamed incremental updates

share one preview-application boundary where possible.

Keep status tracking, output-changing detection, and stale-session handling consistent across both paths.

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test selected_preview_stream_updates_without_poll_delay -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tui/update/update_polling_capture_live.rs src/ui/tui/update/update_polling_capture_dispatch.rs src/application/preview.rs src/ui/tui/mod.rs
git commit -m "feat: apply selected preview stream updates"
```

## Chunk 3: Fallback, observability, and rollout

### Task 5: Keep polling as explicit fallback during rollout

**Files:**
- Modify: `src/ui/tui/update/update_polling_capture_dispatch.rs`
- Modify: `src/ui/tui/update/update_polling_state.rs`
- Modify: `src/ui/tui/performance.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add tests proving:

- selected preview falls back to polling after a stream failure
- background workspaces keep current polling behavior
- performance dialog reports whether the selected preview source is `stream` or `poll`

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test selected_preview_stream_falls_back_to_polling_after_failure -- --nocapture
```

Expected: FAIL because fallback state and reporting do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add:

- explicit fallback state
- scheduler logic that skips foreground polling while a healthy stream is attached
- performance-dialog reporting for foreground source
- event-log entries for connect, disconnect, fallback, and reconnect

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cargo test selected_preview_stream_falls_back_to_polling_after_failure -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui/tui/update/update_polling_capture_dispatch.rs src/ui/tui/update/update_polling_state.rs src/ui/tui/performance.rs src/ui/tui/mod.rs
git commit -m "feat: add preview stream fallback and reporting"
```

### Task 6: Validate and document the transport change

**Files:**
- Modify: `docs/adr/003-interactive-polling-strategy.md`
- Modify: `docs/plans/2026-03-19-selected-preview-stream.md`

- [ ] **Step 1: Document the new foreground transport**

Update the ADR to describe:

- selected preview uses a stream-first transport
- background status polling still uses the current model
- fallback behavior when streaming is unavailable

- [ ] **Step 2: Run targeted tests**

Run:

```bash
cargo test preview_stream_ -- --nocapture
cargo test selected_preview_stream_ -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run required local validation**

Run:

```bash
make precommit
```

Expected: PASS.

- [ ] **Step 4: Capture runtime verification**

Manually verify:

- active selected preview updates immediately under output
- switching selected workspaces reconnects cleanly
- killing the tmux session falls back without breaking the preview pane
- performance dialog reflects the stream source

- [ ] **Step 5: Commit**

```bash
git add docs/adr/003-interactive-polling-strategy.md docs/plans/2026-03-19-selected-preview-stream.md
git commit -m "docs: record selected preview stream plan"
```

## Success Criteria

- Selected preview no longer depends on a fixed adaptive poll interval while the stream is healthy.
- Session switches and stream disconnects are handled without stale preview corruption.
- Polling fallback preserves functionality if streaming fails.
- Background polling cost stays bounded.
- Targeted tests and `make precommit` pass.

## Non-Goals

- No replacement of background workspace status polling in this phase.
- No attempt to stream every workspace at once.
- No removal of the polling path until the stream path is proven stable.

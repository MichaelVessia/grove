# Grove Implementation Plan, Sidecar Architecture + FrankenTUI Practice

Status: Draft  
Date: 2026-02-14  
Audience: implementation agents working in this repo

## Purpose

Align Grove with Sidecar architecture where it improves correctness and UX, while
staying inside FrankenTUI idioms and Grove PRD decisions.

## Locked Decisions

1. Screen mode stays `AltScreen` only, no optional inline mode.
2. Output trust behavior stays current unless a concrete Sidecar mismatch appears.
3. Adopt ftui key handling policy via `ActionMapper` as the app-level key resolver.
4. PRD remains source of truth when Sidecar and Grove differ.

## Source Priority

1. `docs/PRD.md`
2. This plan
3. Sidecar reference (`.reference/sidecar/`)
4. FrankenTUI harness/reference docs (`.reference/frankentui/docs/`)

## Current Gaps To Close

1. Blocking subprocess work still runs on update path:
`src/tui.rs:1545`, `src/tui.rs:2001`, `src/tui.rs:2083`, `src/tui.rs:2232`, `src/tui.rs:2342`.
2. Tmux command path still uses `.status()` in active runtime code:
`src/tui.rs:283`, `src/tui.rs:362`.
3. Interactive poll generation is incremented but never enforced:
`src/tui.rs:764`, `src/tui.rs:1180`.
4. Interactive enter/resize does not force Sidecar-style immediate capture after resize:
`src/tui.rs:1832`, `src/tui.rs:3738`.
5. Key handling is custom routing, not `ActionMapper` policy-driven:
`src/tui.rs:3108`, `src/tui.rs:2652`, `src/interactive.rs:82`.

## Architecture Targets

1. No blocking IO in key/tick/update decision path.
2. Single writer discipline for terminal output during active TUI.
3. Polling chain must be generation-safe and non-duplicating.
4. Interactive entry and resize must produce immediate fresh preview/cursor state.
5. App-level key semantics must be explicit and policy-tested via `ActionMapper`.

## Workstreams

## Workstream 1, Async Poll Pipeline

Scope:
1. Convert preview capture and cursor capture from sync calls in `poll_preview` to
message-driven async tasks.
2. Add result messages for success/failure and apply state mutation only in result handlers.

Files:
1. `src/tui.rs`
2. `src/preview.rs` (if result-apply helpers are extracted)

Acceptance:
1. `Msg::Tick` does not directly call tmux subprocess functions.
2. Poll failures surface as flash/log events, no UI stall.
3. Existing polling behavior and debounced scheduling semantics remain intact.

## Workstream 2, Async Lifecycle Commands

Scope:
1. Move `refresh_workspaces`, workspace creation confirm flow, start/stop agent tmux command loops
to async commands + completion messages.
2. Keep UI transitions deterministic while commands are in flight.

Files:
1. `src/tui.rs`
2. `src/adapters.rs`
3. `src/workspace_lifecycle.rs`

Acceptance:
1. No synchronous git/tmux command loops from direct key handlers.
2. Success/error outcomes are handled through explicit completion messages.
3. Existing flash/status behavior preserved.

## Workstream 3, One-Writer Hardening

Scope:
1. Replace runtime `.status()` paths with captured output paths (`.output()`).
2. Ensure errors are consumed into app logging/flash, never inherited terminal writes.

Files:
1. `src/tui.rs`

Acceptance:
1. No active-runtime `.status()` in tmux command path.
2. Grep check passes for runtime writer violations in `src/`:
no raw `println!`/`eprintln!` during active TUI flow.

## Workstream 4, Poll Generation Correctness

Scope:
1. Thread generation ids through scheduled poll work and completion handling.
2. Drop stale poll results.
3. Remove dead generation fields if unused after refactor.

Files:
1. `src/tui.rs`

Acceptance:
1. Stale results are ignored deterministically.
2. Unit tests prove newer generation supersedes older result.

## Workstream 5, Interactive Resize and Immediate Poll Parity

Scope:
1. On interactive enter, resize target then trigger immediate preview+cursor capture.
2. On `Resize`, if interactive active, resize target and immediately poll.
3. Add resize verify-and-retry once if pane size mismatch persists.

Files:
1. `src/tui.rs`

Acceptance:
1. Interactive entry visibly refreshes within one update cycle.
2. Resize reflects new wrapping/cursor promptly.
3. Verify-retry path tested.

## Workstream 6, ActionMapper Adoption

Scope:
1. Introduce app-level action mapping via ftui `ActionMapper` for non-forwarded keys.
2. Preserve PRD behavior:
modal guards, interactive exit/copy/paste behavior, no attach mode.
3. Keep tmux-forwarding map for interactive send-keys, but gate control flow through policy actions.

Files:
1. `src/tui.rs`
2. `src/interactive.rs` (only where control/action split is needed)

Acceptance:
1. Key priority behavior is table-driven, not implicit branch ordering.
2. Esc/Ctrl+C/Ctrl+D/Ctrl+Q semantics tested with modal/input/task state.
3. Interactive forwarding still preserves existing tmux key translation behavior.

## Workstream 7, Output Trust Model, Keep Current

Scope:
1. Keep current dual-lane behavior:
render lane preserves SGR-safe output for preview rendering,
logic/status lane uses cleaned output for status/change detection.
2. Do not switch to strict sanitize-all unless a concrete bug appears.

Files:
1. `src/agent_runtime.rs`
2. `src/preview.rs`
3. `src/tui.rs` (if wiring changes)

Acceptance:
1. Existing ANSI rendering features remain.
2. Control-sequence injection does not alter app control flow.
3. Tests cover raw-changed vs cleaned-changed behavior.

## Execution Order

1. Workstream 1
2. Workstream 3
3. Workstream 4
4. Workstream 5
5. Workstream 2
6. Workstream 6
7. Workstream 7

Rationale:
1. Poll and writer safety first.
2. Then lifecycle async cleanup.
3. Then key policy migration.

## Test Plan

Per workstream, add/adjust targeted tests only.

Required test categories:
1. Poll scheduling, retention, stale result dropping.
2. Interactive enter/exit and resize immediate refresh.
3. Async start/stop/create/refresh message handling.
4. Key policy precedence (modal vs input vs task vs quit).
5. ANSI preview integrity and cleaned-output status detection.

Likely files:
1. `src/tui.rs` tests module
2. `src/interactive.rs` tests
3. `src/preview.rs` tests
4. `src/agent_runtime.rs` tests

## Definition Of Done

1. All workstreams complete with acceptance criteria met.
2. No blocking subprocess IO in direct key/tick/update decision path.
3. App-level key semantics documented and test-enforced via ActionMapper.
4. Poll chain generation-safe and non-duplicating.
5. Existing PRD behavior still true:
`AltScreen`, no attach mode, modal guards, interactive copy/paste/exit patterns.

## Non-Goals

1. Introducing inline mode.
2. Adding tmux attach feature.
3. Plugin-architecture expansion.

## Implementation Decision Log

### Workstream 1, Async Poll Pipeline (Completed)

1. Added async preview polling path for runtime: `poll_preview` now schedules
background capture tasks and applies results in `Msg::PreviewPollCompleted`.
2. Kept synchronous fallback path for non-background tmux adapters, this keeps
deterministic unit tests and non-threaded adapters working while production uses
the async path (`CommandTmuxInput` reports background support).
3. Added deferred command queue (`deferred_cmds`) so existing internal call
sites can trigger async poll work without broad signature churn.
4. Poll capture failures now emit logs and a flash message (`preview capture failed`)
instead of only logging.
5. Cursor capture is also result-driven in async mode, applied via
`apply_cursor_capture_result`.

### Workstream 3, One-Writer Hardening (Completed)

1. Replaced runtime tmux execution `.status()` path with `.output()` in
`CommandTmuxInput::execute_command`, stderr/status are now captured into errors.
2. Replaced resize preflight `set-option` `.status()` call with `.output()`,
capturing failure details and threading them into the final resize error.
3. Added a source guard test to assert `src/tui.rs` contains no status-call
runtime paths (`tmux_runtime_paths_avoid_status_calls_in_tui_module`).

### Workstream 4, Poll Generation Correctness (Completed)

1. Removed split generation counters and unified on `poll_generation`.
2. Every async poll request now carries a generation id from `poll_generation`.
3. `handle_preview_poll_completed` now drops stale generations and logs
`preview_poll:stale_result_dropped`.
4. Added regression test:
`stale_preview_poll_result_is_dropped_by_generation`.

### Workstream 5, Interactive Resize and Immediate Poll Parity (Completed)

1. Interactive enter now always triggers an immediate poll after resize sync.
2. `Msg::Resize` now triggers immediate poll when interactive mode is active.
3. Added resize verification state (`PendingResizeVerification`) and cursor-based
verify-and-retry-once flow:
   `resize_verify_retry` then `resize_verify_failed` if mismatch persists after retry.
4. Added tests:
`enter_interactive_immediately_polls_preview_and_cursor`,
`resize_in_interactive_mode_immediately_resizes_and_polls`,
`resize_verify_retries_once_then_stops`.

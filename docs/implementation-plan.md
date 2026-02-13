# Grove Implementation Plan

This document turns the PRD into a phased execution plan with explicit
quality gates.

## Open Items Checklist (2026-02-13)

- [x] Frame-registered hit IDs in `view()` and update-path mouse routing from hit grid
- [x] New-workspace modal: existing-branch attach mode + base-branch editing
- [ ] Manual interaction pass for `n` create flow + divider drag across terminal emulators
- [x] Event-log follow-up: expanded event coverage + binary smoke test for `--event-log`
- [ ] Phase 8 closure items: performance/long-session/restart-reattach validation

## Update 2026-02-13, Open-Item Sweep (Hit Grid, Create Dialog, Event Log, Reattach Smoke)

What changed:
- Implemented hit-grid-first mouse routing in `src/tui.rs`:
  - registered pane hit IDs in `view()` renderers (header/sidebar/divider/preview/status)
  - registered per-workspace-row hit data for click selection
  - cached last frame hit grid and routed mouse updates through hit-test IDs (geometry kept as fallback)
  - added render/mouse tests:
    - `tui::tests::view_registers_hit_regions_for_panes_and_workspace_rows`
    - `tui::tests::mouse_workspace_selection_uses_row_hit_data_after_render`
- Implemented full create-dialog branch controls in `src/tui.rs`:
  - new vs existing branch mode toggle
  - editable active branch field (base branch for new mode, existing branch for attach mode)
  - tab/shift-tab field focus cycle with focused-field status hints
  - confirmation now maps to `BranchMode::NewBranch` or `BranchMode::ExistingBranch`
  - added targeted tests for focus cycle, mode toggle, branch editing, and validation paths.
- Expanded event-log coverage in `src/tui.rs`:
  - emits `preview_update/scrolled`
  - emits `preview_update/autoscroll_toggled`
  - enriches dialog open/confirm events with payload fields (workspace, mode, branch value, prompt length, skip-permissions).
- Added binary smoke coverage for event-log path:
  - `tests/event_log_cli_smoke.rs` validates `--event-log` file creation on CLI run (`--print-hello` path).
  - `src/main.rs` now creates event-log file in `--print-hello` mode when `--event-log` is supplied.
- Added restart/reattach validation harness:
  - `scripts/check-restart-reattach.sh`
  - automated local smoke verifies agent session survives Grove restart and interactive reattach works.
- Fixed preview offset invariant at source in `src/preview.rs`:
  - clamps scroll offset to available lines on scroll and capture updates
  - added regression tests:
    - `preview::tests::scroll_up_clamps_offset_to_available_lines`
    - `preview::tests::apply_capture_clamps_existing_offset_when_output_shrinks`

Current status:
- Completed checklist items:
  - hit-grid routing
  - create-dialog branch/base controls
  - event-log expansion + binary smoke
- Phase 8 progress:
  - performance/flicker harness remains green (`scripts/check-codex-flicker.sh`)
  - restart/reattach smoke now automated and green (`scripts/check-restart-reattach.sh`)
- Remaining unchecked items are manual-only validation passes.

Next steps:
- Run manual terminal-emulator pass for create-dialog flow + divider drag in your normal environments.
- Run/record a long-session manual validation (>=30m) to close remaining Phase 8 manual criteria.

## Update 2026-02-13, Interactive Idle Poll Backoff for Flicker

What changed:
- Updated interactive polling logic in `src/agent_runtime.rs`:
  - when in interactive mode, selected workspace, inactivity is over 10s, and
    captured output is unchanged, next poll now backs off to `2s` (was `500ms`).
  - active interactive paths remain unchanged (`50ms` under 2s inactivity,
    `200ms` under 10s inactivity, `500ms` while output is changing).
- Extended `poll_intervals_follow_preview_and_interactive_rules` test to cover
  the new idle-unchanged backoff case.

Current status:
- Targeted runtime test passes locally.
- Codex flicker harness (`scripts/check-codex-flicker.sh`) remains green after
  change.
- Snapshot analysis still indicates unchanged repeated captures in the reported
  repro, now polled less aggressively when idle.

Next steps:
- Re-test the same real interactive repro and capture a new
  `.grove-debug-snapshot.json` to confirm cadence and perceived flicker drop.
- If flicker remains, next step is adding per-tick render timing and cursor
  metadata deltas to snapshot/event log for tighter attribution.

## Update 2026-02-13, Debug Snapshot Hotkey (Ctrl+D)

What changed:
- Added `CaptureRecord` struct and ring buffer (`VecDeque`, capacity 10) to
  `PreviewState` in `src/preview.rs`. Each `apply_capture()` call pushes a
  record with timestamp, raw/cleaned/render output, digest, and change flags.
- Added `write_debug_snapshot()` method to `GroveApp` in `src/tui.rs` that
  serializes all internal state (workspace, mode, focus, viewport, sidebar
  ratio, interactive cursor, preview state, recent captures, current lines,
  last tmux error) to `.grove-debug-snapshot.json` as pretty-printed JSON.
- Wired `Ctrl+D` hotkey in `handle_key()` before dialog/interactive/quit
  routing so it fires in all modes (normal, interactive, dialog).
- Flash message confirms snapshot save (or reports error).
- Added `.grove-debug-snapshot.json` to `.gitignore`.
- Added tests:
  - `preview::tests::capture_record_ring_buffer_caps_at_10`
  - `preview::tests::capture_record_contains_expected_fields`
  - `tui::tests::ctrl_d_triggers_debug_snapshot_in_normal_mode`
  - `tui::tests::ctrl_d_triggers_debug_snapshot_in_interactive_mode`
  - `tui::tests::ctrl_d_triggers_debug_snapshot_during_dialog`
  - `tui::tests::debug_snapshot_file_is_valid_json`
  - `tui::tests::debug_snapshot_includes_recent_captures`

Current status:
- All 137 lib tests pass. Formatting clean. Clippy clean (pre-existing
  warnings only, none in new code).
- Debug snapshots contain consecutive capture history so an agent can identify
  oscillation, ANSI leakage, or cursor corruption from structured data.

Next steps:
- Manual test: run Grove, press `Ctrl+D`, inspect
  `.grove-debug-snapshot.json` for readable capture history.
- Use snapshots to write targeted failing tests when visual bugs are reported.

## Update 2026-02-13, E2E Plan Phase C Structured Event Log

What changed:
- Added `src/event_log.rs`:
  - `Event` NDJSON payload model (`ts`, `event`, `kind`, `data`)
  - `EventLogger` trait
  - `FileEventLogger` append-only file writer
  - `NullEventLogger` no-op implementation
- Added CLI support in `src/main.rs` for `--event-log <path>` with parse tests.
- Wired runtime event logger through `src/lib.rs` and `src/tui.rs`:
  - `run_with_event_log(...)` chooses file/no-op logger
  - `GroveApp` now carries a logger and emits events.
- Added initial event emit points in `src/tui.rs`:
  - state/mode transitions: `selection_changed`, `focus_changed`,
    `mode_changed`, `interactive_entered`, `interactive_exited`
  - dialogs: `dialog_opened`, `dialog_confirmed`, `dialog_cancelled`
  - lifecycle: `agent_started`, `agent_stopped`
  - preview: `output_changed`
  - tmux command + error reporting: `tmux_cmd`, `tmux_error`
  - flash: `flash_shown`
- Added `EventLogReader` utility to `tests/support/mod.rs` with:
  - `read_events`
  - `wait_for(kind, timeout)`
  - `assert_sequence(...)`
- Added focused tests:
  - `src/event_log.rs` logger behavior tests
  - `src/main.rs` CLI parse tests
  - `src/tui.rs` event emission flow tests
  - `tests/event_log_reader.rs` reader utility test

Current status:
- Phase C from `docs/e2e-testing-plan.md` is implemented and green in focused
  local tests.
- Phase A, B, and C are now all in place.

Next steps:
- Expand event coverage incrementally (agent status transitions, dialog payload
  detail, preview scroll/autoscroll toggles).
- Add one binary-level smoke test that launches with `--event-log` and
  validates file creation in a controlled environment.

## Update 2026-02-13, E2E Plan Phase B Buffer Render Assertions

What changed:
- Added buffer-render test helpers in `tests/support/render.rs` and exported the
  module in `tests/support/mod.rs`.
- Added phase-B view assertions in `src/tui.rs` tests:
  - `sidebar_shows_workspace_names`
  - `selected_workspace_row_has_selection_marker`
  - `modal_dialog_renders_over_sidebar`
  - `status_bar_shows_flash_message`
  - `preview_pane_renders_ansi_colors`
- Added render-focused property tests in `src/tui.rs`:
  - `view_never_panics`
  - `view_fills_status_bar_row`

Current status:
- Phase B from `docs/e2e-testing-plan.md` is implemented and green locally.
- Phase A and Phase B are both in place.

Next steps:
- Phase C is complete, move to expanding event coverage and smoke tests.

## Update 2026-02-13, E2E Plan Phase A Property Tests

What changed:
- Implemented phase 1 (Phase A) from `docs/e2e-testing-plan.md` by adding
  `proptest` as a dev dependency (`Cargo.toml`).
- Added property-test generators in `src/tui.rs` test module:
  - `arb_key_event()`
  - `arb_msg()`
- Added four phase-A property tests in `src/tui.rs`:
  - `no_panic_on_random_messages`
  - `selection_always_in_bounds`
  - `modal_exclusivity`
  - `scroll_offset_in_bounds`

Current status:
- Phase-A property testing is in place and green locally.
- Cargo lockfile now includes `proptest` and transitive dev dependencies.

Next steps:
- Phase 2 is complete, move to phase C (structured event log).
- After phase C lands, expand event coverage and decide whether message
  strategies need mouse/paste generation.

## Update 2026-02-13, Codex Interactive Plain Capture + Overlay Guard

What changed:
- For Codex in interactive mode, live preview capture now disables tmux escape
  capture (`include_escape_sequences = false`) so Grove parses plain lines
  instead of Codex’s full ANSI stream (`src/tui.rs`).
- Preview rendering now forces plain-path output for Codex interactive sessions
  and skips Grove’s synthetic cursor overlay for Codex to avoid duplicate
  cursor drawing (`src/tui.rs`).
- Updated TUI tests:
  - interactive flow now expects Codex interactive live capture without
    escapes
  - cursor-overlay assertion is pinned to Claude so overlay behavior remains
    covered (`src/tui.rs` tests).

Current status:
- Reduces Codex-specific double-cursor and multiline overwrite artifacts caused
  by combining ANSI-screen output with Grove cursor injection in preview mode.
- Focused TUI tests are green after the change.

Next steps:
- Manual retest in your normal setup with long multiline prompts in Codex
  interactive mode, confirm cursor stays single and lines no longer overwrite.
- If lag still reproduces, next step is adding a small sampling log for
  per-tick render cost and tmux capture duration to isolate I/O vs render cost.

## Update 2026-02-13, Remove Codex `--no-alt-screen` Default

What changed:
- Removed Codex default `--no-alt-screen` from launch command construction.
  Defaults now match PRD command mapping:
  - normal: `codex`
  - unsafe: `codex --dangerously-bypass-approvals-and-sandbox`
  (`src/agent_runtime.rs`).
- Updated runtime and TUI launch-flow tests to assert new Codex command values
  (`src/agent_runtime.rs`, `src/tui.rs`).
- Updated README harness example to run with `codex` command override
  (`README.md`).

Current status:
- A/B harness runs against real Codex (`codex` vs `codex --no-alt-screen`)
  showed equivalent stability in this environment (no style/plain oscillation
  detected in either mode).
- We no longer carry Codex-specific launch flags that appear redundant given the
  current rendering/input fixes.

Next steps:
- Keep watching for regressions in real usage.
- If edge terminals show flicker with default `codex`, use
  `GROVE_CODEX_CMD='codex --no-alt-screen'` as immediate runtime override while
  diagnosing.

## Update 2026-02-13, Interactive tmux Pane Geometry Sync

What changed:
- Added tmux pane-resize support to the TUI tmux adapter path
  (`src/tui.rs`):
  - new `TmuxInput::resize_session(...)`
  - command implementation sets `window-size` to `manual`, then uses
    `resize-window` with `resize-pane` fallback.
- Added preview output geometry calculation and interactive sync wiring so the
  target tmux session is resized to the preview pane dimensions when entering
  interactive mode, on terminal resize, and while dragging the sidebar divider
  (`src/tui.rs`).
- Added regression coverage asserting interactive entry triggers a resize call
  with expected preview dimensions (`src/tui.rs` tests).

Current status:
- Codex interactive sessions now run at the same geometry Grove renders in the
  preview pane, preventing Codex’s screen-oriented UI from being shown as a
  mismatched bottom slice that looks like broken scrollback.
- Verified by runtime probe: `grove-ws-test-codex` pane changed from default
  geometry to preview-matched geometry after entering interactive mode.
- Focused TUI tests are green.

Next steps:
- Manual retest in your normal terminal workflow: start Codex in Grove, enter
  interactive, and confirm preview content now looks stable without Codex-only
  scrollback artifacts.
- If any residual artifact remains, next likely step is tracking explicit
  preview scroll offsets against pane geometry changes during rapid resize/drag.

## Update 2026-02-13, Restore Codex ANSI Styling In Preview

What changed:
- Removed Codex-only plain-render fallback in preview policy. ANSI rendering is
  now enabled for both Claude and Codex (`src/tui.rs`).
- Live preview capture for Codex now keeps tmux escape capture enabled (`-e`),
  matching Claude capture path (`src/tui.rs` tests).
- Updated TUI regression tests to assert Codex ANSI rendering/capture behavior
  and updated interactive-flow call expectations (`src/tui.rs`).

Current status:
- Codex preview styling now matches styled rendering behavior expected from
  Codex output instead of forced plain text.
- Targeted TUI tests pass, and the local flicker harness still reports no
  style/plain oscillation in this environment.

Next steps:
- Validate visually in your setup with long Codex sessions and active mouse
  interaction to confirm style fidelity and no flicker regression.
- If any flicker returns, add a runtime toggle to switch Codex between ANSI and
  plain capture without code changes.

## Update 2026-02-13, Local Autonomous Codex Flicker Feedback Loop

What changed:
- Added launch-command env overrides in runtime command construction:
  `GROVE_CODEX_CMD` and `GROVE_CLAUDE_CMD` now replace default agent launch
  commands when set (`src/agent_runtime.rs`).
- Added normalization coverage for override values (trim + empty rejection)
  (`src/agent_runtime.rs` tests).
- Added deterministic fake Codex stream emitter script that produces
  ANSI + mouse-fragment heavy output (`scripts/fake-codex-flicker-emitter.sh`).
- Added local harness script that:
  - creates a temporary Codex-marked git worktree
  - launches Grove in a tmux session with `GROVE_CODEX_CMD` pointed at the fake
    emitter
  - automatically selects that workspace, starts the agent, enters interactive
    mode, and sends input
  - samples frame styling via `tmux capture-pane -e` and fails on sustained
    styled/plain mode oscillation (`scripts/check-codex-flicker.sh`).
- Added optional harness override `GROVE_FLICKER_CODEX_CMD` so the same loop can
  run against real Codex (or any command) without changing the script.

Current status:
- We now have a one-command local repro/verification loop with no manual
  interaction required.
- Latest local run completed with no style-oscillation flicker detected by the
  harness heuristic.

Next steps:
- Keep iterating Codex rendering fixes using the harness as the first local
  gate (`scripts/check-codex-flicker.sh`).
- If needed, tune oscillation thresholds by changing
  `GROVE_FLICKER_*` environment variables while collecting real failures.

## Update 2026-02-13, Interactive Split-Mouse Fragment Input Filter

What changed:
- Added interactive-state tracking for recent mouse events and in-progress
  split mouse-fragment detection (`src/interactive.rs`).
- Implemented Sidecar-style fragment suppression for leaked split CSI mouse
  input in interactive mode:
  - starts filtering when a bare `[` arrives immediately after mouse activity
  - drops subsequent fragment chars (`<`, digits, `;`, `M/m`) in a short window
  (`src/interactive.rs`, `src/tui.rs`).
- Wired mouse-event timestamping into the TUI mouse handler so key filtering
  has the needed timing signal (`src/tui.rs`).
- Added focused regression tests for the filter and non-filter paths in both
  interactive state and app-level TUI flow (`src/interactive.rs`, `src/tui.rs`).

Current status:
- Interactive key forwarding now suppresses known split mouse CSI leak patterns
  that can destabilize Codex redraw behavior during mouse/scroll interaction.
- Focused test suites for touched modules are green.

Next steps:
- Manual retest in your environment: keep Codex open, scroll/click/type in
  Grove interactive mode, confirm no styled/plain flicker.
- If flicker persists, next step is instrumenting per-frame render mode + raw
  captured bytes from the running Grove session to isolate non-input causes.

## Update 2026-02-13, Codex Capture Escape Filtering At Source

What changed:
- Added an ANSI-escape capture toggle to tmux preview capture plumbing in
  `src/tui.rs` (`TmuxInput::capture_output` now accepts
  `include_escape_sequences`).
- Switched live preview polling to choose capture mode by selected agent:
  Codex capture now runs without `tmux capture-pane -e`, Claude keeps `-e`.
- Kept interactive copy capture on `Alt+C` using ANSI capture (`-e`) so
  existing copy/paste behavior stays unchanged.
- Added regression tests proving capture mode selection:
  Codex preview path uses plain capture, Claude preview path keeps ANSI capture
  (`src/tui.rs` tests).

Current status:
- Codex preview no longer ingests tmux ANSI escape streams during normal live
  polling, reducing risk of control-sequence-induced render instability and
  flicker.
- Focused TUI test suite is green.

Next steps:
- Manual validation in your terminal: run Codex in Grove interactive mode and
  confirm flicker no longer reproduces during active typing/navigation.
- If any residual flicker remains, next likely step is adding Sidecar-style
  split-mouse-fragment key filtering in interactive key forwarding.

## Update 2026-02-13, Restore Ctrl+Backslash Interactive Exit

What changed:
- Reintroduced `Ctrl+\` as an immediate interactive-exit key path in the
  interactive state machine (`src/interactive.rs`).
- Updated interactive key mapping to recognize both common terminal encodings
  for `Ctrl+\`: modified `\\` and raw control character `\u{1c}`
  (`src/tui.rs`).
- Added fallback recognition for `Ctrl+4`/`Ctrl+|` variants and accepted
  `Repeat` key events so `Ctrl+\`-intent exit works across more terminal
  key-event implementations (`src/tui.rs`).
- Updated interactive status hint text to advertise both exit paths:
  `Esc Esc` and `Ctrl+\` (`src/tui.rs`).
- Added regression tests for direct and control-character `Ctrl+\` exit
  behavior (`src/interactive.rs`, `src/tui.rs`).

Current status:
- Interactive exit no longer depends only on the 150ms double-escape timing
  window.
- Existing `Esc Esc` behavior remains available.

Next steps:
- Manual check in your terminal: verify `Ctrl+\` exits interactive mode on the
  first keypress in Codex and Claude workspaces.

## Update 2026-02-13, Codex Plain Preview Fallback

What changed:
- Added agent-specific preview rendering policy: Codex now uses the plain
  preview render path (sanitized text + cursor marker), while Claude keeps ANSI
  styled rendering (`src/tui.rs`).
- Kept interactive cursor overlay for Codex by applying plain cursor insertion
  on already-sanitized lines (`src/tui.rs`).
- Added unit coverage for the render-policy split by agent
  (`src/tui.rs`).

Current status:
- Codex preview no longer goes through ANSI span rendering logic inside Grove,
  reducing Codex-specific render-state instability without changing Claude.
- Focused TUI tests covering overlay/render-policy paths are green.

Next steps:
- Manual retest in Grove interactive mode on Codex workspace to confirm the
  global styled/plain flip is gone.

## Update 2026-02-13, Control-Byte Sanitization For Preview Stability

What changed:
- Hardened capture sanitization to drop unsafe control bytes from tmux output
  while preserving printable text, tabs, and newlines (`src/agent_runtime.rs`).
- Kept ANSI SGR style support, but now strips C0 control bytes like `\r`,
  `\x0e`, and `\x0f` that can affect terminal/global render state when leaked.
- Added regression test proving terminal control bytes are removed from both
  plain and render capture outputs (`src/agent_runtime.rs`).

Current status:
- Preview render pipeline now blocks non-SGR control traffic that can destabilize
  pane/border rendering during interactive Codex sessions.
- Focused sanitizer + preview tests are green.

Next steps:
- Manual validation in real interactive Codex flow to confirm no more
  styled/plain render flipping when focusing preview.

## Update 2026-02-13, Codex Alternate-Screen Launch Fix

What changed:
- Updated Codex launch command construction to always include
  `--no-alt-screen`, in both normal and unsafe launch modes
  (`src/agent_runtime.rs`).
- Added a focused unit test for Codex command composition so future refactors
  cannot drop the flag (`src/agent_runtime.rs`).
- Updated TUI launch-flow assertions to match the new Codex launch command in
  start/default and unsafe-toggle paths (`src/tui.rs`).

Current status:
- Grove now starts Codex in inline mode in tmux-backed interactive preview,
  reducing alternate-screen rendering corruption in split-pane usage.
- Focused tests covering changed launch paths are green.

Next steps:
- Manual validation in the user environment: start Codex from Grove and confirm
  stable rendering in preview + interactive modes.

## Update 2026-02-13, Preview Rendering Stability

What changed:
- Fixed preview capture sanitization to strip ANSI control sequences before
  line splitting (`src/agent_runtime.rs`), not just mouse fragments.
- Kept mouse-fragment cleanup for bracketed fragments that may appear without
  escape bytes.
- Replaced interactive cursor overlay ANSI inversion with plain ASCII cursor
  marker insertion (`|`) to avoid raw escape text in the preview
  (`src/interactive.rs`).
- Updated and added tests covering ANSI stripping and cursor overlay rendering
  behavior (`src/agent_runtime.rs`, `src/interactive.rs`, `src/tui.rs`).

Current status:
- Manual UI regression reported (raw `[31m` style artifacts in preview) is
  addressed at the sanitizer and overlay layers.
- Focused test set for changed behavior is green.

Next steps:
- Re-run manual interactive preview check with real agent output (colored + mouse
  events) to validate no control-sequence leakage.
- If visual cursor emphasis needs improvement later, move to styled widget spans
  instead of ANSI inline escapes.

## Update 2026-02-13, ANSI Color Fidelity + Ctrl+Backslash Exit

What changed:
- Added dual capture outputs:
  - `cleaned_output` for diffing/scroll logic (plain text).
  - `render_output` that preserves SGR color sequences while stripping other
    control traffic (`src/agent_runtime.rs`).
- Added preview `render_lines` storage and viewport slicing so rendering can use
  color-capable lines while behavior logic uses plain lines (`src/preview.rs`).
- Implemented ANSI SGR parser to styled `ftui::text::Line` spans and switched
  preview pane rendering to styled `Text` instead of joined raw strings
  (`src/tui.rs`).
- Added ANSI-safe cursor marker insertion for interactive mode so marker render
  does not corrupt SGR streams (`src/interactive.rs`, `src/tui.rs`).
- Fixed interactive exit mapping to accept control-character form of
  `Ctrl+\` (`\u{1c}`) in addition to modifier+`\` form (`src/tui.rs`).

Current status:
- Preview now renders colors/styles from agent output instead of showing raw ANSI.
- `Ctrl+\` interactive exit works for both common terminal encodings.
- Targeted regression tests for parser, cursor overlay, capture change, and key
  mapping are green.

Next steps:
- Manual validation in your terminal profile for:
  - truecolor sequences (`38;2;r;g;b`)
  - 256-color sequences (`38;5;n`)
  - `Ctrl+\` on both local and remote tmux clients.

## Update 2026-02-13, Interactive Exit Simplification

What changed:
- Removed `Ctrl+\` interactive exit path from key model/state handling.
- Interactive exit is now single-path: double `Esc`.
- Updated interactive status hints to remove `Ctrl+\` mention.
- Updated interactive flow test to exit via double `Esc`.

Current status:
- Interactive mode behavior is simpler, fewer terminal-specific edge cases.
- Existing `Esc Esc` exit behavior remains unchanged.

Next steps:
- Manual check: enter interactive mode, press `Esc Esc`, confirm immediate exit.

## Why Separate From PRD

`PRD.md` defines what to build (product + technical requirements).
This doc defines how to build it incrementally, with frequent validation.

## Delegation Guardrails

- Treat `docs/PRD.md` as the only normative source, sidecar references are
  rationale only.
- Lock FrankenTUI dependency strategy in Phase 0 (vendored or pinned git SHA)
  before feature work.
- v1 scope is Claude/Codex only, do not add runtime support for other agents.

## Phase Gate (applies to every phase)

1. Scope implemented and reviewed
2. Red first, at least one failing test per new behavior before code changes
3. Green next, minimal code to make new tests pass
4. Refactor last, improve design with all tests still green
5. Full unit test suite passes
6. Manual TUI milestone checklist passes
7. No known P0/P1 defects left open for that phase scope
8. Commit phase work (pre-commit checks must pass: fmt, clippy, tests)

## Test Strategy By Phase

- TDD is mandatory in every phase, no exceptions
- Prefer module-level tests first, integration tests when behavior crosses
  modules (git + tmux + state + UI)
- Work in thin vertical slices, each slice ends with red, green, refactor
- Keep manual milestones short (10-20 min), but run them every phase
- If a manual milestone fails, phase is not complete even with green tests

## TDD Execution Loop (for each slice in a phase)

1. Pick one observable behavior from phase scope
2. Write/extend test, confirm it fails (red)
3. Implement smallest change to pass tests (green)
4. Refactor implementation and tests for clarity (refactor)
5. Repeat until phase scope done, then run manual TUI milestone

## Phase 0, Project Setup + Test Infra (Hello World)

Scope:
- Initialize Rust project structure and dependency graph
- Choose and lock reproducible FrankenTUI dependency source (no local paths)
- Configure lint, format, and test commands used in CI
- Add baseline unit/integration test harness
- Add hello-world domain behavior (non-TUI) to prove toolchain works

TDD targets:
- Hello-world behavior spec (pure function/module) before implementation
- Basic test utilities and fixtures compile/run
- Dependency bootstrap test/doc check for clean-clone build
- Command wiring for `fmt`, `clippy`, `test` is validated

Manual TUI milestone:
- Confirm project builds from clean checkout
- Run test command, see green baseline suite
- Run binary, confirm hello-world CLI output (no TUI yet)

Exit criteria:
- Dependencies are reproducible from clean clone, baseline tests green

## Phase 0.5, Hello World FrankenTUI Boot

Scope:
- Add FrankenTUI dependency and minimal app shell
- Render simple static hello-world frame in alt-screen
- Implement clean enter/exit lifecycle for TUI runtime
- Confirm no local-only path assumptions remain in build config

TDD targets:
- App model init/update/view contract for minimal frame
- Boot/quit action mapping
- Render smoke checks for first frame state

Manual TUI milestone:
- Launch FrankenTUI hello-world screen
- Verify keypress quit path works reliably
- Re-run launch/quit loop 5 times without panic

Exit criteria:
- FrankenTUI dependency and runtime lifecycle proven before feature work

## Phase 1, Grove Core Skeleton (Post-Bootstrap)

Scope:
- Define Grove domain model boundaries (workspace, status, UI mode state)
- Add app reducer/update flow for Grove-specific state transitions
- Add adapter interfaces for git/tmux/system interactions (no real feature flows yet)
- Render Grove shell layout (list, preview, status bar placeholders)

TDD targets:
- Domain invariants and default state construction
- Reducer transitions for selection, focus, and mode changes
- Adapter contract tests (fake implementations for deterministic behavior)

Manual TUI milestone:
- Launch Grove shell screen (not hello world)
- Navigate placeholder list and pane focus keys without errors
- Quit flow remains stable across repeated launch/quit cycles

Exit criteria:
- Product skeleton is ready, bootstrap/setup concerns fully complete in 0/0.5

## Phase 2, Read-Only Worktree Discovery + List UI

Scope:
- Discover main worktree + linked worktrees
- Populate list items and sort order (main pinned, recent activity ordering)
- Status bar hints in list mode
- Show deterministic read-only status (`Main`/`Idle`/`Unknown`) without tmux
  reconciliation yet

TDD targets:
- Worktree parsing and normalization
- Sorting and pinning rules
- Empty/error discovery states

Manual TUI milestone:
- Open repo with 0, 1, and multiple worktrees
- Validate row rendering (name, branch, path, status icon)
- Validate selection movement (`j/k`, arrows)

Exit criteria:
- Accurate list state from real git output, no mutation features yet

## Phase 3, Workspace Lifecycle (Create/Delete + Setup)

Scope:
- New workspace dialog, validation, creation flow
- Existing branch attach behavior (separate existing-branch field)
- Delete dialog and two-stage delete (normal, then force)
- Marker file creation and validation
- `.gitignore` update for Grove marker files
- `.env*` copy from main worktree
- `.grove-setup.sh` execution with env vars

TDD targets:
- Name validation edge cases (workspace slug vs branch attach)
- Create flow command sequencing
- Delete fallback behavior
- Marker + gitignore idempotency
- `.env` copy and setup-script behavior (success + failure warning path)

Manual TUI milestone:
- Create workspace from new branch
- Create workspace from existing branch
- Verify marker files, `.gitignore`, and `.env` copy behavior
- Verify setup script runs once on create and does not block creation on error
- Delete workspace with and without local branch delete option
- Verify main worktree cannot be deleted

Exit criteria:
- Workspace lifecycle stable under normal and error paths

## Phase 4, Agent Lifecycle + Reconciliation + Status Detection

Scope:
- Start/stop agent actions
- Tmux session lifecycle and wrapper integration
- Startup reconciliation (orphaned worktrees, orphaned sessions, missing dirs)
- Existing session reattach behavior on app restart
- Status detection (idle/running/waiting/done/error)
- Polling loop + output change detection signals
- Launcher with prompt + skip-permissions guardrails

TDD targets:
- Agent launch command construction (with/without prompt)
- Stop/restart transitions
- Reconciliation (orphan/missing directory/current-cwd-missing cases)
- Status parsing from tmux output and process state
- Poll debounce/change detection logic
- Skip-permissions default-off and explicit opt-in behavior

Manual TUI milestone:
- Start Claude/Codex in workspace, verify running status
- Stop and restart same workspace
- Validate waiting-state highlight and hints
- Kill tmux session externally, verify orphan detection
- Restart Grove with running sessions, verify reattach

Exit criteria:
- Reliable state transitions for real tmux sessions

## Phase 5, Preview Rendering + Auto-Scroll

Scope:
- ANSI capture/render pipeline
- Preview pane rendering and wrapping
- Auto-scroll pause/resume behavior
- Flash messages in status bar

TDD targets:
- ANSI tokenization/render transforms
- Auto-scroll state logic (at bottom vs paused)
- Output append and viewport calculations

Manual TUI milestone:
- Run agent with colored output, verify ANSI fidelity
- Scroll up, confirm auto-scroll pauses
- Return to bottom (`G`/scroll), confirm auto-scroll resumes
- Validate flash messages for action errors

Exit criteria:
- Preview is trustworthy for long-running agent sessions

## Phase 6, Interactive Mode (Critical Path)

Scope:
- Enter/exit interactive mode
- Keystroke forwarding to tmux
- Escape timing behavior
- Cursor overlay and paste handling

TDD targets:
- Interactive state machine transitions
- Key mapping (named keys, chars, modifiers)
- Double-escape timing window
- Paste sanitization and forwarding behavior

Manual TUI milestone:
- Enter interactive mode on running agent
- Type commands, verify live response
- Validate single vs double escape behavior
- Validate copy/paste shortcuts and expected forwarding

Exit criteria:
- TUI can replace direct tmux attach for core workflows

## Phase 7, Mouse + Dialog UX Completeness

Scope:
- Mouse selection, scrolling, pane focus changes
- Divider drag resize + persistence
- Modal input guards across dialogs

TDD targets:
- Hit region mapping
- Resize ratio bounds and persistence
- Dialog input blocking rules

Manual TUI milestone:
- Click list rows and preview pane focus
- Drag divider repeatedly, restart app, verify persisted ratio
- Open dialogs, verify non-dialog keys are blocked

Exit criteria:
- Keyboard and mouse workflows both production-usable

## Phase 8, Hardening, Regression Suite, Release Candidate

Scope:
- Edge-case closure (session death, missing cwd, orphan cleanup)
- Startup/shutdown reliability
- Final docs and operator runbook
- Validate operational targets from PRD (startup, latency, CPU, memory)

TDD targets:
- Regression tests for all previously fixed bugs
- Cross-module integration tests for startup reconciliation
- Error-path coverage for git/tmux failures
- Performance regression harness for polling/render hot paths

Manual TUI milestone:
- Full end-to-end smoke on clean repo and dirty repo
- Long-session test (>=30 min) with active output
- Restart Grove mid-session, verify reattach behavior

Exit criteria:
- Release candidate quality, no known critical workflow breakage

## Suggested Delivery Rhythm

- Target one phase per PR when possible
- If a phase is large, split into `phase-xa`/`phase-xb`, each with its own
  TDD cycles and manual milestone
- Commit at the end of each phase (or sub-phase) so work is checkpointed;
  pre-commit hooks (fmt, clippy, tests) must pass before the commit lands
- Never merge a phase without both gates (TDD + manual) complete

## Maintenance Notes

- 2026-02-13: Added repo instruction to always update this plan document after
  completed work, before handoff.
- 2026-02-13: Added lowercase `claude.md` symlink to `AGENTS.md` for tool
  compatibility.
- 2026-02-13: Phase 0 implemented.
  Changes: Rust crate bootstrap (`Cargo.toml`, `src/lib.rs`, `src/main.rs`),
  baseline unit/integration tests (`tests/` with fixtures/support), CI
  workflow for `fmt`/`clippy`/`test`, `Makefile` command wiring, Nix
  `devShells.default` (`flake.nix`, `flake.lock`), and FrankenTUI source
  strategy ADR (`docs/adr/002-frankentui-source-strategy.md`).
  Status: Phase 0 exit criteria met locally (tests green, lint/format green,
  clean-clone tooling defined).
  Next: Phase 0.5, add minimal FrankenTUI app boot/quit lifecycle in
  alt-screen mode.
- 2026-02-13: Phase 0.5 hello-world boot implemented.
  Changes: added pinned FrankenTUI dependency in `Cargo.toml`/`Cargo.lock`,
  introduced minimal alt-screen app lifecycle in `src/tui.rs`, wired binary
  default startup to TUI in `src/main.rs`, and kept deterministic CLI hello
  output for tests via `--print-hello`.
  Status: targeted checks passed locally (`fmt`, `clippy`, `--lib`,
  `hello_domain`, `hello_cli`).
  Next: begin Phase 1 domain skeleton (workspace/state/reducer boundaries).
- 2026-02-13: Expanded `flake.nix` dev shell dependencies so project tooling is
  available directly in `nix develop` (Rust toolchain, `git`, `tmux`,
  `gnumake`, `pkg-config`, `openssl`, and core shell utils).
- 2026-02-13: Phase 1 domain skeleton implemented.
  Changes: added explicit domain model types and invariants in
  `src/domain.rs`, reducer-driven app state with selection/focus/mode
  transitions in `src/state.rs`, adapter interfaces plus placeholder
  git/tmux/system bootstrap wiring in `src/adapters.rs`, and replaced the
  hello TUI frame with a Grove shell placeholder layout in `src/tui.rs`
  (list, preview, status bar scaffolding with key-driven navigation).
  Status: Phase 1 TDD targets are green locally (red-green cycle executed for
  reducer and adapter behaviors, tests passing).
  Next: Phase 2 read-only worktree discovery and deterministic list ordering.
- 2026-02-13: Phase 2 read-only worktree discovery + list UI implemented.
  Changes: replaced placeholder worktree bootstrap with git-backed discovery
  in `src/adapters.rs` (`git worktree list --porcelain` parser, branch
  activity extraction, main-pinned recent-activity sorting, detached/unknown
  handling, and explicit empty/error discovery states), expanded workspace
  domain shape in `src/domain.rs` to include `path` and
  `last_activity_unix_secs`, updated state fixtures in `src/state.rs`, and
  updated `src/tui.rs` list rendering to show status icon/name/branch/path
  rows plus context-sensitive list-mode status hints and discovery error/empty
  messaging.
  Status: Phase 2 TDD targets are green locally (parsing, normalization,
  sorting, empty/error states, and shell rendering assertions all passing).
  Next: Phase 3 workspace lifecycle (create/delete/setup, marker files,
  `.gitignore`, `.env*` copy, `.grove-setup.sh` execution).
- 2026-02-13: Phase 3 workspace lifecycle backend implemented.
  Changes: added `src/workspace_lifecycle.rs` with strict workspace create
  request validation (slug workspace names, separate existing-branch mode),
  git command sequencing for create/delete flows (including delete fallback to
  force), marker file write/read validation (`.grove-agent`, `.grove-base`),
  idempotent `.gitignore` entry management for Grove markers/scripts,
  `.env*` copy-on-create behavior, and `.grove-setup.sh` execution via
  injected runner with non-blocking warning path on setup failure.
  Added focused unit tests for all Phase 3 TDD targets listed above.
  Status: Phase 3 backend lifecycle behaviors are green locally via targeted
  tests (`cargo test workspace_lifecycle`).
  Next: Phase 4 agent lifecycle + tmux reconciliation and runtime status
  detection.
- 2026-02-13: Implemented Phase 4-8 core logic slices with tests (backend/model
  first, no full TUI event-loop wiring yet).
  Changes:
  - Phase 4: added `src/agent_runtime.rs` (tmux session naming, launch/stop
    plans, skip-permissions command handling, waiting/status detection,
    reconciliation, poll interval policy, capture hash/change detection with
    mouse fragment stripping).
  - Phase 4 discovery wiring: updated `src/adapters.rs` to discover
    marker-managed workspaces (`.grove-agent`/`.grove-base`), include
    unsupported-marker handling, reconcile against live tmux sessions, and
    surface orphaned sessions in bootstrap state.
  - Domain expansion: updated `src/domain.rs` statuses/icons to include
    `Active/Thinking/Waiting/Done/Error/Unsupported` and added workspace
    metadata (`base_branch`, `is_orphaned`, `supported_agent`).
  - Phase 5: added `src/preview.rs` (auto-scroll pause/resume, scroll burst
    guards, cleaned capture updates, flash message expiry).
  - Phase 6: added `src/interactive.rs` (interactive state machine, key mapping
    to tmux `send-keys`, double-escape exit window, paste wrapping, cursor
    overlay, mouse-fragment guards).
  - Phase 7: added `src/mouse.rs` (hit-testing, divider ratio clamp/drag,
    modal input blocking, ratio serialization helpers).
  - Phase 8: added `src/hardening.rs` (missing-worktree detection/prune signal,
    deleted-cwd recovery helper, orphaned session cleanup candidates, poll
    generation helpers).
  - Added cross-module startup reconciliation coverage in
    `tests/startup_reconciliation.rs`.
  - Updated `src/tui.rs` bootstrap to use live tmux session adapter, extended
    status hints, and rendered orphan marker text.
  Status: all touched checks pass locally (`cargo clippy --all-targets
  --all-features -- -D warnings`, `cargo test --lib`,
  `cargo test --test startup_reconciliation`).
  Next:
  - Wire Phase 5-7 runtime behaviors into the actual FrankenTUI update/view
    loop (interactive entry/exit, preview rendering pipeline, mouse events,
    modal dialogs).
  - Add end-to-end tests around command execution sequencing in the real app
    layer, not only module-level logic.
- 2026-02-13: Began wiring Phase 5 runtime behavior into the real TUI loop.
  Changes: updated `src/tui.rs` event handling to route key events through app
  logic, replaced preview placeholder text with `PreviewState`-driven rendering,
  added preview content refresh on selection changes/enter-preview, and wired
  preview scrolling controls (`j/k`, `PgUp/PgDn`, `G`) with auto-scroll state
  surfaced in the status bar.
  Status: targeted TUI unit tests pass locally (`cargo test tui:: --lib`).
  Next:
  - Continue Phase 5-7 wiring with live tmux capture/poll integration for
    preview updates.
  - Add interactive-mode entry/exit and key forwarding in the app loop.
  - Add mouse event handling (hit testing, divider drag, preview scroll).
- 2026-02-13: Wired interactive-mode entry/exit and tmux key forwarding into
  the real TUI loop.
  Changes: updated `src/tui.rs` to stop global key remapping/quit interception
  at the event boundary, added app-level interactive mode handling with
  `InteractiveState` (`Enter` on running workspaces opens interactive mode,
  `Esc Esc` / `Ctrl+\` exits), and integrated `tmux send-keys` command
  execution for interactive key events (including literal character forwarding
  so keys like `q` are sent to the agent instead of quitting Grove). Added
  focused TUI unit tests covering interactive entry, forwarding, double-escape
  exit, and non-interactive quit behavior.
  Status: targeted TUI checks pass locally (`cargo test tui:: --lib`).
  Next:
  - Wire live tmux capture + polling into preview updates so interactive/preview
    panes show real agent output.
  - Implement mouse event handling in the TUI loop (hit testing, divider drag,
    preview scroll).
- 2026-02-13: Completed the Phase 5-7 follow-up wiring slice for live polling,
  mouse handling, and interactive paste/copy actions in the real TUI loop.
  Changes: extended `src/tui.rs` event mapping and update flow to handle
  `Tick`/`Mouse`/`Paste`/`Resize` events, added periodic tmux output polling via
  `Cmd::tick` + `tmux capture-pane -p -e` (with dynamic poll interval from
  `agent_runtime::poll_interval`), enabled mouse capture and wired hit-tested
  click/scroll/divider-drag interactions using `src/mouse.rs`, and replaced
  interactive `Alt+C`/`Alt+V` no-ops with concrete behavior (capture tmux pane
  text into session-local buffer, then paste back via `send-keys -l`, plus
  bracketed paste event forwarding). Added focused TUI unit tests for each
  behavior and polling integration.
  Status: targeted checks pass locally (`cargo fmt`, `cargo test tui:: --lib`).
  Next:
  - Integrate cursor position polling/overlay in interactive mode using tmux
    pane cursor metadata.
  - Persist divider ratio across sessions (Phase 7 persistence target).
- 2026-02-13: Completed the remaining Phase 5-7 follow-up items from the TUI
  wiring backlog.
  Changes: updated `src/tui.rs` to poll tmux cursor metadata
  (`display-message -p`) alongside pane capture polling, parse cursor position
  and visibility, update `InteractiveState` cursor fields, and render cursor
  overlay on the preview lines shown during interactive mode. Added sidebar
  split persistence using `.grove-sidebar-width` (load on startup, save on
  divider drag changes). Expanded focused TUI tests for cursor metadata parsing,
  cursor overlay rendering in interactive mode, and cross-session split-ratio
  persistence.
  Status: targeted checks pass locally (`cargo fmt`, `cargo test tui:: --lib`).
  Next:
  - Add app-layer end-to-end tests for command sequencing (launch/stop/poll)
    through the real TUI update flow, not just module-level units.
  - Run manual TUI milestone validation for interactive cursor behavior and
    persisted split ratio against real tmux sessions.
- 2026-02-13: Added app-layer command sequencing tests in the real TUI update
  flow.
  Changes: expanded `src/tui.rs` test harness to record tmux execute/capture
  calls, added fast interactive poll-cadence assertion (`50ms` after key input),
  and added an end-to-end interactive update-flow test covering `Tick` preview
  polling, cursor metadata polling, key forwarding, copy capture, paste send,
  and `Ctrl+\` exit sequencing.
  Status: targeted checks pass locally (`cargo fmt`, `cargo test tui:: --lib`,
  `cargo clippy --all-targets --all-features -- -D warnings`).
  Next:
  - Run manual TUI milestone validation against real tmux sessions (cursor
    overlay fidelity and persisted split behavior across restarts).
- 2026-02-13: Wired start/stop agent actions into the TUI app loop and added
  app-layer sequencing coverage.
  Changes: updated `src/tui.rs` to handle non-interactive `[s]start` and
  `[x]stop` actions using `agent_runtime::build_launch_plan` and
  `agent_runtime::stop_plan` command sequences, update selected workspace
  runtime status (`Idle` <-> `Active`), and refresh preview/state after action
  completion. Added mutable selected-workspace accessor in `src/state.rs` to
  support in-place status transitions. Expanded TUI tests for start command
  sequencing, stop command sequencing, and main-worktree start guard.
  Status: targeted checks pass locally (`cargo fmt`, `cargo test tui:: --lib`,
  `cargo clippy --all-targets --all-features -- -D warnings`).
  Next:
  - Add prompt + skip-permissions launch inputs to the real TUI action path.
  - Run manual TUI milestone validation with real tmux sessions for start/stop
    flows and status transitions.
- 2026-02-13: Added prompt + skip-permissions launch inputs to the real start
  action path.
  Changes: updated `src/tui.rs` start flow to read optional per-workspace
  prompt file (`.grove-prompt`) and pass prompt to
  `agent_runtime::build_launch_plan` (launcher script path), added explicit
  unsafe launch toggle (`!`) with default-off behavior and status-bar
  visibility, and threaded skip-permissions into launch request construction.
  Expanded TUI tests for unsafe toggle command flags and prompt-file launcher
  script sequencing.
  Status: targeted checks pass locally (`cargo fmt`, `cargo test tui:: --lib`,
  `cargo clippy --all-targets --all-features -- -D warnings`).
  Next:
  - Add modal/dialog-driven launch prompt and skip-permissions controls to align
    with PRD UX (instead of file/key-based interim input).
  - Run manual TUI milestone validation with real tmux sessions for start/stop,
    prompt launch, and status transitions.
- 2026-02-13: Added modal start-agent dialog flow in the TUI app loop for
  prompt and unsafe launch controls.
  Changes: updated `src/tui.rs` to open a start dialog on `[s]` with prompt
  editing and `[Tab]` unsafe toggle, block background key/mouse input while the
  dialog is active, confirm with `[Enter]` to execute launch plan, and cancel
  with `[Esc]`. Dialog defaults are seeded from `.grove-prompt` and the global
  unsafe default, then applied on confirmation.
  Expanded TUI tests for dialog-open sequencing, background-input guard,
  tab-based unsafe toggle, prompt-launch script path, and existing start/stop
  sequencing.
  Status: targeted checks pass locally (`cargo fmt`, `cargo test tui:: --lib`,
  `cargo clippy --all-targets --all-features -- -D warnings`).
  Next:
  - Run manual TUI milestone validation with real tmux sessions for start/stop,
    dialog prompt launch, cursor overlay, and persisted split ratio behavior.
  - Implement explicit flash-message UX for start-on-running and invalid actions
    to align with PRD status-bar messaging.
- 2026-02-13: Implemented status-bar flash messaging for start/stop guardrails
  and action outcomes.
  Changes: updated `src/tui.rs` to use timed flash messages in the status bar
  for invalid actions (start on main/running, stop when idle, unsupported
  marker) and action outcomes (agent started/stopped, launch failures), with
  auto-expiry on tick. Added focused TUI tests for running-start and idle-stop
  flash behavior and main-worktree start flash guard.
  Status: targeted checks pass locally (`cargo fmt`, `cargo test tui:: --lib`,
  `cargo clippy --all-targets --all-features -- -D warnings`).
  Next:
  - Run manual TUI milestone validation with real tmux sessions for start/stop,
    dialog prompt launch, cursor overlay, persisted split ratio, and flash UX.
- 2026-02-13: Ran scripted manual tmux smoke validation against real TUI runtime
  (non-mocked).
  Changes: executed a real-session smoke workflow by creating a temporary git
  worktree with Grove markers + `.grove-prompt`, launching Grove in tmux,
  selecting workspace, confirming start dialog, verifying agent tmux session
  creation, stopping via `[x]`, verifying session removal, and quitting Grove.
  Validated launcher script materialization (`.grove-start.sh`) from prompt.
  Status: smoke checks passed (`START_OK`, `STOP_OK`, `QUIT_OK`, `LAUNCHER_OK`)
  and temporary worktree/session artifacts were cleaned up.
  Next:
  - Complete manual UI validation for mouse-driven divider persistence and
    interactive cursor overlay fidelity (not covered by the scripted smoke run).
- 2026-02-13: Manual UI pass identified major visual/layout drift from PRD and
  Sidecar-like shell expectations.
  Changes: reviewed live UI output against current implementation and validated
  that `src/tui.rs` still renders a monolithic newline-joined `Paragraph`
  (`shell_lines`) instead of true pane composition. Confirmed missing PRD
  layout architecture pieces: `Flex` split-based two-pane render path,
  bordered sidebar/preview panels, and frame-level hit-region registration in
  `view()`.
  Status: backend/runtime lifecycle behavior is present, but UI composition is
  still effectively a text dump, causing the non-Sidecar appearance.
  Next:
  - Refactor `view()` to render real two-pane chrome (`header + sidebar +
    divider + preview + status`) using `ftui::layout::Flex` and `Block`.
  - Move workspace row rendering and preview rendering into dedicated pane
    renderers instead of `shell_lines`.
  - Replace hard-coded mouse row mapping with pane-relative hit registration
    in `view()` for accurate click selection and divider behavior.
- 2026-02-13: Implemented structural two-pane TUI composition for Sidecar-like
  layout parity.
  Changes: refactored `src/tui.rs` rendering path from monolithic text dump to
  explicit pane composition with `Flex` layout (`header`, `sidebar`, `divider`,
  `preview`, `status`), introduced dedicated pane renderers (including bordered
  sidebar/preview panels), added centered start-agent modal overlay rendering,
  and replaced hard-coded mouse row mapping with geometry-driven region
  detection and pane-relative workspace row selection.
  Status: structural parity goal met for layout architecture, existing TUI unit
  tests remain green after refactor.
  Next:
  - Follow up with frame-registered hit IDs in `view()` (instead of geometry
    math in update path) for full PRD hit-grid alignment.
  - Add render-focused assertions around pane boundaries and modal placement to
    lock layout structure against regressions.
- 2026-02-13: Fixed post-layout usability gaps for workspace creation and
  divider mouse drag.
  Changes: added a keyboard-driven new-workspace modal in `src/tui.rs`
  (`[n]/[N]` open, type name, `[Tab]` toggle agent Claude/Codex, `[Enter]`
  create, `[Esc]` cancel), wired modal confirmation to real workspace lifecycle
  backend (`workspace_lifecycle::create_workspace`) with git/setup execution and
  list refresh via `bootstrap_data`, and expanded divider hit detection to
  accept near-divider mouse down positions so drag-resize reliably triggers.
  Also updated status hints and added focused TUI unit coverage for new dialog
  behavior and near-divider drag.
  Status: targeted TUI tests are green locally after changes.
  Next:
  - Add branch-mode controls (existing-branch attach) and base-branch editing to
    the new-workspace modal for full Phase 3 parity.
  - Run manual interaction pass in live terminal for `n` create flow and drag
    behavior across different terminal emulators.

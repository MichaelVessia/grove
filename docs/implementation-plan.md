# Grove Implementation Plan

This document turns the PRD into a phased execution plan with explicit
quality gates.

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

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
- Never merge a phase without both gates (TDD + manual) complete

## Maintenance Notes

- 2026-02-13: Added repo instruction to always update this plan document after
  completed work, before handoff.
- 2026-02-13: Added lowercase `claude.md` symlink to `AGENTS.md` for tool
  compatibility.

# HANDOFF

## Context
- Repo: `grove`
- Branch: `master`
- Refactor goal: break monolithic `tui.rs` into modular, domain-aligned structure (DDD-inspired), with tests colocated by module.
- User preference: run tests after each phase, commit at milestones.

## Completed Milestones

### Phase 1, move TUI under `ui`
- Commit: `eb9ab96`
- Changes:
  - moved `src/tui.rs` -> `src/ui/tui/mod.rs`
  - added `src/ui/mod.rs`
  - added shim `src/tui.rs` re-exporting run fns
  - updated `src/lib.rs` with `pub mod ui;`
  - fixed include path-coupled test usage
- Gate:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 2, extract terminal/bootstrap/ansi internals
- Commit: `092df8e`
- Changes:
  - extracted from `src/ui/tui/mod.rs`:
    - `src/ui/tui/ansi.rs`
    - `src/ui/tui/bootstrap.rs`
    - `src/ui/tui/terminal.rs`
  - wired imports and visibility adjustments
- Gate:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 3, colocate tests into dedicated files
- Commit: `dc0b0c1`
- Changes:
  - moved large TUI tests to `src/ui/tui/tests/mod.rs`
  - kept `#[cfg(test)] mod tests;` in `src/ui/tui/mod.rs`
  - extracted inline module tests into colocated files:
    - `src/adapters/tests.rs`
    - `src/agent_runtime/tests.rs`
    - `src/config/tests.rs`
    - `src/domain/tests.rs`
    - `src/event_log/tests.rs`
    - `src/hardening/tests.rs`
    - `src/interactive/tests.rs`
    - `src/mouse/tests.rs`
    - `src/preview/tests.rs`
    - `src/state/tests.rs`
    - `src/workspace_lifecycle/tests.rs`
    - `src/zellij_emulator/tests.rs`
  - root test module naming cleanup:
    - `src/lib.rs` -> `#[cfg(test)] mod lib_tests;` with `src/lib_tests.rs`
    - `src/main.rs` -> `#[cfg(test)] mod main_tests;` with `src/main_tests.rs`
- Gates:
  - `cargo test --lib` (pass, 276)
  - `cargo test --bin grove` (pass, 4)
  - re-run both after formatting cleanup (pass)

### Phase 5a, extract core TUI modules (`msg/update/view/dialogs`)
- Commit: pending (this handoff update ships with commit)
- Changes:
  - added `src/ui/tui/msg.rs`
    - moved `Msg` enum
    - moved preview/workspace completion structs
    - moved `impl From<Event> for Msg`
  - added `src/ui/tui/update.rs`
    - moved `init` and `update` logic into `init_model` / `update_model`
  - added `src/ui/tui/view.rs`
    - moved `view` draw + timing logic into `render_model`
  - added `src/ui/tui/dialogs.rs`
    - moved dialog state enums/structs
    - moved shared modal row/render helper fns
    - moved `OverlayModalContent`
  - updated `src/ui/tui/mod.rs`
    - added module wiring (`mod dialogs`, `mod msg`, `mod update`, `mod view`)
    - removed moved definitions
    - `impl Model for GroveApp` now delegates to extracted methods
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5b, move dialog key handlers into `dialogs.rs`
- Commit: pending
- Changes:
  - moved modal/key handler methods from `src/ui/tui/mod.rs` into `src/ui/tui/dialogs.rs`:
    - `handle_keybind_help_key`
    - `handle_project_add_dialog_key`
    - `handle_project_dialog_key`
    - `handle_settings_dialog_key`
    - `handle_delete_dialog_key`
    - `handle_create_dialog_key`
    - `handle_edit_dialog_key`
    - `handle_launch_dialog_key`
  - updated method visibility to `pub(super)` for cross-submodule calls from other `GroveApp` impl blocks
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5c, move dialog/overlay render helpers into `view.rs`
- Commit: pending
- Changes:
  - moved render helper methods from `src/ui/tui/mod.rs` to `src/ui/tui/view.rs`:
    - `render_toasts`
    - `render_launch_dialog_overlay`
    - `render_delete_dialog_overlay`
    - `render_settings_dialog_overlay`
    - `render_project_dialog_overlay`
    - `render_command_palette_overlay`
    - `render_keybind_help_overlay`
    - `render_create_dialog_overlay`
    - `render_edit_dialog_overlay`
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5d, move key and mouse dispatch into `update.rs`
- Commit: pending
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - `handle_mouse_event`
    - `handle_key`
  - `handle_key` now `pub(super)` to preserve direct test access from sibling test module
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

## Current State
- Worktree includes Phase 5a module extraction changes + this handoff update.
- Local branch includes prior refactor commits:
  - `dc0b0c1`
  - `092df8e`
  - `eb9ab96`

## Next Plan (execute in phases)

### Phase 5, split remaining `src/ui/tui/mod.rs`
Status:
- `msg`, `update`, `view`, `dialogs` created and wired.
- dialog key handlers moved into `dialogs.rs`.
- dialog and overlay render helpers moved into `view.rs`.
- key and mouse dispatch moved into `update.rs`.
- Remaining work is further decomposition of large `GroveApp` impl blocks inside `mod.rs`.

Next sub-targets:
- move remaining render helpers and pane rendering into `view.rs`
- move remaining input/event orchestration into `update.rs`

Rules:
- keep behavior unchanged
- smallest possible moves per commit
- no compatibility shims unless required
- run focused tests first, then broader gate

Suggested gate for each sub-phase:
- `cargo test --lib ui::tui::tests -- --nocapture`
- if touching cross-module state, also run `cargo test --lib`

Commit after each stable chunk.

### Phase 6, separate non-UI concerns out of UI layer
- Identify logic in `ui/tui` that belongs in infra/application/domain (session lifecycle glue, polling strategies, runtime integration boundaries).
- Move behind explicit module boundaries.
- Validate both multiplexer paths (`tmux`, `zellij`) for parity.

### Phase 7, align crate tree to DDD shape
Proposed target top-level modules:
- `src/domain/`
- `src/application/`
- `src/infrastructure/`
- `src/ui/`

Move incrementally, preserving compile + test green at each step.

### Phase 8, cleanup
- remove transitional re-exports/shims no longer needed
- refresh docs for new module map
- final full test pass

## Guardrails For Next Agent
- Do not squash milestones unless asked.
- Re-run tests after each phase.
- Keep test files colocated with owning module.
- Preserve type safety, avoid temporary weak abstractions.
- For session lifecycle / capture / key forwarding / polling changes, verify `tmux` and `zellij` paths.
- Keep keybind/command discoverability in sync if changed.

## Handy Commands
- `git log --oneline -n 10`
- `git status --short`
- `cargo test --lib ui::tui::tests -- --nocapture`
- `cargo test --lib`
- `cargo test --bin grove`

# Proposed Code File Reorganization Plan

## Scope And Assumptions

- I found no literal `src/x` path in this repo.
- I treated `src/ui/tui` as the target, because it is the main scattered hotspot.
- I audited all Rust files in `src/ui/tui` (`98` files), plus external call sites in `src/main.rs` and cross-layer dependencies used by TUI code.

## Audit Summary

### Size + concentration

- `src/` has `140` Rust files.
- `src/ui/tui` has `98` Rust files (`70%` of all Rust files under `src/`).
- `79/98` files in `src/ui/tui` use `use super::*`, high implicit coupling.

### Largest files (high refactor pressure)

- `src/ui/tui/replay.rs` (2334)
- `src/ui/tui/view_overlays_help.rs` (679)
- `src/ui/tui/view_chrome_sidebar.rs` (594)
- `src/ui/tui/bootstrap_app.rs` (463)
- `src/ui/tui/dialogs.rs` (443)
- `src/ui/tui/update_navigation_preview.rs` (434)
- `src/ui/tui/dialogs_projects_add.rs` (426)
- `src/ui/tui/update_input_key_events.rs` (399)
- `src/ui/tui/update_input_mouse.rs` (392)
- `src/ui/tui/mod.rs` (377)
- `src/ui/tui/update_polling_state.rs` (365)

### Architectural clusters (current, functional)

- Bootstrapping: `bootstrap_*`
  - config load, startup defaults, discovery hydration, app construction.
- Command system: `commands*`, `update_navigation_commands.rs`, `update_navigation_palette.rs`.
  - palette specs, context hints, command dispatch and enablement.
- Dialog system: `dialogs*`, `dialogs_state_*`, `view_overlays_*`.
  - modal state + key handlers + overlay rendering.
- Input handling: `update_input_*`, `update_navigation_preview.rs`.
  - key/mouse/paste mapping, interactive mode, tmux send queue.
- Polling/runtime loop: `update_tick.rs`, `update_polling_*`.
  - adaptive scheduling, capture dispatch, status reconciliation.
- Rendering: `view*`, `selection*`, `text/*`, `ansi/*`.
  - frame composition, sidebar/preview/status, text shaping + ANSI.
- Logging + replay: `logging_*`, `replay.rs`.
  - structured runtime events, debug-record replay and state/frame verification.
- Platform adapter: `terminal/*`.
  - tmux I/O, clipboard, cursor metadata parsing.

## Problems To Solve

- Files are grouped by prefix, not folder domain.
- Too many files live directly under `src/ui/tui/`.
- Heavy implicit imports (`use super::*`) hide dependency boundaries.
- `mod.rs` is overloaded (model type defs + module wiring + imports).
- Some files are too large for fast local reasoning (`replay.rs`, help overlay, sidebar renderer, project dialog behavior).

## Reorganization Goals

- One additional nesting level only (no deep tree).
- Immediate discoverability by feature domain.
- Phase 1 must be low-risk and behavior-preserving.
- Keep compile/test risk controlled, do not trigger broad visibility rewrites in first pass.

## Proposed Target Layout (Phase 1, low-risk)

```text
src/ui/tui/
  mod.rs
  msg.rs
  shared.rs
  replay.rs
  runner.rs
  selection.rs
  ansi.rs
  text.rs
  terminal.rs

  bootstrap/
    bootstrap_app.rs
    bootstrap_config.rs
    bootstrap_discovery.rs

  commands/
    commands.rs
    commands_hints.rs
    commands_palette.rs

  dialogs/
    dialogs.rs
    dialogs_confirm.rs
    dialogs_create_key.rs
    dialogs_create_setup.rs
    dialogs_delete.rs
    dialogs_edit.rs
    dialogs_launch.rs
    dialogs_merge.rs
    dialogs_projects_add.rs
    dialogs_projects_key.rs
    dialogs_projects_state.rs
    dialogs_settings.rs
    dialogs_state_create_edit.rs
    dialogs_state_lifecycle.rs
    dialogs_state_project_settings.rs
    dialogs_stop.rs
    dialogs_update_from_base.rs

  logging/
    logging_frame.rs
    logging_input.rs
    logging_state.rs

  update/
    update.rs
    update_core.rs
    update_input_interactive.rs
    update_input_interactive_clipboard.rs
    update_input_interactive_send.rs
    update_input_key_events.rs
    update_input_keybinding.rs
    update_input_keys.rs
    update_input_mouse.rs
    update_lifecycle_create.rs
    update_lifecycle_start.rs
    update_lifecycle_stop.rs
    update_lifecycle_workspace_completion.rs
    update_lifecycle_workspace_refresh.rs
    update_navigation_commands.rs
    update_navigation_palette.rs
    update_navigation_preview.rs
    update_navigation_summary.rs
    update_polling_capture_cursor.rs
    update_polling_capture_dispatch.rs
    update_polling_capture_live.rs
    update_polling_capture_scroll.rs
    update_polling_capture_task.rs
    update_polling_capture_workspace.rs
    update_polling_state.rs
    update_tick.rs

  view/
    view.rs
    view_chrome_divider.rs
    view_chrome_header.rs
    view_chrome_shared.rs
    view_chrome_sidebar.rs
    view_layout.rs
    view_overlays_confirm.rs
    view_overlays_create.rs
    view_overlays_edit.rs
    view_overlays_help.rs
    view_overlays_projects.rs
    view_overlays_settings.rs
    view_overlays_workspace_delete.rs
    view_overlays_workspace_launch.rs
    view_overlays_workspace_merge.rs
    view_overlays_workspace_stop.rs
    view_overlays_workspace_update.rs
    view_preview.rs
    view_preview_content.rs
    view_preview_shell.rs
    view_selection_interaction.rs
    view_selection_logging.rs
    view_selection_mapping.rs
    view_status.rs

  tests/
    mod.rs
    runtime_flow/
      mod.rs
      flow_a.rs
      flow_b.rs
      flow_c.rs
```

## Why This Phase 1 Layout Is Optimal

- No semantic change, pure file-system grouping.
- Keeps existing module names intact, so internal call graph stays stable.
- Allows `mod.rs` to use `#[path = "..."]` and preserve current privacy behavior.
- Removes root-level file sprawl immediately.
- Enables later targeted refactors per folder without massive one-shot churn.

## Exact Move Map (Phase 1)

- `src/ui/tui/bootstrap_*.rs -> src/ui/tui/bootstrap/bootstrap_*.rs`
- `src/ui/tui/commands*.rs -> src/ui/tui/commands/commands*.rs`
- `src/ui/tui/dialogs*.rs -> src/ui/tui/dialogs/dialogs*.rs`
- `src/ui/tui/logging_*.rs -> src/ui/tui/logging/logging_*.rs`
- `src/ui/tui/update*.rs -> src/ui/tui/update/update*.rs`
- `src/ui/tui/view*.rs -> src/ui/tui/view/view*.rs`
- keep these where they are in phase 1: `mod.rs`, `msg.rs`, `shared.rs`, `runner.rs`, `replay.rs`, `selection.rs`, wrappers `ansi.rs`, `text.rs`, `terminal.rs`, plus existing `ansi/`, `text/`, `terminal/`, `tests/`.

## Calling Code Impact Tracker

### Phase 1 impact (low-risk, contained)

- Primary edit: `src/ui/tui/mod.rs`
  - change each moved module declaration to `#[path = "<folder>/<file>.rs"] mod <same_name>;`
  - keep module identifiers unchanged (`dialogs_delete`, `update_polling_state`, etc).
- No functional API change expected for external callers.

External callers to preserve (current):

- `src/main.rs:139` `emit_replay_fixture`
- `src/main.rs:143` `ReplayOptions`
- `src/main.rs:147` `replay_debug_record`
- `src/main.rs:180` `run_with_debug_record`
- `src/main.rs:183` `run_with_event_log`

Internal direct `crate::ui::tui::...` references to keep compiling:

- `src/ui/tui/tests/runtime_flow/flow_b.rs:2802` (`CursorMetadata`)
- `src/ui/tui/tests/runtime_flow/flow_a.rs:1788`
- `src/ui/tui/tests/runtime_flow/flow_a.rs:1887`
- `src/ui/tui/tests/runtime_flow/flow_a.rs:1955`
  - (`ConfirmDialogField`)

## Merge / Consolidation Recommendations (Phase 2)

These are high-value after Phase 1 grouping.

1. Merge command catalog files
- Merge `commands.rs`, `commands_hints.rs`, `commands_palette.rs` into:
  - `commands/catalog.rs` (enum + spec)
  - `commands/help.rs` (hint mapping)
  - `commands/palette.rs` (palette action mapping)
- Rationale: these are one cohesive bounded context.

2. Merge dialog state-only files
- Merge `dialogs_state_create_edit.rs`, `dialogs_state_lifecycle.rs`, `dialogs_state_project_settings.rs` into `dialogs/state.rs`.
- Rationale: shared state types become discoverable in one place.

3. Merge tiny navigation/polling leaf files
- `update_navigation_summary.rs` into `update_navigation_preview.rs`.
- `update_input_keys.rs` into `update_input_key_events.rs`.
- `update_polling_capture_scroll.rs` into `update_polling_capture_live.rs`.
- Rationale: these files are small and tightly coupled to their parent behavior files.

## Split Recommendations (Phase 2)

1. Split `replay.rs` (2334)
- Proposed split:
  - `replay/types.rs` (Replay* wire structs/enums)
  - `replay/codec.rs` (from/to `Msg` and domain conversions)
  - `replay/trace_parser.rs` (`parse_replay_trace`, logged-line parsing)
  - `replay/engine.rs` (`replay_debug_record`, invariants, frame hashing)
  - `replay/fixtures.rs` (`emit_replay_fixture`, naming)
- Rationale: isolates serialization model from replay runtime logic.

2. Split `view_overlays_help.rs` (679)
- Proposed split:
  - `view/help_palette_overlay.rs`
  - `view/help_keybind_overlay.rs`
  - `view/help_rows.rs` (row builders)
- Rationale: palette rendering and help rendering are distinct concerns.

3. Split `view_chrome_sidebar.rs` (594)
- Proposed split:
  - `view/sidebar_model.rs` (line structs, hit metadata)
  - `view/sidebar_build.rs` (line construction)
  - `view/sidebar_render.rs` (render + hit registration)
- Rationale: easier to reason about layout bugs vs data bugs.

4. Split `dialogs_projects_add.rs` (426)
- Proposed split:
  - `dialogs/projects_crud.rs`
  - `dialogs/projects_reorder.rs`
  - `dialogs/projects_defaults.rs`
- Rationale: currently mixes add/delete/reorder/defaults persistence + validation.

5. Split `mod.rs` (377)
- Proposed split:
  - `model.rs` (`GroveApp`, trackers, dialog enum)
  - `mod.rs` only wiring/reexports.
- Rationale: reduce high-friction navigation entrypoint.

## Import + Visibility Strategy (Critical To Avoid Breakage)

Phase 1:

- Keep all module names unchanged.
- Use only `#[path]` rewiring in `mod.rs`.
- Keep existing `pub(super)` / private method visibility unchanged.
- This avoids cross-module privacy breakage from deeper namespace changes.

Phase 2 (if renaming module identifiers):

- Replace broad `use super::*` gradually with explicit imports or constrained prelude modules.
- For methods needed across new submodules, convert selected items from `pub(super)` to `pub(in crate::ui::tui)` (not broader than necessary).
- Do this incrementally per subdomain, not all at once.

## Execution Plan

### Phase 1 (safe, no-brainer)

1. Move files into grouped folders (as above).
2. Update `src/ui/tui/mod.rs` with `#[path = ...]` for moved modules.
3. Compile and run TUI-focused tests.

### Phase 2 (targeted cleanup)

1. Merge tiny/highly-coupled files.
2. Split giant files by concern.
3. Reduce `use super::*` incrementally.
4. Keep public TUI API surface stable (`run_*`, replay API).

## Validation Plan

Run after each phase (or each sub-step in phase 2):

1. `cargo test ui::tui::tests::`
2. `cargo test ui::tui::replay::tests::`
3. `cargo test main_tests::`
4. `make precommit`

(Use narrower subsets while iterating file-by-file, then run all four before handoff.)

## Risks + Mitigations

- Risk: privacy breakages if module names change with deeper nesting.
  - Mitigation: Phase 1 path-based move only, no module identifier change.
- Risk: hidden coupling due `use super::*`.
  - Mitigation: phase-by-phase explicit import reduction after structure is stable.
- Risk: replay behavior regressions during `replay.rs` split.
  - Mitigation: keep golden replay tests and frame/state hash checks green after each extraction.

## Recommendation

- Approve Phase 1 first (pure structural grouping, minimal behavior risk).
- After Phase 1 lands green, execute Phase 2 in small PR-sized steps, starting with `replay.rs` split and command/dialog state consolidation.

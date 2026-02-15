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
- Commit: `a49d2c4`
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
- Commit: `7c1bda9`
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
- Commit: `1b78f7b`
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
- Commit: `6bd2566`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - `handle_mouse_event`
    - `handle_key`
  - `handle_key` now `pub(super)` to preserve direct test access from sibling test module
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5e, move paste and non-interactive key flow into `update.rs`
- Commit: `38194da`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - `handle_paste_event`
    - `enter_preview_or_interactive`
    - `handle_non_interactive_key`
  - `enter_preview_or_interactive` now `pub(super)` because it is called from parent-module command palette action handling
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5f, move workspace selection helpers into `update.rs`
- Commit: `e7dec48`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - `sidebar_workspace_index_at_y`
    - `select_workspace_by_mouse`
    - `select_workspace_by_index`
  - `select_workspace_by_index` now `pub(super)` because it is used by parent-module project focus logic
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5g, move pane render helpers into `view.rs`
- Commit: `fa7042a`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/view.rs`:
    - status/hint helpers (`unsafe_label`, `status_bar_line`, `keybind_hints_line`)
    - pane/style/render helpers (`pane_border_style`, `workspace_agent_color`, `render_header`, `render_sidebar`, `render_divider`, `render_preview_pane`, `render_status_line`, `shell_lines`, related animation helpers)
  - set cross-submodule test-facing methods to `pub(super)` where needed
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5h, move interactive input pipeline into `update.rs`
- Commit: `8bd17d6`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - `map_interactive_key`
    - interactive send queue/dispatch/completion methods
    - clipboard paste/read helpers
    - `handle_interactive_key`
  - made `copy_interactive_selection_or_visible` `pub(super)` in parent module for sibling access
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5i, move keybinding and interactive-entry helpers into `update.rs`
- Commit: `d529cae`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - keybinding helpers (`is_quit_key`, `is_ctrl_char_key`, `keybinding_*`, `apply_keybinding_action`)
    - interactive entry helpers (`can_enter_interactive`, `enter_interactive`)
    - focused-tab helpers (`preview_agent_tab_is_focused`, `preview_git_tab_is_focused`)
    - start/help entry points (`can_start_selected_workspace`, `open_keybind_help`)
  - exposed sibling-needed methods as `pub(super)`
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5j, move selection movement helpers into `update.rs`
- Commit: `dae3113`
- Changes:
  - moved `persist_sidebar_ratio` and `move_selection` to `src/ui/tui/update.rs`
  - set `move_selection` to `pub(super)` for command-palette use from parent module
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5k, move project dialog operations into `dialogs.rs`
- Commit: `7ee7e06`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/dialogs.rs`:
    - project filter/index helpers
    - project dialog open/add/create persistence flow
  - `open_project_dialog` set to `pub(super)` for update-module key handling
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5l, move settings dialog operations into `dialogs.rs`
- Commit: `c2aa6fa`
- Changes:
  - moved settings open/save/session-check helpers into `src/ui/tui/dialogs.rs`
  - `open_settings_dialog` set to `pub(super)` for sibling callers
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5m, move layout and selection helpers into `view.rs`
- Commit: `5d3f76b`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/view.rs`:
    - layout/hit-testing helpers (`view_layout_for_size`, `view_layout`, `hit_region_for_point`, etc.)
    - cursor overlay helpers
    - preview text-mapping/selection/logging/highlight helpers
    - copy-selected-or-visible preview output helper
  - added `pub(super)` visibility where parent/update/tests require cross-submodule access
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5n, move start/delete dialog orchestration into `dialogs.rs`
- Commit: `5839585`
- Changes:
  - moved `open_start_dialog`, `open_delete_dialog`, `confirm_delete_dialog` to `src/ui/tui/dialogs.rs`
  - `open_start_dialog` and `open_delete_dialog` set to `pub(super)` (update/module/tests call-sites)
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5o, move create/edit dialog state logic into `dialogs.rs`
- Commit: `3970575`
- Changes:
  - moved create/edit dialog helpers from `src/ui/tui/mod.rs` to `src/ui/tui/dialogs.rs`:
    - selected branch/project helpers
    - open/create/edit dialog setup
    - edit save flow
    - create branch picker/filter helpers
  - methods needed cross-submodule/tests exposed as `pub(super)`:
    - `open_create_dialog`
    - `open_edit_dialog`
    - `clear_create_branch_picker`
    - `refresh_create_branch_filtered`
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5p, move command palette and modal/summary helpers into `update.rs`
- Commit: `b163c8a`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - command palette action build/open/execute helpers
    - modal-open helpers
    - preview summary + output dimensions helpers
    - preview tab cycle + workspace summary/splash helpers
  - exposed cross-submodule/test methods as `pub(super)`:
    - `build_command_palette_actions`
    - `open_command_palette`
    - `modal_open`
    - `refresh_preview_summary`
    - `preview_output_dimensions`
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5q, move preview polling and capture runtime into `update.rs`
- Commit: `d444350`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - git preview session helpers (`git_tab_session_name`, lazygit session prep)
    - preview/status poll target and capture application pipeline
    - cursor capture + resize verify flow
    - sync/async preview polling orchestration
    - preview scroll/jump handlers
  - exposed cross-submodule methods as `pub(super)`:
    - `git_tab_session_name`
    - `prepare_live_preview_session`
    - `interactive_target_session`
    - `workspace_status_poll_targets`
    - `poll_preview`
    - `sync_interactive_session_geometry`
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5r, move lifecycle and workspace refresh flow into `update.rs`
- Commit: `ec108ab`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - delete/create/start/stop completion handlers and run helpers
    - workspace refresh sync/background flow
    - create/start dialog confirmation execution paths
    - lifecycle git helper functions
  - exposed cross-submodule methods as `pub(super)`:
    - `apply_delete_workspace_completion`
    - `run_delete_workspace`
    - `workspace_lifecycle_error_message`
    - `refresh_workspaces`
    - `confirm_create_dialog`
    - `confirm_start_dialog`
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5s, move adaptive tick scheduling and activity tracking into `update.rs`
- Commit: `805390d`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - adaptive poll interval + tick due/schedule helpers
    - status digest/output changing tracking helpers
    - activity frame tracking + visual-working predicate
  - exposed cross-submodule/test methods as `pub(super)`:
    - `status_is_visually_working`
    - `push_agent_activity_frame`
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5t, move dialog text-input modifier helper into `dialogs.rs`
- Commit: `f634be8`
- Changes:
  - moved `allows_text_input_modifiers` from `src/ui/tui/mod.rs` to `src/ui/tui/dialogs.rs`
  - no behavior changes
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5u, extract shared text/formatting helpers into `text.rs`
- Commit: `912bb07`
- Changes:
  - added `src/ui/tui/text.rs`
    - moved pure text/formatting helpers out of `src/ui/tui/mod.rs`:
      - preview text helpers (`line_visual_width`, `visual_substring`, `visual_grapheme_at`)
      - truncation/padding helpers (`truncate_for_log`, `truncate_to_display_width`, `pad_or_truncate_to_display_width`)
      - chrome/status composition helpers (`chrome_bar_line`, `keybind_hint_spans`)
      - ANSI text stripping helper (`ansi_line_to_plain_text`)
  - wired `mod text;` + imports in `src/ui/tui/mod.rs`
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5v, extract text selection state into `selection.rs`
- Commit: `0858c4f`
- Changes:
  - added `src/ui/tui/selection.rs`
    - moved `TextSelectionPoint` and `TextSelectionState` structs + impls out of `src/ui/tui/mod.rs`
    - preserved sibling/test access via `pub(super)` visibility for fields and methods used by `view.rs` and tests
  - wired `mod selection;` + imports in `src/ui/tui/mod.rs`
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5w, extract label/timing helpers into `logging.rs`
- Commit: `d16af8c`
- Changes:
  - added `src/ui/tui/logging.rs`
    - moved helper methods from `src/ui/tui/mod.rs`:
      - mode/focus/region labels (`mode_label`, `focus_label`, `focus_name`, `mode_name`, `hit_region_name`)
      - shared timing/message-kind helpers (`duration_millis`, `msg_kind`)
  - methods exposed as `pub(super)` for sibling module callers (`update.rs`, `view.rs`, tests)
  - wired `mod logging;` in `src/ui/tui/mod.rs`
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5x, move transition and toast logging orchestration into `logging.rs`
- Commit: `3cc83bf`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/logging.rs`:
    - `TransitionSnapshot` struct
    - transition event helpers (`capture_transition_snapshot`, `emit_transition_events`)
    - dialog/tmux/toast logging helpers (`log_dialog_event_with_fields`, `log_dialog_event`, `log_tmux_error`, `execute_tmux_command`, `show_toast`)
  - methods exposed as `pub(super)` for sibling callers in `update.rs`, `dialogs.rs`, and tests
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5y, move input trace logging helpers into `logging.rs`
- Commit: `d598eb3`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/logging.rs`:
    - input event + key/action labels (`log_input_event_with_fields`, `interactive_action_kind`, `interactive_key_kind`)
    - pending interactive input queue helpers (`track_pending_interactive_input`, `clear_pending_inputs_for_session`, `clear_pending_sends_for_session`, `drain_pending_inputs_for_session`)
    - pending queue metrics/scheduling (`pending_input_depth`, `oldest_pending_input_age_ms`, `schedule_interactive_debounced_poll`)
  - methods required by sibling modules kept `pub(super)`
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5z, move frame debug logging helpers into `logging.rs`
- Commit: `55612c8`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/logging.rs`:
    - `frame_lines_hash`
    - `frame_buffer_lines`
    - `log_frame_render`
  - `log_frame_render` exposed as `pub(super)` for `view.rs` caller
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5aa, move deferred command/input-seq helpers into `update.rs`
- Commit: `1cb3248`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - `queue_cmd`
    - `merge_deferred_cmds`
    - `next_input_seq`
  - `queue_cmd` exposed as `pub(super)` because `dialogs.rs` queues background tasks
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5ab, move workspace helper methods into `update.rs`
- Commit: `e5686a0`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/update.rs`:
    - `selected_workspace_name`
    - `selected_workspace_path`
    - `workspace_session_name`
  - methods exposed as `pub(super)` for sibling callers (`logging.rs`)
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5ac, move app bootstrap constructors into `bootstrap.rs`
- Commit: `aa91ca5`
- Changes:
  - moved from `src/ui/tui/mod.rs` to `src/ui/tui/bootstrap.rs`:
    - app constructor/bootstrap methods (`new_with_event_logger`, `new_with_debug_recorder`, `from_parts*`)
    - test bootstrap helper (`projects_from_bootstrap`)
  - adjusted visibility to `pub(super)` where sibling modules/tests need access
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5ad, move app path/dependency structs into `bootstrap.rs`
- Commit: `0d9b21d`
- Changes:
  - moved `AppPaths` and `AppDependencies` from `src/ui/tui/mod.rs` to `src/ui/tui/bootstrap.rs`
  - exported via `pub(super)` and kept test construction compatibility (`AppDependencies` fields remain accessible to tests)
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib --quiet` (pass, 276)

### Phase 5ae, move shared TUI constants/types/theme into `shared.rs`
- Commit: `9ce1826`
- Changes:
  - added `src/ui/tui/shared.rs`
    - moved shared constants from `src/ui/tui/mod.rs`:
      - layout + hit IDs
      - command palette action IDs
      - polling/animation/input constants
    - moved shared types/theme from `src/ui/tui/mod.rs`:
      - `UiTheme` + `ui_theme`
      - `HitRegion`, `PreviewTab`, `ViewLayout`
      - cursor/preview/input structs (`CursorMetadata`, `PreviewContentViewport`, `InputTraceContext`, pending interactive structs)
  - updated `src/ui/tui/mod.rs`:
    - wired `mod shared;` + `use shared::*;`
    - removed moved definitions
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 5af, move TUI run entrypoints into `runner.rs`
- Commit: `ea873d9`
- Changes:
  - added `src/ui/tui/runner.rs`
    - moved run entrypoint functions from `src/ui/tui/mod.rs`:
      - `run`
      - `run_with_event_log`
      - `run_with_debug_record`
      - internal `run_with_logger`
  - updated `src/ui/tui/mod.rs`:
    - wired `mod runner;`
    - re-exported public entrypoints with `pub use runner::{run, run_with_debug_record, run_with_event_log};`
    - removed now-unused imports related to runner internals
  - no behavior changes, relocation only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 276)

### Phase 6a, move git-tab session naming helper to `agent_runtime`
- Commit: `3366bf3`
- Changes:
  - added `git_session_name_for_workspace(&Workspace) -> String` to `src/agent_runtime.rs`
    - centralizes `"{workspace_session}-git"` naming alongside other session-name helpers
  - updated UI callers in `src/ui/tui/update.rs` and `src/ui/tui/view.rs`
    - removed local `git_tab_session_name` method from TUI update logic
    - switched git tab lazygit/session lookup callsites to `git_session_name_for_workspace`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `git_session_name_uses_project_context_when_present`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 277)

### Phase 6b, centralize workspace status poll policy in `agent_runtime`
- Commit: `e4859a5`
- Changes:
  - added `workspace_should_poll_status(&Workspace, MultiplexerKind) -> bool` in `src/agent_runtime.rs`
    - moved multiplexer-specific poll eligibility policy out of `ui/tui`
  - updated `src/ui/tui/update.rs` to use runtime helper inside `workspace_status_poll_targets`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `workspace_status_poll_policy_requires_supported_agent_for_all_multiplexers`
    - `workspace_status_poll_policy_differs_between_tmux_and_zellij_for_idle_non_main`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 279)

### Phase 6c, move tmux missing-session error classification to `agent_runtime`
- Commit: `3b365f0`
- Changes:
  - added `tmux_capture_error_indicates_missing_session(&str) -> bool` in `src/agent_runtime.rs`
  - removed duplicate helper from `src/ui/tui/update.rs`
  - updated TUI call sites to use runtime helper in:
    - live preview capture error handling
    - background workspace status capture error handling
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `tmux_missing_session_error_detection_matches_known_patterns`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 280)

### Phase 6d, add `Workspace`-ref session name helper in `agent_runtime`
- Commit: `87bb133`
- Changes:
  - added `session_name_for_workspace_ref(&Workspace) -> String` in `src/agent_runtime.rs`
    - keeps project-aware session naming in runtime boundary
  - updated `git_session_name_for_workspace` to compose from new helper
  - updated `src/ui/tui/update.rs` workspace session wrapper to call runtime helper
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `session_name_for_workspace_ref_uses_project_context_when_present`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 281)

### Phase 6e, move live-preview session/target selection helpers to `agent_runtime`
- Commit: `19b5ecd`
- Changes:
  - added runtime helpers in `src/agent_runtime.rs`:
    - `live_preview_agent_session(Option<&Workspace>) -> Option<String>`
    - `workspace_status_session_target(&Workspace, MultiplexerKind, Option<&str>) -> Option<String>`
  - updated UI callers in `src/ui/tui/update.rs`:
    - `selected_session_for_live_preview` now delegates agent-tab session selection to runtime helper
    - `workspace_status_poll_targets` now delegates per-workspace target selection/filtering to runtime helper
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `live_preview_agent_session_requires_live_workspace_session`
    - `workspace_status_session_target_skips_selected_live_session`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 36)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 283)

### Phase 6f, remove UI wrapper for workspace session naming
- Commit: `a9c50a6`
- Changes:
  - removed `workspace_session_name` wrapper from `src/ui/tui/update.rs`
  - replaced call sites with direct runtime boundary call `session_name_for_workspace_ref(workspace)`
  - no behavior changes, ownership cleanup only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6g, centralize interactive-entry eligibility policy in `agent_runtime`
- Commit: `5bfd1a4`
- Changes:
  - added `workspace_can_enter_interactive(Option<&Workspace>, bool) -> bool` in `src/agent_runtime.rs`
    - runtime now owns git-tab vs agent-session eligibility policy for interactive entry
  - updated UI callers in `src/ui/tui/update.rs`:
    - `can_enter_interactive` now delegates to runtime helper
    - agent-tab path in `enter_interactive` now uses `live_preview_agent_session`
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `workspace_can_enter_interactive_depends_on_preview_tab_mode`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 37)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6h, move delete-session command construction to `agent_runtime`
- Commit: `af85ec7`
- Changes:
  - added `kill_workspace_session_command(Option<&str>, &str, MultiplexerKind) -> Vec<String>` in `src/agent_runtime.rs`
    - centralizes workspace session-name construction + multiplexer-specific kill command shape
  - updated `src/ui/tui/update.rs`:
    - `run_delete_workspace` now delegates kill command construction to runtime helper
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `kill_workspace_session_command_uses_project_scoped_tmux_session_name`
    - `kill_workspace_session_command_uses_zellij_config_for_zellij`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 39)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6i, move workspace status poll target construction to `agent_runtime`
- Commit: `6136c11`
- Changes:
  - added runtime struct + helper in `src/agent_runtime.rs`:
    - `WorkspaceStatusTarget`
    - `workspace_status_targets_for_polling(&[Workspace], MultiplexerKind, Option<&str>) -> Vec<WorkspaceStatusTarget>`
  - updated UI caller in `src/ui/tui/update.rs`:
    - `workspace_status_poll_targets` now maps runtime targets into UI message target structs
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `workspace_status_targets_for_polling_skip_selected_session`
    - `workspace_status_targets_for_polling_include_idle_non_main_for_zellij`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 41)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6j, dedupe poll-target type by aliasing UI to runtime target
- Commit: `4d6b323`
- Changes:
  - updated `src/ui/tui/msg.rs`:
    - replaced local `WorkspaceStatusPollTarget` struct with type alias to runtime `WorkspaceStatusTarget`
  - updated `src/ui/tui/update.rs`:
    - removed now-redundant mapping shim in `workspace_status_poll_targets`
    - method now returns runtime poll targets directly
  - no behavior changes, type ownership/boundary cleanup only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 288)

### Phase 6k, move preview-tab session target resolution to `agent_runtime`
- Commit: `28a1c3d`
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `workspace_session_for_preview_tab(Option<&Workspace>, bool, Option<&str>) -> Option<String>`
  - updated UI callers in `src/ui/tui/update.rs`:
    - `selected_session_for_live_preview` now delegates final session resolution to runtime helper
    - `enter_interactive` now delegates final session resolution to runtime helper
    - git-tab lazygit readiness/launch behavior remains unchanged in UI
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `workspace_session_for_preview_tab_respects_preview_tab_mode`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 42)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6l, move git preview readiness policy to `agent_runtime`
- Commit: `7e3f610`
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `git_preview_session_if_ready(Option<&Workspace>, &HashSet<String>) -> Option<String>`
  - updated UI caller in `src/ui/tui/update.rs`:
    - `selected_session_for_live_preview` now delegates git-session readiness resolution to runtime helper
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `git_preview_session_if_ready_requires_matching_ready_session`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 43)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6m, centralize live-preview tab session policy in `agent_runtime`
- Commit: `0da752e`
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `live_preview_session_for_tab(Option<&Workspace>, bool, &HashSet<String>) -> Option<String>`
  - updated UI caller in `src/ui/tui/update.rs`:
    - `selected_session_for_live_preview` now delegates tab-mode session selection entirely to runtime helper
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `live_preview_session_for_tab_uses_git_or_agent_policy`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 44)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6n, centralize live-preview capture target policy in `agent_runtime`
- Commit: `843f3c2`
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `live_preview_capture_target_for_tab(Option<&Workspace>, bool, &HashSet<String>) -> Option<(String, bool)>`
  - updated UI caller in `src/ui/tui/update.rs`:
    - `selected_session_for_live_preview` now delegates both session selection and capture-mode policy to runtime helper
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `live_preview_capture_target_for_tab_sets_capture_mode`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 45)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6o, introduce typed live-preview poll target in `agent_runtime`
- Commit: `a3a13bf`
- Changes:
  - added runtime type in `src/agent_runtime.rs`:
    - `LivePreviewTarget { session_name, include_escape_sequences }`
  - updated runtime helper in `src/agent_runtime.rs`:
    - `live_preview_capture_target_for_tab` now returns `Option<LivePreviewTarget>`
  - updated UI callers in `src/ui/tui/update.rs`:
    - `selected_session_for_live_preview`, `prepare_live_preview_session`, sync/async poll wiring, and interactive-entry git path now use `LivePreviewTarget` instead of tuple plumbing
  - updated runtime imports in `src/ui/tui/mod.rs`
  - updated focused runtime test in `src/agent_runtime/tests.rs`:
    - `live_preview_capture_target_for_tab_sets_capture_mode` now asserts typed target
  - no behavior changes, type ownership/boundary cleanup only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 45)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6p, move live-preview exclusion glue for status polling to `agent_runtime`
- Commit: `16ead82`
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `workspace_status_targets_for_polling_with_live_preview(&[Workspace], MultiplexerKind, Option<&LivePreviewTarget>) -> Vec<WorkspaceStatusTarget>`
  - updated UI caller in `src/ui/tui/update.rs`:
    - `workspace_status_poll_targets` now accepts `Option<&LivePreviewTarget>` and delegates selected-live-session exclusion to runtime helper
    - sync/async poll callsites now pass `live_preview.as_ref()` directly (no UI-side session-name extraction)
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `workspace_status_targets_for_polling_with_live_preview_skips_selected_session`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 46)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6q, inline live-preview session shim in UI prepare path
- Commit: `ce477d5`
- Changes:
  - removed `selected_session_for_live_preview` wrapper from `src/ui/tui/update.rs`
  - `prepare_live_preview_session` now directly calls runtime helper for non-git tab
  - no behavior changes, UI shim cleanup only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6r, remove status-poll wrapper in UI update flow
- Commit: `0a46446`
- Changes:
  - removed `workspace_status_poll_targets` wrapper from `src/ui/tui/update.rs`
  - updated sync and async poll callsites in `src/ui/tui/update.rs` to call
    `workspace_status_targets_for_polling_with_live_preview` directly
  - updated wrapper-oriented TUI tests in `src/ui/tui/tests/mod.rs`:
    - `zellij_workspace_status_poll_targets_include_idle_workspaces`
    - `tmux_workspace_status_poll_targets_skip_idle_workspaces`
    - both now assert runtime helper output directly
  - no behavior changes, UI shim cleanup only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6s, centralize workspace start eligibility policy in `agent_runtime`
- Commit: `d3cba15`
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `workspace_can_start_agent(Option<&Workspace>) -> bool`
  - updated UI caller in `src/ui/tui/update.rs`:
    - `can_start_selected_workspace` now delegates status/support policy to runtime helper and keeps only `start_in_flight` UI gating
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `workspace_can_start_agent_depends_on_status_and_support`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 47)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6t, centralize workspace stop eligibility policy in `agent_runtime`
- Commit: `c61c1da`
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `workspace_can_stop_agent(Option<&Workspace>) -> bool`
  - updated UI caller in `src/ui/tui/update.rs`:
    - `can_stop_selected_workspace` now delegates session-status policy to runtime helper and keeps only `stop_in_flight` UI gating
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `workspace_can_stop_agent_depends_on_session_status`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 48)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6u, inline stop-policy UI wrapper callsites
- Commit: `2b37bac`
- Changes:
  - removed `can_stop_selected_workspace` wrapper from `src/ui/tui/update.rs`
  - updated callsites in `src/ui/tui/update.rs`:
    - command-palette action inclusion now directly checks `workspace_can_stop_agent(...)` plus local `stop_in_flight` guard
    - `stop_selected_workspace_agent` now directly checks `workspace_can_stop_agent(...)`
  - no behavior changes, UI shim cleanup only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6v, inline start-policy UI wrapper callsites
- Commit: `824ffce`
- Changes:
  - removed `can_start_selected_workspace` wrapper from `src/ui/tui/update.rs`
  - updated callsites in `src/ui/tui/update.rs` and `src/ui/tui/dialogs.rs`:
    - command-palette action inclusion now directly checks `workspace_can_start_agent(...)` plus local `start_in_flight` guard
    - start flow now directly checks `workspace_can_start_agent(...)`
  - no behavior changes, UI shim cleanup only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6w, inline interactive-entry policy UI wrapper callsites
- Commit: `a5f5bff`
- Changes:
  - removed `can_enter_interactive` wrapper from `src/ui/tui/update.rs`
  - updated callsites in `src/ui/tui/update.rs` to call
    `workspace_can_enter_interactive(...)` directly in:
    - command-palette action inclusion
    - `enter_interactive` guard
  - no behavior changes, UI shim cleanup only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6x, move delete-workspace execution into `workspace_lifecycle`
- Commit: `e6ffffc`
- Changes:
  - added delete lifecycle boundary in `src/workspace_lifecycle.rs`:
    - `DeleteWorkspaceRequest`
    - `delete_workspace(DeleteWorkspaceRequest, MultiplexerKind) -> (Result<(), String>, Vec<String>)`
    - private git/command execution helpers used by delete flow
  - updated UI caller in `src/ui/tui/dialogs.rs`:
    - `confirm_delete_dialog` now builds `DeleteWorkspaceRequest` and delegates delete execution to `workspace_lifecycle::delete_workspace`
  - removed delete execution helpers from `src/ui/tui/update.rs`:
    - `run_delete_workspace`
    - `run_delete_worktree_git`
    - `run_delete_local_branch_git`
    - `run_git_command`
  - updated module imports in `src/ui/tui/mod.rs`
  - added focused lifecycle tests in `src/workspace_lifecycle/tests.rs`:
    - `delete_workspace_prunes_missing_worktree`
    - `delete_workspace_records_branch_delete_failure_as_warning`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib workspace_lifecycle::tests -- --nocapture` (pass, 12)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 297)

### Phase 6y, move lifecycle error message mapping into `workspace_lifecycle`
- Commit: `ee13b44`
- Changes:
  - added `workspace_lifecycle_error_message(&WorkspaceLifecycleError) -> String` in `src/workspace_lifecycle.rs`
  - updated UI callsites to use lifecycle boundary helper:
    - `src/ui/tui/update.rs` create-request validation and create-completion error toasts
    - `src/ui/tui/dialogs.rs` edit-dialog save error toast
  - removed `workspace_lifecycle_error_message` helper from `src/ui/tui/update.rs`
  - updated module imports in `src/ui/tui/mod.rs`
  - added focused lifecycle test in `src/workspace_lifecycle/tests.rs`:
    - `workspace_lifecycle_error_messages_are_user_friendly`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib workspace_lifecycle::tests -- --nocapture` (pass, 13)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 298)

### Phase 6z, move launch/stop command execution helpers into `agent_runtime`
- Commit: `34e7784`
- Changes:
  - added runtime execution helpers in `src/agent_runtime.rs`:
    - `execute_launch_plan(LaunchPlan) -> std::io::Result<()>`
    - `execute_commands(&[Vec<String>]) -> std::io::Result<()>`
    - private command execution helpers for stderr/status shaping
  - updated UI caller in `src/ui/tui/update.rs`:
    - background start flow now delegates to `execute_launch_plan`
    - background stop flow now delegates to `execute_commands`
  - removed UI-only helpers from `src/ui/tui/update.rs`:
    - `run_start_agent_plan`
    - `run_stop_commands`
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_commands_runs_successful_command_sequence`
    - `execute_commands_returns_error_for_missing_program`
    - `execute_launch_plan_writes_launcher_script_and_executes_commands`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 51)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 301)

### Phase 6aa, extract sync-mode start/stop command execution helpers from `update.rs`
- Commit: `5044871`
- Changes:
  - added shared infrastructure helpers on `GroveApp` in `src/ui/tui/logging.rs`:
    - `execute_tmux_commands(&[Vec<String>]) -> std::io::Result<()>`
    - `execute_launch_plan_sync(&LaunchPlan) -> std::io::Result<()>`
  - updated sync callsites in `src/ui/tui/update.rs`:
    - start flow now delegates sync launch execution to `execute_launch_plan_sync`
    - stop flow now delegates sync command sequence execution to `execute_tmux_commands`
  - removed inline sync start/stop execution loops from `src/ui/tui/update.rs`
  - no behavior changes, infrastructure extraction only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 301)

### Phase 6ab, reuse shared sync launch executor for lazygit startup path
- Commit: `1325e8f`
- Changes:
  - updated `ensure_lazygit_session_for_selected_workspace` in `src/ui/tui/update.rs` to delegate launch-plan execution to `execute_launch_plan_sync` instead of inlining script/prelaunch/launch command steps
  - adjusted `execute_launch_plan_sync` in `src/ui/tui/logging.rs` to preserve prior script-write error message prefix (`launcher script write failed: ...`)
  - no behavior changes, duplicated sync launch execution removal only
- Gates:
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 301)

### Phase 6ac, move sync command/launch sequencing into `agent_runtime`
- Commit: `2d637bc` (consolidated with phases 6ad/6ae)
- Changes:
  - added runtime helpers in `src/agent_runtime.rs`:
    - `execute_commands_with(&[Vec<String>], impl FnMut(&[String]) -> std::io::Result<()>)`
    - `execute_launch_plan_with(&LaunchPlan, impl FnMut(&[String]) -> std::io::Result<()>)`
  - updated sync UI callsites in `src/ui/tui/update.rs`:
    - lazygit launch sync path now uses `execute_launch_plan_with(..., |cmd| self.execute_tmux_command(cmd))`
    - agent start sync path now uses `execute_launch_plan_with(..., |cmd| self.execute_tmux_command(cmd))`
    - agent stop sync path now uses `execute_commands_with(..., |cmd| self.execute_tmux_command(cmd))`
  - removed now-redundant UI helpers from `src/ui/tui/logging.rs`:
    - `execute_tmux_commands`
    - `execute_launch_plan_sync`
  - updated runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_commands_with_uses_supplied_executor`
    - `execute_launch_plan_with_prefixes_script_write_errors`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 53)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 303)

### Phase 6ad, move single-command sync execution through runtime boundary
- Commit: `2d637bc` (consolidated with phases 6ac/6ae)
- Changes:
  - added runtime helper in `src/agent_runtime.rs`:
    - `execute_command_with(&[String], impl FnOnce(&[String]) -> std::io::Result<()>)`
  - updated runtime helper composition in `src/agent_runtime.rs`:
    - `execute_commands_with` now delegates each command through `execute_command_with`
    - `execute_launch_plan_with` now delegates final launch command through `execute_command_with`
  - updated UI command execution path in `src/ui/tui/logging.rs`:
    - `execute_tmux_command` now delegates command invocation to `execute_command_with`, keeping UI logging/toast policy unchanged
  - updated runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_command_with_skips_empty_commands`
    - `execute_command_with_invokes_executor_for_non_empty_commands`
  - no behavior changes, ownership/boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 55)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 305)

### Phase 6ae, dedupe runtime launch executors behind shared internal helper
- Commit: `2d637bc` (consolidated with phases 6ac/6ad)
- Changes:
  - added internal runtime helper in `src/agent_runtime.rs`:
    - `execute_launch_plan_internal(&LaunchPlan, write_launcher_script, execute)`
  - updated runtime public executors in `src/agent_runtime.rs`:
    - `execute_launch_plan` now delegates to shared helper while preserving unprefixed script-write errors
    - `execute_launch_plan_with` now delegates to shared helper while preserving prefixed script-write errors (`launcher script write failed: ...`)
  - updated runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_launch_plan_keeps_unprefixed_script_write_errors`
  - no behavior changes, duplication cleanup only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 56)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 306)

### Phase 6af, introduce runtime command-executor abstraction
- Commit: `bc28049`
- Changes:
  - added runtime abstraction in `src/agent_runtime.rs`:
    - `CommandExecutor` trait with command + launcher-script execution hooks
    - `ProcessCommandExecutor`
    - `DelegatingCommandExecutor`
    - shared execution entrypoints:
      - `execute_commands_with_executor`
      - `execute_launch_plan_with_executor`
  - refactored runtime execution wrappers in `src/agent_runtime.rs`:
    - `execute_commands` and `execute_launch_plan` now use `ProcessCommandExecutor`
    - `execute_commands_with` and `execute_launch_plan_with` now use `DelegatingCommandExecutor`
    - preserved script-write error behavior:
      - background path stays unprefixed
      - delegated path stays prefixed (`launcher script write failed: ...`)
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_commands_with_executor_skips_empty_commands`
    - `execute_launch_plan_with_executor_runs_prelaunch_then_launch`
  - no behavior changes, boundary abstraction only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 58)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 308)

### Phase 6ag, centralize runtime execution result shaping helpers
- Commit: `2f45daf`
- Changes:
  - added runtime result helpers in `src/agent_runtime.rs`:
    - `execute_launch_plan_result(LaunchPlan) -> Result<(), String>`
    - `execute_launch_plan_with_result(&LaunchPlan, impl FnMut(&[String]) -> std::io::Result<()>) -> Result<(), String>`
    - `execute_commands_result(&[Vec<String>]) -> Result<(), String>`
    - `execute_commands_with_result(&[Vec<String>], impl FnMut(&[String]) -> std::io::Result<()>) -> Result<(), String>`
  - updated UI callers in `src/ui/tui/update.rs`:
    - sync launch/stop paths now use `*_with_result` helpers directly (no local `to_string` mapping)
    - background launch/stop tasks now use `*_result` helpers directly (no local `map_err` mapping)
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_commands_result_returns_string_errors`
    - `execute_commands_with_result_returns_string_errors`
    - `execute_launch_plan_with_result_prefixes_script_write_errors`
    - `execute_launch_plan_result_keeps_unprefixed_script_write_errors`
  - no behavior changes, boundary/result-shaping move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 62)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 312)

### Phase 6ah, add runtime dispatch helper for sync vs process execution
- Commit: `dec61b5`
- Changes:
  - added runtime dispatch type + helpers in `src/agent_runtime.rs`:
    - `CommandExecutionMode::{Process, Delegating}`
    - `execute_launch_plan_for_mode(&LaunchPlan, CommandExecutionMode) -> Result<(), String>`
    - `execute_commands_for_mode(&[Vec<String>], CommandExecutionMode) -> Result<(), String>`
  - refactored runtime result wrappers in `src/agent_runtime.rs`:
    - `execute_launch_plan_result`, `execute_launch_plan_with_result`, `execute_commands_result`, and `execute_commands_with_result` now delegate through mode-based dispatch
  - updated UI callsites in `src/ui/tui/update.rs`:
    - sync and background start/stop/lazygit execution paths now call mode-based runtime dispatch directly
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_commands_for_mode_process_returns_string_errors`
    - `execute_launch_plan_for_mode_delegating_prefixes_script_write_errors`
  - no behavior changes, dispatch-boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 64)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 314)

### Phase 6ai, collapse runtime execution API surface to mode entrypoints
- Commit: `f977236`
- Changes:
  - removed redundant runtime wrappers in `src/agent_runtime.rs`:
    - `execute_launch_plan_result`
    - `execute_launch_plan_with_result`
    - `execute_commands_result`
    - `execute_commands_with_result`
  - retained mode-based and executor-based entrypoints as preferred runtime execution API
  - updated runtime tests in `src/agent_runtime/tests.rs`:
    - removed coverage tied to removed wrapper functions
    - kept/expanded mode-based coverage (`execute_commands_for_mode_process_returns_string_errors`, `execute_launch_plan_for_mode_delegating_prefixes_script_write_errors`)
  - no behavior changes, API-surface cleanup only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 60)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 310)

### Phase 6aj, add runtime lifecycle execution facade helpers
- Commit: `3eddb24`
- Changes:
  - added runtime lifecycle facade helpers in `src/agent_runtime.rs`:
    - `execute_launch_request_for_mode(&LaunchRequest, MultiplexerKind, CommandExecutionMode) -> (String, Result<(), String>)`
    - `execute_shell_launch_request_for_mode(&ShellLaunchRequest, MultiplexerKind, CommandExecutionMode) -> (String, Result<(), String>)`
    - `execute_stop_session_for_mode(&str, MultiplexerKind, CommandExecutionMode) -> Result<(), String>`
  - updated UI callsites in `src/ui/tui/update.rs`:
    - lazygit launch path now executes from shell launch request through runtime facade
    - agent start sync/background paths now execute from launch request through runtime facade
    - agent stop sync/background paths now execute from session name through runtime facade
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `execute_launch_request_for_mode_returns_session_name_on_error`
    - `execute_stop_session_for_mode_delegating_runs_stop_sequence`
  - no behavior changes, runtime lifecycle orchestration boundary move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 62)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)
  - `cargo test --lib` (pass, 312)

### Phase 6ak, move launch request construction into `agent_runtime`
- Commit: `d9ef81a`
- Changes:
  - added runtime request-builder helpers in `src/agent_runtime.rs`:
    - `launch_request_for_workspace(&Workspace, Option<String>, Option<String>, bool, Option<u16>, Option<u16>) -> LaunchRequest`
    - `shell_launch_request_for_workspace(&Workspace, String, String, Option<u16>, Option<u16>) -> ShellLaunchRequest`
  - updated UI callsites in `src/ui/tui/update.rs`:
    - `start_selected_workspace_agent_with_options` now builds `LaunchRequest` through runtime helper
    - `ensure_lazygit_session_for_selected_workspace` now builds `ShellLaunchRequest` through runtime helper
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime tests in `src/agent_runtime/tests.rs`:
    - `launch_request_for_workspace_copies_workspace_context_and_options`
    - `shell_launch_request_for_workspace_uses_workspace_path_and_options`
  - no behavior changes, request-shaping ownership move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 64)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6al, move stop-workspace execution session resolution to `agent_runtime`
- Commit: `6a0da9e`
- Changes:
  - added runtime lifecycle helper in `src/agent_runtime.rs`:
    - `execute_stop_workspace_for_mode(&Workspace, MultiplexerKind, CommandExecutionMode) -> (String, Result<(), String>)`
  - updated UI callsites in `src/ui/tui/update.rs`:
    - `stop_selected_workspace_agent` now delegates session-name resolution + stop execution to runtime helper for both sync and background paths
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `execute_stop_workspace_for_mode_uses_project_scoped_session_name`
  - no behavior changes, stop-lifecycle ownership move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 65)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6am, add typed launch execution outcome in `agent_runtime`
- Commit: `1ca52aa`
- Changes:
  - added runtime outcome type and helper in `src/agent_runtime.rs`:
    - `LaunchExecutionResult { workspace_name, workspace_path, session_name, result }`
    - `execute_launch_request_with_result_for_mode(&LaunchRequest, MultiplexerKind, CommandExecutionMode) -> LaunchExecutionResult`
  - updated UI callsites in `src/ui/tui/update.rs`:
    - `start_selected_workspace_agent_with_options` now uses typed runtime launch outcome for both sync and background paths
    - added local mapper `start_agent_completion_from_runtime` for `Msg::StartAgentCompleted` payload shaping
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `execute_launch_request_with_result_for_mode_includes_workspace_context`
  - no behavior changes, launch lifecycle payload ownership move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 66)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6an, add typed stop execution outcome in `agent_runtime`
- Commit: `286d266`
- Changes:
  - added runtime outcome type and helper in `src/agent_runtime.rs`:
    - `StopExecutionResult { workspace_name, workspace_path, session_name, result }`
    - `execute_stop_workspace_with_result_for_mode(&Workspace, MultiplexerKind, CommandExecutionMode) -> StopExecutionResult`
  - updated UI callsites in `src/ui/tui/update.rs`:
    - `stop_selected_workspace_agent` now uses typed runtime stop outcome for both sync and background paths
    - added local mapper `stop_agent_completion_from_runtime` for `Msg::StopAgentCompleted` payload shaping
  - updated runtime imports in `src/ui/tui/mod.rs`
  - added focused runtime test in `src/agent_runtime/tests.rs`:
    - `execute_stop_workspace_with_result_for_mode_includes_workspace_context`
  - no behavior changes, stop lifecycle payload ownership move only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 67)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6ao, unify typed lifecycle outcomes in `agent_runtime`
- Commit: `ad254f6`
- Changes:
  - consolidated runtime lifecycle outcome shapes in `src/agent_runtime.rs`:
    - removed duplicated `LaunchExecutionResult` and `StopExecutionResult`
    - introduced shared `SessionExecutionResult { workspace_name, workspace_path, session_name, result }`
  - updated runtime helpers in `src/agent_runtime.rs`:
    - `execute_launch_request_with_result_for_mode` now returns `SessionExecutionResult`
    - `execute_stop_workspace_with_result_for_mode` now returns `SessionExecutionResult`
  - updated UI runtime mappers in `src/ui/tui/update.rs`:
    - `start_agent_completion_from_runtime` and `stop_agent_completion_from_runtime` now accept shared `SessionExecutionResult`
  - updated runtime imports in `src/ui/tui/mod.rs`
  - no behavior changes, lifecycle outcome type deduplication only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 67)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6ap, move lifecycle outcome conversion into `msg.rs`
- Commit: `d80e32f`
- Changes:
  - added conversion impls in `src/ui/tui/msg.rs`:
    - `impl From<SessionExecutionResult> for StartAgentCompletion`
    - `impl From<SessionExecutionResult> for StopAgentCompletion`
  - updated UI callsites in `src/ui/tui/update.rs`:
    - start/stop sync + background flows now use `completion.into()` at message/completion boundaries
    - removed local conversion helpers from `update.rs`
  - cleaned runtime import list in `src/ui/tui/mod.rs`
  - no behavior changes, UI conversion glue consolidation only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 67)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6aq, scope tuple lifecycle executors to `agent_runtime`
- Commit: `2085f6a`
- Changes:
  - narrowed runtime helper visibility in `src/agent_runtime.rs`:
    - `execute_launch_request_for_mode` from `pub` to private `fn`
    - `execute_stop_session_for_mode` from `pub` to private `fn`
    - `execute_stop_workspace_for_mode` from `pub` to private `fn`
  - kept typed runtime lifecycle entrypoints public (`execute_*_with_result_for_mode`) as UI boundary API
  - no behavior changes, runtime API-surface cleanup only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 67)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 6ar, inline typed lifecycle execution paths in `agent_runtime`
- Commit: `5ce9c52`
- Changes:
  - removed now-redundant private tuple helpers in `src/agent_runtime.rs`:
    - `execute_launch_request_for_mode`
    - `execute_stop_session_for_mode`
    - `execute_stop_workspace_for_mode`
  - inlined lifecycle execution logic directly into typed helpers:
    - `execute_launch_request_with_result_for_mode`
    - `execute_stop_workspace_with_result_for_mode`
  - updated runtime tests in `src/agent_runtime/tests.rs`:
    - removed coverage tied to deleted tuple helpers
    - expanded typed stop helper test to assert full command sequence and project-scoped session
  - no behavior changes, internal call-depth/API cleanup only
- Gates:
  - `cargo test --lib agent_runtime::tests -- --nocapture` (pass, 64)
  - `cargo test --lib ui::tui::tests -- --nocapture` (pass, 180)

### Phase 7a, move `agent_runtime` module under `src/application/`
- Commit: `aa8e75d`
- Changes:
  - moved module files:
    - `src/agent_runtime.rs` -> `src/application/agent_runtime.rs`
    - `src/agent_runtime/tests.rs` -> `src/application/agent_runtime/tests.rs`
  - added `src/application/mod.rs` with `pub mod agent_runtime;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod agent_runtime;`
    - added `pub mod application;`
  - updated imports across crate from `crate::agent_runtime::...` to `crate::application::agent_runtime::...`
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7b, move `workspace_lifecycle` module under `src/application/`
- Commit: `e5e4e9a`
- Changes:
  - moved module files:
    - `src/workspace_lifecycle.rs` -> `src/application/workspace_lifecycle.rs`
    - `src/workspace_lifecycle/tests.rs` -> `src/application/workspace_lifecycle/tests.rs`
  - updated `src/application/mod.rs` with `pub mod workspace_lifecycle;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod workspace_lifecycle;`
  - updated imports across crate from `crate::workspace_lifecycle::...` to `crate::application::workspace_lifecycle::...`
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7c, move `zellij_emulator` module under `src/infrastructure/`
- Commit: `ff8b08e`
- Changes:
  - moved module files:
    - `src/zellij_emulator.rs` -> `src/infrastructure/zellij_emulator.rs`
    - `src/zellij_emulator/tests.rs` -> `src/infrastructure/zellij_emulator/tests.rs`
  - added `src/infrastructure/mod.rs` with `pub mod zellij_emulator;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod zellij_emulator;`
    - added `pub mod infrastructure;`
  - updated imports from `crate::zellij_emulator::...` to `crate::infrastructure::zellij_emulator::...`
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7d, move `preview` module under `src/application/`
- Commit: `941ef5c`
- Changes:
  - moved module files:
    - `src/preview.rs` -> `src/application/preview.rs`
    - `src/preview/tests.rs` -> `src/application/preview/tests.rs`
  - updated `src/application/mod.rs` with `pub mod preview;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod preview;`
  - updated imports from `crate::preview::...` to `crate::application::preview::...`
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7e, move `interactive` module under `src/application/`
- Commit: `a969cee`
- Changes:
  - moved module files:
    - `src/interactive.rs` -> `src/application/interactive.rs`
    - `src/interactive/tests.rs` -> `src/application/interactive/tests.rs`
  - updated `src/application/mod.rs` with `pub mod interactive;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod interactive;`
  - updated imports from `crate::interactive::...` to `crate::application::interactive::...`
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7f, move `mouse` module under `src/ui/`
- Commit: `2a4dc5d`
- Changes:
  - moved module files:
    - `src/mouse.rs` -> `src/ui/mouse.rs`
    - `src/mouse/tests.rs` -> `src/ui/mouse/tests.rs`
  - updated `src/ui/mod.rs` with `mod mouse;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod mouse;`
  - updated imports from `crate::mouse::...` to `crate::ui::mouse::...` in TUI modules
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7g, move `state` module under `src/ui/`
- Commit: `5baa43d`
- Changes:
  - moved module files:
    - `src/state.rs` -> `src/ui/state.rs`
    - `src/state/tests.rs` -> `src/ui/state/tests.rs`
  - updated `src/ui/mod.rs` with `mod state;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod state;`
  - updated imports from `crate::state::...` to `crate::ui::state::...` in TUI modules and tests
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7h, move `adapters` module under `src/infrastructure/`
- Commit: `696ff76`
- Changes:
  - moved module files:
    - `src/adapters.rs` -> `src/infrastructure/adapters.rs`
    - `src/adapters/tests.rs` -> `src/infrastructure/adapters/tests.rs`
  - updated `src/infrastructure/mod.rs` with `pub mod adapters;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod adapters;`
  - updated imports from `crate::adapters::...` to `crate::infrastructure::adapters::...` in TUI modules and tests
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7i, move `config` module under `src/infrastructure/`
- Commit: `9d08f17`
- Changes:
  - moved module files:
    - `src/config.rs` -> `src/infrastructure/config.rs`
    - `src/config/tests.rs` -> `src/infrastructure/config/tests.rs`
  - updated `src/infrastructure/mod.rs` with `pub mod config;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod config;`
  - updated imports from `crate::config::...` to `crate::infrastructure::config::...` across application, infrastructure, and UI modules/tests
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7j, move `event_log` module under `src/infrastructure/`
- Commit: `c120bc9`
- Changes:
  - moved module files:
    - `src/event_log.rs` -> `src/infrastructure/event_log.rs`
    - `src/event_log/tests.rs` -> `src/infrastructure/event_log/tests.rs`
  - updated `src/infrastructure/mod.rs` with `pub mod event_log;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod event_log;`
  - updated imports from `crate::event_log::...` to `crate::infrastructure::event_log::...` in TUI modules and tests
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7k, move `hardening` module under `src/application/`
- Commit: `67bb3a9`
- Changes:
  - moved module files:
    - `src/hardening.rs` -> `src/application/hardening.rs`
    - `src/hardening/tests.rs` -> `src/application/hardening/tests.rs`
  - updated `src/application/mod.rs` with `pub mod hardening;`
  - updated crate root in `src/lib.rs`:
    - removed `pub mod hardening;`
  - no behavior changes, crate-tree alignment move only
- Gates:
  - `cargo test --lib` (pass, 314)

### Phase 7l, move `domain` module to directory layout
- Commit: `1d93a36`
- Changes:
  - moved module file:
    - `src/domain.rs` -> `src/domain/mod.rs`
  - no behavior changes, crate-tree shape alignment only
- Gates:
  - `cargo test --lib` (pass, 314)
  - `cargo test --bin grove` (pass, 4)

### Phase 8a, remove transitional `tui` shim and stale root import usage
- Commit: `3e0ea40`
- Changes:
  - removed transitional shim:
    - deleted `src/tui.rs`
    - removed `pub mod tui;` from `src/lib.rs`
  - updated run helper wrappers in `src/lib.rs` to call `ui::tui::...` directly
  - updated `src/main.rs` event-log reference to `grove::infrastructure::event_log::FileEventLogger`
  - cleaned `src/ui/tui/mod.rs` bootstrap imports to avoid non-test unused import warnings
  - no behavior changes, cleanup-only
- Gates:
  - `cargo test --lib` (pass, 314)
  - `cargo test --bin grove` (pass, 4)

## Current State
- Worktree is clean after phase 8a and 7l commits.
- Recent refactor commits on local `master`:
  - `1d93a36` refactor(domain): move domain module to directory layout
  - `3e0ea40` refactor(cleanup): remove tui shim module
  - `bd0876c` docs(handoff): record phase 7k
  - `67bb3a9` refactor(application): move hardening module under application
  - `824a05a` docs(handoff): record phase 7j
  - `c120bc9` refactor(infrastructure): move event_log module under infrastructure
  - `50a803f` docs(handoff): record phase 7i
  - `9d08f17` refactor(infrastructure): move config module under infrastructure
  - `bc50a64` docs(handoff): record phase 7h
  - `696ff76` refactor(infrastructure): move adapters module under infrastructure
  - `a88d3f4` docs(handoff): record phase 7g
  - `5baa43d` refactor(ui): move state module under ui
  - `2ff2ce2` docs(handoff): record phase 7f
  - `2a4dc5d` refactor(ui): move mouse module under ui
  - `d213859` docs(handoff): record phase 7e
  - `a969cee` refactor(application): move interactive module under application
  - `262dfef` docs(handoff): record phase 7d
  - `941ef5c` refactor(application): move preview module under application
  - `05f505d` docs(handoff): record phase 7c
  - `ff8b08e` refactor(infrastructure): move zellij emulator under infrastructure
  - `35d04a8` docs(handoff): record phase 7b
  - `e5e4e9a` refactor(application): move workspace lifecycle under application
  - `c6fcfe2` docs(handoff): record phase 7a
  - `aa8e75d` refactor(application): move agent runtime under application
  - `4fc9053` docs(handoff): record phase 6ar
  - `5ce9c52` refactor(runtime): inline typed lifecycle execution paths
  - `3d61013` docs(handoff): record phase 6aq
  - `2085f6a` refactor(runtime): scope tuple lifecycle executors to module
  - `93283d7` docs(handoff): record phase 6ap
  - `d80e32f` refactor(ui): move lifecycle outcome conversion into msg
  - `3d3f680` docs(handoff): record phase 6ao
  - `ad254f6` refactor(runtime): unify session execution outcome type
  - `240452c` docs(handoff): record phase 6an
  - `286d266` refactor(runtime): add typed stop execution outcome
  - `46db066` docs(handoff): record phase 6am
  - `1ca52aa` refactor(runtime): add typed launch execution outcome
  - `a02eac1` docs(handoff): record phase 6al
  - `6a0da9e` refactor(runtime): stop sessions via workspace lifecycle helper
  - `d857df9` docs(handoff): record phase 6ak
  - `d9ef81a` refactor(runtime): build launch requests from workspace
  - `e2d7be2` docs(handoff): record phase 6aj
  - `3eddb24` refactor(runtime): add lifecycle execution facade helpers
  - `dd9d499` docs(handoff): record phase 6ai
  - `f977236` refactor(runtime): trim execution wrapper surface
  - `04dd021` docs(handoff): record phase 6ah
  - `dec61b5` refactor(runtime): add mode-based execution dispatch
  - `b7d8be7` docs(handoff): record phase 6ag
  - `2f45daf` refactor(runtime): centralize launch and stop result mapping
  - `c233767` docs(handoff): record phase 6af
  - `bc28049` refactor(runtime): add command executor abstraction
  - `69fbdde` docs(handoff): mark phases 6ac-6ae committed
  - `b6d262b` docs(handoff): record phases 6ac-6ae
  - `2d637bc` refactor(runtime): route sync execution through runtime command helpers
  - `1325e8f` phase 6ab
  - `5044871` phase 6aa
  - `34e7784` phase 6z
  - `ee13b44` phase 6y
  - `e6ffffc` phase 6x
  - `a5f5bff` phase 6w
  - `824ffce` phase 6v
  - `2b37bac` phase 6u
  - `c61c1da` phase 6t
  - `d3cba15` phase 6s
  - `0a46446` phase 6r
  - `ce477d5` phase 6q
  - `16ead82` phase 6p
  - `a3a13bf` phase 6o
  - `843f3c2` phase 6n
  - `0da752e` phase 6m
  - `7e3f610` phase 6l
  - `28a1c3d` phase 6k
  - `4d6b323` phase 6j
  - `6136c11` phase 6i
  - `af85ec7` phase 6h
  - `5bfd1a4` phase 6g
  - `a9c50a6` phase 6f
  - `19b5ecd` phase 6e
  - `87bb133` phase 6d
  - `3b365f0` phase 6c
  - `e4859a5` phase 6b
  - `3366bf3` phase 6a
  - `ea873d9` phase 5af
  - `9ce1826` phase 5ae
  - `0d9b21d` phase 5ad
  - `aa91ca5` phase 5ac
  - `e5686a0` phase 5ab
  - `1cb3248` phase 5aa
  - `55612c8` phase 5z
  - `d598eb3` phase 5y
  - `3cc83bf` phase 5x
  - `d16af8c` phase 5w
  - `0858c4f` phase 5v
  - `912bb07` phase 5u
  - `f634be8` phase 5t
  - `805390d` phase 5s
  - `ec108ab` phase 5r
  - `d444350` phase 5q
  - `b163c8a` phase 5p
  - `3970575` phase 5o
  - `5839585` phase 5n
  - `5d3f76b` phase 5m
  - `c2aa6fa` phase 5l
  - `7ee7e06` phase 5k

## Next Plan (execute in phases)

### Phase 5, split remaining `src/ui/tui/mod.rs`
Status:
- `msg`, `update`, `view`, `dialogs` created and wired.
- dialog key handlers and most dialog orchestration moved into `dialogs.rs`.
- view/layout/selection/render helpers moved into `view.rs`.
- key/mouse/input/interactivity orchestration moved into `update.rs`.
- lifecycle execution/completions moved into `update.rs`.
- preview polling/capture/runtime integration moved into `update.rs`.
- adaptive polling/activity tracking moved into `update.rs`.
- shared text/formatting helpers moved into `text.rs`.
- text selection structs/state moved into `selection.rs`.
- shared label/timing helpers moved into `logging.rs`.
- transition + dialog/tmux/toast logging helpers moved into `logging.rs`.
- input trace logging helpers moved into `logging.rs`.
- frame debug logging helpers moved into `logging.rs`.
- deferred command/input-seq helpers moved into `update.rs`.
- workspace helper methods moved into `update.rs`.
- app bootstrap constructors moved into `bootstrap.rs`.
- app path/dependency structs moved into `bootstrap.rs`.
- shared constants/types/theme moved into `shared.rs`.
- run entrypoints moved into `runner.rs`.
- `mod.rs` is now mostly module root + app state struct.

Next sub-targets:
- phase 6 runtime-boundary extraction is functionally complete for lifecycle start/stop flow
- phase 7 crate-tree alignment is complete
- phase 8 cleanup is complete (shim removal + final full gates)

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

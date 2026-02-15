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
- Commit: uncommitted (worktree changes)
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

## Current State
- Worktree has uncommitted phase 6i changes:
  - `src/agent_runtime.rs`
  - `src/agent_runtime/tests.rs`
  - `src/ui/tui/mod.rs`
  - `src/ui/tui/update.rs`
- Recent refactor commits on local `master`:
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
- continue phase 6 boundary work for non-UI runtime logic
- next candidate: move remaining non-UI session-target helpers from `ui/tui/update.rs` (for example interactive-entry session resolution helpers) into runtime/application layer
- keep phase-6 moves tiny and parity-safe across both multiplexers

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

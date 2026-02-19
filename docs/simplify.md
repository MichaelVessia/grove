# Simplification Plan

Ordered by impact/effort ratio. Each item is a self-contained change that should compile and pass tests independently.

## 1. Remove dead `multiplexer` parameter (6 call sites)

`MultiplexerKind` is a single-variant enum (`Tmux`). Every function that accepts it immediately discards it with `let _ = multiplexer;`. The `next()`/`previous()` methods on it are no-ops that return `self`.

- [x] Remove `multiplexer` param from all functions in `src/application/agent_runtime.rs` (lines 184, 410, 434, 558, 812)
- [x] Remove `multiplexer` param from `input_for_multiplexer` in `src/ui/tui/bootstrap_config.rs:113-116`, inline `Box::new(CommandTmuxInput)` at call site
- [x] Remove `next()`/`previous()` no-op methods on `MultiplexerKind`
- [x] Update all callers

## 2. Unify 3 `execute_command` / `stderr_or_status` copies

Three near-identical command execution functions exist with slightly different error types.

- [x] Extract shared `execute_command` and `stderr_or_status` into a common module (e.g. `infrastructure::process`)
- [x] Replace `agent_runtime.rs:781-805` standalone version
- [x] Replace `ui/tui/terminal/tmux.rs:80-107` (`CommandTmuxInput` methods)
- [x] Replace `workspace_lifecycle.rs:658-675` (`run_command`)
- [x] Unify error types with a common mapping strategy

## 3. Consolidate duplicate path comparison functions

Two identical functions with different names: `paths_refer_to_same_location` and `project_paths_equal`. Both do `match (left.canonicalize().ok(), right.canonicalize().ok())` with the same fallback.

- [x] Keep one canonical version in a shared location
- [x] Update `src/application/workspace_lifecycle.rs:651-656`
- [x] Update `src/ui/tui/bootstrap_config.rs:67-74`

## 4. Session launch/completion deduplication (~200 lines)

`ensure_lazygit_session_*` and `ensure_workspace_shell_session_*` follow identical patterns. Same for their completion handlers. Six `HashSet` fields on `GroveApp` form two identical triplets.

- [ ] Create a `SessionTracker` struct encapsulating `ready`, `failed`, `in_flight` HashSets
- [ ] Replace the six fields (`lazygit_ready_sessions`, `lazygit_failed_sessions`, `lazygit_launch_in_flight`, `shell_ready_sessions`, `shell_failed_sessions`, `shell_launch_in_flight`) with two `SessionTracker` instances
- [ ] Extract a generic session launch function parameterized by session kind
- [ ] Extract a generic completion handler
- [ ] Update `src/ui/tui/update_navigation_preview.rs:19-119, 121-238, 346-406, 408-475`

## 5. Dialog enum (enforce one-dialog-at-a-time at type level)

The app struct carries 8 `Option<XDialogState>` fields but only one can be open at a time.

- [ ] Create `enum ActiveDialog { Launch(..), Delete(..), Merge(..), UpdateFromBase(..), Create(..), Edit(..), Project(..), Settings(..) }`
- [ ] Replace the 8 `Option` fields with `active_dialog: Option<ActiveDialog>`
- [ ] Update all dialog open/close/render/input sites
- [ ] Update `src/ui/tui/bootstrap_app.rs:160-168`

## 6. Modal render boilerplate (~100 lines)

Six dialog render files follow identical structure. The final ~15 lines of each differ only by `dialog_width`, `dialog_height`, `title`, `border_color`, and `HIT_ID`.

- [x] Extract `render_modal_dialog(frame, area, body, width, height, title, border_color, hit_id)` helper
- [x] Refactor `view_overlays_workspace_launch.rs`
- [x] Refactor `view_overlays_workspace_delete.rs`
- [x] Refactor `view_overlays_workspace_merge.rs`
- [x] Refactor `view_overlays_workspace_update.rs`
- [x] Refactor `view_overlays_edit.rs`
- [x] Refactor `view_overlays_create.rs`

## 7. Verbose logging boilerplate

`LogEvent::new(...).with_data(...)` builder calls span 10+ lines each. `log_frame_render` alone is 137 lines.

- [ ] Create a macro or helper for common log patterns, e.g. `self.log("category", "action", &[("key", &value)])`
- [ ] Refactor `src/ui/tui/logging_frame.rs:40-176`
- [ ] Refactor `src/ui/tui/logging_state.rs:93-154`
- [ ] Refactor logging calls in `src/ui/tui/update_navigation_preview.rs`

## 8. Extract `capture_dimensions()` method

Identical 4-line computation repeated 3 times.

- [x] Add `fn capture_dimensions(&self) -> (u16, u16)` to `GroveApp`
- [x] Replace `src/ui/tui/update_lifecycle_start.rs:22-26`
- [x] Replace `src/ui/tui/update_navigation_preview.rs:33-37`
- [x] Replace `src/ui/tui/update_navigation_preview.rs:149-153`

## 9. Unify `mode_label`/`mode_name` and `focus_label`/`focus_name`

Two pairs of near-identical functions differ only by casing (PascalCase vs snake_case) and one extra check.

- [x] Unify into one function per concept in `src/ui/tui/logging_state.rs:13-43`
- [x] Update all callers

## 10. Merge duplicate constructors

`new_with_event_logger` and `new_with_debug_recorder` differ by a single `Some(ts)` vs `None` argument.

- [x] Replace both with a single `new(event_log, debug_record_start_ts: Option<u64>)` in `src/ui/tui/bootstrap_app.rs:18-49`
- [x] Update callers in `runner.rs`

## 11. Flatten constructor chain

`from_parts` -> `from_parts_with_projects` -> `from_parts_with_clipboard_and_projects` is 3 levels deep. `from_parts` is test-only.

- [x] Flatten to two constructors: one for tests, one for production
- [x] Update `src/ui/tui/bootstrap_app.rs:100-121`

## 12. Remove `AppPaths` single-field wrapper

Wraps a single `PathBuf`, only destructured at `bootstrap_app.rs:136`.

- [x] Replace `AppPaths` with `PathBuf` directly in `src/ui/tui/bootstrap_config.rs:10-19`
- [x] Update call sites

## 13. Named struct for `parse_start_options`

Returns an unnamed 3-tuple `(Option<String>, Option<String>, bool)`.

- [x] Create `StartOptions { prompt, pre_launch_command, skip_permissions }` struct
- [x] Update `src/ui/tui/dialogs_state_lifecycle.rs:23-35`
- [x] Update caller in `update_lifecycle_start.rs:126-127`

## 14. Remove `hello_message` scaffolding

Leftover scaffolding function with a dedicated test that adds no value.

- [x] Inline at call site in `src/main.rs:115` or replace with version string
- [x] Delete `src/lib.rs:6-8`
- [x] Delete test in `src/lib_tests.rs`

## 15. Gate `missing_workspace_paths` as test-only

`pub` function only called from tests.

- [x] Mark `#[cfg(test)]` or move into test module in `src/application/hardening.rs:15-24`
- [x] Update integration test usage in `tests/startup_reconciliation.rs:81` before gating symbol

## 16. Inline `input_for_multiplexer`

Always returns `Box::new(CommandTmuxInput)` regardless of input. Covered partly by item 1 but worth noting as a standalone cleanup if item 1 is split.

- [x] Inline at call site in `src/ui/tui/bootstrap_config.rs:113-116`

## 17. Remove `run_tui_with_*` thin wrappers

`lib.rs` has `run_tui_with_event_log` and `run_tui_with_debug_record` that just forward to `ui::tui::run_with_*`. After item 10 merges the constructors, these can collapse too.

- [x] Merge into a single `run_tui` entry point or have `main.rs` call `ui::tui` directly
- [x] Update `src/lib.rs:10-19`

# Simplification Plan

## Priority 1: Dead Code

### 1a. Unused `start_workspace_with_mode` / `stop_workspace_with_mode`

- `src/application/agent_runtime/execution.rs:19-31`
- Never called. One-liner wrappers. Delete both.

### 1b. Unused `restart_workspace`

- `src/application/agent_runtime/restart.rs:15-21`
- Never called. Delegates to `execute_restart_workspace_in_pane_with_result`.
  Delete.

### 1c. Unused `cleanup_commands_for_task`

- `src/application/session_cleanup.rs:109-111`
- Only used by tests in the same file. Tests can call
  `kill_task_session_commands` directly. Delete wrapper.

## Priority 2: Service Indirection

### 2a. `RuntimeService` trait with single implementation

- `src/application/services/runtime_service.rs` (360 lines)
- `RuntimeService` trait defines 11 methods. `CommandRuntimeService` is the only
  implementation. Every trait method forwards to the corresponding
  `agent_runtime` function. No mocking, no alternatives.
- Callers in `ui/tui/model.rs` could import from `agent_runtime` directly.
  Remove trait and struct, keep only the free functions if a thin facade is
  desired.

### 2b. `workspace_service.rs` pass-through functions

- `src/application/services/workspace_service.rs:75-98`
- `workspace_lifecycle_error_message`, `write_workspace_base_marker`,
  `delete_workspace`, `merge_workspace`, `update_workspace_from_base` are
  one-liner pass-throughs to `workspace_lifecycle`.
- `RuntimeSessionTerminator` (lines 18-50) could live in `workspace_lifecycle`
  itself.

## Priority 3: Duplicate Code

### 3a. `claude_project_dir_name` duplicated

- `src/application/agent_runtime/status.rs:253-266` (`#[cfg(test)]`)
- `src/application/agent_runtime/agents/claude.rs:128-140`
- Identical logic. Tests in `status.rs` should reference the canonical version
  in `claude.rs`.

### 3b. Duplicate tmux session listing

- `src/application/services/workspace_service.rs:52-73`
  (`list_tmux_session_names`)
- `src/application/session_cleanup.rs:113-133` (`list_tmux_sessions`)
- Both run `tmux list-sessions` with slightly different format strings. Share a
  single invocation with the richer format; callers that only need names discard
  the extra fields.

### 3c. Duplicate `unique_test_dir` helpers

- `src/application/agent_runtime/mod.rs:296`
- `src/application/agent_runtime/agents/opencode.rs:426`
- `src/application/agent_runtime/agents/codex.rs:408`
- `src/infrastructure/config.rs:304`
- `src/ui/tui/replay/mod.rs:52`
- Five near-identical functions. Consolidate into a single `#[cfg(test)]`
  utility module.

## Priority 4: Always-True Abstraction

### 4a. `supports_in_pane_restart` always returns true

- `src/application/agent_runtime/agents/mod.rs:11-15`
- Matches all `AgentType` variants and returns `true`. Remove function and
  simplify callers.

### 4b. `reconcile_with_sessions` forwarding

- `src/application/agent_runtime/reconciliation.rs:9-19`
- Clones `workspaces` via `to_vec()` and forwards to
  `reconcile_with_sessions_owned`. Merge into one function taking
  `Vec<Workspace>`; callers that need to keep the original clone at the call
  site.

## Priority 5: Minor Simplifications

### 5a. `session_name_matches_base_session` string matching

- `src/application/agent_runtime/execution.rs:316-338`
- Four cases with format+compare. Replace with a single `strip_prefix` approach:
  check if `session_name` starts with `base_session_name`, then inspect the
  suffix.

### 5b. `workspace_session_names_for_cleanup` dedup

- `src/application/agent_runtime/execution.rs:351-369`
- Uses both a `seen` HashSet and `matched` Vec. tmux sessions should not have
  duplicates. Simplify to `filter` + `collect`.

### 5c. Config load functions return full `GroveConfig`

- `src/infrastructure/config.rs:194-233`
- `load_global_from_path` returns `GroveConfig` with empty
  `projects`/`task_order`/`attention_acks`. `load_projects_from_path` returns
  `GroveConfig` with default `sidebar_width_pct`/`theme`. Each really represents
  a partial view. Return partial types and merge at the `load_from_path` call
  site.

## Priority 6: Structural

### 6a. Giant test module in `agent_runtime/mod.rs`

- `src/application/agent_runtime/mod.rs:234-2988`
- ~2750 lines of tests in `mod.rs`. Each sub-module (`restart`, `status`,
  `sessions`, etc.) could house its own tests, reducing `mod.rs` to data
  structures and re-exports.

### 6b. Re-export block in `agent_runtime/mod.rs`

- `src/application/agent_runtime/mod.rs:16-76`
- 60 lines of `pub use` and `#[cfg(test)] use`. Many consumed only by tests
  within the same module. Test code could import directly from sub-modules via
  `super::` paths.

### 6c. Monolithic shared test import block

- `src/application/agent_runtime/mod.rs:236-273`
- Single `mod shared` importing every function for test use. Grows linearly,
  fragile. Tests should import what they need directly.

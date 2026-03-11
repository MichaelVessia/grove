# Workspace List Actions Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let users grow and shrink tasks from the workspace list with pane-scoped `a`, `d`, and `D`.

**Architecture:** Reuse the existing create/delete dialogs, but give them explicit target modes. Add one task lifecycle path for appending a worktree to an existing task, and split delete execution so single-worktree deletion uses workspace lifecycle while task deletion keeps the existing task lifecycle path.

**Tech Stack:** Rust, Grove TUI, task lifecycle, workspace lifecycle, ftui command palette/help metadata

---

### Task 1: Add failing key-routing tests

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests for:
  - `a` in workspace list opens create dialog for selected task
  - `a` in preview still opens launch dialog
  - `D` does nothing outside workspace list
  - `d` opens single-worktree delete dialog

**Step 2: Run test to verify it fails**

Run: `cargo test workspace_list --lib`

Expected: FAIL on missing pane-scoped behavior

**Step 3: Write minimal implementation**

- Gate these commands by `PaneFocus`
- Add dialog target metadata needed by assertions

**Step 4: Run test to verify it passes**

Run: `cargo test workspace_list --lib`

Expected: PASS

### Task 2: Add failing add-worktree lifecycle tests

**Files:**
- Modify: `src/application/task_lifecycle.rs`
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/update/update_lifecycle_create.rs`

**Step 1: Write the failing tests**

- Add application test for appending a worktree to an existing task and rewriting `task.toml`
- Add TUI test for create dialog completion attaching to the existing task, not creating a new task

**Step 2: Run test to verify it fails**

Run: `cargo test add_worktree --lib`

Expected: FAIL on missing lifecycle API

**Step 3: Write minimal implementation**

- Add an add-worktree request/result
- Reuse task create worktree materialization logic
- Persist updated task manifest

**Step 4: Run test to verify it passes**

Run: `cargo test add_worktree --lib`

Expected: PASS

### Task 3: Add failing single-worktree delete tests

**Files:**
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/dialogs/dialogs_delete.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_delete.rs`

**Step 1: Write the failing tests**

- Add tests for deleting one selected worktree from a multi-worktree task
- Add test for last-worktree warning copy

**Step 2: Run test to verify it fails**

Run: `cargo test delete_workspace --lib`

Expected: FAIL on task-only delete behavior

**Step 3: Write minimal implementation**

- Add delete dialog target mode
- Route single-worktree delete through workspace lifecycle delete
- Keep task delete path for `D`

**Step 4: Run test to verify it passes**

Run: `cargo test delete_workspace --lib`

Expected: PASS

### Task 4: Update discoverability surfaces

**Files:**
- Modify: `src/ui/tui/commands/meta.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add assertions for help/command palette text reflecting workspace-list-only structural actions

**Step 2: Run test to verify it fails**

Run: `cargo test keybind_help --lib`

Expected: FAIL on stale descriptions

**Step 3: Write minimal implementation**

- Update command metadata strings and any pane-specific availability rules

**Step 4: Run test to verify it passes**

Run: `cargo test keybind_help --lib`

Expected: PASS

### Task 5: Verify

**Files:**
- Modify: none

**Step 1: Run focused tests**

Run:

```bash
cargo test add_worktree --lib
cargo test delete_workspace --lib
cargo test keybind_help --lib
```

Expected: PASS

**Step 2: Run required local validation**

Run: `make precommit`

Expected: PASS

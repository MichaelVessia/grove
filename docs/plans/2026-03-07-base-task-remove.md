# Base Task Remove Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let users remove base-checkout tasks from Grove's task list without deleting the checkout, while preserving destructive delete behavior for normal tasks.

**Architecture:** Add task-level base-task detection and route delete through two paths: destructive delete for normal tasks, non-destructive manifest removal for base tasks. Update the delete dialog and command gating so base tasks present "remove from list" copy and disable irrelevant options.

**Tech Stack:** Rust, Grove TUI dialogs, task lifecycle, task manifest persistence, Rust unit tests

---

### Task 1: Add failing lifecycle tests for base-task removal

**Files:**
- Modify: `src/application/task_lifecycle/delete.rs`
- Test: `src/application/task_lifecycle/delete.rs`

**Step 1: Write the failing test**

Add a unit test that constructs a task whose worktree path equals its repository
path, runs delete with a manifest root, and asserts the manifest entry is
removed while the checkout directory remains.

**Step 2: Run test to verify it fails**

Run: `cargo test base_task`
Expected: FAIL because delete still removes the task root or tries destructive
git cleanup.

**Step 3: Write minimal implementation**

Add task-level base detection and branch the delete lifecycle so base tasks skip
git worktree removal, branch deletion, and checkout removal.

**Step 4: Run test to verify it passes**

Run: `cargo test base_task`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/task_lifecycle/delete.rs
git commit -m "feat: allow non-destructive base task removal"
```

### Task 2: Add failing TUI tests for base-task dialog behavior

**Files:**
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/dialogs/dialogs_delete.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_delete.rs`
- Modify: `src/ui/tui/update/update_navigation_palette.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a regression test asserting the delete key opens the dialog for a base task
and that base-only destructive options are disabled or defaulted off.

**Step 2: Run test to verify it fails**

Run: `cargo test delete_key_on_main_workspace`
Expected: FAIL because the UI still blocks the action.

**Step 3: Write minimal implementation**

Remove the UI guard, expose base-task state in the dialog, update copy to
"remove from list" for base tasks, and disable local-branch cleanup toggles.

**Step 4: Run test to verify it passes**

Run: `cargo test delete_key_on_main_workspace`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/mod.rs src/ui/tui/dialogs/dialogs_delete.rs src/ui/tui/view/view_overlays_workspace_delete.rs src/ui/tui/update/update_navigation_palette.rs
git commit -m "feat(tui): allow base tasks to be removed from list"
```

### Task 3: Run focused validation

**Files:**
- Modify: `src/application/task_lifecycle/delete.rs`
- Modify: `src/ui/tui/dialogs/dialogs_delete.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_delete.rs`
- Modify: `src/ui/tui/update/update_navigation_palette.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Run focused tests**

Run: `cargo test base_task`

**Step 2: Run TUI delete tests**

Run: `cargo test delete_key_on_main_workspace`

**Step 3: Run required local validation**

Run: `make precommit`
Expected: PASS

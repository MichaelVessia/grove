# Task Create Base Branch Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove manual base-branch input from task creation and resolve a base branch per selected project from project defaults or git auto-detection.

**Architecture:** Keep project defaults as the user-configured source of truth, but move branch resolution into task creation so each selected repository resolves independently. This requires a small request-model change, a git helper for default-branch detection, and TUI cleanup to remove the obsolete shared branch field.

**Tech Stack:** Rust, FrankenTUI, git worktrees, existing TUI integration tests, existing task lifecycle tests

---

### Task 1: Remove Shared Base Branch From Create Dialog Tests

**Files:**
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/view/view_overlays_create.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a test that opens the New Task dialog and asserts the manual form no longer includes a base-branch field or branch picker copy.

**Step 2: Run test to verify it fails**

Run: `cargo test new_task_dialog_does_not_render_base_branch_field -- --exact`
Expected: FAIL because the dialog still renders `BaseBranch`.

**Step 3: Write minimal implementation**

Remove create-dialog base-branch state, rendering, and focus/navigation branches that only exist for that field.

**Step 4: Run test to verify it passes**

Run: `cargo test new_task_dialog_does_not_render_base_branch_field -- --exact`
Expected: PASS

### Task 2: Add Per-Project Base Branch Resolution Tests

**Files:**
- Modify: `src/application/task_lifecycle.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/application/task_lifecycle.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add one lifecycle test proving mixed repositories can create worktrees with different base branches, and one UI/request test proving create-task no longer submits a shared `base_branch`.

**Step 2: Run test to verify it fails**

Run: `cargo test create_task_resolves_base_branch_per_repository -- --exact`
Expected: FAIL because task creation still uses one shared branch.

**Step 3: Write minimal implementation**

Change the create-task request shape to omit shared base branch and resolve the branch per repository inside the lifecycle layer.

**Step 4: Run test to verify it passes**

Run: `cargo test create_task_resolves_base_branch_per_repository -- --exact`
Expected: PASS

### Task 3: Implement Git Auto-Detection

**Files:**
- Modify: `src/ui/tui/bootstrap/bootstrap_config.rs`
- Modify: `src/application/task_lifecycle.rs`
- Test: `src/application/task_lifecycle.rs`
- Test: `src/ui/tui/bootstrap/bootstrap_config.rs`

**Step 1: Write the failing test**

Add tests for repositories with no configured default that resolve `main` and `master` from git state.

**Step 2: Run test to verify it fails**

Run: `cargo test detect_repository_base_branch_prefers_default_then_current_then_common_names -- --exact`
Expected: FAIL because no such detector exists.

**Step 3: Write minimal implementation**

Add a helper that inspects git metadata for the remote HEAD/default branch, then falls back to current branch, then local `main`/`master`.

**Step 4: Run test to verify it passes**

Run: `cargo test detect_repository_base_branch_prefers_default_then_current_then_common_names -- --exact`
Expected: PASS

### Task 4: Validate And Regress

**Files:**
- Modify: `src/ui/tui/view/view_overlays_help/keybind_overlay.rs`
- Test: touched files above

**Step 1: Run focused tests**

Run:
- `cargo test new_task_dialog_does_not_render_base_branch_field -- --exact`
- `cargo test create_task_resolves_base_branch_per_repository -- --exact`
- `cargo test detect_repository_base_branch_prefers_default_then_current_then_common_names -- --exact`

Expected: PASS

**Step 2: Run fast repo validation**

Run: `make precommit`
Expected: PASS

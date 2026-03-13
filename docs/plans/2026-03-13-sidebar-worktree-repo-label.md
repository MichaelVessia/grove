# Sidebar Worktree Repo Label Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show each sidebar worktree row with its repository name so single-repo tasks are easier to scan.

**Architecture:** Keep the change local to sidebar row rendering. Reuse the existing `Workspace` project context, append `(<repo>)` only when it adds new information, and preserve the existing branch elision rules so the label does not become redundant.

**Tech Stack:** Rust, Grove TUI render tests

---

### Task 1: Add repo context to sidebar worktree labels

**Files:**
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/view/view_chrome_sidebar/build.rs`

**Step 1: Write the failing test**

- Add a sidebar render test proving a single-repo task shows `feature-a (grove)` and does not re-add a redundant `· feature-a` branch suffix.

**Step 2: Run test to verify it fails**

Run: `cargo test sidebar_shows_repo_name_for_single_repo_task --lib`

Expected: FAIL because the sidebar currently renders only the task/worktree name.

**Step 3: Write minimal implementation**

- Format the sidebar label as `<workspace> (<repo>)` when `project_name` exists and differs from the workspace name.
- Keep branch rendering keyed off the base workspace name so branch elision still works.

**Step 4: Run test to verify it passes**

Run: `cargo test sidebar_shows_repo_name_for_single_repo_task --lib`

Expected: PASS

### Task 2: Verify local checks

**Files:**
- Modify: none

**Step 1: Run focused tests**

Run: `cargo test sidebar_shows_repo_name_for_single_repo_task --lib`

Expected: PASS

**Step 2: Run required local validation**

Run: `make precommit`

Expected: PASS

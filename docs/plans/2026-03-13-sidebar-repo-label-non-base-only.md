# Sidebar Repo Label Non-Base Only Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Show the repository suffix in sidebar worktree rows only for non-base tasks.

**Architecture:** Keep the change local to sidebar rendering. Drive the label rule from `workspace.is_main` instead of comparing workspace and project names, so base tasks stay clean while derived tasks still surface repo context.

**Tech Stack:** Rust, Grove TUI render tests

---

### Task 1: Lock the base vs non-base sidebar behavior with tests

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

- Add a render test proving a base task named `web-monorepo` with repo `monorepo` hides the repo suffix, while a non-base task `gsops-4` for the same repo still shows `(monorepo)`.

**Step 2: Run test to verify it fails**

Run: `cargo test sidebar_hides_repo_name_for_base_task_even_when_repo_differs --lib`

Expected: FAIL because the current rule appends the suffix whenever the names differ.

**Step 3: Write minimal implementation**

- Update sidebar label formatting to append `(<repo>)` only when `!workspace.is_main`.

**Step 4: Run test to verify it passes**

Run: `cargo test sidebar_hides_repo_name_for_base_task_even_when_repo_differs --lib`

Expected: PASS

### Task 2: Verify

**Files:**
- Modify: none

**Step 1: Run focused tests**

Run:

```bash
cargo test sidebar_hides_repo_name_for_base_task_even_when_repo_differs --lib
cargo test sidebar_shows_repo_name_for_single_repo_task --lib
```

Expected: PASS

**Step 2: Run required local validation**

Run: `make precommit`

Expected: PASS

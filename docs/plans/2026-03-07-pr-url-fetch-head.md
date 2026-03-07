# PR URL Fetch Head Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `From GitHub PR` create tasks from the remote PR head commit instead of the repository base branch.

**Architecture:** Add explicit PR branch-source metadata to `CreateTaskRequest`, then teach task lifecycle creation to switch between the existing base-branch worktree path and a PR path that fetches `origin pull/<n>/head` and creates the worktree from `FETCH_HEAD`. Keep PR-mode validation and single-project semantics in the TUI, with manual mode unchanged.

**Tech Stack:** Rust, git worktrees, existing task lifecycle and TUI test harness

---

### Task 1: Add PR Source Metadata To Task Creation Requests

**Files:**
- Modify: `src/application/task_lifecycle.rs`
- Modify: `src/ui/tui/update/update_lifecycle_create.rs`
- Modify: `src/ui/tui/replay/types/completion.rs`
- Test: `src/application/task_lifecycle.rs`

**Step 1: Write the failing test**

Add a lifecycle request test asserting a request can represent PR mode explicitly.

```rust
#[test]
fn create_task_request_accepts_pull_request_source() {
    let request = CreateTaskRequest {
        task_name: "pr-123".to_string(),
        repositories: vec![fixture_repository("flohome")],
        agent: AgentType::Codex,
        branch_source: TaskBranchSource::PullRequest { number: 123 },
    };

    assert!(request.validate().is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_task_request_accepts_pull_request_source -- --exact`
Expected: FAIL because `TaskBranchSource` does not exist yet.

**Step 3: Write minimal implementation**

Introduce explicit request metadata instead of inferring PR behavior from the task name alone.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskBranchSource {
    BaseBranch,
    PullRequest { number: u64 },
}

pub struct CreateTaskRequest {
    pub task_name: String,
    pub repositories: Vec<RepositoryConfig>,
    pub agent: AgentType,
    pub branch_source: TaskBranchSource,
}
```

Update TUI request creation so:

```rust
branch_source: TaskBranchSource::BaseBranch
branch_source: TaskBranchSource::PullRequest { number: parsed.number }
```

**Step 4: Run test to verify it passes**

Run: `cargo test create_task_request_accepts_pull_request_source -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/task_lifecycle.rs src/ui/tui/update/update_lifecycle_create.rs src/ui/tui/replay/types/completion.rs
git commit -m "refactor: add explicit pr task branch source"
```

### Task 2: Teach Task Lifecycle To Fetch PR Head Refs

**Files:**
- Modify: `src/application/task_lifecycle/create.rs`
- Modify: `src/application/task_lifecycle.rs`
- Test: `src/application/task_lifecycle.rs`

**Step 1: Write the failing test**

Add a lifecycle test asserting PR-mode task creation fetches the GitHub PR ref and creates the worktree from `FETCH_HEAD`.

```rust
#[test]
fn create_task_in_root_fetches_pull_request_head_before_worktree_add() {
    let git = StubGitRunner::default();
    let request = CreateTaskRequest {
        task_name: "pr-123".to_string(),
        repositories: vec![fixture_repository("flohome")],
        agent: AgentType::Codex,
        branch_source: TaskBranchSource::PullRequest { number: 123 },
    };

    let _ = create_task_in_root(tasks_root(), &request, &git, &setup, &setup_command);

    assert_eq!(
        git.calls(),
        vec![
            vec!["fetch", "origin", "pull/123/head"],
            vec!["worktree", "add", "-b", "pr-123", worktree_path, "FETCH_HEAD"],
        ]
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_task_in_root_fetches_pull_request_head_before_worktree_add -- --exact`
Expected: FAIL because task lifecycle always uses the base-branch path.

**Step 3: Write minimal implementation**

Split worktree creation args by branch source:

```rust
match request.branch_source {
    TaskBranchSource::BaseBranch => {
        git worktree add -b <task> <path> <base_branch>
    }
    TaskBranchSource::PullRequest { number } => {
        git fetch origin pull/<number>/head
        git worktree add -b <task> <path> FETCH_HEAD
    }
}
```

Preserve base-branch marker writing by continuing to resolve and store the repository base branch even in PR mode.

**Step 4: Run test to verify it passes**

Run: `cargo test create_task_in_root_fetches_pull_request_head_before_worktree_add -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/task_lifecycle/create.rs src/application/task_lifecycle.rs
git commit -m "feat: fetch pr head for task creation"
```

### Task 3: Keep TUI PR Create Flow Wired To The New Lifecycle Contract

**Files:**
- Modify: `src/ui/tui/update/update_lifecycle_create.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add or update the PR-mode integration test to verify the task still creates successfully with the new explicit PR branch source.

```rust
#[test]
fn create_dialog_pr_mode_creates_task_with_single_repository() {
    // existing integration setup
    // confirm create
    assert_eq!(app.state.selected_task().map(|task| task.slug.as_str()), Some("pr-123"));
    assert_eq!(app.state.selected_task().map(|task| task.worktrees.len()), Some(1));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_pr_mode_creates_task_with_single_repository -- --exact`
Expected: FAIL until the TUI passes the PR number through `branch_source`.

**Step 3: Write minimal implementation**

In `confirm_create_dialog`, build the request with explicit branch source metadata:

```rust
let branch_source = match dialog.tab {
    CreateDialogTab::Manual => TaskBranchSource::BaseBranch,
    CreateDialogTab::PullRequest => TaskBranchSource::PullRequest { number: parsed.number },
};
```

Avoid reparsing or duplicating validation after this point.

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_pr_mode_creates_task_with_single_repository -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/update/update_lifecycle_create.rs src/ui/tui/mod.rs
git commit -m "refactor: wire pr task creation through branch source"
```

### Task 4: Focused Validation

**Files:**
- Test: `src/application/task_lifecycle.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Run focused lifecycle and TUI tests**

```bash
cargo test create_task_request_accepts_pull_request_source -- --exact
cargo test create_task_in_root_fetches_pull_request_head_before_worktree_add -- --exact
cargo test create_dialog_pr_mode_creates_task_with_single_repository -- --exact
cargo test create_dialog_creates_task_with_multiple_repositories
```

Expected: PASS

**Step 2: Run required local validation**

Run: `make precommit`
Expected: PASS

**Step 3: Commit**

```bash
git add src/application/task_lifecycle.rs src/application/task_lifecycle/create.rs src/ui/tui/update/update_lifecycle_create.rs src/ui/tui/mod.rs
git commit -m "test: validate pr task head fetch flow"
```

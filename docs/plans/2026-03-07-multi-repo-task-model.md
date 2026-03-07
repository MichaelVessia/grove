# Multi-Repo Task Model Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Grove's top-level workspace model with a first-class task model that supports one or many repository worktrees under a real task root and parent agent session.

**Architecture:** Introduce `Task` as the primary domain object, keep git worktrees as per-repository children, and move persistence/discovery to task manifests under `~/.grove/tasks/`. Migrate runtime, lifecycle, and UI from repository-first workspace handling to task-first orchestration with worktree-scoped git operations.

**Tech Stack:** Rust, FrankenTUI, tmux, git worktrees, TOML persistence, existing Grove replay/logging/test harnesses

---

### Task 1: Rename Core Concepts In The Domain

**Files:**
- Modify: `src/domain/mod.rs`
- Modify: `src/infrastructure/config.rs`
- Modify: `docs/PRD.md`
- Test: `src/domain/mod.rs`
- Test: `src/infrastructure/config.rs`

**Step 1: Write the failing test**

Add assertions that the domain exposes `Task`, `Worktree`, and `RepositoryConfig`-style concepts instead of relying on top-level `Workspace` and `ProjectConfig` naming.

```rust
#[test]
fn task_accepts_single_repository_worktree() {
    let task = Task::try_new(
        "flohome-launch".to_string(),
        "flohome-launch".to_string(),
        PathBuf::from("/tmp/.grove/tasks/flohome-launch"),
        vec![Worktree::try_new(
            "flohome".to_string(),
            PathBuf::from("/repos/flohome"),
            PathBuf::from("/tmp/.grove/tasks/flohome-launch/flohome"),
            "flohome-launch".to_string(),
        ).expect("worktree should be valid")],
    );
    assert!(task.is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test task_accepts_single_repository_worktree -- --exact`
Expected: FAIL because `Task` and the new constructor do not exist yet.

**Step 3: Write minimal implementation**

Create the new domain structs and validation entry points in `src/domain/mod.rs`. Keep old types only as short-lived compile scaffolding during the refactor, then delete them before moving on.

```rust
pub struct Task {
    pub name: String,
    pub slug: String,
    pub root_path: PathBuf,
    pub branch: String,
    pub worktrees: Vec<Worktree>,
}

pub struct Worktree {
    pub repository_name: String,
    pub repository_path: PathBuf,
    pub path: PathBuf,
    pub branch: String,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test task_accepts_single_repository_worktree -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/domain/mod.rs src/infrastructure/config.rs docs/PRD.md
git commit -m "refactor: rename workspace domain to task model"
```

### Task 2: Add Task Manifest Persistence

**Files:**
- Create: `src/infrastructure/task_manifest.rs`
- Modify: `src/infrastructure/mod.rs`
- Modify: `src/infrastructure/config.rs`
- Test: `src/infrastructure/task_manifest.rs`

**Step 1: Write the failing test**

Add a round-trip manifest test for a task with multiple worktrees.

```rust
#[test]
fn task_manifest_round_trips_multi_repo_task() {
    let task = fixture_task();
    let encoded = encode_task_manifest(&task).expect("manifest should encode");
    let decoded = decode_task_manifest(&encoded).expect("manifest should decode");
    assert_eq!(decoded, task);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test task_manifest_round_trips_multi_repo_task -- --exact`
Expected: FAIL because `task_manifest.rs` does not exist.

**Step 3: Write minimal implementation**

Add a dedicated manifest module for `.grove/task.toml` read/write, separate from static repository config.

```rust
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct TaskManifest {
    pub name: String,
    pub slug: String,
    pub branch: String,
    pub created_at_unix_secs: i64,
    pub worktrees: Vec<TaskManifestWorktree>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test task_manifest_round_trips_multi_repo_task -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/infrastructure/task_manifest.rs src/infrastructure/mod.rs src/infrastructure/config.rs
git commit -m "feat: add task manifest persistence"
```

### Task 3: Replace Workspace Lifecycle With Task Lifecycle

**Files:**
- Create: `src/application/task_lifecycle.rs`
- Create: `src/application/task_lifecycle/create.rs`
- Create: `src/application/task_lifecycle/delete.rs`
- Modify: `src/application/services/workspace_service.rs`
- Modify: `src/application/workspace_lifecycle.rs`
- Test: `src/application/task_lifecycle.rs`
- Test: `src/application/task_lifecycle/create.rs`
- Test: `src/application/task_lifecycle/delete.rs`

**Step 1: Write the failing test**

Add a test that creates a task with two repositories and verifies both worktrees are created under one task root.

```rust
#[test]
fn create_task_builds_one_worktree_per_repository_under_task_root() {
    let result = create_task(&fixture_repo_set(), &fixture_request(), &fake_git_runner())
        .expect("task should create");
    assert_eq!(result.worktrees.len(), 2);
    assert!(result.root_path.ends_with("flohome-launch"));
    assert!(result.worktrees[0].path.starts_with(&result.root_path));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_task_builds_one_worktree_per_repository_under_task_root -- --exact`
Expected: FAIL because task lifecycle APIs do not exist.

**Step 3: Write minimal implementation**

Move creation and deletion orchestration into a task lifecycle module. Keep git commands worktree-scoped inside the task creation loop.

```rust
pub struct CreateTaskRequest {
    pub task_name: String,
    pub repositories: Vec<RepositoryConfig>,
    pub base_branch: String,
}

pub fn create_task(...) -> Result<CreateTaskResult, TaskLifecycleError> {
    // create task root
    // create one worktree per repository
    // write manifest
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test create_task_builds_one_worktree_per_repository_under_task_root -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/task_lifecycle.rs src/application/task_lifecycle src/application/services/workspace_service.rs src/application/workspace_lifecycle.rs
git commit -m "feat: add task lifecycle orchestration"
```

### Task 4: Replace Repo-First Discovery With Task-First Discovery

**Files:**
- Create: `src/application/task_discovery.rs`
- Modify: `src/application/services/discovery_service.rs`
- Modify: `src/infrastructure/adapters.rs`
- Modify: `src/infrastructure/adapters/workspace.rs`
- Test: `src/application/task_discovery.rs`
- Test: `src/application/services/discovery_service.rs`

**Step 1: Write the failing test**

Add a test that task discovery loads tasks from manifests without enumerating configured repositories first.

```rust
#[test]
fn bootstrap_data_loads_tasks_from_task_manifests() {
    let bootstrap = bootstrap_data_for_tasks(&fixture_task_roots());
    assert_eq!(bootstrap.tasks.len(), 2);
    assert_eq!(bootstrap.tasks[0].worktrees.len(), 3);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test bootstrap_data_loads_tasks_from_task_manifests -- --exact`
Expected: FAIL because discovery still returns workspaces aggregated from repositories.

**Step 3: Write minimal implementation**

Add `TaskDiscoveryService` and make it the source of truth. Use git/tmux inspection only to validate declared worktrees and session state.

```rust
pub fn bootstrap_data_for_tasks(task_roots: &[PathBuf]) -> BootstrapData {
    // read task manifests
    // validate worktrees
    // attach runtime state
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test bootstrap_data_loads_tasks_from_task_manifests -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/task_discovery.rs src/application/services/discovery_service.rs src/infrastructure/adapters.rs src/infrastructure/adapters/workspace.rs
git commit -m "refactor: make discovery task first"
```

### Task 5: Split Runtime Session Scope Between Task Root And Worktree

**Files:**
- Modify: `src/application/agent_runtime/launch_plan.rs`
- Modify: `src/application/agent_runtime/sessions.rs`
- Modify: `src/application/agent_runtime/reconciliation.rs`
- Modify: `src/application/services/runtime_service.rs`
- Test: `src/application/agent_runtime/launch_plan.rs`
- Test: `src/application/agent_runtime/sessions.rs`
- Test: `src/application/agent_runtime/reconciliation.rs`

**Step 1: Write the failing test**

Add a test that task-root sessions and worktree sessions produce distinct names and targets.

```rust
#[test]
fn session_names_distinguish_task_root_and_worktree_scope() {
    assert_eq!(
        session_name_for_task("flohome-launch"),
        "grove-task-flohome-launch"
    );
    assert_eq!(
        session_name_for_worktree("flohome-launch", "flohome"),
        "grove-wt-flohome-launch-flohome"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test session_names_distinguish_task_root_and_worktree_scope -- --exact`
Expected: FAIL because runtime names only know about repository-scoped workspaces.

**Step 3: Write minimal implementation**

Introduce explicit session scope helpers and update runtime reconciliation to poll both the selected task session and selected worktree session set.

```rust
pub enum SessionScope {
    TaskRoot,
    Worktree,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test session_names_distinguish_task_root_and_worktree_scope -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/agent_runtime/launch_plan.rs src/application/agent_runtime/sessions.rs src/application/agent_runtime/reconciliation.rs src/application/services/runtime_service.rs
git commit -m "feat: add task and worktree runtime scopes"
```

### Task 6: Move UI State From Workspace List To Task Tree

**Files:**
- Modify: `src/ui/state.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/view/view_chrome_sidebar/build.rs`
- Modify: `src/ui/tui/view/view_status.rs`
- Modify: `src/ui/tui/view/view_preview.rs`
- Test: `src/ui/state.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a UI state test that a selected task exposes task-level selection plus worktree-level selection.

```rust
#[test]
fn app_state_tracks_selected_task_and_selected_worktree() {
    let state = AppState::new(vec![fixture_task()]);
    assert_eq!(state.selected_task().map(|task| task.slug.as_str()), Some("flohome-launch"));
    assert_eq!(
        state.selected_worktree().map(|worktree| worktree.repository_name.as_str()),
        Some("flohome")
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test app_state_tracks_selected_task_and_selected_worktree -- --exact`
Expected: FAIL because `AppState` still stores a flat `Vec<Workspace>`.

**Step 3: Write minimal implementation**

Refactor state to store tasks and selection scopes. Update the sidebar builder to render tasks with nested worktrees.

```rust
pub struct AppState {
    pub tasks: Vec<Task>,
    pub selected_task_index: usize,
    pub selected_worktree_index: usize,
    pub focus: PaneFocus,
    pub mode: UiMode,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test app_state_tracks_selected_task_and_selected_worktree -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/state.rs src/ui/tui/model.rs src/ui/tui/view/view_chrome_sidebar/build.rs src/ui/tui/view/view_status.rs src/ui/tui/view/view_preview.rs src/ui/tui/mod.rs
git commit -m "refactor: render tasks and nested worktrees in the ui"
```

### Task 7: Replace Create/Delete/Merge/Update UI Flows

**Files:**
- Modify: `src/ui/tui/update/update_lifecycle_create.rs`
- Modify: `src/ui/tui/dialogs/dialogs_delete.rs`
- Modify: `src/ui/tui/dialogs/dialogs_merge.rs`
- Modify: `src/ui/tui/dialogs/dialogs_update_from_base.rs`
- Modify: `src/ui/tui/view/view_overlays_create.rs`
- Modify: `src/ui/tui/view/view_overlays_help/keybind_overlay.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add an interaction test that task creation selects multiple repositories and creates one task, not one workspace.

```rust
#[test]
fn create_dialog_creates_task_with_multiple_repositories() {
    let mut app = fixture_app();
    app.open_create_task_dialog();
    app.toggle_create_task_repository("flohome");
    app.toggle_create_task_repository("terraform-fastly");
    app.confirm_create_task_dialog();
    assert!(app.flash_message().contains("task"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_creates_task_with_multiple_repositories -- --exact`
Expected: FAIL because the create dialog only supports one selected project and one workspace request.

**Step 3: Write minimal implementation**

Replace the create dialog request shape with `CreateTaskRequest`, add multi-select repository UI, and scope destructive operations so task delete is task-wide while merge/update remain selected-worktree actions.

```rust
pub struct CreateTaskRequest {
    pub task_name: String,
    pub repository_indices: Vec<usize>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_creates_task_with_multiple_repositories -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/update/update_lifecycle_create.rs src/ui/tui/dialogs/dialogs_delete.rs src/ui/tui/dialogs/dialogs_merge.rs src/ui/tui/dialogs/dialogs_update_from_base.rs src/ui/tui/view/view_overlays_create.rs src/ui/tui/view/view_overlays_help/keybind_overlay.rs src/ui/tui/mod.rs
git commit -m "feat: add task creation and scoped lifecycle dialogs"
```

### Task 8: Migrate Logging, Replay, Cleanup, And Benchmarks

**Files:**
- Modify: `src/ui/tui/logging/logging_state.rs`
- Modify: `src/ui/tui/replay/types/bootstrap.rs`
- Modify: `src/ui/tui/replay/types/state.rs`
- Modify: `src/application/session_cleanup.rs`
- Modify: `src/application/scale_benchmark.rs`
- Test: `src/ui/tui/replay/mod.rs`
- Test: `src/application/session_cleanup.rs`
- Test: `src/application/scale_benchmark.rs`

**Step 1: Write the failing test**

Add tests that replay/bootstrap and cleanup operate on tasks and worktrees, not raw workspaces.

```rust
#[test]
fn session_cleanup_targets_task_and_worktree_sessions() {
    let commands = cleanup_commands_for_task(&fixture_task());
    assert!(commands.iter().any(|cmd| cmd.contains("grove-task-")));
    assert!(commands.iter().any(|cmd| cmd.contains("grove-wt-")));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test session_cleanup_targets_task_and_worktree_sessions -- --exact`
Expected: FAIL because cleanup and replay only know about workspace-scoped sessions.

**Step 3: Write minimal implementation**

Update replay snapshots, logging fields, cleanup flows, and scale benchmarks to record task identity and nested worktree identity.

```rust
pub struct ReplayTask {
    pub slug: String,
    pub worktrees: Vec<ReplayWorktree>,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test session_cleanup_targets_task_and_worktree_sessions -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/logging/logging_state.rs src/ui/tui/replay/types/bootstrap.rs src/ui/tui/replay/types/state.rs src/application/session_cleanup.rs src/application/scale_benchmark.rs src/ui/tui/replay/mod.rs
git commit -m "refactor: move logging and replay to the task model"
```

### Task 9: Add Migration Docs And Validation Coverage

**Files:**
- Create: `docs/migrations/2026-03-task-model.md`
- Modify: `docs/PRD.md`
- Modify: `.github/workflows/` files as needed if command names or assertions change
- Test: `src/ui/tui/mod.rs`
- Test: `make precommit`

**Step 1: Write the failing test**

Add or update end-to-end tests that assert the new UI vocabulary and task creation behavior.

```rust
#[test]
fn keybind_help_mentions_tasks_and_worktrees() {
    let app = fixture_app();
    let help = app.rendered_help_text();
    assert!(help.contains("task"));
    assert!(help.contains("worktree"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test keybind_help_mentions_tasks_and_worktrees -- --exact`
Expected: FAIL because help text still references projects and workspaces.

**Step 3: Write minimal implementation**

Write the migration guide, update PRD language, and bring help text, command palette copy, and CI expectations into sync.

```md
# Task Model Migration

- `workspace` -> `task`
- `project` -> `repository`
- each legacy workspace becomes a single-worktree task
```

**Step 4: Run test to verify it passes**

Run: `cargo test keybind_help_mentions_tasks_and_worktrees -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add docs/migrations/2026-03-task-model.md docs/PRD.md .github/workflows src/ui/tui/mod.rs
git commit -m "docs: add task model migration guide"
```

### Final Verification

**Step 1: Run targeted Rust tests for touched modules**

Run:

```bash
cargo test task_accepts_single_repository_worktree -- --exact
cargo test task_manifest_round_trips_multi_repo_task -- --exact
cargo test create_task_builds_one_worktree_per_repository_under_task_root -- --exact
cargo test bootstrap_data_loads_tasks_from_task_manifests -- --exact
cargo test session_names_distinguish_task_root_and_worktree_scope -- --exact
cargo test app_state_tracks_selected_task_and_selected_worktree -- --exact
cargo test create_dialog_creates_task_with_multiple_repositories -- --exact
cargo test session_cleanup_targets_task_and_worktree_sessions -- --exact
cargo test keybind_help_mentions_tasks_and_worktrees -- --exact
```

Expected: PASS for each targeted test.

**Step 2: Run local required validation**

Run: `make precommit`
Expected: PASS

**Step 3: Commit any remaining doc or test updates**

```bash
git add docs src .github/workflows
git commit -m "test: finish task model migration coverage"
```

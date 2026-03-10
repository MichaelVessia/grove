# Base Task Creation + Upstream Sync Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let users create base tasks (repo root registrations) from the new-task dialog and pull upstream changes with optional propagation to downstream workspaces.

**Architecture:** Feature 1 adds a `Base` variant to `CreateDialogTab` alongside `Manual` and `PullRequest`. Feature 2 adds an "Update upstream" command on base worktrees that pulls then optionally merges into related workspaces. Both features are additive, touching the create dialog state/view/key handling and the existing update-from-base infrastructure.

**Tech Stack:** Rust, FrankenTUI (Elm/MVU), tmux integration.

---

### Task 1: Add `CreateBaseTaskRequest` and `create_base_task` to task lifecycle

**Files:**
- Modify: `src/application/task_lifecycle.rs`
- Modify: `src/application/task_lifecycle/create.rs`

**Step 1: Write the failing test**

Add to `src/application/task_lifecycle.rs` in the `tests` module:

```rust
#[test]
fn create_base_task_registers_repo_root_as_main_worktree() {
    let temp = TestDir::new("create-base");
    let tasks_root = temp.path.join("tasks");
    let repo_root = temp.path.join("repos").join("flo360");
    fs::create_dir_all(&repo_root).expect("repo root should exist");

    let request = CreateBaseTaskRequest {
        repository: repository(repo_root.clone()),
        agent: AgentType::Codex,
        base_branch: "main".to_string(),
    };
    let result = create_base_task_in_root(tasks_root.as_path(), &request)
        .expect("base task should create");

    assert_eq!(result.task.name, "flo360");
    assert_eq!(result.task.worktrees.len(), 1);
    let worktree = &result.task.worktrees[0];
    assert_eq!(worktree.path, repo_root);
    assert_eq!(worktree.repository_path, repo_root);
    assert_eq!(worktree.branch, "main");
    assert!(worktree.is_main_checkout());
    assert!(task_manifest_path(&result.task_root).exists());
}

#[test]
fn create_base_task_validates_repository_path_exists() {
    let temp = TestDir::new("create-base-missing");
    let tasks_root = temp.path.join("tasks");
    let repo_root = temp.path.join("repos").join("nonexistent");

    let request = CreateBaseTaskRequest {
        repository: repository(repo_root),
        agent: AgentType::Codex,
        base_branch: "main".to_string(),
    };
    let result = create_base_task_in_root(tasks_root.as_path(), &request);

    assert!(result.is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib -- task_lifecycle::tests::create_base_task -v`
Expected: FAIL (struct and function don't exist yet)

**Step 3: Implement `CreateBaseTaskRequest` and `create_base_task_in_root`**

In `src/application/task_lifecycle.rs`, add the request struct after `CreateTaskRequest`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBaseTaskRequest {
    pub repository: RepositoryConfig,
    pub agent: AgentType,
    pub base_branch: String,
}
```

Add the public entry point after `create_task_in_root`:

```rust
pub fn create_base_task(
    request: &CreateBaseTaskRequest,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    let home_directory = dirs::home_dir().ok_or(TaskLifecycleError::HomeDirectoryUnavailable)?;
    let tasks_root = home_directory.join(".grove").join("tasks");
    create_base_task_in_root(tasks_root.as_path(), request)
}

pub fn create_base_task_in_root(
    tasks_root: &Path,
    request: &CreateBaseTaskRequest,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    create::create_base_task_in_root(tasks_root, request)
}
```

In `src/application/task_lifecycle/create.rs`, add the implementation:

```rust
pub(super) fn create_base_task_in_root(
    tasks_root: &Path,
    request: &CreateBaseTaskRequest,
) -> Result<CreateTaskResult, TaskLifecycleError> {
    let repo_name = repo_directory_name(&request.repository)?;
    if !request.repository.path.exists() {
        return Err(TaskLifecycleError::Io(format!(
            "repository path does not exist: {}",
            request.repository.path.display()
        )));
    }

    let task_root = tasks_root.join(&repo_name);
    fs::create_dir_all(&task_root).map_err(|error| TaskLifecycleError::Io(error.to_string()))?;

    let worktree = Worktree::try_new(
        request.repository.name.clone(),
        request.repository.path.clone(),
        request.repository.path.clone(),
        request.base_branch.clone(),
        request.agent,
        WorkspaceStatus::Main,
    )
    .map_err(|error| TaskLifecycleError::TaskInvalid(format!("{error:?}")))?
    .with_base_branch(Some(request.base_branch.clone()));

    let task = create_task_domain(
        &repo_name,
        &request.base_branch,
        &task_root,
        vec![worktree],
    )?;
    write_task_manifest(&task_root, &task)?;

    Ok(CreateTaskResult {
        task_root,
        task,
        warnings: Vec::new(),
    })
}
```

Add imports for `CreateBaseTaskRequest` to the test module's `use super::` block, and add `create_base_task_in_root` to the public exports.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib -- task_lifecycle::tests::create_base_task -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/application/task_lifecycle.rs src/application/task_lifecycle/create.rs
git commit -m "feat: add create_base_task to task lifecycle"
```

---

### Task 2: Add `Base` tab to create dialog state

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`

**Step 1: Add `Base` variant to `CreateDialogTab`**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogTab {
    Manual,
    PullRequest,
    Base,
}
```

Update `CreateDialogTab` methods:

```rust
impl CreateDialogTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::PullRequest => "From GitHub PR",
            Self::Base => "Base",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Manual => Self::PullRequest,
            Self::PullRequest => Self::Base,
            Self::Base => Self::Manual,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Manual => Self::Base,
            Self::PullRequest => Self::Manual,
            Self::Base => Self::PullRequest,
        }
    }
}
```

Update `CreateDialogField::first_for_tab`:

```rust
pub(super) fn first_for_tab(tab: CreateDialogTab) -> Self {
    match tab {
        CreateDialogTab::Manual => Self::WorkspaceName,
        CreateDialogTab::PullRequest => Self::Project,
        CreateDialogTab::Base => Self::Project,
    }
}
```

Update `CreateDialogField::next` and `previous` to add `Base` arms. Base tab flow: `Project -> CreateButton -> CancelButton -> Project`.

```rust
pub(super) fn next(self, tab: CreateDialogTab) -> Self {
    match tab {
        CreateDialogTab::Manual => match self { /* ... unchanged ... */ },
        CreateDialogTab::PullRequest => match self { /* ... unchanged ... */ },
        CreateDialogTab::Base => match self {
            Self::Project => Self::CreateButton,
            Self::CreateButton => Self::CancelButton,
            Self::CancelButton => Self::Project,
            Self::WorkspaceName | Self::PullRequestUrl => Self::Project,
        },
    }
}

pub(super) fn previous(self, tab: CreateDialogTab) -> Self {
    match tab {
        CreateDialogTab::Manual => match self { /* ... unchanged ... */ },
        CreateDialogTab::PullRequest => match self { /* ... unchanged ... */ },
        CreateDialogTab::Base => match self {
            Self::Project => Self::CancelButton,
            Self::CreateButton => Self::Project,
            Self::CancelButton => Self::CreateButton,
            Self::WorkspaceName | Self::PullRequestUrl => Self::Project,
        },
    }
}
```

**Step 2: Run precommit**

Run: `make precommit`
Expected: PASS (compiles, all existing tests pass)

**Step 3: Commit**

```bash
git add src/ui/tui/dialogs/state.rs
git commit -m "feat: add Base tab variant to create dialog state"
```

---

### Task 3: Build repo picker for Base tab (filtering repos without existing base tasks)

**Files:**
- Modify: `src/ui/tui/dialogs/dialogs_create_setup.rs`
- Modify: `src/ui/tui/update/update_lifecycle_create.rs`

This task adds the logic to populate the project picker with only repos that don't already have a base task. It also handles the `Base` tab confirmation flow.

**Step 1: Read existing `open_create_dialog` and `selected_create_dialog_projects`**

Read: `src/ui/tui/dialogs/dialogs_create_setup.rs` fully to understand project picker setup.

**Step 2: Add filtering logic**

In `dialogs_create_setup.rs`, add a helper to detect which project indices already have base tasks:

```rust
fn project_indices_with_base_task(&self) -> HashSet<usize> {
    let mut indices = HashSet::new();
    for (project_index, project) in self.projects.iter().enumerate() {
        let has_base = self.state.workspaces.iter().any(|ws| {
            ws.is_main
                && ws.project_path.as_ref()
                    .map_or(false, |path| refer_to_same_location(path, &project.path))
        });
        if has_base {
            indices.insert(project_index);
        }
    }
    indices
}
```

When the Base tab is active and the project picker opens, filter out projects that already have base tasks. The project picker already has `filtered_project_indices`, so extend the filter predicate in the picker rebuild logic to exclude indices present in `project_indices_with_base_task()` when `dialog.tab == CreateDialogTab::Base`.

**Step 3: Handle Base tab confirmation in `confirm_create_dialog`**

In `src/ui/tui/update/update_lifecycle_create.rs`, add a `CreateDialogTab::Base` arm in `confirm_create_dialog`:

```rust
CreateDialogTab::Base => {
    let base_branch = resolve_repository_base_branch(&project)
        .unwrap_or_else(|_| "main".to_string());
    // no task_name input needed, derive from repo dir name
    let repo_name = project.path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("base")
        .to_string();
    // ... build CreateBaseTaskRequest and execute
}
```

Wire up the execution path similarly to the Manual/PullRequest paths, using `create_base_task` instead of `create_task`.

Add a new `Msg` variant or reuse `CreateWorkspaceCompleted` since `CreateTaskResult` is the same type.

**Step 4: Run precommit**

Run: `make precommit`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/dialogs/dialogs_create_setup.rs src/ui/tui/update/update_lifecycle_create.rs
git commit -m "feat: wire Base tab confirmation to create_base_task"
```

---

### Task 4: Render Base tab in create dialog overlay

**Files:**
- Modify: `src/ui/tui/view/view_overlays_create.rs`

**Step 1: Read existing render logic**

Read: `src/ui/tui/view/view_overlays_create.rs` fully.

**Step 2: Add Base tab rendering**

Add the "Base" tab label to the tab row (alongside "Manual" and "From GitHub PR").

Add a `CreateDialogTab::Base` arm in the body rendering section. The Base tab body should show:

1. "Project" field (same project picker as Manual/PR tabs)
2. A description line: "Register repo root as a base task for upstream updates"
3. Create/Cancel buttons

No task name input. No PR URL input. Just project picker and buttons.

**Step 3: Run precommit**

Run: `make precommit`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/tui/view/view_overlays_create.rs
git commit -m "feat: render Base tab in create dialog overlay"
```

---

### Task 5: Handle key events for Base tab

**Files:**
- Modify: `src/ui/tui/dialogs/dialogs_create_key.rs`

**Step 1: Read existing key handling**

Read: `src/ui/tui/dialogs/dialogs_create_key.rs` fully.

**Step 2: Update key handler for Base tab**

The key handler dispatches based on `dialog.focused_field` and `dialog.tab`. Add `CreateDialogTab::Base` handling:

- Character input: no text fields to type into (project picker handles its own keys)
- Enter on Project field: opens project picker (same as other tabs)
- Enter on CreateButton: calls `confirm_create_dialog()`
- Tab/S-Tab: cycle through `Project -> CreateButton -> CancelButton`

Most of this should already work because field navigation uses `CreateDialogField::next(tab)` which we updated in Task 2. The main thing to verify is that character input doesn't try to write to non-existent fields.

**Step 3: Run precommit**

Run: `make precommit`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/tui/dialogs/dialogs_create_key.rs
git commit -m "feat: handle key events for Base tab in create dialog"
```

---

### Task 6: Integration test for base task creation via TUI

**Files:**
- Modify: `src/ui/tui/mod.rs` (test module)

**Step 1: Write the test**

Follow the pattern of existing create dialog tests in `src/ui/tui/mod.rs`. The test should:

1. Set up a GroveApp with projects
2. Open the create dialog
3. Switch to the Base tab (simulate Alt+] twice or Alt+[ once)
4. Select a project in the picker
5. Confirm creation
6. Assert: task created with `is_main_checkout() == true`, `path == repository_path`

Look at existing tests like `create_dialog_manual_mode_creates_workspace` for the pattern.

**Step 2: Run the test**

Run: `cargo test --lib -- tui::tests::create_dialog_base_tab -v`
Expected: PASS

**Step 3: Commit**

```bash
git add src/ui/tui/mod.rs
git commit -m "test: integration test for base task creation via TUI"
```

---

### Task 7: Add "Pull Upstream" command and keybind

**Files:**
- Modify: `src/ui/tui/commands/catalog.rs`
- Modify: `src/ui/tui/commands/meta.rs`
- Modify: `src/ui/tui/update/update_navigation_commands.rs`

**Step 1: Add `PullUpstream` to `UiCommand`**

In `catalog.rs`, add `PullUpstream` variant after `UpdateFromBase`:

```rust
PullUpstream,
```

Update `UiCommand::ALL` array (increment the array size to 41) and add the variant.

**Step 2: Add metadata in `meta.rs`**

Add a new entry to `COMMAND_META`:

```rust
// PullUpstream
UiCommandMeta {
    palette: Some(PaletteCommandSpec {
        id: "palette:pull_upstream",
        title: "Pull Upstream",
        description: "Pull upstream and propagate to workspaces (U)",
        tags: &["pull", "upstream", "sync", "fetch", "update", "U"],
        category: "Task",
    }),
    help_hints: &[HelpHintSpec {
        context: HelpHintContext::Workspace,
        label: "pull_upstream",
        key: "U",
        action: "pull upstream + propagate",
    }],
    keybindings: &[KeybindingSpec {
        scope: KeybindingScope::NonInteractive,
        code: KeyCodeMatch::Char('U'),
        modifiers: KeyModifiersMatch::None,
    }],
},
```

Add the match arm in `UiCommand::meta()`.

**Step 3: Wire command execution**

In `update_navigation_commands.rs`, add the handler:

```rust
UiCommand::PullUpstream => {
    self.open_pull_upstream_dialog();
}
```

**Step 4: Run precommit**

Run: `make precommit`
Expected: FAIL (method doesn't exist yet, but should compile if you add an empty stub)

Add a stub in the dialog file (Task 8 will implement it).

**Step 5: Commit**

```bash
git add src/ui/tui/commands/catalog.rs src/ui/tui/commands/meta.rs src/ui/tui/update/update_navigation_commands.rs
git commit -m "feat: add PullUpstream command and keybind (U)"
```

---

### Task 8: Implement pull upstream dialog (pull + propagate prompt)

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Create: `src/ui/tui/dialogs/dialogs_pull_upstream.rs`
- Modify: `src/ui/tui/dialogs/mod.rs` (or `dialogs.rs`, wherever dialog modules are declared)
- Modify: `src/ui/tui/msg.rs`
- Modify: `src/ui/tui/update/update.rs`

**Step 1: Add dialog state**

In `state.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PullUpstreamDialogState {
    pub(super) task_slug: Option<String>,
    pub(super) project_name: String,
    pub(super) project_path: PathBuf,
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) propagate_targets: Vec<PropagateTarget>,
    pub(super) phase: PullUpstreamPhase,
    pub(super) focused_field: PullUpstreamDialogField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PropagateTarget {
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) task_slug: Option<String>,
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PullUpstreamPhase {
    Confirm,
    PullComplete { propagate_count: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PullUpstreamDialogField {
    PullButton,
    CancelButton,
}

cyclic_field_nav!(pub(super) PullUpstreamDialogField {
    PullButton, CancelButton,
});
```

**Step 2: Add messages**

In `msg.rs`:

```rust
PullUpstreamCompleted(PullUpstreamCompletion),
```

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PullUpstreamCompletion {
    pub(super) workspace_name: String,
    pub(super) base_branch: String,
    pub(super) result: Result<(), String>,
    pub(super) propagate_targets: Vec<PropagateTarget>,
}
```

**Step 3: Implement dialog opening**

In `dialogs_pull_upstream.rs`:

```rust
impl GroveApp {
    pub(super) fn open_pull_upstream_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        if !workspace.is_main {
            self.show_info_toast("pull upstream is only available on base worktrees");
            return;
        }

        let project_path = match workspace.project_path.clone() {
            Some(path) => path,
            None => {
                self.show_info_toast("workspace has no project path");
                return;
            }
        };

        // Find all non-main workspaces that share this repo and base branch
        let propagate_targets: Vec<PropagateTarget> = self.state.workspaces.iter()
            .filter(|ws| !ws.is_main)
            .filter(|ws| ws.project_path.as_ref()
                .map_or(false, |path| refer_to_same_location(path, &project_path)))
            .filter(|ws| ws.base_branch.as_deref() == Some(workspace.branch.as_str()))
            .map(|ws| PropagateTarget {
                workspace_name: ws.name.clone(),
                workspace_branch: ws.branch.clone(),
                workspace_path: ws.path.clone(),
                task_slug: ws.task_slug.clone(),
                project_name: ws.project_name.clone(),
                project_path: ws.project_path.clone(),
            })
            .collect();

        self.set_pull_upstream_dialog(PullUpstreamDialogState {
            task_slug: workspace.task_slug.clone(),
            project_name: workspace.project_name.unwrap_or_default(),
            project_path,
            workspace_name: workspace.name.clone(),
            workspace_path: workspace.path.clone(),
            base_branch: workspace.branch.clone(),
            propagate_targets,
            phase: PullUpstreamPhase::Confirm,
            focused_field: PullUpstreamDialogField::PullButton,
        });
        // ... logging, mode/focus changes ...
    }
}
```

**Step 4: Implement confirmation**

The confirm handler should:
1. Run `git pull --ff-only origin {base_branch}` via the existing update infrastructure.
2. On success, if `propagate_targets` is non-empty, show toast "Pulled. Update N workspaces from base?" and offer a follow-up prompt.
3. On failure, show error toast.

For propagation, reuse the existing `update_workspace_from_base` by dispatching one `UpdateWorkspaceFromBaseRequest` per target sequentially. Show results per workspace.

**Step 5: Wire into `ActiveDialog` enum and message handler**

Add `PullUpstream(PullUpstreamDialogState)` variant to `ActiveDialog`. Add the accessor/setter pattern used by other dialogs. Handle the `Msg::PullUpstreamCompleted` in `update.rs`.

**Step 6: Run precommit**

Run: `make precommit`
Expected: PASS

**Step 7: Commit**

```bash
git add src/ui/tui/dialogs/ src/ui/tui/msg.rs src/ui/tui/update/
git commit -m "feat: implement pull upstream dialog with propagation"
```

---

### Task 9: Render pull upstream dialog

**Files:**
- Create: `src/ui/tui/view/view_overlays_pull_upstream.rs`
- Modify: view module declarations to include the new file

**Step 1: Implement render function**

Follow the pattern from `view_overlays_workspace_update.rs`. The dialog should show:

- Title: "Pull Upstream"
- Workspace name, branch, path
- Strategy: "git pull --ff-only origin {base_branch}"
- If propagate_targets is non-empty: "After pull, N workspace(s) can be updated from base"
- PullButton / CancelButton
- Help hint: same navigation pattern as update dialog

**Step 2: Wire render into the main overlay dispatcher**

In the file that dispatches overlay rendering based on `ActiveDialog`, add the `PullUpstream` arm.

**Step 3: Run precommit**

Run: `make precommit`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/tui/view/
git commit -m "feat: render pull upstream dialog overlay"
```

---

### Task 10: Handle key events for pull upstream dialog

**Files:**
- Modify: `src/ui/tui/dialogs/dialogs_pull_upstream.rs`

**Step 1: Add key handler**

Follow the pattern from `dialogs_update_from_base.rs`:

- Escape/q: cancel
- Enter on PullButton: confirm pull
- Tab/S-Tab/C-n/C-p: navigate fields
- h/l: switch buttons

**Step 2: Wire into main key dispatcher**

In the file that routes key events to dialog handlers based on `ActiveDialog`, add the `PullUpstream` arm.

**Step 3: Run precommit**

Run: `make precommit`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/tui/dialogs/
git commit -m "feat: handle key events for pull upstream dialog"
```

---

### Task 11: Propagation flow after successful pull

**Files:**
- Modify: `src/ui/tui/dialogs/dialogs_pull_upstream.rs` (or completion handler file)

**Step 1: Implement propagation on pull success**

In the pull upstream completion handler:

1. If pull succeeded and `propagate_targets` is non-empty:
   - Show info toast: "Pulled {base_branch}. Updating N workspace(s)..."
   - For each target, dispatch an `UpdateWorkspaceFromBaseRequest` sequentially.
   - Collect results.
   - Show summary toast: "Updated N/M workspaces. K failed."
2. If pull succeeded and no targets: show success toast.
3. If pull failed: show error toast.

The sequential dispatch can use the existing `Cmd::task` pattern. Chain them or run sequentially within a single task closure.

**Step 2: Run precommit**

Run: `make precommit`
Expected: PASS

**Step 3: Commit**

```bash
git add src/ui/tui/
git commit -m "feat: propagate base pull to downstream workspaces"
```

---

### Task 12: Integration tests for pull upstream flow

**Files:**
- Modify: `src/ui/tui/mod.rs` (test module)

**Step 1: Write tests**

Test cases:
1. `pull_upstream_only_available_on_base_worktree` - selecting non-main workspace shows info toast
2. `pull_upstream_finds_propagate_targets` - verifies targets are collected from workspaces sharing same repo + base branch
3. `pull_upstream_dialog_shows_target_count` - dialog state has correct propagate_targets

Follow existing test patterns for dialog behavior.

**Step 2: Run tests**

Run: `cargo test --lib -- tui::tests::pull_upstream -v`
Expected: PASS

**Step 3: Commit**

```bash
git add src/ui/tui/mod.rs
git commit -m "test: integration tests for pull upstream dialog"
```

---

### Task 13: Final precommit and cleanup

**Step 1: Run full precommit**

Run: `make precommit`
Expected: PASS

**Step 2: Review all changes**

Verify:
- Base tab appears in create dialog with three tabs
- Only repos without existing base tasks appear in Base tab picker
- Base task creation produces correct task.toml with path == repository_path
- `U` keybind opens pull upstream dialog on base worktrees
- Pull upstream runs git pull --ff-only
- After pull, propagation updates downstream workspaces
- Help modal and command palette show new entries

**Step 3: Final commit if any cleanup needed**

```bash
git commit -m "chore: cleanup after base task + upstream sync implementation"
```

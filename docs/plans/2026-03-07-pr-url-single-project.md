# PR URL Single-Project Create Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `From PR URL` a single-project-only task creation flow that matches repo-scoped GitHub PR semantics.

**Architecture:** Keep `Manual` mode as the only multi-project task creation path. In the create dialog state and rendering, PR mode should use only the primary selected project. In the create confirmation path, PR mode should always construct a one-repository `CreateTaskRequest`, preserving existing PR URL parsing and repo-match validation.

**Tech Stack:** Rust, FrankenTUI, existing Grove task lifecycle and TUI test harness

---

### Task 1: Constrain Create Dialog State In PR Mode

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_setup.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a dialog behavior test asserting that PR mode uses only the currently selected project and does not depend on `selected_repository_indices`.

```rust
#[test]
fn create_dialog_pr_mode_uses_single_selected_project() {
    let mut app = GroveApp::for_test();
    app.open_create_dialog();
    let dialog = app.create_dialog_mut().expect("dialog should open");
    dialog.tab = CreateDialogTab::PullRequest;
    dialog.project_index = 1;
    dialog.selected_repository_indices = vec![0, 1];

    let repositories = app.selected_create_dialog_projects();

    assert_eq!(repositories.len(), 1);
    assert_eq!(repositories[0].name, app.projects[1].name);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_pr_mode_uses_single_selected_project -- --exact`
Expected: FAIL because PR mode still returns all included repositories.

**Step 3: Write minimal implementation**

Update create-dialog project selection helpers so PR mode always resolves to exactly one project, the current `project_index`.

```rust
pub(super) fn selected_create_dialog_projects(&self) -> Vec<ProjectConfig> {
    let Some(dialog) = self.create_dialog() else {
        return Vec::new();
    };

    match dialog.tab {
        CreateDialogTab::Manual => dialog
            .selected_repository_indices
            .iter()
            .filter_map(|index| self.projects.get(*index).cloned())
            .collect(),
        CreateDialogTab::PullRequest => self
            .projects
            .get(dialog.project_index)
            .cloned()
            .into_iter()
            .collect(),
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_pr_mode_uses_single_selected_project -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/dialogs/state.rs src/ui/tui/dialogs/dialogs_create_setup.rs src/ui/tui/mod.rs
git commit -m "refactor: make pr create mode single project"
```

### Task 2: Remove Multi-Project UI From The PR Tab

**Files:**
- Modify: `src/ui/tui/view/view_overlays_create.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a rendering test asserting the PR tab no longer shows the `Included` row.

```rust
#[test]
fn create_dialog_pr_mode_hides_included_projects_row() {
    let mut app = GroveApp::for_test();
    app.open_create_dialog();
    let dialog = app.create_dialog_mut().expect("dialog should open");
    dialog.tab = CreateDialogTab::PullRequest;

    let rendered = render_create_dialog_for_test(&app);

    assert!(!rendered.contains("[Included]"));
    assert!(rendered.contains("[Project]"));
    assert!(rendered.contains("[PR URL]"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_pr_mode_hides_included_projects_row -- --exact`
Expected: FAIL because the PR tab still renders `Included`.

**Step 3: Write minimal implementation**

Remove the `Included` row from the PR tab branch in the create overlay renderer. Keep `Included` in manual mode only. Update hint text if it still implies multi-project behavior in PR mode.

```rust
CreateDialogTab::PullRequest => {
    lines.push(project_row(...));
    lines.push(pr_url_row(...));
    lines.push(name_row(...));
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_pr_mode_hides_included_projects_row -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_overlays_create.rs src/ui/tui/mod.rs
git commit -m "refactor: remove multi project ui from pr create tab"
```

### Task 3: Tighten Create Confirmation Around Single-Project PR Mode

**Files:**
- Modify: `src/ui/tui/update/update_lifecycle_create.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a test asserting PR mode confirmation builds a create request with exactly one repository, even if manual-mode selection state contains extras.

```rust
#[test]
fn confirm_create_pr_mode_submits_single_repository_request() {
    let mut app = GroveApp::for_test();
    app.open_create_dialog();
    let dialog = app.create_dialog_mut().expect("dialog should open");
    dialog.tab = CreateDialogTab::PullRequest;
    dialog.project_index = 0;
    dialog.selected_repository_indices = vec![0, 1];
    dialog.pr_url = "https://github.com/owner/repo/pull/123".to_string();

    let repositories = app.selected_create_dialog_projects();

    assert_eq!(repositories.len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test confirm_create_pr_mode_submits_single_repository_request -- --exact`
Expected: FAIL because the selection helper still leaks multi-project state or the confirmation path does not enforce the contract clearly enough.

**Step 3: Write minimal implementation**

Make the PR-mode confirmation path clearly repo-scoped:

- resolve exactly one selected project
- keep existing PR URL parsing
- keep existing selected-project vs PR-repo validation
- log repository count as `1` for PR mode

If useful, replace the generic `selected_create_dialog_projects()` call with explicit branching in `confirm_create_dialog`.

```rust
let repositories = match dialog.tab {
    CreateDialogTab::Manual => self.selected_create_dialog_projects(),
    CreateDialogTab::PullRequest => vec![project.clone()],
};
```

**Step 4: Run test to verify it passes**

Run: `cargo test confirm_create_pr_mode_submits_single_repository_request -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/update/update_lifecycle_create.rs src/ui/tui/mod.rs
git commit -m "refactor: enforce single repo pr task creation"
```

### Task 4: Align Labels And Copy With The New Model

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/view/view_overlays_create.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

Add a test for the PR tab label and visible copy to match the single-project model.

```rust
#[test]
fn create_dialog_pr_tab_uses_repo_scoped_copy() {
    assert_eq!(CreateDialogTab::PullRequest.label(), "From GitHub PR");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_pr_tab_uses_repo_scoped_copy -- --exact`
Expected: FAIL because the current label is `From PR URL`.

**Step 3: Write minimal implementation**

Update the tab label and any nearby hint text to reflect that this mode bootstraps a task from a GitHub PR for one project.

```rust
Self::PullRequest => "From GitHub PR",
```

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_pr_tab_uses_repo_scoped_copy -- --exact`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/dialogs/state.rs src/ui/tui/view/view_overlays_create.rs src/ui/tui/mod.rs
git commit -m "refactor: clarify pr create dialog copy"
```

### Task 5: Run Focused Validation

**Files:**
- Test: `src/ui/tui/mod.rs`

**Step 1: Run focused TUI tests**

Run the targeted create-dialog tests added or modified above.

```bash
cargo test create_dialog_pr_mode_uses_single_selected_project -- --exact
cargo test create_dialog_pr_mode_hides_included_projects_row -- --exact
cargo test confirm_create_pr_mode_submits_single_repository_request -- --exact
cargo test create_dialog_pr_tab_uses_repo_scoped_copy -- --exact
```

Expected: PASS

**Step 2: Run required local validation**

Run: `make precommit`
Expected: PASS

**Step 3: Commit**

```bash
git add src/ui/tui/mod.rs
git commit -m "test: validate pr create single project flow"
```

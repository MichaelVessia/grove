# Workspace Jump Fast Switching Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep Grove's current `/` workspace jump UX, but tune it for faster switching via safer empty-query behavior, MRU-aware ordering, and more scanable rows.

**Architecture:** Reuse the existing `CommandPalette`-backed workspace jump path. Add lightweight workspace-visit recency state to the Grove TUI model, update it from the existing workspace-selection flow, and reshape workspace jump action titles and descriptions so repo or worktree identity dominates both ranking and scan speed. Avoid replacing the widget or forking FrankenTUI.

**Tech Stack:** Rust, Grove TUI update/view modules, FrankenTUI `CommandPalette`, `cargo test`, `make precommit`

---

## File Structure

**Modify:**

- `src/ui/tui/model.rs`
  Add workspace-visit recency state to the app model.
- `src/ui/tui/bootstrap/bootstrap_app.rs`
  Initialize the new recency state from the selected workspace on startup.
- `src/ui/tui/update/update_input_key_events.rs`
  Update workspace-selection change handling to record visits.
- `src/ui/tui/update/update_lifecycle_workspace_refresh.rs`
  Prune recency state after refresh so removed workspaces do not linger.
- `src/ui/tui/update/update_navigation_palette.rs`
  Rebuild workspace jump action ordering and row text for fast switching.
- `src/ui/tui/mod.rs`
  Add and update regression tests for empty-query no-op, MRU ordering, repo-prefix bias, and row text.

**Do not modify unless blocked:**

- `.reference/frankentui/...`
  This refinement should stay inside Grove's current ftui usage.

## Task 1: Add MRU State And Safe Empty-Query Behavior

**Files:**

- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/bootstrap/bootstrap_app.rs`
- Modify: `src/ui/tui/update/update_input_key_events.rs`
- Modify: `src/ui/tui/update/update_lifecycle_workspace_refresh.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests for empty-query no-op and recency tracking**

Add these tests near the existing workspace-jump tests in `src/ui/tui/mod.rs`:

```rust
    #[test]
    fn workspace_jump_empty_query_enter_is_noop() {
        let mut app = fixture_app();
        let selected_before = app
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.clone());
        let mode_before = app.state.mode;

        app.open_workspace_jump_palette();

        let (quit, _) = app.handle_key(key_press(KeyCode::Enter));

        assert!(!quit);
        assert!(!app.dialogs.command_palette.is_visible());
        assert_eq!(app.dialogs.palette_mode, None);
        assert_eq!(
            app.state
                .selected_workspace()
                .map(|workspace| workspace.path.clone()),
            selected_before
        );
        assert_eq!(app.state.mode, mode_before);
    }

    #[test]
    fn workspace_jump_orders_non_current_results_by_mru() {
        let mut app = fixture_app();
        let main_path = main_workspace_path();
        let feature_path = feature_workspace_path();

        let _ = app.state.select_workspace_path(feature_path.as_path());
        app.handle_workspace_selection_changed();
        let _ = app.state.select_workspace_path(main_path.as_path());
        app.handle_workspace_selection_changed();

        app.open_workspace_jump_palette();

        let visible_ids: Vec<String> = app
            .dialogs
            .command_palette
            .results()
            .map(|item| item.action.id.clone())
            .collect();

        assert_eq!(
            visible_ids.get(0).and_then(|id| {
                app.dialogs.workspace_jump_action_targets.get(id)
            }),
            Some(&main_path)
        );
        assert_eq!(
            visible_ids.get(1).and_then(|id| {
                app.dialogs.workspace_jump_action_targets.get(id)
            }),
            Some(&feature_path)
        );
    }
```

- [ ] **Step 2: Run the new tests and verify they fail**

Run:

```bash
cargo test workspace_jump_empty_query_enter_is_noop workspace_jump_orders_non_current_results_by_mru --lib -- --nocapture
```

Expected:

```text
one test fails because Enter on empty query still focuses preview for the current workspace
one test fails because non-current results still follow static workspace order
```

- [ ] **Step 3: Add workspace recency state to the TUI model**

Update `src/ui/tui/model.rs` by adding a visit-order field to `GroveApp` state:

```rust
    workspace_visit_order: Vec<PathBuf>,
```

Keep the representation simple:

- index `0` is most recent
- no duplicates
- current workspace may appear at index `0`

This field should live alongside other UI-derived state in the main TUI model, not in `AppState`, because it is a runtime ranking concern, not canonical task data.

- [ ] **Step 4: Initialize recency state during bootstrap**

Update `src/ui/tui/bootstrap/bootstrap_app.rs` so the initial selected workspace seeds the visit order:

```rust
        let initial_visit_order = state
            .selected_workspace()
            .map(|workspace| vec![workspace.path.clone()])
            .unwrap_or_default();
```

Then assign it in the app construction:

```rust
            workspace_visit_order: initial_visit_order,
```

- [ ] **Step 5: Record visits from the existing workspace-selection hook**

Update `src/ui/tui/update/update_input_key_events.rs` by adding a helper and calling it at the top of `handle_workspace_selection_changed()`:

```rust
    fn record_selected_workspace_visit(&mut self) {
        let Some(path) = self.selected_workspace_path() else {
            return;
        };

        self.workspace_visit_order
            .retain(|existing| !refer_to_same_location(existing.as_path(), path.as_path()));
        self.workspace_visit_order.insert(0, path);
    }
```

Call it here:

```rust
    pub(super) fn handle_workspace_selection_changed(&mut self) {
        self.record_selected_workspace_visit();
        if self.session.interactive.is_some() {
            self.exit_interactive_to_list();
        }
        // existing body continues
    }
```

This keeps mouse, keyboard, and palette-driven workspace changes on the same recency path.

- [ ] **Step 6: Prune recency state after workspace refresh**

Update both refresh completion paths in `src/ui/tui/update/update_lifecycle_workspace_refresh.rs` to keep only still-present paths:

```rust
        self.workspace_visit_order.retain(|path| {
            self.state.workspaces.iter().any(|workspace| {
                refer_to_same_location(workspace.path.as_path(), path.as_path())
            })
        });
        if let Some(path) = self.selected_workspace_path() {
            self.workspace_visit_order
                .retain(|existing| !refer_to_same_location(existing.as_path(), path.as_path()));
            self.workspace_visit_order.insert(0, path);
        }
```

- [ ] **Step 7: Make empty-query Enter a true no-op**

Update `src/ui/tui/update/update_navigation_palette.rs` so executing the selected workspace while the workspace jump query is empty dismisses the palette without changing focus or preview state.

Use a guard like:

```rust
        if self.dialogs.command_palette.query().trim().is_empty() && already_selected {
            self.dialogs.command_palette.close();
            self.dialogs.palette_mode = None;
            return false;
        }
```

- [ ] **Step 8: Run the focused tests and verify they pass**

Run:

```bash
cargo test workspace_jump_empty_query_enter_is_noop workspace_jump_orders_non_current_results_by_mru --lib -- --nocapture
```

Expected:

```text
2 passed
```

- [ ] **Step 9: Commit**

```bash
git add src/ui/tui/model.rs src/ui/tui/bootstrap/bootstrap_app.rs src/ui/tui/update/update_input_key_events.rs src/ui/tui/update/update_lifecycle_workspace_refresh.rs src/ui/tui/update/update_navigation_palette.rs src/ui/tui/mod.rs
git commit -m "fix(tui): add mru state for workspace jump"
```

## Task 2: Retune Action Construction For Fast Switching

**Files:**

- Modify: `src/ui/tui/update/update_navigation_palette.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests for ranking bias and row shape**

Add these tests in `src/ui/tui/mod.rs` near the existing fuzzy-matching coverage:

```rust
    #[test]
    fn workspace_jump_repo_prefix_beats_branch_only_match() {
        let mut app = fixture_app();

        app.open_workspace_jump_palette();
        app.dialogs.command_palette.set_query("gro");

        let selected = app
            .dialogs
            .command_palette
            .selected_action()
            .expect("selected result");

        let selected_path = app
            .dialogs
            .workspace_jump_action_targets
            .get(selected.id.as_str())
            .expect("selected path");

        assert_eq!(selected_path, &main_workspace_path());
    }

    #[test]
    fn workspace_jump_rows_prioritize_repo_and_branch_in_visible_text() {
        let mut app = fixture_app();

        app.open_workspace_jump_palette();

        let selected = app
            .dialogs
            .command_palette
            .selected_action()
            .expect("selected result");

        assert!(
            selected.title.contains("grove"),
            "repo or worktree name should stay in the title: {}",
            selected.title
        );
        assert!(
            selected.title.contains("master"),
            "branch should stay in the title for quick scanning: {}",
            selected.title
        );
        assert!(
            selected
                .description
                .as_deref()
                .is_some_and(|description| !description.is_empty()),
            "task context should stay visible in description"
        );
    }
```

- [ ] **Step 2: Run the tests and verify they fail**

Run:

```bash
cargo test workspace_jump_repo_prefix_beats_branch_only_match workspace_jump_rows_prioritize_repo_and_branch_in_visible_text --lib -- --nocapture
```

Expected:

```text
ranking test may fail because current title construction gives equal weight to branch and path terms
row-shape test fails because the current title is overloaded with all searchable terms
```

- [ ] **Step 3: Split visible text from searchable terms**

Refactor `src/ui/tui/update/update_navigation_palette.rs` to stop using one overloaded title builder for every purpose.

Replace `workspace_jump_action_title()` with helpers shaped like:

```rust
    fn workspace_jump_visible_title(&self, workspace: &Workspace) -> String {
        format!("{} · {}", workspace.name, workspace.branch)
    }

    fn workspace_jump_visible_description(&self, workspace: &Workspace) -> String {
        workspace
            .task_slug
            .as_deref()
            .and_then(|task_slug| self.state.tasks.iter().find(|task| task.slug == task_slug))
            .map(|task| task.name.clone())
            .unwrap_or_else(|| workspace.path.display().to_string())
    }
```

Keep path searchable through tags or hidden search text, not by bloating the visible title.

- [ ] **Step 4: Bias search toward repo and task identity**

When constructing each `PaletteActionItem`, put stronger identifiers first in the title and tags:

```rust
            let title = self.workspace_jump_visible_title(workspace);
            let description = self.workspace_jump_visible_description(workspace);
            actions.push(Self::palette_action(
                id,
                title,
                description,
                &[
                    workspace.name.as_str(),
                    workspace.project_name.as_deref().unwrap_or(""),
                    workspace.branch.as_str(),
                    workspace.task_slug.as_deref().unwrap_or(""),
                    workspace
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(""),
                ],
                "Workspace",
            ));
```

Do not reintroduce the full path into the visible title. If extra path searchability is needed, add basename and relevant tokens to tags.

- [ ] **Step 5: Order actions current-first, then MRU, then remaining workspaces**

Refactor `build_workspace_jump_actions()` so workspace iteration order is:

1. current workspace
2. remaining workspaces in `workspace_visit_order`
3. all remaining workspaces in stable existing order

Use a path set to avoid duplicates:

```rust
        let mut ordered_paths = Vec::new();
        if let Some(current) = self.selected_workspace_path() {
            ordered_paths.push(current);
        }
        for path in &self.workspace_visit_order {
            if !ordered_paths.iter().any(|existing| {
                refer_to_same_location(existing.as_path(), path.as_path())
            }) {
                ordered_paths.push(path.clone());
            }
        }
```

Then append any workspace paths not already present by walking `self.state.workspaces`.

- [ ] **Step 6: Run the focused ranking and presentation tests**

Run:

```bash
cargo test workspace_jump_repo_prefix_beats_branch_only_match workspace_jump_rows_prioritize_repo_and_branch_in_visible_text --lib -- --nocapture
```

Expected:

```text
2 passed
```

- [ ] **Step 7: Commit**

```bash
git add src/ui/tui/update/update_navigation_palette.rs src/ui/tui/mod.rs
git commit -m "fix(tui): tune workspace jump for fast switching"
```

## Task 3: Run Full Workspace Jump Regression Slice And Final Validation

**Files:**

- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/update/update_navigation_palette.rs`
- Modify: `src/ui/tui/update/update_input_key_events.rs`
- Modify: `src/ui/tui/update/update_lifecycle_workspace_refresh.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/bootstrap/bootstrap_app.rs`

- [ ] **Step 1: Add a refresh-pruning regression**

Add this test in `src/ui/tui/mod.rs` if no existing refresh test covers it:

```rust
    #[test]
    fn workspace_jump_refresh_prunes_missing_mru_paths() {
        let mut app = fixture_app();
        app.workspace_visit_order = vec![
            PathBuf::from("/tmp/missing-workspace"),
            feature_workspace_path(),
        ];

        app.refresh_workspaces_sync_with_root(None, None);

        assert!(app.workspace_visit_order.iter().all(|path| {
            app.state.workspaces.iter().any(|workspace| {
                refer_to_same_location(workspace.path.as_path(), path.as_path())
            })
        }));
    }
```

- [ ] **Step 2: Run the workspace jump regression slice**

Run:

```bash
cargo test workspace_jump --lib
```

Expected:

```text
all workspace_jump tests pass
```

- [ ] **Step 3: Run required local validation**

Run:

```bash
make precommit
```

Expected:

```text
cargo fmt --check, cargo check, and cargo clippy succeed
```

- [ ] **Step 4: Commit**

```bash
git add src/ui/tui/mod.rs src/ui/tui/update/update_navigation_palette.rs src/ui/tui/update/update_input_key_events.rs src/ui/tui/update/update_lifecycle_workspace_refresh.rs src/ui/tui/model.rs src/ui/tui/bootstrap/bootstrap_app.rs
git commit -m "test(tui): cover workspace jump fast switching"
```

## Self-Review

- Spec coverage:
  - safe empty-query behavior is covered in Task 1
  - MRU ordering is covered in Task 1 and Task 3
  - stronger repo or task bias and shorter rows are covered in Task 2
  - final validation is covered in Task 3
- Placeholder scan:
  - no `TODO`, `TBD`, or "implement later" placeholders remain
- Type consistency:
  - recency state is consistently referred to as `workspace_visit_order`
  - selection updates route through `handle_workspace_selection_changed()`

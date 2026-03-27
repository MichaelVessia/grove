# FTUI FocusGraph Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Grove's manual pane and dialog focus state with FTUI `FocusManager` / `FocusGraph`, including modal focus traps, pane spatial navigation, and dialog-internal focus.

**Architecture:** Add one `FocusManager` to `GroveApp`, define stable `FocusId` constants in `shared.rs`, and migrate focus in two layers. First, bridge old and new systems so tests can lock in behavior while the graph and modal traps land. Then switch read and write paths over to FTUI focus helpers, remove `PaneFocus` and dialog `focused_field` enums, and keep hit-testing and replay compatibility only at the boundaries that still need them.

**Tech Stack:** Rust, Grove TUI modules, FrankenTUI focus widgets (`FocusManager`, `FocusGraph`, `FocusNode`, `NavDirection`), targeted `cargo test`, `make precommit`.

---

## Chunk 1: Main Focus Infrastructure

### Task 1: Add stable focus IDs and a shadow `FocusManager`

**Files:**
- Modify: `src/ui/tui/shared.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add focused tests in `src/ui/tui/mod.rs` for:
- main app boot creates focus on workspace list by default
- toggling pane focus keeps FTUI focus state aligned with `PaneFocus`
- clicking preview moves FTUI focus to preview
- clicking sidebar moves FTUI focus back to workspace list

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test focus_manager_shadow_ -- --nocapture`

Expected: FAIL because Grove has no `FocusManager` or `FocusId` layer yet.

- [ ] **Step 3: Write the minimal implementation**

In `src/ui/tui/shared.rs`:
- add `FOCUS_ID_WORKSPACE_LIST`
- add `FOCUS_ID_PREVIEW`
- add `FOCUS_GROUP_MAIN_PANES`
- reserve grouped ID ranges or named constants for each dialog family

In `src/ui/tui/model.rs`:
- import FTUI focus types
- add `focus_manager: FocusManager` to `GroveApp`
- add helpers to:
  - build the two-pane graph
  - sync `PaneFocus -> FocusManager`
  - sync `FocusManager -> PaneFocus`
  - expose `workspace_list_focused()` and `preview_focused()`
- initialize focus to workspace list during app bootstrap

In `src/ui/tui/mod.rs` test helpers:
- expose or assert focus through helper methods instead of peeking directly at
  FTUI internals everywhere

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test focus_manager_shadow_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit the infrastructure**

```bash
git add src/ui/tui/shared.rs src/ui/tui/model.rs src/ui/tui/mod.rs
git commit -m "feat: add shadow ftui focus manager"
```

### Task 2: Rebuild pane bounds and focusability from live layout

**Files:**
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/view/view_chrome_sidebar/build.rs`
- Modify: `src/ui/tui/view/view_preview.rs`
- Modify: `src/ui/tui/update/update_navigation_commands.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add regressions covering:
- sidebar and preview nodes get updated bounds after resize
- hidden sidebar makes workspace-list focus unavailable
- restoring the sidebar makes workspace-list focusable again

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test focus_bounds_ sidebar_hidden_focus_ -- --nocapture`

Expected: FAIL because the graph does not yet track live layout geometry or
sidebar visibility.

- [ ] **Step 3: Write the minimal implementation**

In `src/ui/tui/model.rs`:
- store or derive the latest pane rectangles after layout
- update `FocusNode.bounds` for list and preview whenever layout changes
- set the list node `is_focusable` to `false` when `sidebar_hidden` is true
- if current focus becomes invalid because the sidebar hid, move focus to
  preview

In `src/ui/tui/view/view_chrome_sidebar/build.rs` and
`src/ui/tui/view/view_preview.rs`:
- reuse the same layout-derived areas to keep hit testing and focus bounds in
  sync

In `src/ui/tui/update/update_navigation_commands.rs`:
- when toggling sidebar visibility, refresh focus node state immediately

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test focus_bounds_ sidebar_hidden_focus_ -- --nocapture`

Expected: PASS.

## Chunk 2: Modal Traps With Legacy Dialog State Still Present

### Task 3: Add modal groups and focus-trap lifecycle for every dialog family

**Files:**
- Modify: `src/ui/tui/shared.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/dialogs/dialogs.rs`
- Modify: `src/ui/tui/dialogs/dialogs_confirm.rs`
- Modify: `src/ui/tui/dialogs/dialogs_delete.rs`
- Modify: `src/ui/tui/dialogs/dialogs_edit.rs`
- Modify: `src/ui/tui/dialogs/dialogs_launch.rs`
- Modify: `src/ui/tui/dialogs/dialogs_merge.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_pull_upstream.rs`
- Modify: `src/ui/tui/dialogs/dialogs_rename_tab.rs`
- Modify: `src/ui/tui/dialogs/dialogs_session_cleanup.rs`
- Modify: `src/ui/tui/dialogs/dialogs_settings.rs`
- Modify: `src/ui/tui/dialogs/dialogs_stop.rs`
- Modify: `src/ui/tui/dialogs/dialogs_update_from_base.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add regressions for:
- opening each dialog traps focus inside that dialog
- closing each dialog restores prior pane focus
- nested project add/defaults dialogs trap over the parent project dialog
- `Tab` and `Shift+Tab` stay inside the trapped group

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test modal_focus_trap_ dialog_focus_restore_ project_dialog_focus_trap_ -- --nocapture`

Expected: FAIL because dialogs do not yet create focus groups or traps.

- [ ] **Step 3: Write the minimal implementation**

In `src/ui/tui/shared.rs`:
- add stable group IDs for each dialog family
- add stable `FocusId` constants for all currently focusable dialog controls

In `src/ui/tui/model.rs`:
- add helpers to register/remove dialog nodes
- add helpers to create groups and push/pop traps
- centralize "dialog opened" and "dialog closed" focus lifecycle so each dialog
  file does not hand-roll trap handling

In the dialog open/close modules listed above:
- register the right focus IDs when the dialog opens
- focus the first logical field
- pop the trap and remove nodes when it closes
- keep existing `focused_field` enums temporarily so behavior stays stable while
  reads are migrated later

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test modal_focus_trap_ dialog_focus_restore_ project_dialog_focus_trap_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit the modal-trap layer**

```bash
git add src/ui/tui/shared.rs src/ui/tui/model.rs src/ui/tui/dialogs/dialogs.rs src/ui/tui/dialogs/dialogs_confirm.rs src/ui/tui/dialogs/dialogs_delete.rs src/ui/tui/dialogs/dialogs_edit.rs src/ui/tui/dialogs/dialogs_launch.rs src/ui/tui/dialogs/dialogs_merge.rs src/ui/tui/dialogs/dialogs_projects_state.rs src/ui/tui/dialogs/dialogs_pull_upstream.rs src/ui/tui/dialogs/dialogs_rename_tab.rs src/ui/tui/dialogs/dialogs_session_cleanup.rs src/ui/tui/dialogs/dialogs_settings.rs src/ui/tui/dialogs/dialogs_stop.rs src/ui/tui/dialogs/dialogs_update_from_base.rs src/ui/tui/mod.rs
git commit -m "feat: trap dialog focus with ftui groups"
```

## Chunk 3: Migrate Main-Pane Reads And Writes

### Task 4: Switch pane behavior from `PaneFocus` checks to focus helpers

**Files:**
- Modify: `src/ui/tui/update/update_input_key_events.rs`
- Modify: `src/ui/tui/update/update_navigation_commands.rs`
- Modify: `src/ui/tui/update/update_navigation_palette.rs`
- Modify: `src/ui/tui/update/update_polling_capture_diff.rs`
- Modify: `src/ui/tui/update/update_polling_state.rs`
- Modify: `src/ui/tui/update/update_lifecycle_create.rs`
- Modify: `src/ui/tui/update/update_lifecycle_stop.rs`
- Modify: `src/ui/tui/update/update_input_interactive.rs`
- Modify: `src/ui/tui/terminal/preview_stream.rs`
- Modify: `src/ui/tui/view/view_chrome_sidebar/build.rs`
- Modify: `src/ui/tui/view/view_chrome_sidebar/render.rs`
- Modify: `src/ui/tui/view/view_preview.rs`
- Modify: `src/ui/tui/view/view_status.rs`
- Modify: `src/ui/tui/performance.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add or update regressions for:
- command palette enablement still respects list vs preview focus
- preview polling still activates only when preview is focused
- status line reports focus correctly
- clicking or keyboarding into preview still acknowledges attention as before

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test preview_focus_ command_palette_focus_ status_focus_ -- --nocapture`

Expected: FAIL when old `PaneFocus` checks are replaced or diverge from FTUI
focus state.

- [ ] **Step 3: Write the minimal implementation**

- replace `self.state.focus == PaneFocus::...` read sites with
  `workspace_list_focused()` / `preview_focused()`
- keep `PaneFocus` dual-written for now so replay and untouched code still work
- route `ToggleFocus`, `FocusPreview`, and `FocusList` through `FocusManager`
  first, then sync legacy state as a bridge

- [ ] **Step 4: Add left/right pane navigation**

In `src/ui/tui/update/update_input_key_events.rs` and
`src/ui/tui/update/update_navigation_commands.rs`:
- wire non-interactive left/right pane navigation through
  `focus_manager.navigate(NavDirection::Left/Right)`
- preserve current interactive-mode bypass so tmux still owns arrow keys there
- keep `Tab`/`Shift+Tab` behavior consistent with the two-node main pane group

- [ ] **Step 5: Run the targeted tests to verify they pass**

Run: `cargo test preview_focus_ command_palette_focus_ status_focus_ pane_arrow_navigation_ -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit the pane migration**

```bash
git add src/ui/tui/update/update_input_key_events.rs src/ui/tui/update/update_navigation_commands.rs src/ui/tui/update/update_navigation_palette.rs src/ui/tui/update/update_polling_capture_diff.rs src/ui/tui/update/update_polling_state.rs src/ui/tui/update/update_lifecycle_create.rs src/ui/tui/update/update_lifecycle_stop.rs src/ui/tui/update/update_input_interactive.rs src/ui/tui/terminal/preview_stream.rs src/ui/tui/view/view_chrome_sidebar/build.rs src/ui/tui/view/view_chrome_sidebar/render.rs src/ui/tui/view/view_preview.rs src/ui/tui/view/view_status.rs src/ui/tui/performance.rs src/ui/tui/mod.rs
git commit -m "refactor: drive pane focus through ftui"
```

## Chunk 4: Migrate Dialog-Internal Focus

### Task 5: Convert simple dialogs from `focused_field` enums to FTUI focus IDs

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_confirm.rs`
- Modify: `src/ui/tui/dialogs/dialogs_delete.rs`
- Modify: `src/ui/tui/dialogs/dialogs_merge.rs`
- Modify: `src/ui/tui/dialogs/dialogs_pull_upstream.rs`
- Modify: `src/ui/tui/dialogs/dialogs_session_cleanup.rs`
- Modify: `src/ui/tui/dialogs/dialogs_settings.rs`
- Modify: `src/ui/tui/dialogs/dialogs_stop.rs`
- Modify: `src/ui/tui/dialogs/dialogs_update_from_base.rs`
- Modify: `src/ui/tui/view/view_overlays_confirm.rs`
- Modify: `src/ui/tui/view/view_overlays_session_cleanup.rs`
- Modify: `src/ui/tui/view/view_overlays_settings.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_delete.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_merge.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_stop.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_update.rs`
- Modify: `src/ui/tui/view/view_overlays_pull_upstream.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add regressions proving:
- button highlight and toggle highlight come from FTUI focus state
- `Tab`, `Shift+Tab`, arrows, and space still operate the same visible control
- dialog submit/cancel actions fire from the FTUI-focused control

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test confirm_dialog_focus_ delete_dialog_focus_ settings_dialog_focus_ session_cleanup_focus_ -- --nocapture`

Expected: FAIL because view and key logic still read `focused_field`.

- [ ] **Step 3: Write the minimal implementation**

In `src/ui/tui/dialogs/state.rs`:
- delete the simple dialog `focused_field` fields and their cyclic enums when a
  dialog is fully migrated
- keep non-focus dialog business state only

In the dialog key handlers and overlay renderers:
- replace `dialog.focused_field == ...` with `dialog_field_focused(...)`
- route next/previous traversal through `focus_next()` / `focus_prev()`
- route button actions through the current `FocusId`

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test confirm_dialog_focus_ delete_dialog_focus_ settings_dialog_focus_ session_cleanup_focus_ -- --nocapture`

Expected: PASS.

### Task 6: Convert text-input dialogs and project dialogs to FTUI focus-driven input syncing

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_key.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_setup.rs`
- Modify: `src/ui/tui/dialogs/dialogs_edit.rs`
- Modify: `src/ui/tui/dialogs/dialogs_launch.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_defaults.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_key.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_search.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_rename_tab.rs`
- Modify: `src/ui/tui/update/update_input_mouse.rs`
- Modify: `src/ui/tui/view/view_overlays_create.rs`
- Modify: `src/ui/tui/view/view_overlays_edit.rs`
- Modify: `src/ui/tui/view/view_overlays_projects.rs`
- Modify: `src/ui/tui/view/view_overlays_rename_tab.rs`
- Modify: `src/ui/tui/view/view_overlays_workspace_launch.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add regressions for:
- focused text input gets cursor and accepts typed characters
- switching fields moves text-input focus correctly
- project add dialog path/name inputs sync to FTUI focus
- project defaults dialog input focus and save/cancel buttons work
- nested project dialogs restore focus to the parent project dialog on close
- create dialog tab-specific field order still matches current behavior

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test create_dialog_focus_ launch_dialog_focus_ project_add_dialog_focus_ project_defaults_dialog_focus_ rename_tab_dialog_focus_ -- --nocapture`

Expected: FAIL because text inputs still derive focus from local enums and
manual `sync_focus()` calls.

- [ ] **Step 3: Write the minimal implementation**

In `src/ui/tui/dialogs/state.rs`:
- remove migrated `focused_field` members
- delete now-dead cyclic focus enums and traversal helpers
- keep non-focus state like text values, selected project indices, and picker
  state

In the dialog handlers:
- derive active field from current `FocusId`
- sync `TextInput::set_focused(...)` from focus-manager helpers
- keep special-case field order where the existing UX depends on it
  (`LaunchDialogField`, `CreateDialogField`, nested project dialogs)

In `src/ui/tui/update/update_input_mouse.rs`:
- clicking any dialog control should both perform the old action and focus the
  corresponding `FocusId`

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test create_dialog_focus_ launch_dialog_focus_ project_add_dialog_focus_ project_defaults_dialog_focus_ rename_tab_dialog_focus_ -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit the dialog migration**

```bash
git add src/ui/tui/dialogs/state.rs src/ui/tui/dialogs/dialogs_create_key.rs src/ui/tui/dialogs/dialogs_create_setup.rs src/ui/tui/dialogs/dialogs_edit.rs src/ui/tui/dialogs/dialogs_launch.rs src/ui/tui/dialogs/dialogs_projects_defaults.rs src/ui/tui/dialogs/dialogs_projects_key.rs src/ui/tui/dialogs/dialogs_projects_search.rs src/ui/tui/dialogs/dialogs_projects_state.rs src/ui/tui/dialogs/dialogs_rename_tab.rs src/ui/tui/update/update_input_mouse.rs src/ui/tui/view/view_overlays_create.rs src/ui/tui/view/view_overlays_edit.rs src/ui/tui/view/view_overlays_projects.rs src/ui/tui/view/view_overlays_rename_tab.rs src/ui/tui/view/view_overlays_workspace_launch.rs src/ui/tui/mod.rs
git commit -m "refactor: migrate dialog focus to ftui"
```

## Chunk 5: Delete Legacy Focus State And Finish Validation

### Task 7: Remove `PaneFocus`, bridge helpers, and replay-only legacy mapping

**Files:**
- Modify: `src/ui/state.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/logging/logging_state.rs`
- Modify: `src/ui/tui/replay/types/bootstrap.rs`
- Modify: `src/ui/tui/tasks.rs`
- Modify: `src/ui/tui/update/update_input_keybinding.rs`
- Modify: `src/ui/tui/mod.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add regressions for:
- no runtime behavior depends on `PaneFocus`
- replay bootstrap still restores the correct initial pane focus
- logging still records a useful focus label or name after the enum removal

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test replay_focus_ logging_focus_ pane_focus_removed_ -- --nocapture`

Expected: FAIL because runtime and replay paths still depend on `PaneFocus`.

- [ ] **Step 3: Write the minimal implementation**

- remove `PaneFocus` from `src/ui/state.rs`
- delete bridge helpers from `src/ui/tui/model.rs`
- update logging to emit focus labels from FTUI helper methods
- keep legacy replay decoding only at the bootstrap boundary if old fixtures
  still serialize pane focus names

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test replay_focus_ logging_focus_ pane_focus_removed_ -- --nocapture`

Expected: PASS.

### Task 8: Final verification and doc cleanup

**Files:**
- Modify: `docs/superpowers/specs/2026-03-26-ftui-focusgraph-design.md`
- Modify: `docs/superpowers/plans/2026-03-26-ftui-focusgraph.md`

- [ ] **Step 1: Run focused focus regression suites**

Run: `cargo test focus_manager_shadow_ focus_bounds_ modal_focus_trap_ dialog_focus_restore_ pane_arrow_navigation_ create_dialog_focus_ launch_dialog_focus_ project_add_dialog_focus_ project_defaults_dialog_focus_ replay_focus_ -- --nocapture`

Expected: PASS.

- [ ] **Step 2: Run required repo validation**

Run: `make precommit`

Expected: PASS.

- [ ] **Step 3: Update docs if implementation drifted**

If execution landed in slightly different files or required one extra helper,
update this plan and the design doc before handoff so they still match the code
that shipped.

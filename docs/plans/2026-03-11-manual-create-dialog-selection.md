# Manual Create Dialog Selection Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove the misleading manual-mode `Project` row so the create dialog only shows committed included-repo selection.

**Architecture:** Keep the existing create-dialog state shape. Reuse `CreateDialogField::Project` as the manual-mode `Included` focus target, but render only the committed `selected_repository_indices` summary in manual mode. Preserve single-project UI in PR and base flows.

**Tech Stack:** Rust, Grove TUI dialog state/update/view tests, FrankenTUI rendering.

---

### Task 1: Add regression tests for manual-mode rendering

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add a rendering test asserting manual mode does not show `[Project]`.
- Add a rendering test asserting manual mode shows `[Included]` with `Enter browse`.
- Add a rendering test asserting PR mode still shows `[Project]`.

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_manual_mode`

Expected: FAIL because manual mode still renders `[Project]`.

**Step 3: Write minimal implementation**

- Update the create overlay renderer so manual mode renders only an actionable
  `Included` row instead of separate `Project` and static `Included` rows.
- Keep PR mode unchanged.

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_manual_mode`

Expected: PASS

### Task 2: Preserve navigation and picker behavior

**Files:**
- Modify: `src/ui/tui/view/view_overlays_create.rs`
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_key.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing test**

- Add a regression test that focuses the manual-mode project-selection field,
  opens the picker, cancels it, and asserts the rendered `Included` summary is
  unchanged.

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_manual_included_picker_cancel_preserves_selection`

Expected: FAIL if manual-mode rendering still depends on picker cursor state.

**Step 3: Write minimal implementation**

- Reuse the existing focus slot for the actionable `Included` row.
- Keep picker cancel behavior unchanged.
- Update hint text if needed so manual mode copy refers to the actionable
  included-repo selection.

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_manual_included_picker_cancel_preserves_selection`

Expected: PASS

### Task 3: Validate locally

**Files:**
- Modify: `src/ui/tui/help_catalog.rs`

**Step 1: Update help copy if it still implies a separate manual-mode project row**

**Step 2: Run focused verification**

Run: `cargo test create_dialog_manual_mode`

Run: `cargo test create_dialog_manual_included_picker_cancel_preserves_selection`

**Step 3: Run required repo validation**

Run: `make precommit`

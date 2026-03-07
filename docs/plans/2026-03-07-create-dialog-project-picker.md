# Create Dialog Project Picker Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make project selection in the new-task dialog explicit and scalable by replacing the inline project field with a dedicated picker subview.

**Architecture:** Extend create-dialog state with an internal project-picker mode that reuses the existing project-dialog filtering and scrolling behavior. Keep the create modal shell, swap the body when the picker is active, and allow returning to the parent create form without losing user input.

**Tech Stack:** Rust, FrankenTUI modal rendering, existing Grove dialog state/update/view patterns.

---

### Task 1: Add failing tests for the picker UX

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add coverage for opening a project picker from the create dialog.
- Add coverage for selecting a project from the picker.
- Add coverage for the empty-state hint telling the user to use `p` first.

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_project_picker`

**Step 3: Write minimal implementation**

- Add create-dialog state for picker visibility, filter text, filtered indices, and selected row.
- Route key handling for picker mode before normal create-form handling.
- Render the picker subview with bounded visible rows and explicit hints.

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_project_picker`

### Task 2: Wire project selection and empty-state behavior

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_setup.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_key.rs`
- Modify: `src/ui/tui/view/view_overlays_create.rs`

**Step 1: Write the failing test**

- Add coverage that picker selection updates the create dialog project and branch defaults.

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_project_picker_selection_updates_defaults`

**Step 3: Write minimal implementation**

- Update create-dialog defaults and branch candidates when picker selection is confirmed.
- Show explicit action-oriented copy on the main form for the project row.

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_project_picker_selection_updates_defaults`

### Task 3: Validate and polish

**Files:**
- Modify: `src/ui/tui/view/view_overlays_help/keybind_overlay.rs`

**Step 1: Run focused verification**

Run: `cargo test create_dialog_project_picker`

**Step 2: Run repo minimum validation**

Run: `make precommit`

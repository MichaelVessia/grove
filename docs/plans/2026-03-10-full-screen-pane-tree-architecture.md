# Full Screen PaneTree Architecture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Grove's bespoke full-screen layout model with a single ftui `PaneTree` that owns header, workspace content, and status.

**Architecture:** Introduce a Grove-owned pane adapter with semantic pane roles layered on top of ftui `PaneTree`. Migrate render and hit-test code to solved pane-role rects, then delete `ViewLayout` and related top-level geometry helpers once parity is reached.

**Tech Stack:** Rust, ftui `PaneTree` and `PaneLayout`, Grove TUI render/update loop, focused TUI regression tests in `src/ui/tui/mod.rs`, `make precommit`.

---

### Task 1: Introduce semantic pane roles and a canonical full-screen pane tree

**Files:**
- Create: `src/ui/tui/panes.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/bootstrap/bootstrap_app.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - the app builds a canonical full-screen pane tree on bootstrap
  - all required semantic roles resolve to a pane id
  - normal and tiny viewports still solve into rects for required panes

**Step 2: Run test to verify it fails**

Run: `cargo test pane_tree_bootstrap`

Expected: FAIL because Grove has no pane model or semantic pane roles.

**Step 3: Write minimal implementation**

- Add `PaneRole` for:
  - `Header`
  - `Workspace`
  - `WorkspaceList`
  - `Preview`
  - `Status`
- Add `GrovePaneModel` that owns:
  - the canonical `PaneTree`
  - role-to-id lookup
  - a helper to solve layout for a viewport
- Initialize `GrovePaneModel` during app bootstrap.

**Step 4: Run test to verify it passes**

Run: `cargo test pane_tree_bootstrap`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/panes.rs src/ui/tui/model.rs src/ui/tui/bootstrap/bootstrap_app.rs src/ui/tui/mod.rs
git commit -m "feat: add canonical full-screen pane tree model"
```

### Task 2: Route top-level rendering through solved pane-role rects

**Files:**
- Modify: `src/ui/tui/view/view.rs`
- Modify: `src/ui/tui/view/view_layout.rs`
- Modify: `src/ui/tui/shared.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - header renders in the header pane rect
  - status renders in the status pane rect
  - workspace list and preview render in their solved pane rects

**Step 2: Run test to verify it fails**

Run: `cargo test pane_tree_render_layout`

Expected: FAIL because top-level rendering still depends on `ViewLayout`.

**Step 3: Write minimal implementation**

- Solve the pane tree once per frame.
- Replace top-level `view_layout_for_size(...)` use in render flow with
  semantic pane rect lookup.
- Keep current pane-local renderer behavior unchanged.
- Reduce `view_layout.rs` to only helpers that still apply, or delete code that
  is no longer needed.

**Step 4: Run test to verify it passes**

Run: `cargo test pane_tree_render_layout`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view.rs src/ui/tui/view/view_layout.rs src/ui/tui/shared.rs src/ui/tui/mod.rs
git commit -m "refactor: render grove through solved pane rects"
```

### Task 3: Replace top-level hit classification with pane-role lookup

**Files:**
- Modify: `src/ui/tui/view/view_layout.rs`
- Modify: `src/ui/tui/update/update_input_mouse.rs`
- Modify: `src/ui/tui/view/view_selection_mapping.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - points in solved header/status rects classify correctly
  - points in solved workspace-list and preview rects classify correctly
  - existing row/link hit-grid behavior still wins when available

**Step 2: Run test to verify it fails**

Run: `cargo test pane_tree_hit_regions`

Expected: FAIL because hit classification still uses legacy `ViewLayout`
rect math.

**Step 3: Write minimal implementation**

- Replace coarse hit-region detection with pane-role lookup from solved rects.
- Keep fine-grained hit-grid decoding for workspace rows, PR links, and preview
  local controls.
- Delete legacy top-level divider hit logic if no longer needed.

**Step 4: Run test to verify it passes**

Run: `cargo test pane_tree_hit_regions`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_layout.rs src/ui/tui/update/update_input_mouse.rs src/ui/tui/view/view_selection_mapping.rs src/ui/tui/mod.rs
git commit -m "refactor: use pane roles for top-level hit testing"
```

### Task 4: Migrate workspace and preview interactions to pane rect lookup

**Files:**
- Modify: `src/ui/tui/update/update_navigation_preview.rs`
- Modify: `src/ui/tui/update/update_input_mouse.rs`
- Modify: `src/ui/tui/view/view_preview_content.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - clicking a workspace row still selects the workspace
  - clicking preview tabs still switches tabs
  - preview scrolling still uses the preview pane rect
  - interactive preview entry still targets the solved preview pane

**Step 2: Run test to verify it fails**

Run: `cargo test pane_tree_workspace_preview_interactions`

Expected: FAIL because interaction code still derives geometry from
`ViewLayout`.

**Step 3: Write minimal implementation**

- Replace remaining `view_layout()` lookups in workspace and preview code with
  semantic pane rect helpers.
- Keep existing pane-local block inner calculations for row and tab placement.
- Remove obsolete assumptions tied to sidebar/divider geometry.

**Step 4: Run test to verify it passes**

Run: `cargo test pane_tree_workspace_preview_interactions`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/update/update_navigation_preview.rs src/ui/tui/update/update_input_mouse.rs src/ui/tui/view/view_preview_content.rs src/ui/tui/mod.rs
git commit -m "refactor: migrate workspace interactions to pane rects"
```

### Task 5: Remove ViewLayout and legacy top-level geometry helpers

**Files:**
- Modify: `src/ui/tui/view/view_layout.rs`
- Modify: `src/ui/tui/shared.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/mouse.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Delete dead code**

- Remove `ViewLayout`.
- Remove obsolete sidebar-width and divider-specific helpers if no longer used.
- Remove now-unused shared structs or imports.
- Keep only helpers still needed for pane-local behavior.

**Step 2: Run focused regression tests**

Run: `cargo test pane_tree_`

Expected: PASS

**Step 3: Run interaction regression tests touched by the migration**

Run: `cargo test mouse_`

Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/tui/view/view_layout.rs src/ui/tui/shared.rs src/ui/tui/model.rs src/ui/mouse.rs src/ui/tui/mod.rs
git commit -m "refactor: remove legacy full-screen layout helpers"
```

### Task 6: Validate overlays stay outside the pane tree

**Files:**
- Modify: `src/ui/tui/view/view.rs`
- Modify: `src/ui/tui/view/view_overlays_projects.rs`
- Modify: `src/ui/tui/view/view_overlays_help.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - dialogs still render on top of pane content
  - command palette still renders on top of panes
  - help and toast overlays remain unaffected by pane layout

**Step 2: Run test to verify it fails**

Run: `cargo test pane_tree_overlays`

Expected: FAIL if overlay ordering or viewport assumptions still depend on the
old layout path.

**Step 3: Write minimal implementation**

- Audit overlay entry points to ensure they still render against the full frame
  area, not pane-local rects.
- Fix any stale viewport assumptions uncovered by the tests.

**Step 4: Run test to verify it passes**

Run: `cargo test pane_tree_overlays`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view.rs src/ui/tui/view/view_overlays_projects.rs src/ui/tui/view/view_overlays_help.rs src/ui/tui/mod.rs
git commit -m "test: lock overlay behavior over pane tree layout"
```

### Task 7: Run repo minimum validation

**Files:**
- Modify: `src/ui/tui/panes.rs`
- Modify: `src/ui/tui/view/view.rs`
- Modify: `src/ui/tui/view/view_layout.rs`
- Modify: `src/ui/tui/update/update_input_mouse.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Run focused pane-tree regression suites**

Run: `cargo test pane_tree_`

Expected: PASS

**Step 2: Run the touched mouse and preview regression suites**

Run: `cargo test mouse_`

Expected: PASS

**Step 3: Run repo minimum validation**

Run: `make precommit`

Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/tui/panes.rs src/ui/tui/view/view.rs src/ui/tui/view/view_layout.rs src/ui/tui/update/update_input_mouse.rs src/ui/tui/mod.rs
git commit -m "refactor: migrate grove to full-screen pane tree layout"
```

# Workspace Sidebar Density Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compress the workspace sidebar to one line per workspace while preserving task headers, status readability, and correct mouse/selection behavior.

**Architecture:** Keep the existing task-grouped sidebar structure, but collapse workspace rows from a three-line block to a single rendered line. Task headers remain separate rows for both multi-workspace and single-workspace tasks, and status/PR metadata moves inline with right-edge priority rules. Update row mapping and tests together so hit-testing continues to match the rendered shape.

**Tech Stack:** Rust, Grove TUI, FrankenTUI `VirtualizedList`, existing sidebar render model and TUI behavior tests.

---

## Chunk 1: Sidebar Row Model

### Task 1: Shrink workspace row height to one rendered line

**Files:**
- Modify: `src/ui/tui/view/view_chrome_sidebar/build.rs`
- Modify: `src/ui/tui/view/view_chrome_sidebar/model.rs`
- Modify: `src/ui/tui/shared.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write failing render tests for one-line workspace rows**

Add or update sidebar tests in `src/ui/tui/mod.rs` to assert:
- a workspace row no longer renders a second metadata line
- a task with one workspace still renders a task header row
- a task with multiple workspaces renders one header plus one row per workspace

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test sidebar_ -- --nocapture`
Expected: existing sidebar row-count / row-content assertions fail until the builder is updated

- [ ] **Step 3: Update the sidebar builder to emit one line per workspace**

In `src/ui/tui/view/view_chrome_sidebar/build.rs`:
- keep task headers as separate `SidebarListLine::project(...)` rows
- collapse workspace name, inline metadata, and right-aligned status into a single `SidebarListLine::workspace(...)`
- remove the dedicated second workspace line and trailing blank spacer line
- stop repeating the task name inside child workspace rows
- keep branch / `(monorepo)` metadata inline when needed
- keep status priority as:
  - `WAITING`
  - `WORKING`
  - delete/orphan markers
  - PR metadata

In `src/ui/tui/view/view_chrome_sidebar/model.rs`:
- keep the existing row rendering primitives if they still fit
- only adjust line rendering helpers if the one-line layout needs different clipping or right-edge behavior

In `src/ui/tui/shared.rs`:
- change `WORKSPACE_ITEM_HEIGHT` to match the new one-line-per-workspace model

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test sidebar_ -- --nocapture`
Expected: updated sidebar layout tests pass

- [ ] **Step 5: Commit the row-model refactor**

```bash
git add src/ui/tui/view/view_chrome_sidebar/build.rs src/ui/tui/view/view_chrome_sidebar/model.rs src/ui/tui/shared.rs src/ui/tui/mod.rs
git commit -m "refactor: compress workspace sidebar rows"
```

## Chunk 2: Selection And Mouse Mapping

### Task 2: Reconcile row maps and hit-testing with the denser layout

**Files:**
- Modify: `src/ui/tui/view/view_chrome_sidebar/build.rs`
- Modify: `src/ui/tui/update/update_input_mouse.rs`
- Test: `src/ui/tui/mod.rs`

- [ ] **Step 1: Write failing hit-testing tests for the compressed row shape**

Add or update tests in `src/ui/tui/mod.rs` for:
- selecting the second workspace inside a multi-workspace task by click
- selected workspace staying aligned after the row map changes
- sidebar mouse wheel / click behavior still resolving the intended workspace index

- [ ] **Step 2: Run the targeted tests to verify failure**

Run: `cargo test sidebar_workspace_index_ -- --nocapture`
Expected: one or more row-map / mouse-hit tests fail until the new row geometry is wired through

- [ ] **Step 3: Update sidebar row mapping and mouse lookup logic**

In `src/ui/tui/view/view_chrome_sidebar/build.rs`:
- make `sidebar_workspace_row_map()` reflect the new list shape
- ensure only actual workspace rows map to workspace indices

In `src/ui/tui/update/update_input_mouse.rs`:
- verify `sidebar_workspace_index_at_point(...)` still uses the rebuilt row map correctly
- adjust any assumptions that relied on the old `WORKSPACE_ITEM_HEIGHT == 3` geometry

- [ ] **Step 4: Run the targeted tests to verify they pass**

Run: `cargo test sidebar_workspace_index_ -- --nocapture`
Run: `cargo test mouse_wheel_on_sidebar_moves_workspace_selection -- --nocapture`
Expected: row hit-testing and selection behavior pass with the denser layout

- [ ] **Step 5: Commit the interaction updates**

```bash
git add src/ui/tui/view/view_chrome_sidebar/build.rs src/ui/tui/update/update_input_mouse.rs src/ui/tui/mod.rs
git commit -m "fix: align sidebar hit testing with dense rows"
```

## Chunk 3: Regression Coverage And Cleanup

### Task 3: Lock in dense layout behavior and run required verification

**Files:**
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/view/view_chrome_sidebar/render.rs` (only if a render assumption still references the old height)
- Modify: `src/ui/tui/view/view_chrome_sidebar/build.rs` (cleanup only if needed)

- [ ] **Step 1: Add the remaining behavior regressions**

Cover these cases in `src/ui/tui/mod.rs`:
- multi-workspace task renders a visible header plus one row per workspace
- single-workspace task still renders a visible header
- inline status shares the row with name/metadata
- duplicate task/workspace naming stays suppressed
- selected-row background still wraps the intended single row

- [ ] **Step 2: Run the focused sidebar density tests**

Run: `cargo test sidebar_ -- --nocapture`
Run: `cargo test waiting_workspace_row_ -- --nocapture`
Run: `cargo test working_workspace_row_ -- --nocapture`
Expected: dense layout keeps the status semantics and render behavior intact

- [ ] **Step 3: Remove dead assumptions from render code**

If `src/ui/tui/view/view_chrome_sidebar/render.rs` or related helpers still assume the old three-line workspace block:
- delete the obsolete logic
- keep rendering behavior minimal and consistent with the new one-line model

- [ ] **Step 4: Run required project validation**

Run: `make precommit`
Expected: fmt, check, and clippy all pass

- [ ] **Step 5: Commit the final polish**

```bash
git add src/ui/tui/mod.rs src/ui/tui/view/view_chrome_sidebar/build.rs src/ui/tui/view/view_chrome_sidebar/render.rs
git commit -m "test: cover dense sidebar layout"
```

# Ftui Native Scrolling Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace remaining custom scrolling in Grove with ftui-native scroll primitives while preserving current keyboard, mouse, preview, and selection behavior.

**Architecture:** Migrate each scrollable surface independently. Use `VirtualizedListState` for preview because Grove needs direct scroll-state introspection for selection and copy behavior, `ListState` for the create-dialog project picker, and the ftui `CommandPalette` widget for command palette rendering. Ship the work in three phases with one commit per phase.

**Tech Stack:** Rust, FrankenTUI widgets (`VirtualizedList`, `List`, `CommandPalette`), Grove TUI update/view state, Rust test suite in `src/ui/tui/mod.rs`.

---

### Task 1: Add preview migration regression tests

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add focused tests for preview scroll up/down.
- Add focused tests for preview page up/page down.
- Add focused tests for jump-to-bottom restoring follow mode.
- Add focused tests proving visible preview selection/copy uses the ftui-derived visible range.

**Step 2: Run test to verify it fails**

Run: `cargo test preview_mode_keys_scroll_and_jump_to_bottom`

Expected: FAIL because preview still uses custom offset/autoscroll state.

**Step 3: Write minimal implementation**

- Adjust the preview-facing tests so they assert ftui-native preview scroll state instead of `preview.offset` and `preview.auto_scroll`.

**Step 4: Run test to verify it passes**

Run: `cargo test preview_mode_keys_scroll_and_jump_to_bottom`

Expected: PASS.

### Task 2: Add preview-native scroll state

**Files:**
- Modify: `src/application/preview.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/update/update_navigation_preview.rs`
- Modify: `src/ui/tui/update/update_polling_capture_live.rs`

**Step 1: Write the failing test**

- Add or extend a regression test proving incoming preview captures preserve follow mode when at bottom and preserve manual position when scrolled away.

**Step 2: Run test to verify it fails**

Run: `cargo test preview_scroll_emits_scrolled_and_autoscroll_events`

Expected: FAIL because preview state still owns custom scroll behavior.

**Step 3: Write minimal implementation**

- Remove `offset` and `auto_scroll` from `PreviewState`.
- Add preview `VirtualizedListState` to `GroveApp`.
- Convert preview scroll helpers to use `VirtualizedListState.scroll`, `.page_up`, `.page_down`, `.scroll_to_bottom`, `.follow_mode`, and `.set_follow`.
- Preserve telemetry by logging from the new ftui-derived state transitions.

**Step 4: Run test to verify it passes**

Run: `cargo test preview_scroll_emits_scrolled_and_autoscroll_events`

Expected: PASS.

**Step 5: Commit**

```bash
git add src/application/preview.rs src/ui/tui/model.rs src/ui/tui/update/update_navigation_preview.rs src/ui/tui/update/update_polling_capture_live.rs src/ui/tui/mod.rs
git commit -m "refactor: move preview scrolling to ftui state"
```

### Task 3: Render preview through native virtualized scrolling

**Files:**
- Modify: `src/ui/tui/view/view_preview.rs`
- Modify: `src/ui/tui/view/view_preview_content.rs`
- Modify: `src/ui/tui/view/view_selection_mapping.rs`
- Modify: `src/ui/tui/view/view_selection_interaction.rs`
- Modify: `src/ui/tui/view/view_status.rs`
- Modify: `src/ui/tui/view/view_selection_logging.rs`

**Step 1: Write the failing test**

- Add a regression test proving preview selection/copy still returns the expected visible lines after scrolling.

**Step 2: Run test to verify it fails**

Run: `cargo test copy_interactive_selection_or_visible`

Expected: FAIL because visible-range mapping still assumes custom preview offset math.

**Step 3: Write minimal implementation**

- Derive preview visible start/end from `VirtualizedListState.scroll_offset_clamped(...)` and `visible_count()`.
- Render preview output with ftui-native virtualized scrolling semantics.
- Update selection hit-testing, copy-visible behavior, and debug/status output to read ftui state.

**Step 4: Run test to verify it passes**

Run: `cargo test copy_interactive_selection_or_visible`

Expected: PASS.

### Task 4: Verify preview phase

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Run focused preview verification**

Run: `cargo test preview_`

Expected: PASS for preview-focused tests.

**Step 2: Run repo minimum validation**

Run: `make precommit`

Expected: PASS.

### Task 5: Add create-picker migration regression tests

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add focused tests proving create picker selection stays visible while navigating a filtered list.
- Add coverage for mouse or keyboard selection after scrolling the picker.

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_project_picker`

Expected: FAIL because create picker still uses manual scroll-window math.

**Step 3: Write minimal implementation**

- Update tests to assert `ListState`-backed scrolling behavior.

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_project_picker`

Expected: PASS.

### Task 6: Replace create-picker manual windowing with `ListState`

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_setup.rs`
- Modify: `src/ui/tui/dialogs/dialogs_create_key.rs`
- Modify: `src/ui/tui/view/view_overlays_create.rs`
- Modify: `src/ui/tui/update/update_input_mouse.rs`

**Step 1: Write the failing test**

- Add a regression test proving the selected filtered project remains visible after repeated `Down`, `Tab`, or `Ctrl-n` navigation.

**Step 2: Run test to verify it fails**

Run: `cargo test create_dialog_project_picker_selection_updates_defaults`

Expected: FAIL because render/update paths still compute custom scroll offsets.

**Step 3: Write minimal implementation**

- Add `ListState` to the create picker dialog state.
- Replace `create_dialog_project_picker_scroll_offset(...)`.
- Render picker rows with ftui `List`.
- Route picker key and mouse selection through `ListState`.

**Step 4: Run test to verify it passes**

Run: `cargo test create_dialog_project_picker_selection_updates_defaults`

Expected: PASS.

**Step 5: Commit**

```bash
git add src/ui/tui/dialogs/state.rs src/ui/tui/dialogs/dialogs_create_setup.rs src/ui/tui/dialogs/dialogs_create_key.rs src/ui/tui/view/view_overlays_create.rs src/ui/tui/update/update_input_mouse.rs src/ui/tui/mod.rs
git commit -m "refactor: use ftui list state in create picker"
```

### Task 7: Add command-palette migration regression tests

**Files:**
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add focused tests for command palette paging and keeping the selected action visible.
- Add coverage proving palette rendering still exposes the selected action title and executes it.

**Step 2: Run test to verify it fails**

Run: `cargo test command_palette`

Expected: FAIL because Grove still manually slices command-palette results.

**Step 3: Write minimal implementation**

- Adjust tests to assert native widget-driven rendering and scroll behavior.

**Step 4: Run test to verify it passes**

Run: `cargo test command_palette`

Expected: PASS.

### Task 8: Switch command palette to native widget rendering

**Files:**
- Modify: `src/ui/tui/view/view_overlays_help/palette_overlay.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/commands/meta.rs`
- Modify: `src/ui/tui/update/update_input_key_events.rs`

**Step 1: Write the failing test**

- Add a regression test proving page-down and end navigation render the expected selected action when many actions are registered.

**Step 2: Run test to verify it fails**

Run: `cargo test palette_page_`

Expected: FAIL because the custom Grove overlay is still active.

**Step 3: Write minimal implementation**

- Delete manual result-window calculations in palette overlay rendering.
- Render the ftui `CommandPalette` widget directly with Grove theming.
- Keep Grove action registration and execution wiring unchanged.

**Step 4: Run test to verify it passes**

Run: `cargo test palette_page_`

Expected: PASS.

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_overlays_help/palette_overlay.rs src/ui/tui/model.rs src/ui/tui/commands/meta.rs src/ui/tui/update/update_input_key_events.rs src/ui/tui/mod.rs
git commit -m "refactor: use native ftui command palette rendering"
```

### Task 9: Final verification

**Files:**
- Modify: `docs/plans/2026-03-10-ftui-native-scrolling-design.md`
- Modify: `docs/plans/2026-03-10-ftui-native-scrolling.md`

**Step 1: Run targeted verification**

Run: `cargo test preview_ create_dialog_project_picker command_palette`

Expected: PASS for all touched areas.

**Step 2: Run repo minimum validation**

Run: `make precommit`

Expected: PASS.

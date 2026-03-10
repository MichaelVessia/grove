# Keybind Help Registry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Grove's custom keybind help overlay with a native ftui help pipeline backed by `HelpRegistry`, `HelpIndex`, and grouped help rendering.

**Architecture:** Introduce one help catalog that owns command-derived and synthetic help entries. Build a `HelpRegistry` and `HelpIndex` from that catalog, then render the help modal with ftui-native grouped help (`KeybindingHints`, which composes `Help`) instead of Grove's handwritten row layout. Keep existing open/close behavior and command-palette enablement rules unchanged.

**Tech Stack:** Rust, Grove TUI modules, FrankenTUI widgets (`HelpRegistry`, `HelpIndex`, `Help`, `KeybindingHints`, `Modal`), Rust tests in `src/ui/tui/mod.rs`.

---

### Task 1: Replace opaque help labels with structured help metadata

**Files:**
- Create: `src/ui/tui/help_catalog.rs`
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/commands/catalog.rs`
- Modify: `src/ui/tui/commands/meta.rs`
- Modify: `src/ui/tui/commands/help.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add a regression test proving every keybound `UiCommand` that is meant to be
  discoverable produces at least one structured help entry.
- Add a regression test proving the help catalog keeps command/help parity the
  same way `ui_command_help_hint_labels_match_context_command_lists` does
  today, but against structured help entries instead of raw labels.

**Step 2: Run test to verify it fails**

Run: `cargo test ui_command_help_`

Expected: FAIL because help data still exists only as preformatted label
strings in `HelpHintSpec`.

**Step 3: Write minimal implementation**

- Add a new help catalog module that defines structured help entry specs for
  Grove.
- Replace `HelpHintSpec { context, label }` with structured help metadata that
  carries `context`, `category`, `key`, and `action`.
- Update helper methods in `commands/help.rs` so they return structured help
  entries instead of string labels.
- Keep `palette` metadata intact for now.

**Step 4: Run test to verify it passes**

Run: `cargo test ui_command_help_`

Expected: PASS.

**Step 5: Commit**

```bash
git add src/ui/tui/help_catalog.rs src/ui/tui/mod.rs src/ui/tui/commands/catalog.rs src/ui/tui/commands/meta.rs src/ui/tui/commands/help.rs
git commit -m "refactor: structure tui help metadata"
```

### Task 2: Build native ftui help registry and searchable index

**Files:**
- Modify: `src/ui/tui/help_catalog.rs`
- Modify: `src/ui/tui/update/update_navigation_palette.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add a regression test proving the new help registry includes current
  discoverability content for:
  - `?` help
  - `Ctrl+K` command palette
  - mouse capture toggle
  - start parent agent
  - project modal remove shortcut
- Add a regression test proving `HelpIndex` finds those entries by user-facing
  search terms like `parent agent`, `mouse capture`, and `remove`.

**Step 2: Run test to verify it fails**

Run: `cargo test help_index_`

Expected: FAIL because Grove does not yet build a `HelpRegistry` or
`HelpIndex`.

**Step 3: Write minimal implementation**

- In `help_catalog.rs`, register command-derived entries plus synthetic entries
  for:
  - palette search/navigation
  - interactive reserved keys
  - modal navigation patterns
- Build a `HelpRegistry` from the current app state.
- Build a `HelpIndex` from the loaded registry entries.
- Keep command-palette enablement logic in
  `update_navigation_palette.rs`, but expose the same command availability to
  the help catalog so disabled commands do not register as active help rows.

**Step 4: Run test to verify it passes**

Run: `cargo test help_index_`

Expected: PASS.

### Task 3: Replace the handwritten help modal body with native ftui help rendering

**Files:**
- Modify: `src/ui/tui/view/view_overlays_help.rs`
- Modify: `src/ui/tui/view/view_overlays_help/keybind_overlay.rs`
- Delete: `src/ui/tui/view/view_overlays_help/rows.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Update the overlay rendering tests so they assert the native grouped help
  output instead of bespoke row-builder details.
- Add a regression test proving the overlay still renders global, preview, and
  modal sections from registered help content.

**Step 2: Run test to verify it fails**

Run: `cargo test keybind_help_`

Expected: FAIL because the overlay is still assembled manually in
`keybind_overlay.rs`.

**Step 3: Write minimal implementation**

- Remove manual section-title, wrapping, and labeled-row assembly.
- Build grouped ftui help data from the shared help catalog.
- Render the help modal body with `KeybindingHints` in full grouped mode,
  preserving the existing modal shell and backdrop.
- Delete `rows.rs` if no helpers remain after the native render path lands.

**Step 4: Run test to verify it passes**

Run: `cargo test keybind_help_`

Expected: PASS.

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_overlays_help.rs src/ui/tui/view/view_overlays_help/keybind_overlay.rs src/ui/tui/mod.rs
git rm src/ui/tui/view/view_overlays_help/rows.rs
git commit -m "refactor: render keybind help with ftui help widgets"
```

### Task 4: Preserve modal behavior and remove drift-prone paths

**Files:**
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/dialogs/dialogs.rs`
- Modify: `src/ui/tui/update/update_input_key_events.rs`
- Modify: `src/ui/tui/view/view_status.rs`
- Test: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add or keep focused regressions for:
  - `?` opening help
  - `Esc`, `Enter`, and `?` closing help
  - help blocking background navigation
  - footer/status still reporting help mode correctly

**Step 2: Run test to verify it fails**

Run: `cargo test question_key_opens_keybind_help_modal keybind_help_modal_closes_on_escape keybind_help_modal_blocks_navigation_keys`

Expected: FAIL if any old control path still depends on deleted custom overlay
state or strings.

**Step 3: Write minimal implementation**

- Remove any now-dead helpers or state that only existed to support the custom
  row renderer.
- Keep `keybind_help_open` behavior unchanged unless the native widget requires
  a small state container.
- Keep the footer state chip and key-hint line stable.

**Step 4: Run test to verify it passes**

Run: `cargo test question_key_opens_keybind_help_modal keybind_help_modal_closes_on_escape keybind_help_modal_blocks_navigation_keys`

Expected: PASS.

### Task 5: Final verification

**Files:**
- Modify: `docs/plans/2026-03-10-keybind-help-registry-design.md`
- Modify: `docs/plans/2026-03-10-keybind-help-registry.md`

**Step 1: Run focused help validation**

Run: `cargo test keybind_help_ ui_command_help_ help_index_`

Expected: PASS.

**Step 2: Run repo minimum validation**

Run: `make precommit`

Expected: PASS.

**Step 3: Update docs if implementation drifted**

- If the final code landed in different files or needed one extra helper,
  update this plan and the design doc before handoff so execution notes stay
  honest.

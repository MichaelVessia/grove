# Project Modal Native Rebuild Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild Grove's project modal stack as native ftui widgets with real text editing, bracketed paste, clipboard actions, and fuzzy-ranked repo-root search for Add Project.

**Architecture:** Replace project modal `FtText` assembly with ftui-native `Modal` content composed from `Block`, `TextInput`, `List`, and `Paragraph`. Move project dialog state to widget-owned state, add async path-discovery completions, and delete the existing manual string-mutation input path.

**Tech Stack:** Rust, ftui `Modal`, ftui `TextInput`, ftui `List`/`ListState`, Grove async `Cmd::Task` update flow, focused TUI/unit tests in `src/ui/tui/mod.rs`.

---

### Task 1: Replace project dialog state with native widget state

**Files:**
- Modify: `src/ui/tui/dialogs/state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_state.rs`
- Modify: `src/ui/tui/model.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - opening the project dialog creates a focused native filter input state
  - opening Add Project creates native `Name` and `Path` input state plus an empty result list
  - opening Project Defaults creates native input state for each editable field

**Step 2: Run test to verify it fails**

Run: `cargo test project_dialog_native_state`

Expected: FAIL because project dialog state still uses raw strings and old focus enums.

**Step 3: Write minimal implementation**

- Replace raw dialog text fields with ftui widget state:
  - `TextInput` for filter and editable fields
  - `ListState` for project rows and add-project suggestions
- Add explicit focus enums for widget regions only where needed.
- Remove the old manual `focused_field` model for text-editable project fields.
- Keep domain values derivable from widget state instead of duplicated string storage.

**Step 4: Run test to verify it passes**

Run: `cargo test project_dialog_native_state`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/dialogs/state.rs src/ui/tui/dialogs/dialogs_projects_state.rs src/ui/tui/model.rs src/ui/tui/mod.rs
git commit -m "refactor: replace project dialog state with native widgets"
```

### Task 2: Rebuild the project switcher modal as native ftui widgets

**Files:**
- Modify: `src/ui/tui/view/view_overlays_projects.rs`
- Modify: `src/ui/tui/dialogs/dialogs.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_key.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_state.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - typing in the project modal filter updates results via native widget input
  - pressing Enter on the selected project still focuses the project
  - mouse click on a project row updates selection
  - Esc clears the filter first, then closes the modal

**Step 2: Run test to verify it fails**

Run: `cargo test project_dialog_native_switcher`

Expected: FAIL because the project modal still renders through `OverlayModalContent` and manual row text.

**Step 3: Write minimal implementation**

- Stop using `OverlayModalContent` for project dialogs.
- Render the project switcher body with native ftui widgets:
  - `Block` for panel chrome
  - `TextInput` for filter
  - `List` for projects
  - `Paragraph` for hints and empty states
- Route key and mouse events into the focused widget/list state.
- Delete obsolete row-based project-dialog rendering helpers.

**Step 4: Run test to verify it passes**

Run: `cargo test project_dialog_native_switcher`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_overlays_projects.rs src/ui/tui/dialogs/dialogs.rs src/ui/tui/dialogs/dialogs_projects_key.rs src/ui/tui/dialogs/dialogs_projects_state.rs src/ui/tui/mod.rs
git commit -m "feat: rebuild project switcher modal with native ftui widgets"
```

### Task 3: Add fuzzy scorer and bounded repo-root discovery for Add Project

**Files:**
- Create: `src/ui/tui/dialogs/dialogs_projects_search.rs`
- Modify: `src/ui/tui/mod.rs`
- Modify: `src/ui/tui/msg.rs`
- Modify: `src/ui/tui/update/update.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_key.rs`

**Step 1: Write the failing tests**

- Add unit tests for:
  - nearest existing ancestor resolution
  - fuzzy scoring prefers basename and word-start matches
  - shorter/higher-confidence repo-root matches sort first
- Add TUI tests for:
  - typing in Add Project `Path` refreshes ranked suggestions
  - selecting a suggestion fills the path field
  - selecting a suggestion auto-fills `Name` when empty

**Step 2: Run test to verify it fails**

Run: `cargo test project_add_dialog_fuzzy_search`

Expected: FAIL because no search module or async completion exists.

**Step 3: Write minimal implementation**

- Add a local fuzzy scorer and repo-root scan helper.
- Add async completion message for path-search results.
- Start a bounded repo-root scan when the `Path` input changes.
- Cache and rerank results in memory as the query changes.
- Fill `Path` and optional auto-name from the chosen suggestion.
- Delete the old substring-only helper where it is no longer used.

**Step 4: Run test to verify it passes**

Run: `cargo test project_add_dialog_fuzzy_search`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/dialogs/dialogs_projects_search.rs src/ui/tui/mod.rs src/ui/tui/msg.rs src/ui/tui/update/update.rs src/ui/tui/dialogs/dialogs_projects_state.rs src/ui/tui/dialogs/dialogs_projects_key.rs
git commit -m "feat: add fuzzy repo-root search to add project modal"
```

### Task 4: Rebuild Add Project and Project Defaults inputs around ftui `TextInput`

**Files:**
- Modify: `src/ui/tui/view/view_overlays_projects.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_key.rs`
- Modify: `src/ui/tui/update/update_input_key_events.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - terminal paste inserts text into focused Add Project and Project Defaults inputs
  - Tab and Shift-Tab move between native inputs, list, and buttons
  - Enter on the selected suggestion confirms the suggestion before Add
  - existing add-project validation still fires on invalid input

**Step 2: Run test to verify it fails**

Run: `cargo test project_dialog_native_inputs`

Expected: FAIL because project dialogs still do not route paste events into native widget state.

**Step 3: Write minimal implementation**

- Render Add Project and Project Defaults with native `TextInput` widgets.
- Route `Event::Paste` into the focused project dialog widget before interactive-session paste handling.
- Use widget-owned cursor/selection behavior instead of manual char push/pop.
- Keep existing add/save validation and persistence behavior.

**Step 4: Run test to verify it passes**

Run: `cargo test project_dialog_native_inputs`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/view/view_overlays_projects.rs src/ui/tui/dialogs/dialogs_projects_key.rs src/ui/tui/update/update_input_key_events.rs src/ui/tui/mod.rs
git commit -m "feat: use native ftui inputs in project dialogs"
```

### Task 5: Wire explicit clipboard actions and update discoverability

**Files:**
- Modify: `src/ui/tui/dialogs/dialogs_projects_key.rs`
- Modify: `src/ui/tui/view/view_overlays_projects.rs`
- Modify: `src/ui/tui/view/view_overlays_help/keybind_overlay.rs`
- Modify: `src/ui/tui/commands/meta.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Write the failing tests**

- Add tests covering:
  - `Alt-v` pastes clipboard text into the focused project-dialog input
  - `Alt-c` copies selected text, else full focused field value
  - keybind help shows the new modal clipboard shortcuts
  - command/help discoverability stays in sync

**Step 2: Run test to verify it fails**

Run: `cargo test project_dialog_clipboard_shortcuts`

Expected: FAIL because project dialogs do not currently consume clipboard shortcuts.

**Step 3: Write minimal implementation**

- Reuse Grove's `ClipboardAccess` adapter for project-dialog copy/paste.
- Implement `Alt-c` and `Alt-v` only in project-dialog input contexts.
- Update modal hints, help text, and any command/help discoverability surface required by Grove policy.

**Step 4: Run test to verify it passes**

Run: `cargo test project_dialog_clipboard_shortcuts`

Expected: PASS

**Step 5: Commit**

```bash
git add src/ui/tui/dialogs/dialogs_projects_key.rs src/ui/tui/view/view_overlays_projects.rs src/ui/tui/view/view_overlays_help/keybind_overlay.rs src/ui/tui/commands/meta.rs src/ui/tui/mod.rs
git commit -m "feat: add clipboard shortcuts to project dialogs"
```

### Task 6: Remove obsolete modal plumbing and run focused validation

**Files:**
- Modify: `src/ui/tui/view/view_overlays_projects.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_state.rs`
- Modify: `src/ui/tui/dialogs/dialogs_projects_key.rs`
- Modify: `src/ui/tui/dialogs/dialogs.rs`
- Modify: `src/ui/tui/mod.rs`

**Step 1: Delete dead code**

- Remove unused row-render helpers, raw-string project-dialog paths, and any replaced enums/struct fields.
- Make sure no compatibility adapter remains for the deleted modal path.

**Step 2: Run focused project-dialog test suites**

Run: `cargo test project_dialog_native`

Expected: PASS

**Step 3: Run repo minimum validation**

Run: `make precommit`

Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/tui/view/view_overlays_projects.rs src/ui/tui/dialogs/dialogs_projects_state.rs src/ui/tui/dialogs/dialogs_projects_key.rs src/ui/tui/dialogs/dialogs.rs src/ui/tui/mod.rs
git commit -m "refactor: remove legacy project modal plumbing"
```

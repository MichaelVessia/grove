# FTUI FocusGraph Migration Design

**Date:** 2026-03-26

## Summary

Replace Grove's manual pane and dialog focus tracking with FTUI's
`FocusManager` and `FocusGraph`. Make focus a first-class runtime concern owned
by `GroveApp`, use stable `FocusId` constants for panes and dialog controls,
trap focus inside modals, and remove the current mix of `PaneFocus`,
`focused_field` enums, and scattered `modal_open()` guards.

## Problem

Focus in Grove is split across several systems:

- `PaneFocus` in `src/ui/state.rs` drives list vs preview behavior
- dialog-local `focused_field` enums in `src/ui/tui/dialogs/state.rs` drive
  modal keyboard navigation
- text inputs also keep their own focused state and require manual sync
- mouse handlers set focus indirectly through ad hoc state writes
- many command guards depend on `self.state.focus == ...`
- modal blocking is often expressed as `modal_open()` instead of real focus
  containment

That creates three concrete problems:

1. focus behavior is duplicated and drift-prone
2. modal containment is manual, not structural
3. future spatial nav, extra panes, and richer dialogs become harder than they
   should be

## Decision

Use FTUI focus primitives as the keyboard-focus source of truth.

### Source of truth

`GroveApp` will own one `FocusManager`. It will contain:

- a `FocusGraph` for pane and dialog relationships
- a main-panes group for tab traversal
- modal groups for dialog traversal
- a trap stack for active modal containment

`AppState` will no longer own `PaneFocus` once the migration is complete.

### Stable focus identity

Add a new focus-ID namespace in `src/ui/tui/shared.rs`, separate from hit IDs.
Each keyboard-focusable element gets a stable `FocusId`, including:

- main panes:
  - workspace list
  - preview
- dialog roots and dialog controls:
  - buttons
  - toggles
  - text inputs
  - project-search result list
  - nested project add/defaults controls

The IDs must be stable enough for tests, replay bootstrapping, and debug
logging.

### Mouse remains hit-driven

Mouse routing should keep using hit regions and hit data. FTUI focus does not
replace Grove's click hit-testing. Instead, clicks that imply keyboard focus
must also call `focus_manager.focus(...)`.

### Modal focus model

Opening a dialog will:

1. register its focus nodes
2. create a group containing those nodes
3. push a trap for that group
4. focus the dialog's first logical field

Closing a dialog will:

1. pop the trap
2. remove the dialog's nodes
3. restore prior focus via FTUI

This preserves Grove's current modal behavior while replacing manual
containment with structural focus rules.

## Architecture

### Focus ownership

Add a small focus layer inside `src/ui/tui/`:

- focus constants in `shared.rs`
- `GroveApp` helpers in `model.rs` for:
  - building the main graph
  - syncing node bounds from layout
  - querying current pane/dialog focus
  - opening/closing dialog traps
  - translating replay bootstrap focus into `FocusId`

The focus layer should stay narrow. It should not become a second UI state
system. `FocusManager` owns only keyboard focus, not selection or dialog data.

### Main-pane graph

The initial graph contains two primary nodes:

- workspace list
- preview

They should have:

- explicit left/right edges for stable pane navigation
- tab order entries for `Tab` and `Shift+Tab`
- dynamic bounds updated from the current layout solver
- `is_focusable = false` for the sidebar node when the sidebar is hidden

### Dialog graph model

Each dialog becomes a focus group. The dialog code keeps its business state, but
focus order comes from FTUI instead of local enums.

For simple confirm-style dialogs, the group is just buttons or checkboxes plus
buttons. For text-input dialogs, each input gets its own `FocusId`. When a
field gains or loses focus, the corresponding `TextInput` focus flag is synced
from `FocusManager`.

Nested project dialogs need explicit group ownership:

- outer project dialog group
- nested add/defaults sub-dialog group when open
- top-most trap wins

### Read/write helpers

Avoid scattering raw `current() == Some(...)` checks everywhere. Add small
helpers on `GroveApp`, for example:

- `workspace_list_focused()`
- `preview_focused()`
- `focus_dialog_field(id)`
- `dialog_field_focused(id)`

This keeps call sites readable and limits FTUI-specific knowledge.

## Migration strategy

Do this in stages, not a big bang.

### Stage 1: Shadow existing state

Add `FocusManager` and keep `PaneFocus` plus dialog `focused_field` enums.
Bridge helpers dual-write focus so behavior stays unchanged while tests are
added.

### Stage 2: Trap modals

Register dialog groups and traps on open/close, but keep existing enums for
reads. This isolates modal lifecycle first.

### Stage 3: Migrate reads

Switch view styling, command gating, status text, and key handling from
`PaneFocus` / `focused_field` reads to focus-manager helpers.

### Stage 4: Migrate writes

Switch keyboard and mouse updates to call FTUI focus APIs directly. Add spatial
left/right pane nav and dialog tab traversal via FTUI.

### Stage 5: Delete old focus state

Remove `PaneFocus`, remove dialog field enums that only existed for focus
tracking, simplify text-input sync, and keep replay compatibility only at the
serialization boundary if still needed.

## Scope

In scope:

- main pane focus migration
- modal trapping via FTUI
- dialog-internal focus migration for all current dialogs
- left/right pane navigation
- focus restoration on modal close
- keeping mouse click behavior aligned with keyboard focus
- replay bootstrap mapping for focus

Out of scope:

- changing task/workspace selection semantics
- replacing hit testing with FTUI focus
- changing interactive tmux key forwarding rules
- redesigning dialogs or adding new panes

## Testing

Add or update regression coverage for:

- pane focus switching from tab and left/right navigation
- sidebar-hidden focusability
- click-to-focus for list and preview
- modal trap behavior
- nested modal trap behavior in project flows
- focus restoration after dialog close
- command palette enablement based on FTUI focus state
- text-input focus syncing for create, edit, rename, launch, and project dialogs
- replay bootstrap preserving the correct focused pane

Tests should stay behavior-oriented. Assert what the user can do, not private
focus-manager internals unless a helper is specifically the public contract.

## Risks

### Dialog migration sprawl

The issue is not just pane focus. `src/ui/tui/dialogs/state.rs` contains many
dialog field enums and cyclic traversal helpers. The migration should remove
them in clusters rather than partially converting one dialog at a time without
tests.

### Hidden sidebar edge cases

When the sidebar is hidden, the list pane must become unfocusable immediately.
Otherwise focus can get trapped on an invisible node, breaking commands and
status styling.

### Interactive mode ownership

Arrow keys and tab-like traversal must continue to defer to tmux when Grove is
in interactive mode. FTUI focus nav should only run in Grove-controlled
non-interactive contexts.

### Replay compatibility

Existing replay fixtures may still serialize old pane focus concepts. Convert at
the bootstrap boundary instead of leaking legacy enums back into runtime code.

## Why This Shape

- aligns Grove with native FTUI primitives already available in the reference
  codebase
- removes duplicated focus logic and manual sync points
- gives Grove real modal containment and focus restoration
- makes future panes and richer dialogs cheaper to add
- keeps selection, hit testing, and runtime state separate from keyboard focus

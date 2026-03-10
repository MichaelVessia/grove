# Full Screen PaneTree Architecture Design

**Date:** 2026-03-10

## Summary

Replace Grove's bespoke top-level `ViewLayout` geometry with a single ftui
`PaneTree` that owns the entire viewport. Header, workspace content, and status
become first-class panes. Overlays remain outside the pane tree and render on
top.

## Problem

Grove currently splits the screen with custom top-level layout helpers and then
threads those rects through render, hit testing, and mouse behavior. That works
for today's fixed shell, but it hard-codes Grove around one specific shape:

- fixed header row
- fixed status row
- one sidebar pane
- one preview pane

This makes future pane growth awkward. Any new top-level pane requires another
round of manual geometry changes, plus corresponding hit-test and resize logic.

## Decision

Adopt ftui `PaneTree` as Grove's one canonical top-level layout model.

The entire screen becomes a solved pane tree:

- `header`
- `workspace`
- `status`

The `workspace` subtree starts as:

- `workspace_list`
- `preview`

So the initial built-in structure is:

```text
root
- header
- workspace
  - workspace_list
  - preview
- status
```

No backwards compatibility layer. Delete `ViewLayout` once the new model is in
place.

## Why This Shape

This is the cleanest long-term architecture because it gives Grove one geometry
system, one growth path, and one source of truth for pane identity.

It avoids an awkward split where the middle of the app uses real panes but the
header and status remain special-case chrome with separate layout rules.

It also means that future changes like additional preview panes, utility panes,
or alternate workspace splits are expressed as pane-tree mutations instead of
custom rectangle math.

## Non-Goals

Not part of the first rollout:

- named user layouts
- layout migration logic
- persistence of arbitrary pane workspaces
- exposing generic pane editing to users
- turning overlays and dialogs into panes

The first version should establish the architecture, not a full tiling window
manager.

## Architecture

### Core Model

Add a Grove-owned pane adapter around ftui `PaneTree`.

Suggested shape:

- `PaneRole` enum for semantic identity:
  - `Header`
  - `Workspace`
  - `WorkspaceList`
  - `Preview`
  - `Status`
- `GrovePaneModel` wrapping:
  - `PaneTree`
  - stable mapping between `PaneRole` and `PaneId`

Most Grove code should never touch raw `PaneId` directly. It should ask the
adapter for semantic pane rects by role.

### Canonical Tree

The initial tree is built once in a canonical form:

- vertical root split: header vs remaining content
- vertical split in remaining content: workspace vs status
- horizontal split inside workspace: workspace list vs preview

Header and status are still panes, but Grove policy treats them as protected
core panes. They are not closeable and do not participate in generic user pane
editing.

### Rendering

Rendering flow becomes:

1. solve `PaneTree` for the current viewport
2. resolve rects for semantic pane roles
3. render pane content by role
4. render overlays on top

This preserves Grove's current overlay model while removing the bespoke
top-level layout helper.

### Hit Testing

High-level hit detection should move from custom rectangle classification to
pane-role lookup from solved pane rects.

Fine-grained interactions still belong to pane-local renderers and hit-grid
regions:

- workspace rows
- pull request links
- preview tabs
- any future pane-local controls

This keeps the pane tree responsible for coarse layout and Grove renderers
responsible for local semantics.

### Policy Layer

Although the underlying model is flexible, Grove should start with a strict
policy layer:

- no arbitrary pane closing
- no arbitrary pane creation from the UI
- no named layouts
- no workspace snapshot persistence

Pane operations remain internal implementation tools until Grove needs them as
a product feature.

## Data Flow

Per frame:

1. obtain viewport rect
2. solve `PaneTree`
3. cache or expose solved layout through `GrovePaneModel`
4. render `header`, `workspace_list`, `preview`, and `status`
5. register fine-grained hit regions inside panes
6. render overlays and transient UI

For input:

1. map pointer point to semantic pane role
2. route coarse interaction by pane role
3. delegate fine-grained row or link behavior to hit-grid data when available

## Migration Plan

### Phase 1: Introduce Pane Model

- add `PaneRole`
- add `GrovePaneModel`
- build the canonical full-screen tree
- add tests proving required panes always exist

### Phase 2: Replace Top-Level Geometry

- replace `view_layout()` use sites with pane-role rect lookup
- replace custom region classification with pane-role hit detection
- keep existing pane-local render behavior

### Phase 3: Delete Legacy Layout

- remove `ViewLayout`
- remove top-level rect math helpers
- remove obsolete sidebar ratio helpers if no longer needed

### Phase 4: Optional Pane Operations

Only after parity:

- decide whether to expose split ratio control through pane operations
- decide whether to allow more panes in the workspace subtree

## Testing

Required coverage:

- canonical pane tree creation tests
- solve-layout tests for normal and tiny viewports
- top-level hit-region tests by semantic pane role
- regression tests for workspace selection and preview interactions using
  solved pane rects
- overlay rendering tests proving overlays remain independent of pane layout

Existing interaction tests should be rewritten around semantic pane lookup
instead of `ViewLayout`.

## Risks

### Raw PaneId Leakage

If `PaneId` spreads through Grove, the code will become hard to reason about.
The fix is strict semantic access through `PaneRole`.

### Over-Adopting ftui Interaction Machinery

ftui provides far more pane interaction support than Grove currently needs. The
first rollout should adopt the pane model and solver, not the entire generic
interaction stack.

### Over-Modeling Transient UI

Dialogs, toasts, help, and command palette should stay overlays. Treating them
as panes would complicate the core model for little gain.

## Recommendation

Go full-screen `PaneTree` now, with a Grove policy wrapper that keeps the first
implementation simple and product-shaped.

This is the best long-term architecture because it unifies layout under ftui
without forcing Grove to immediately expose generic pane management features.

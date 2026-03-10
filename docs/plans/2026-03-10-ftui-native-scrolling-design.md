# Ftui Native Scrolling Design

**Date:** 2026-03-10

## Summary

Replace remaining Grove-managed scrolling with ftui-native primitives across
the app. Keep the existing UX semantics, but move scroll state ownership into
ftui for preview, create-dialog project picking, and command palette results.

## Problem

Grove currently mixes native and custom scrolling:

- sidebar already uses ftui `VirtualizedList`
- preview owns custom `offset` and `auto_scroll` state
- create-dialog project picker computes its own visible window
- command palette has ftui state, but Grove re-renders and slices results

That split creates inconsistent behavior and duplicates framework logic for
scrolling, paging, visibility, and follow/bottom semantics.

## Decision

Use ftui-native primitives as the source of truth for all user-visible
scrolling surfaces.

### Preview

Use ftui `VirtualizedList` plus `VirtualizedListState`.

Do not use `LogViewer` for preview. `LogViewer` has good follow-mode behavior,
but Grove's preview selection and mouse mapping need direct access to visible
range, scroll offset, and follow state. `VirtualizedListState` exposes the
state Grove needs without reintroducing manual scroll math.

Preview state should stop owning:

- `offset`
- `auto_scroll`

Preview state should keep owning:

- plain/rendered line storage
- capture ring
- digest tracking

### Create Dialog Project Picker

Use ftui `List` plus `ListState`.

Delete custom visible-window math in
`view_overlays_create.rs`. Selection remains Grove-owned at the dialog level,
but visible rows and scroll offset become `ListState` concerns.

### Command Palette

Use the ftui `CommandPalette` widget directly for rendering and scroll state.

Grove should keep:

- action registration
- action dispatch
- styling/theme integration

Grove should delete:

- manual visible-window math
- manual result slicing
- custom command-palette row rendering

## Architecture

### State Ownership

Move scrolling responsibility to ftui state objects:

- preview: `VirtualizedListState`
- create picker: `ListState`
- command palette: existing ftui `CommandPalette`

Grove remains responsible for:

- deriving item data
- selection side effects
- telemetry
- hit-region routing not already covered by ftui widgets

### Preview Data Flow

1. Capture updates append/replace preview lines.
2. Preview rendering feeds the visible line set through a native virtualized
   list.
3. Keyboard and mouse scroll commands mutate `VirtualizedListState`.
4. Selection mapping derives visible start/end from the ftui state instead of
   custom bottom-relative offset math.
5. Jump-to-bottom re-enables follow mode through ftui state.

### Overlay Data Flow

Create picker:

1. Filter refresh updates the filtered item vector.
2. `ListState.select(...)` keeps the active item in view.
3. Rendering uses ftui `List`, not manual row slicing.

Command palette:

1. Grove updates the existing `CommandPalette` state with actions and query.
2. ftui widget renders the overlay and manages internal scroll position.
3. Grove only handles execution of the returned selected action.

## Phasing

Use separate commits to keep risk bounded.

### Phase 1

Preview migration to `VirtualizedListState`.

This is the highest-risk change because it touches selection, copy behavior,
mouse scroll, follow mode, status text, replay state, and telemetry.

### Phase 2

Create-dialog project picker migration to `ListState`.

This is lower risk and isolated to dialog state, key handling, and rendering.

### Phase 3

Command palette migration to native ftui widget rendering.

This is medium risk because it changes overlay layout/rendering, but the state
object already exists and already owns selection/scroll behavior.

## Deleted Behavior

Delete Grove-managed scroll windowing and offset math where ftui already
provides it:

- preview bottom-relative scroll math
- create picker scroll-offset helper
- command palette result slicing helper logic

Do not add an intermediate Grove scroll abstraction.

## Error Handling

- Empty preview or empty filtered lists should still render existing fallback
  text.
- When filtered content shrinks, ftui state should clamp selection and scroll
  offset naturally.
- Preview follow mode should remain enabled only when the viewport is at the
  bottom.

## Testing

Add regression coverage for:

- preview line scrolling, page scrolling, and jump-to-bottom
- preview follow-mode transitions during incoming capture updates
- preview selection/copy using the ftui-derived visible range
- create picker navigation keeping selection visible
- command palette navigation and paging using native rendering

## Why This Shape

- aligns with the ftui-first constraint in this repo
- removes duplicated scroll logic
- keeps preview compatible with Grove's custom selection model
- breaks the refactor into small reviewable commits

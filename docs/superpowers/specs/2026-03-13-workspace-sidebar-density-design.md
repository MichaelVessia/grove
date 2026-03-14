# Workspace Sidebar Density Design

## Context

The sidebar status work made runtime signals clearer, but the row layout is still
too tall. Each workspace currently consumes a three-line block:

- line 1: workspace name and branch
- line 2: status or PR metadata
- line 3: blank spacer

That wastes vertical space, especially when most rows have little or no status
content. It also wastes horizontal space by splitting metadata that can fit on a
single line.

The goal is to make the sidebar denser without losing task grouping clarity.

## Decision

Adopt a one-line workspace row layout, while keeping task headers visible for
all tasks, including single-workspace tasks.

This keeps the high-value structure:

- task headers still define groups
- multi-workspace tasks still read as grouped clusters
- single-workspace tasks still look like tasks, not stray rows

But it removes the low-value spacing:

- no second workspace metadata line
- no trailing blank spacer line per workspace

## Row Model

Each workspace row becomes a single line:

- workspace name first
- optional branch or compact workspace metadata inline after the name
- runtime status right-aligned at the far edge when present

Examples:

- `grove · master`
- `gsops-4 (monorepo)                             WAITING`
- `varsity-event-hub · feature/varsity           WORKING`

## Grouping Rules

### Multi-workspace tasks

Render a compact task header, then render each workspace as a one-line row
under that header.

Example shape:

```text
web-monorepo  [3]
  web-monorepo · main
  gsops-4 (monorepo)                  WAITING
  varsity-event-hub · feature/varsity WORKING
```

### Single-workspace tasks

Keep a compact task header even when the task has only one workspace.

Example shape:

```text
grove [1]
  grove · master
```

This was chosen over collapsing the header, because collapsing made
single-workspace tasks lose visual distinction from surrounding rows.

## Inline Metadata Rules

- Do not repeat the task name inside child workspace rows.
- Keep branch text inline when it differs from the workspace display name.
- Preserve repo or project disambiguation text such as `(monorepo)` when needed.
- Keep PR metadata available, but only if it still fits the one-line model.
  If space is tight, runtime status wins over PR metadata.

## Status Placement

Status remains secondary to the workspace label, but should use the reclaimed
horizontal space at the far right.

Priority:

1. `WAITING`
2. `WORKING`
3. delete/orphan indicators
4. PR metadata
5. nothing

## Visual Direction

- Preserve the current selected-row background treatment.
- Preserve the current task header tone, but tighten vertical spacing around it.
- Preserve the left-side task/workspace indentation language.
- Keep runtime status terse and right-aligned.
- Avoid adding new badges, pills, or decorative chrome.

This should feel like a denser version of the current sidebar, not a brand new
component.

## Implementation Shape

At a high level:

- reduce each workspace row from three rendered lines to one
- update sidebar row hit mapping to match the new height
- keep task headers as separate rows
- collapse status rendering into the same line as workspace metadata
- rebalance truncation so the name stays readable and status remains visible

Likely touch points:

- `src/ui/tui/view/view_chrome_sidebar/build.rs`
- `src/ui/tui/view/view_chrome_sidebar/model.rs`
- `src/ui/tui/view/view_chrome_sidebar/render.rs`
- `src/ui/tui/shared.rs`
- sidebar mouse hit-testing and row-map tests in `src/ui/tui/mod.rs`

## Testing

Add or update behavior tests for:

- multi-workspace task renders as one compact header plus one row per workspace
- single-workspace task still renders a task header
- selected-row highlighting still aligns correctly
- sidebar row hit-testing still maps mouse clicks to the correct workspace
- status appears inline on the same row
- duplicate task/workspace naming stays suppressed where expected

## Non-Goals

- Redesigning task grouping semantics
- Changing the runtime meaning of `WAITING` or `WORKING`
- Adding new sidebar modes or filters
- Removing task headers entirely

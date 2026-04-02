# Workspace Jump Fast Switching Design

## Summary

Grove's workspace jump should keep its current centered fuzzy-picker shape and
optimize for fast switching, not broader situational awareness.

The goal is to reduce keystrokes and make the top result more predictable
without redesigning the interaction model. The user should be able to press
`/`, type a small prefix, and trust that `Enter` will land on the intended
workspace. Opening on an empty query should remain safe and stable, with the
current workspace selected and empty-query `Enter` acting as a no-op.

## Goals

- Preserve the current workspace jump interaction model.
- Optimize ranking for fast, predictable switching.
- Improve row scanability without expanding the UI.
- Keep empty-query behavior safe and unsurprising.

## Non-Goals

- Replacing the workspace jump palette with a new overlay type.
- Merging workspace jump into the command palette.
- Optimizing primarily for attention management or status triage.
- Adding hidden empty-query toggle behavior.

## User Decisions

- The current interaction shape is good enough to keep.
- The feature should optimize for fast switching.
- Empty-query `Enter` should be a no-op.

## Current Problem

The current jump palette works, but it still behaves like a generic fuzzy
surface rather than a tuned switching primitive.

The main gaps are:

- non-current results do not yet reflect workspace recency
- fuzzy matches can overweight branch or path fragments relative to workspace identity
- visible rows carry more text than needed for fast visual scanning

None of these require a new widget. They are ranking and presentation issues.

## Accepted UX

### Open State

When the jump palette opens with an empty query:

- the current workspace remains selected
- the full result list may remain visible
- pressing `Enter` performs no action

This preserves safety. Empty open is for orientation, not a hidden toggle.

### Ranking Priorities

Once the user begins typing, ranking should favor the identifiers that most
often map to intentional switching:

1. exact or prefix matches on repository or worktree name
2. exact or prefix matches on task slug or task name
3. fuzzy matches on repository, worktree, task, and branch
4. weaker matches on path-derived terms

Branch and path should remain searchable, but they should not routinely outrank
stronger workspace-identity matches.

### Recency

For empty query and weakly tied query results, non-current workspaces should
prefer more recently visited workspaces over colder ones.

The current workspace should remain the selected item on open. Recency is for
ordering the rest of the list, not for turning empty-query `Enter` into a
toggle.

### Row Shape

Rows should optimize for scan speed.

Preferred visible emphasis:

- primary: repository or worktree name
- adjacent context: branch
- secondary context: task name or slug

Absolute path may remain available as searchable or secondary text, but it
should not dominate the main scan line.

### Stability

While the query is being edited, selection should feel stable. Ranking changes
should improve confidence in the top hit, not make the highlight jump around in
surprising ways on weak matches.

## Architecture

### Keep Using FrankenTUI `CommandPalette`

This refinement does not justify replacing the existing primitive.

The current `CommandPalette` already provides:

- fuzzy matching
- keyboard navigation
- match highlighting
- modal rendering

The refinement should stay within the current Grove palette mode structure and
adjust action construction, result ordering, and query behavior rather than
introducing a new overlay type.

### Recency Tracking

Grove should track workspace visit recency in app state using existing
workspace-selection events.

The minimal model is:

- selected workspace path remains canonical current selection
- an additional recency ordering or timestamp map tracks prior visits

This data should be updated whenever workspace selection actually changes. It
does not need persistence in this cut unless Grove already persists comparable
UI navigation state.

### Ranking Implementation

Use the existing `CommandPalette` scorer as the base matcher, then shape inputs
and ordering to better fit fast switching.

Preferred implementation order:

1. improve searchable titles and tags to better weight repo and task identity
2. pre-order actions using current-workspace-first and MRU for the remainder
3. only add explicit post-score tie-breaking if action construction and
   ordering prove insufficient

This keeps the solution simple and aligned with the current primitive.

## Testing

Add or update tests that assert user-visible behavior.

### Empty Query

- opening jump selects the current workspace
- empty-query `Enter` is a no-op

### Ranking

- repository prefix matches outrank weaker branch-only matches
- task prefix matches outrank weaker path-only matches
- more recently visited non-current workspaces sort ahead of colder ones when
  query strength is otherwise tied

### Presentation

- visible row text remains compact and includes repo or worktree plus branch
- search still works for task, branch, and path-derived terms

## Risks

The main UX risk is over-tuning ranking so aggressively that legitimate branch
lookups become harder. Branch and path must stay searchable even if they lose
priority against stronger workspace-name matches.

The main implementation risk is adding complex custom scoring logic on top of
FrankenTUI too early. This refinement should stay biased toward simpler action
ordering and searchable-text shaping before introducing custom tie-breakers.

## Open Questions

None for this refinement cut.

# Workspace Sidebar Status Design

## Summary

The workspace sidebar currently uses an animated text treatment to imply that an
agent is "running". In practice that effect is unreliable because it conflates
multiple meanings:

- live session exists
- output changed recently
- agent is actively working
- agent is waiting on the user

The accepted direction is to remove sidebar animation entirely and make the row
communicate explicit, action-oriented state:

- `WAITING` is the highest-priority state
- `WORKING` is secondary
- raw session presence is de-emphasized

When a workspace is waiting on the user, the sidebar should show a short waiting
snippet and temporarily replace PR metadata on that line.

## Goals

- Make "this agent is waiting on me" obvious from the sidebar alone.
- Make "this agent is actively working" legible without relying on animation.
- Stop using live-session existence as a primary sidebar signal.
- Keep the implementation transient and UI-scoped, not persisted in task
  manifests.

## Non-Goals

- Redesigning preview status behavior.
- Changing persisted task or worktree manifest schema.
- Introducing new animated effects for the workspace list.
- Exposing raw tmux session counts in the sidebar.

## User Decisions

- Prioritize `Waiting` over all other sidebar runtime states.
- Keep `Working` visible, but less important than `Waiting`.
- Remove `N running` from workspace rows.
- Use static text, not animation, for sidebar state.
- When waiting, replace PR metadata with the waiting snippet.

## Current Problem

Today the sidebar row mixes two unrelated mechanisms:

1. A literal `N running` label derived from running agent tabs.
2. An animated accent treatment driven by recent output-change heuristics.

That creates a semantic mismatch:

- a row can look active because output changed recently, even if it is no longer
  the most important item
- a row can have a live session but no actionable state
- `Waiting` is visually underpowered even though it is the state the user cares
  about most

## Accepted Approach

### Sidebar Semantics

The second line of each workspace row should use this precedence:

1. `WAITING · <snippet>`
2. `WORKING`
3. `Deleting...`
4. `session ended`
5. PR metadata
6. nothing

Implications:

- `WAITING` wins over PR metadata.
- `WORKING` is shown only when the row is not waiting.
- Session-count text is removed entirely.
- Sidebar activity animation is removed entirely.

### Waiting Snippet

The snippet should be a short, cleaned excerpt derived from the existing waiting
prompt detector. Examples:

- `WAITING · approve plan changes`
- `WAITING · press enter to continue`
- `WAITING · try "continue"`

Rules:

- derive from cleaned captured output, not raw ANSI output
- keep it transient, scoped to UI polling state
- truncate to fit the available row width
- clear it as soon as the workspace is no longer in `Waiting`

### Working Semantics

`WORKING` should be shown when the workspace is currently treated as doing work
under the existing runtime heuristics, but without any animated styling.

This keeps the current detection investment while removing the misleading
implication that animation itself is authoritative.

## Architecture

### Domain Model

Do not add waiting-snippet fields to `Task`, `Worktree`, or `Workspace`.

Reason:

- the snippet is ephemeral
- it depends on recent capture output
- it should disappear naturally on refresh, session loss, or state change

The existing `WorkspaceStatus` enum remains the durable coarse status model.

### UI State

Add transient sidebar-specific waiting text storage to polling/UI state, keyed
by workspace path.

Expected shape:

- `HashMap<PathBuf, String>` alongside existing workspace status digest and
  output-change tracking

This state should be updated whenever a status capture is processed from either:

- selected live preview polling
- background workspace status polling

This state should be cleared when:

- status tracking is cleared globally
- status tracking is cleared for a workspace
- a session disappears
- the resolved workspace status is no longer `Waiting`

### Rendering

Sidebar rendering should stop using activity labels and animated text effects in
workspace rows.

Instead, line 2 should be assembled from explicit semantic labels:

- waiting label plus snippet
- working label
- existing delete/orphan indicators
- existing PR metadata when no higher-priority runtime label is present

The preview pane may keep its existing activity styling. This change is scoped
to the workspace list.

## Data Flow

1. Polling captures tmux output.
2. Output is normalized through the existing cleaned-output path.
3. Status resolution continues to use existing status detection logic.
4. If the resolved status is `Waiting`, extract the waiting prompt snippet from
   cleaned output.
5. Store the snippet in transient polling/UI state for that workspace.
6. Sidebar render reads:
   - coarse `WorkspaceStatus`
   - transient waiting snippet
   - delete/orphan flags
   - PR metadata
7. Sidebar line 2 renders the highest-priority applicable label.

## Error Handling

- If waiting-prompt extraction fails, still render `WAITING` without a snippet.
- If the workspace has no transient snippet, never synthesize one from stale
  data.
- On missing-session paths, clear transient status-tracking state for that
  workspace so stale `WAITING` text cannot persist.
- On refresh/rebuild paths, transient waiting snippets should be reconstructed
  only from fresh polling, never from persisted config or manifest state.

## Testing Strategy

Add or update focused tests covering:

- waiting rows render `WAITING` and suppress PR metadata
- waiting rows render snippet text when available
- waiting rows still render plain `WAITING` when snippet extraction is absent
- working rows render static `WORKING`
- sidebar rows no longer render `N running`
- sidebar rows no longer depend on animated activity labels
- waiting snippet state clears when:
  - status changes away from `Waiting`
  - workspace status tracking is cleared
  - session disappears

Prefer behavior tests at the sidebar render level plus targeted state-update
tests for snippet lifecycle.

## Files Likely To Change

- `src/ui/tui/view/view_chrome_sidebar/build.rs`
- `src/ui/tui/view/view_chrome_shared.rs`
- `src/ui/tui/update/update_polling_capture_workspace.rs`
- `src/ui/tui/update/update_polling_capture_live.rs`
- `src/ui/tui/update/update_polling_state.rs`
- `src/ui/tui/model.rs`
- `src/ui/tui/mod.rs` tests

## Open Questions

None at design level. The user approved:

- waiting-first semantics
- static text for working
- removal of running counts
- waiting snippets replacing PR metadata while blocked

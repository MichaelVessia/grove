# Workspace Jump Palette Design

## Summary

Grove should support fast global workspace navigation with a dedicated jump
picker opened by `/`.

This is a non-interactive-only feature. When opened, the picker should use
FrankenTUI's existing `CommandPalette` fuzzy matcher to search across the
current workspace list using task name, repository or worktree name, and
branch. Pressing `Enter` should select the matched workspace, preserve that
workspace's current preview tab, and focus the preview pane.

This should feel like a small navigation primitive, not a new command system.

## Goals

- Let the user jump to any workspace quickly from anywhere in the non-interactive UI.
- Reuse FrankenTUI primitives instead of building a custom fuzzy finder.
- Keep the interaction global and simple: open, type, enter, land in preview.
- Preserve workspace-local context by keeping the target workspace's current tab.
- Keep the implementation small and aligned with Grove's existing selection and preview flow.

## Non-Goals

- Adding a general slash-command system.
- Intercepting `/` in interactive mode.
- Searching freeform preview tab titles in this cut.
- Mixing workspace results into Grove's existing command palette.
- Changing preview tab persistence or tab selection behavior.

## User Decisions

- Trigger is bare `/`.
- The bind is active only in non-interactive UI flows.
- The picker is global.
- Search should cover task name, repository or worktree name, and branch.
- Search should not include tab titles in v1.
- `Enter` should preserve the target workspace's current selected tab.
- Use FrankenTUI's default command palette behavior, including default empty-query behavior.

## Current Problem

Today Grove supports incremental workspace navigation with the sidebar and a
separate command palette, but it does not have a fast direct jump flow for
large task sets.

As the number of tasks and worktrees grows, linear `j/k` navigation becomes
slow and cognitively expensive. The user already knows roughly what they want,
for example a task name, repo name, or branch fragment, but Grove does not
offer a fuzzy jump surface keyed to those identifiers.

## Accepted UX

### Trigger And Scope

Pressing `/` should open a dedicated workspace jump picker.

The key should be handled only when Grove is not in interactive mode and no
text-entry dialog is actively consuming typed input. In interactive mode, `/`
must continue to pass through to the attached program unchanged.

### Search Behavior

The picker should be populated from the current `AppState.workspaces`.

Each result should be fuzzy-matchable by a combination of:

- task name or slug
- repository or worktree name
- branch name

The implementation should use FrankenTUI's `CommandPalette` scoring directly.
Do not add a separate matcher, wrapper scorer, or post-ranking heuristic in
this cut.

### Result Presentation

Results should optimize for scanability and disambiguation:

- title: repository or worktree name
- description: task name plus branch
- tags: task name, task slug, repository or worktree name, branch

This keeps the visible rows compact while still making the hidden search corpus
rich enough for fuzzy lookup.

### Enter Behavior

When the user presses `Enter` on a result:

- select the target workspace
- run the normal workspace-selection side effects
- keep that workspace's existing active preview tab
- focus the preview pane

The action is "jump and preview", not just "move sidebar selection".

### Empty Query

Use FrankenTUI's default palette behavior.

That means the picker may show all workspaces on open and have an initial
selection before the user types. This is acceptable for v1 because it keeps the
implementation simple and consistent with the underlying primitive.

## Architecture

### Reuse FrankenTUI `CommandPalette`

Grove already uses FrankenTUI's `CommandPalette` for command search. The
workspace jump feature should reuse that same widget type rather than building
a custom overlay or custom fuzzy list.

This satisfies Grove's FrankenTUI-first rule and keeps input handling,
rendering, match highlighting, and result navigation consistent with the rest
of the app.

### Keep Workspace Jump Separate From Command Search

Although both features use the same widget primitive, workspace jump should be
a separate palette mode or separate palette state, not a merged command list.

Why:

- command results and workspace results serve different intents
- mixed ranking would make results noisier
- workspace jump needs direct selection side effects, not command dispatch
- help and product language stay clearer with `Ctrl+K` for commands and `/` for jump

### Stable Result Identity

Each jump result should use a stable workspace identifier derived from the
workspace path.

The cleanest shape is an action id like:

- `workspace:<absolute-path>`

On execute, Grove should resolve the workspace by path, select it via existing
selection helpers, then run the normal selection-changed flow and focus preview.

Do not rely on transient flat indexes as palette identities.

### Selection And Preview Routing

Workspace jump should reuse Grove's current selection plumbing:

- `AppState::select_workspace_path(...)`
- `handle_workspace_selection_changed()`
- `focus_main_pane(FOCUS_ID_PREVIEW)`

This keeps preview refresh, scroll reset, activity reset, and other existing
selection side effects centralized rather than duplicated in the jump feature.

## Command Model

Add a dedicated UI action for opening workspace jump.

This action is distinct from the existing command palette action and should:

- be keybound to `/`
- be enabled only outside interactive mode
- be disabled while text-entry modals are active

The actual result execution path should be separate from command-palette command
execution because result ids represent workspaces, not `UiCommand`s.

## Discoverability

Update both discoverability surfaces together:

- keybind help
- command or palette help text where relevant

The user should be able to learn:

- `Ctrl+K` opens command search
- `/` opens workspace jump

These should stay distinct in the product language.

## Testing

Add regression tests that assert behavior, not implementation details.

### Workspace Jump Open

- `/` opens the workspace jump picker in non-interactive mode
- `/` does nothing while interactive mode is active
- `/` does not steal input from text-entry dialogs

### Fuzzy Matching

- task-name fragments can find a workspace
- repository or worktree-name fragments can find a workspace
- branch-name fragments can find a workspace

These tests should rely on the visible selected result or executed result, not
internal scorer state.

### Execute Behavior

- `Enter` on a workspace result selects the target workspace
- executing a result preserves the target workspace's current active tab
- executing a result focuses preview
- executing a result triggers normal workspace-selection side effects

### Discoverability

- help catalog includes `/` workspace jump
- existing command palette help remains accurate

## Risks

The main product risk is accidental key interception in places where `/` should
remain plain text input. The implementation should guard carefully around
interactive mode and text-entry dialogs.

The main UX risk is result ambiguity when several workspaces share repository
names. Showing task plus branch in the description should keep that manageable
without expanding the visible row too much.

There is also a small architecture risk if workspace jump duplicates selection
side effects instead of reusing them. The implementation should route through
existing selection helpers.

## Open Questions

None for this cut.

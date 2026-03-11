# Manual Create Dialog Selection Design

## Problem

In manual task creation, the dialog shows both:

- `Project`, driven by `project_index`
- `Included`, driven by `selected_repository_indices`

Those states can diverge. The dialog then appears to show a selected project the
user did not actually include. This is especially confusing when the picker
remembers its last cursor position.

## Decision

Remove the separate `Project` row from manual mode. Manual mode should expose
only the committed multi-repo selection state.

`From GitHub PR` and base-task flows keep single-project selection because those
flows require exactly one project.

## UX

### Manual mode

Show:

- `Task`
- `Base`
- `Included`
- defaults hint

Behavior:

- `Included` is the only visible project-selection summary.
- Pressing `Enter` on `Included` opens the picker.
- Picker highlight is temporary.
- Canceling the picker leaves `Included` unchanged.
- Confirming or toggling items in the picker updates `Included`.

### Pull request mode

Keep the existing single-project `Project` row and project picker behavior.

### Base mode

Keep the existing single-project `Project` row and project picker behavior.

## State model

No new state is needed.

- Keep `project_index` for picker cursor/default target and for single-project
  flows.
- Keep `selected_repository_indices` as the source of truth for manual-mode
  selection.
- In manual mode, do not render `project_index` on the main form.

## Navigation

In manual mode, focus order becomes:

1. `Task`
2. `Base`
3. `Included`
4. `Create`
5. `Cancel`

The existing `Project` field slot can be reused as the `Included` field to
avoid introducing another focus enum variant.

## Testing

Add regression coverage for:

- manual mode no longer rendering a `Project` row
- manual mode rendering `Included` as the actionable browse field
- picker cancel preserving visible selection
- PR mode still rendering the single-project `Project` row

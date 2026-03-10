# Project Modal Native Rebuild Design

**Date:** 2026-03-10

## Summary

Rebuild the project modal stack as native ftui widgets instead of rendering
editable rows as preformatted text. The rebuilt modal should support real text
editing, bracketed paste, mouse focus, and fzf-style path search for the Add
Project flow.

## Problem

The current project dialogs render inputs as plain text rows inside
`OverlayModalContent` and mutate raw `String` fields through bespoke key
handlers. That blocks native widget behavior:

- no real `TextInput` editing model
- paste works only in selected flows, not in project dialogs
- no selection-aware copy behavior
- no native list state, hit regions, or cursor placement
- project filtering is substring-only, not ranked fuzzy search

This code is harder to extend and duplicates behavior already present in ftui.

## Decision

Replace the current project dialog implementation with a full native ftui modal
composition:

1. Keep ftui `Modal` as the overlay container.
2. Replace `OverlayModalContent` usage in project dialogs with a widget tree
   built from ftui components.
3. Store native widget state in dialog state:
   - `TextInput` for editable fields
   - `ListState` for selectable result lists
4. Route key, paste, mouse, and clipboard events through widget-aware dialog
   handlers.
5. Replace substring filtering with ranked fuzzy matching.
6. Add async path discovery for Add Project suggestions, rooted at the nearest
   existing ancestor of the typed path, falling back to `$HOME`.

No backwards compatibility layer. Delete the old project modal row-rendering
and manual text mutation code.

## UX Shape

### Main Project Modal

- Native filter `TextInput` at the top.
- Native `List` for project rows below.
- Ranked fuzzy search over project name and full path.
- Mouse click focuses filter or selects a row.
- Enter activates the selected project.

### Add Project Modal

- `Name` and `Path` are ftui `TextInput`s.
- A live results list appears below `Path`.
- Typing in `Path` triggers fuzzy-ranked repo-root suggestions.
- Enter on a suggestion fills the path field.
- If `Name` is empty, selecting a suggestion auto-fills it from the repo
  directory name.
- Add and Cancel remain explicit actions at the bottom.

### Project Defaults Modal

- Replace the current text-row fields with native `TextInput`s.
- Keep existing field set and semantics.
- Use the same focus, paste, and clipboard model as Add Project.

## Search Model

Use a local fuzzy scorer, not shelling out to `fzf`.

Ranking inputs for Add Project suggestions:

1. basename of repo root
2. full path

Scoring rules:

- case-insensitive fuzzy character match
- bonus for word starts (`/`, `_`, `-`, `.`, space)
- bonus for consecutive matches
- bonus for shorter candidates
- basename score weighted above full-path score

When the path field is empty, show likely repo roots from the scan root rather
than an empty list.

## Search Discovery

Search candidates come from filesystem repo-root discovery:

1. Expand `~`.
2. Determine the nearest existing ancestor of the typed path.
3. If none exists, use `$HOME`.
4. Walk downward from that root and collect directories containing `.git`.
5. Bound traversal so the modal stays responsive.
6. Deliver results back via async task completion and rerank in-memory as the
   query changes.

The first implementation can stay local-only and bounded. No hidden migrations
or compatibility logic.

## Input and Clipboard Behavior

Paste:

- Terminal bracketed paste should work automatically through ftui `TextInput`.
- App-level paste events should route into the focused project modal input.

Copy:

- Reuse Grove's existing clipboard adapter for explicit copy behavior.
- `Alt-c` copies selected text from the focused input, else copies the full
  focused field value.
- `Alt-v` pastes system clipboard text into the focused input when terminal
  paste is unavailable or inconvenient.

Mouse:

- Clicking an input focuses it.
- Clicking a suggestion row selects it.
- Double-click or Enter on the selected suggestion confirms it.

## Architecture

### State

Project dialog state should own widget-native state instead of raw text fields:

- filter `TextInput`
- project `ListState`
- add-dialog `TextInput`s and suggestion `ListState`
- defaults-dialog `TextInput`s
- path search request generation and last completed result set

### View

Project dialog rendering should move away from `FtText` assembly and render a
composed modal body with:

- `Block`
- `TextInput`
- `List`
- `Paragraph` for hints and empty states

### Update

Project dialog update code should:

- delegate `Event::Key` and `Event::Paste` into focused ftui widgets
- map widget state changes back to domain actions
- trigger async path search refreshes
- apply completion messages

## Deleted Code

Delete or replace:

- project dialog row-based editable rendering
- manual `String` push/pop input logic for project dialogs
- substring-only project filtering helper for the modal

Keep only the domain behavior that still matters, like add-project validation
and config persistence.

## Error Handling

- If path search fails, keep the modal usable and show a small inline error.
- If a search root does not exist, fall back to `$HOME`.
- If Add is pressed with an invalid repo path, keep the existing validation
  error behavior.
- If clipboard access fails, show the existing non-fatal error/toast path.

## Testing

- TUI regression tests for project modal paste handling
- TUI regression tests for native focus and list navigation
- fuzzy scorer unit tests
- repo-root discovery tests
- add-project tests for auto-filling name from selected suggestion
- clipboard shortcut tests for `Alt-c` and `Alt-v`

## Why This Shape

- Uses ftui the way the stack intends
- removes duplicated input behavior
- gives paste support almost for free
- unlocks list hit regions and richer mouse interaction
- keeps fuzzy search fully local and testable


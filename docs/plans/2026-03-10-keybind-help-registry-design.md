# Keybind Help Registry Design

**Date:** 2026-03-10

## Summary

Replace Grove's handwritten keybind help overlay with a native ftui help stack.
Use `HelpRegistry` as the source of truth, `HelpIndex` for searchable
discoverability, and ftui's grouped help rendering (`KeybindingHints`, which
wraps `Help`) for the modal UI.

## Problem

The current overlay in
`src/ui/tui/view/view_overlays_help/keybind_overlay.rs` manually assembles
sections, rows, wrapping, and copy. Its content is split across multiple
sources:

- `UiCommandMeta.help_hints` stores preformatted strings
- `UiCommandMeta.palette` stores separate discoverability text
- modal, palette, and interactive help copy is embedded directly in the view

That duplication makes the help overlay drift-prone. A keybind can be changed
in command metadata without the help overlay or discoverability tests noticing.

## Decision

Use a shared help catalog to generate native ftui help content.

### Source of truth

Promote command help metadata from opaque labels like `"? help"` into
structured records:

- context
- category
- key text
- action text
- optional long description / search terms

Keep command-palette metadata where it is, but derive help registrations from
the same command definitions instead of maintaining a second handwritten view.

### Non-command coverage

Some current help rows are not single commands, for example:

- palette search and navigation behavior
- interactive reserved keys
- modal-specific navigation patterns

Those should become explicit synthetic help entries in a dedicated help catalog
module instead of staying embedded in the renderer.

### Rendering

Render the modal with native ftui help widgets:

- `HelpRegistry` stores structured content
- `HelpIndex` indexes that content for tests and future search use
- `KeybindingHints` renders grouped categories and context sections
- `Help` remains the underlying native layout primitive used by
  `KeybindingHints`

This keeps the implementation inside the ftui widget system and avoids
rebuilding grouped help layout inside Grove.

## Architecture

### Help catalog module

Add a dedicated module under `src/ui/tui/` for help registration. It should:

- collect command-derived help entries from `UiCommandMeta`
- define synthetic entries for palette, interactive, and modal flows
- build the visible help model for the current app state
- expose helpers to build a `HelpRegistry`, `HelpIndex`, and grouped
  `KeybindingHints`

### Context model

Keep Grove's existing help contexts because they already describe the overlay
shape well:

- global
- workspace
- list
- preview agent
- preview shell
- preview git
- interactive
- palette
- modals

The visible modal should still show the same major buckets users rely on, but
those buckets should be derived from registered help entries, not manual row
assembly.

### Discoverability contract

Treat "discoverable" as a shared invariant:

1. keybound commands must register help content
2. help-visible entries must be searchable through `HelpIndex`
3. command palette availability rules stay unchanged
4. adding or changing a keybind should require touching one metadata source,
   not a second manual overlay list

## Scope

In scope:

- replace the custom keybind help modal body with ftui-native help rendering
- centralize help registration
- add index-backed regression tests for discoverability
- delete dead custom row-building helpers

Out of scope:

- changing command palette search to query `HelpIndex`
- changing keybind behavior
- redesigning footer text or command categories beyond what the migration
  requires

## Testing

Add or update regression coverage for:

- command metadata to help-registry parity
- `HelpIndex` finding command and synthetic help entries by user-facing terms
- help overlay rendering with native grouped sections
- existing open, close, and input-blocking behavior while help is open

## Why This Shape

- matches the issue's native ftui direction
- removes manual overlay maintenance
- preserves Grove's current help content coverage
- keeps command discoverability and help rendering tied to one data source

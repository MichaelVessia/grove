# Cache Eviction for Agent Runtime Caches

## Motivation

Preventive bounding. These caches grow monotonically today. While typical usage
keeps them small, long-running Grove sessions with many workspace churn could
accumulate stale entries. Bounding them is cheap insurance.

## Steps

1. **Add cache constants in `mod.rs`**
   (`src/application/agent_runtime/mod.rs`)
   - `SESSION_LOOKUP_CACHE_MAX_ENTRIES: usize` (size cap for both codex +
     opencode session-lookup caches)
   - `SESSION_LOOKUP_EVICTION_TTL: Duration` (eviction TTL for session-lookup
     entries, separate from read-hit freshness)
   - `MESSAGE_STATUS_CACHE_MAX_ENTRIES: usize` (size cap only, no TTL)
   - Keep existing read-hit refresh constants unchanged:
     - `CODEX_SESSION_LOOKUP_REFRESH_INTERVAL` (30s)
     - `OPENCODE_SESSION_LOOKUP_REFRESH_INTERVAL` (500ms)

2. **Add a shared `prune_by_oldest` helper** (in `shared.rs` or `mod.rs`, next
   to constants)
   - Single free function: takes `&mut HashMap<K, V>`, max size, two closures:
     - `Fn(&V) -> Instant` for insertion/check timestamp
     - `Fn(&V) -> T` for oldest ordering key where `T: Ord`
   - Accept an optional TTL duration.
   - First pass: remove entries older than TTL (if TTL provided).
   - Second pass: if still over max size, sort remaining by timestamp, drop
     oldest until at cap.
   - ~10 lines, no trait, no abstraction.

3. **Use existing timestamps in entries, do not add extra fields**
   - Codex `SessionLookupCacheEntry` already has `checked_at` (use that).
   - OpenCode `SessionLookupCacheEntry` already has `checked_at` (use that).
   - `MessageStatusCacheEntry` uses `modified_at` for oldest-first size pruning.
   - **No TTL for `MessageStatusCacheEntry`**. It already invalidates by file
     mtime, TTL would force re-parsing unchanged files for no benefit.

4. **Call prune on write paths only**
   - Session-lookup caches (codex + opencode): prune with TTL + size cap after
     inserting.
   - Message-status cache (codex): prune with size cap only (oldest
     `modified_at` first) after inserting.
   - No read-path pruning. Reads already skip stale entries via existing
     freshness checks.

5. **Keep all prune+mutate operations under existing mutex locks, no lock model
   change.**

6. **Add focused tests** in `codex.rs` and `opencode.rs`:
   - Size cap prunes oldest entries when over limit.
   - TTL expiry removes stale session-lookup entries (codex + opencode only).
   - Message-status cache retains entries within size cap regardless of age.
   - Post-eviction lookup recomputes correctly.
   - Add test-only cache reset helpers and call them at test start to avoid
     cross-test contamination from global `OnceLock` caches.

7. **Run only touched tests first** (`cargo test` for those modules), then
   `make precommit`.

8. **Handoff** with exact file:line deltas, commands run, and any follow-up
   risks.

## Scope

- **In scope**: codex session-lookup cache, codex message-status cache,
  opencode session-lookup cache.
- **Out of scope**: generic cache type, cross-module refactor, LRU data
  structures.

## Review Gates (resolve before coding)

1. Confirm max-entry values for session-lookup (1024?) and message-status (512?
   1024?).
2. Confirm session-lookup eviction TTL (5 minutes?).
3. Confirm eviction implementation: full scan + sort by timestamp, O(n log n),
   acceptable at these sizes.

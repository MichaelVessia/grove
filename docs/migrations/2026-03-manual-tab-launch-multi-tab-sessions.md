# Migration: 2026-03 Manual Tab Launch + Multi-Tab Sessions

This migration guide is for the manual-tab-launch rollout (permanent Home tab, manual tab launch, multi-tab session model).

## Who Needs This

Anyone who already has Grove workspaces and/or running `tmux` sessions created before this rollout.

## What Changed

- Workspace tabs are now dynamic, with a permanent `Home` tab.
- Agent and shell sessions are launched manually via tabs (`a`, `s`, `g`).
- Session naming is now tab-instance based:
  - Agent: `grove-ws-<project>-<workspace>-agent-<n>`
  - Shell: `grove-ws-<project>-<workspace>-shell-<n>`
  - Git: `grove-ws-<project>-<workspace>-git`
- Startup tab restore uses tmux tab metadata (`@grove_workspace_path`, `@grove_tab_kind`, etc).

## Expected Impact After Upgrade

- Existing worktree directories are still discovered.
- Legacy sessions without tab metadata are not restored into tabs.
- Users can see `Home` with no running tabs even if legacy sessions still exist in tmux.

## Migration Runbook

Run from your Grove repo root.

1. Preview Grove-managed cleanup candidates:

```bash
grove cleanup sessions --include-stale --include-attached
# fallback: cargo run -- cleanup sessions --include-stale --include-attached
```

2. Apply Grove-managed cleanup:

```bash
grove cleanup sessions --include-stale --include-attached --apply
# fallback: cargo run -- cleanup sessions --include-stale --include-attached --apply
```

If you want to preserve attached tmux sessions, omit `--include-attached`.

3. List remaining legacy Grove sessions (sessions missing tab metadata):

```bash
tmux list-sessions -F '#{session_name}' \
| rg '^grove-ws-' \
| while IFS= read -r session; do
    kind="$(tmux show-options -qv -t "$session" @grove_tab_kind 2>/dev/null || true)"
    if [ -z "$kind" ]; then
      echo "$session"
    fi
  done
```

4. If the list looks correct, kill those legacy sessions:

```bash
tmux list-sessions -F '#{session_name}' \
| rg '^grove-ws-' \
| while IFS= read -r session; do
    kind="$(tmux show-options -qv -t "$session" @grove_tab_kind 2>/dev/null || true)"
    if [ -z "$kind" ]; then
      tmux kill-session -t "$session"
      echo "killed $session"
    fi
  done
```

5. Relaunch Grove, open desired tabs from `Home` (`a`, `s`, `g`).

## Optional: Adopt Legacy Sessions In-Place (Rename + Metadata)

If you want to preserve live legacy sessions instead of killing them, use:

```bash
scripts/migrations/adopt-legacy-tmux-sessions-2026-03.sh
```

Then apply:

```bash
scripts/migrations/adopt-legacy-tmux-sessions-2026-03.sh --apply
```

Notes:
- Default mode is dry-run.
- By default, attached sessions are skipped.
- Add `--include-attached` if you explicitly want attached sessions migrated too.

## Team Announcement Snippet

```text
We merged Grove's manual-tab-launch + multi-tab session model.

If you had existing Grove tmux sessions from before this merge, run the migration guide:

docs/migrations/2026-03-manual-tab-launch-multi-tab-sessions.md

Why: old sessions without tab metadata are not auto-adopted into new tabs.
```

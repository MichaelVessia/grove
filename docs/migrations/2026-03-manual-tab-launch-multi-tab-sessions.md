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

## Recommended: Agent-Driven Migration

Do not run commands manually first, have your coding agent run this migration for you.

From your Grove repo root, paste this to your agent:

```text
Run the migration in docs/migrations/2026-03-manual-tab-launch-multi-tab-sessions.md.

Requirements:
1) Run dry-run commands first and show me output.
2) Before any destructive step (`--apply`, `tmux kill-session`, session rename), ask for explicit confirmation.
3) Prefer adopting legacy sessions with `scripts/migrations/adopt-legacy-tmux-sessions-2026-03.sh` over killing them.
4) After apply, verify there are no remaining legacy Grove sessions (missing `@grove_tab_kind`).
5) Summarize exactly what changed.
```

## Agent Runbook (Preferred Path)

Run from your Grove repo root.

1. Preview Grove-managed cleanup candidates:

```bash
grove cleanup sessions --include-stale --include-attached
# fallback: cargo run -- cleanup sessions --include-stale --include-attached
```

2. Preview legacy session adoption (dry-run):

```bash
scripts/migrations/adopt-legacy-tmux-sessions-2026-03.sh
```

3. Ask user for confirmation, then apply adoption:

```bash
scripts/migrations/adopt-legacy-tmux-sessions-2026-03.sh --apply
```

Add `--include-attached` only if the user confirms attached sessions should be modified.

4. Verify there are no remaining legacy sessions (missing tab metadata):

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

5. Only if legacy sessions still remain, ask user confirmation and kill only those leftovers:

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

6. Relaunch Grove, open desired tabs from `Home` (`a`, `s`, `g`).

## Manual Fallback (No Agent)

If you are not using an agent, run this exactly in order:

```bash
scripts/migrations/adopt-legacy-tmux-sessions-2026-03.sh
```

Review output, then apply:

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

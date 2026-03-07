# Migration: 2026-03 Task Model Single-Workspace Import

This migration guide is for the task-model rollout where existing legacy
workspace discovery needs to be converted into task manifests.

## Who Needs This

Anyone who already has Grove projects/workspaces from before the task-model
rollout, and now opens Grove to an empty app with no tasks/projects visible.

## What Changed

- Grove now boots from task manifests under `~/.grove/tasks/`.
- Legacy config-backed project/workspace discovery is not used once task mode is
  active.
- Existing worktree directories still exist on disk, but Grove needs a
  `task.toml` manifest for each task before it can show them again.

## Migration Shape For This Repo

For this one-time migration, keep it simple:

- each legacy workspace becomes one task
- each migrated task has exactly one worktree
- do not move, rename, or recreate existing worktree directories
- use the existing workspace directory as both:
  - `task.root_path`
  - `worktrees[0].path`

If two workspaces would produce the same task slug, prefix with repository name.

## Expected Impact After Upgrade

- Existing directories remain where they are today.
- Grove will show one task per legacy workspace after manifests are written.
- Running legacy `grove-ws-*` tmux sessions are not part of this manifest
  migration automatically.
- Users may need to relaunch agents after migration unless they separately
  choose to clean up or adopt old sessions.

## Recommended: Agent-Driven Migration

Do not edit files manually first, have your coding agent run this migration for
you.

From your Grove repo root, paste this to your agent:

```text
Run the migration in docs/migrations/2026-03-task-model-single-workspace-agent-migration.md.

Goal:
- Migrate all legacy Grove workspaces into task manifests
- Each legacy workspace becomes exactly one single-worktree task
- Do not move or rename existing worktree directories

Requirements:
1) Run discovery in dry-run mode first and show me:
   - configured repositories/projects found
   - legacy workspaces discovered
   - proposed task slugs
   - proposed manifest paths
2) Before any file writes, ask for explicit confirmation.
3) Create timestamped backups before writing:
   - ~/.grove/tasks.bak-<timestamp> if ~/.grove/tasks exists
   - Grove config files that contain project definitions
4) For each discovered legacy workspace, create exactly one manifest at:
   - ~/.grove/tasks/<task-slug>/.grove/task.toml
5) For each manifest:
   - name = legacy workspace name
   - slug = unique task slug
   - root_path = existing workspace path
   - branch = legacy workspace branch
   - one worktree only
6) For that one worktree:
   - repository_name = configured project name, or repo basename if missing
   - repository_path = project/repo root
   - path = existing workspace path
   - branch = legacy workspace branch
   - base_branch = existing Grove base marker if present, otherwise preserve discovered value, otherwise default to "main"
   - agent = preserve discovered/default agent if available, otherwise "codex"
   - status = "idle"
   - is_orphaned = false
   - supported_agent = true
   - pull_requests = []
7) Do not modify workspace directories themselves.
8) Do not delete or alter tmux sessions unless I explicitly ask.
9) After writing manifests, verify:
   - manifest count matches discovered legacy workspace count
   - Grove can now discover tasks from ~/.grove/tasks
   - show me 2-3 example manifests
10) Summarize exactly what changed:
   - repositories scanned
   - workspaces migrated
   - slug collisions resolved
   - backup paths
   - files written
```

## Agent Runbook (Preferred Path)

Run from your Grove repo root.

1. Determine where Grove project definitions live.

Check the platform-appropriate Grove config directory first:

- macOS: `~/Library/Application Support/grove/`
- Linux: `~/.config/grove/`

Read whichever file currently contains project definitions, usually:

- `config.toml`
- `projects.toml`

2. Discover legacy workspaces from the configured repositories.

For each configured repository/project:

```bash
git -C <repo-root> worktree list --porcelain
```

Also inspect existing Grove metadata when present:

```bash
cat <workspace-path>/.grove/base
```

Use that to build the migration plan:

- workspace name
- workspace path
- repository name
- repository path
- branch
- base branch
- proposed task slug
- target manifest path

3. Show the dry-run plan to the user.

The dry-run should include:

- total repositories scanned
- total workspaces discovered
- total manifests to write
- any slug collisions and the chosen resolution

4. Ask for confirmation, then create backups.

Back up:

- `~/.grove/tasks` if it exists
- Grove config files read during discovery

5. Write one task manifest per legacy workspace.

Manifest location:

```text
~/.grove/tasks/<task-slug>/.grove/task.toml
```

Manifest shape:

```toml
name = "<workspace-name>"
slug = "<task-slug>"
root_path = "<existing-workspace-path>"
branch = "<workspace-branch>"

[[worktrees]]
repository_name = "<repository-name>"
repository_path = "<repository-root>"
path = "<existing-workspace-path>"
branch = "<workspace-branch>"
base_branch = "<base-branch>"
last_activity_unix_secs = null
agent = "codex"
status = "idle"
is_orphaned = false
supported_agent = true
pull_requests = []
```

Preserve better values when already known, but do not invent extra structure.

6. Verify the migration.

At minimum:

- count manifests under `~/.grove/tasks`
- confirm count matches discovered workspaces
- re-read a few manifests
- if possible, validate discovery by reopening Grove or using the app's task
  discovery path

7. Leave tmux sessions alone by default.

If the user wants cleanup after the manifest migration, handle that as a second,
explicitly confirmed step.

## Manual Fallback (No Agent)

If you are not using an agent, do this exactly:

1. Back up `~/.grove/tasks` if it exists.
2. Back up Grove config files that define projects.
3. For each configured repository, run:

```bash
git -C <repo-root> worktree list --porcelain
```

4. For each discovered legacy workspace, create:

```text
~/.grove/tasks/<task-slug>/.grove/task.toml
```

5. Write one single-worktree manifest pointing at the existing workspace path.
6. Relaunch Grove.

Notes:

- Do not move directories.
- Do not nest the existing worktree under `~/.grove/tasks/<task-slug>/`.
- `~/.grove/tasks/` stores manifests, not the migrated workspace data itself.

## Team Announcement Snippet

```text
We merged Grove's task-model rollout.

If Grove now opens with no tasks/projects, run the one-time migration guide:

docs/migrations/2026-03-task-model-single-workspace-agent-migration.md

This imports each legacy workspace as one single-worktree task manifest without
moving existing directories.
```

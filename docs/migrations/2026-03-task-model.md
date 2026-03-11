# Migration: 2026-03 Task Model

This is the canonical migration guide for moving pre-task-model Grove data into
the current task-first architecture.

## Who Needs This

You need this migration if all of the following are true:

- you used Grove before the task-model rollout
- you still have legacy Grove worktrees on disk
- Grove now starts with no tasks, or fewer tasks than you expect

You do not need this migration if all of your tasks were created by current
task-first Grove.

## What Changed

Grove no longer discovers runtime state from legacy workspace-era storage.

- startup and refresh now read task manifests only
- task manifests live under `~/.grove/tasks/<task-slug>/.grove/task.toml`
- the runtime model is `task -> worktrees`
- task-root tmux sessions use `grove-task-<task-slug>`
- worktree tmux sessions use `grove-wt-<task-slug>-<repository>`

Important consequences:

- old `~/.grove/workspaces` directories are not discovered automatically
- old `grove-ws-*` tmux sessions are not adopted automatically
- migration must happen outside Grove, before Grove can see legacy work

## Migration Policy

The supported migration shape is:

- each legacy Grove workspace becomes one task
- each migrated task has exactly one worktree
- existing worktree directories stay where they are
- Grove gains a new manifest entry under `~/.grove/tasks/`

This is the safest migration because it does not move git worktrees and does
not rely on runtime compatibility shims.

New tasks created by Grove use the canonical task directory layout. Imported
legacy tasks may keep their worktree path outside `~/.grove/tasks/`, with the
manifest directory acting as the task entrypoint.

## Before You Start

Back up:

- `~/.grove/tasks/`, if it already exists
- Grove config files, usually:
  - macOS: `~/Library/Application Support/grove/config.toml`
  - macOS: `~/Library/Application Support/grove/projects.toml`
  - Linux: `~/.config/grove/config.toml`
  - Linux: `~/.config/grove/projects.toml`

Current split-config shape:

- `config.toml` stores only global settings such as `sidebar_width_pct`,
  `theme`, and `launch_skip_permissions`
- `projects.toml` stores repository discovery state under `[[projects]]`, plus
  top-level `task_order` and `[[attention_acks]]`

Do not delete legacy worktrees or tmux sessions first.

## What Counts As A Legacy Grove Workspace

For a configured repository, treat a non-main git worktree as a legacy Grove
workspace only if at least one of these is true:

- `<worktree>/.grove/base` exists
- `<worktree>/.grove/agent` exists
- the worktree lives under the old Grove-managed root, typically
  `~/.grove/workspaces/`

Do not import arbitrary non-Grove git worktrees by accident.

## Data Mapping

Map legacy data to the task manifest like this:

| Legacy concept | Task manifest field |
| --- | --- |
| workspace name | `task.name` |
| unique slug | `task.slug` |
| existing workspace path | `task.root_path` |
| workspace branch | `task.branch` |
| configured repository display name | `worktrees[].repository_name` |
| repository root | `worktrees[].repository_path` |
| existing workspace path | `worktrees[].path` |
| workspace branch | `worktrees[].branch` |
| `.grove/base` contents | `worktrees[].base_branch` |
| legacy agent marker or chosen default | `worktrees[].agent` |

Use these default values unless you have a better known value:

- `last_activity_unix_secs = null`
- `status = "idle"`
- `is_orphaned = false`
- `supported_agent = true`
- `pull_requests = []`

Allowed agent strings:

- `"claude"`
- `"codex"`
- `"opencode"`

Allowed status strings include:

- `"main"`
- `"idle"`
- `"active"`
- `"thinking"`
- `"waiting"`
- `"done"`
- `"error"`
- `"unknown"`
- `"unsupported"`

For migration, use `"idle"` unless you are intentionally authoring another
state.

## Slug Rules

Task slugs must be non-empty and use only:

- `A-Z`
- `a-z`
- `0-9`
- `_`
- `-`

Recommended slug rule:

- start with the legacy workspace name if it is already slug-safe
- otherwise normalize it to the allowed character set
- if two workspaces would collide, prefix with repository name

## Recommended Agent-Driven Runbook

From the Grove repo root, give your coding agent this prompt:

```text
Run the migration in docs/migrations/2026-03-task-model.md.

Requirements:
1. Discover legacy Grove workspaces in dry-run mode first.
2. Before any file writes, show me:
   - repositories scanned
   - legacy workspaces found
   - proposed task slugs
   - proposed manifest paths
3. Ask for explicit confirmation before writing anything.
4. Create timestamped backups of:
   - ~/.grove/tasks if it exists
   - Grove config files used for discovery
5. Migrate each legacy workspace as exactly one single-worktree task.
6. Do not move or rename existing worktree directories.
7. Do not kill or rename tmux sessions unless I explicitly ask.
8. Ensure every migrated worktree has a valid .grove/base file.
9. After writing manifests, verify:
   - manifest count matches migrated workspace count
   - Grove can now discover tasks from ~/.grove/tasks
   - show me 2-3 example manifests
10. Summarize:
   - repositories scanned
   - workspaces migrated
   - slug collisions
   - backup paths
   - files written
```

## Manual Runbook

### 1. Find The Configured Repositories

Read Grove's `projects.toml` from the global config directory. Repository
definitions live under `[[projects]]`.

The same file may also contain:

- top-level `task_order`
- top-level `[[attention_acks]]`

Those entries are part of real global config state but are not inputs to this
task-manifest migration.

Each repository entry needs:

- `name`
- `path`
- optional `defaults.base_branch`

These map to:

- `repository_name`
- `repository_path`

### 2. Discover Legacy Workspaces

For each configured repository root:

```bash
git -C <repo-root> worktree list --porcelain
```

For each listed worktree:

- exclude the main checkout whose path equals the repository root
- inspect candidate markers:

```bash
test -f <worktree-path>/.grove/base && cat <worktree-path>/.grove/base
test -f <worktree-path>/.grove/agent && cat <worktree-path>/.grove/agent
```

Collect:

- workspace path
- workspace branch
- repository name
- repository root
- configured project default base branch
- base branch
- agent

Decision rules:

- `base_branch`
  - first choice: `<worktree>/.grove/base`
  - otherwise: non-empty `projects.defaults.base_branch`
  - otherwise: detect the same way Grove does:
    - `git -C <repo-root> symbolic-ref --quiet --short refs/remotes/origin/HEAD`
    - `git -C <repo-root> branch --show-current`
    - local branch `main`
    - local branch `master`
  - if all of those fail, stop and choose a base branch explicitly, do not
    silently invent `"main"`
- `agent`
  - first choice: `<worktree>/.grove/agent`
  - otherwise: `"codex"`

### 3. Create The Manifest Directory

For each migrated workspace, create:

```text
~/.grove/tasks/<task-slug>/.grove/task.toml
```

The manifest directory is required even if the actual worktree remains
elsewhere.

### 4. Ensure The Worktree Still Has `.grove/base`

Current merge/update flows require a non-empty base marker inside the migrated
worktree itself.

If the file is missing, create it:

```bash
mkdir -p <worktree-path>/.grove
printf '%s\n' '<base-branch>' > <worktree-path>/.grove/base
```

### 5. Write The Manifest

Template:

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

Example:

```toml
name = "feature-auth-v2"
slug = "grove-feature-auth-v2"
root_path = "/Users/me/.grove/workspaces/grove-a1b2c3/grove-feature-auth-v2"
branch = "feature-auth-v2"

[[worktrees]]
repository_name = "grove"
repository_path = "/Users/me/src/grove"
path = "/Users/me/.grove/workspaces/grove-a1b2c3/grove-feature-auth-v2"
branch = "feature-auth-v2"
base_branch = "main"
last_activity_unix_secs = null
agent = "codex"
status = "idle"
is_orphaned = false
supported_agent = true
pull_requests = []
```

### 6. Verify The Migration

At minimum:

1. Count manifests:

```bash
find ~/.grove/tasks -path '*/.grove/task.toml' | wc -l
```

2. Re-read a few manifests:

```bash
sed -n '1,200p' ~/.grove/tasks/<task-slug>/.grove/task.toml
```

3. Relaunch Grove and confirm the migrated tasks appear.

If Grove still does not show a migrated task, check:

- the manifest path is exactly `~/.grove/tasks/<task-slug>/.grove/task.toml`
- `slug`, `branch`, and `repository_name` are non-empty
- `agent` is one of `claude`, `codex`, `opencode`
- `status` is a supported value, usually `idle`

## Tmux Sessions

This migration does not adopt legacy `grove-ws-*` tmux sessions.

After task manifests exist, you have two options:

- simplest: leave old sessions alone, reopen Grove, and launch fresh
  task/worktree sessions
- optional follow-up: clean up old sessions separately

If you also need the tmux session migration, use:

- [docs/migrations/2026-03-manual-tab-launch-multi-tab-sessions.md](/Users/michael.vessia/projects/grove/docs/migrations/2026-03-manual-tab-launch-multi-tab-sessions.md)

## Common Mistakes

- Importing the main repository checkout as a migrated task.
- Forgetting to create `~/.grove/tasks/<task-slug>/.grove/task.toml`.
- Assuming the manifest alone is enough, but leaving `<worktree>/.grove/base`
  missing.
- Reusing the same slug for two workspaces.
- Expecting old `grove-ws-*` sessions to appear automatically after migration.

## Success Criteria

The migration is complete when:

1. every intended legacy workspace has a manifest entry under `~/.grove/tasks`
2. each migrated worktree still exists on disk
3. each migrated worktree has a valid `.grove/base`
4. Grove launches and discovers the migrated tasks
5. old session cleanup, if desired, is handled as a separate explicit step

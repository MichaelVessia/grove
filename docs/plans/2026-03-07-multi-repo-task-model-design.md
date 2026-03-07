# Multi-Repo Task Model Design

## Goal

Replace Grove's top-level `workspace` concept with a first-class `Task` that can span one or many repositories, while keeping real git worktrees as the per-repository primitive underneath.

## Why This Change

Grove started from a repo-first model:

- configure repositories
- discover git worktrees per repository
- treat each worktree as the top-level item in the UI

That model breaks down for cross-repo delivery work. The user intent is not "manage three unrelated worktrees". The user intent is "work on one task that touches three repositories". The current domain makes that intent invisible, so users have to manually coordinate multiple workspaces and agent sessions.

The new model should make the user concept primary:

- `Repository` is where code comes from
- `Task` is what the user is trying to deliver
- `Worktree` is one repository checkout inside that task

## Core Decision

Adopt `Task` as the top-level domain object for both single-repository and multi-repository work.

Single-repository work is not a separate mode. It is a `Task` with one `Worktree`.

This avoids a split model where old "workspace" behavior coexists with new "aggregate workspace" behavior. The UI, lifecycle, runtime, and persistence layers all move to task-first terminology and task-first state.

## Domain Model

### Repository

A configured source repository.

Maps to today's `ProjectConfig`.

Suggested fields:

```rust
pub struct RepositoryConfig {
    pub name: String,
    pub path: PathBuf,
    pub defaults: RepositoryDefaults,
}
```

### Task

The top-level unit of work.

Every task has a real filesystem root under `~/.grove/tasks/<task-slug>/`.

Suggested fields:

```rust
pub struct Task {
    pub name: String,
    pub slug: String,
    pub root_path: PathBuf,
    pub branch: String,
    pub repositories: Vec<RepositoryRef>,
    pub worktrees: Vec<Worktree>,
    pub parent_agent: Option<AgentSession>,
    pub created_at_unix_secs: i64,
}
```

### Worktree

One repository-specific checkout inside a task.

Each worktree belongs to exactly one task and one repository.

Suggested fields:

```rust
pub struct Worktree {
    pub repository_name: String,
    pub repository_path: PathBuf,
    pub path: PathBuf,
    pub branch: String,
    pub base_branch: Option<String>,
    pub agent: AgentType,
    pub status: WorktreeStatus,
    pub is_orphaned: bool,
    pub supported_agent: bool,
    pub pull_requests: Vec<PullRequest>,
}
```

### AgentSession

Existing runtime session concept stays, but its scope expands.

Suggested fields:

```rust
pub enum SessionScope {
    TaskRoot,
    Worktree,
}

pub struct AgentSession {
    pub scope: SessionScope,
    pub session_name: String,
    pub pane_id: String,
    pub status: RuntimeStatus,
    pub last_output_at: Option<i64>,
}
```

## Filesystem Layout

Each task gets a real managed root, not a virtual grouping and not a symlink tree.

```text
~/.grove/tasks/flohome-launch/
  .grove/task.toml
  infra-base-services/
  terraform-fastly/
  flohome/
```

Properties of this layout:

- the task root is the canonical cwd for the parent agent
- each child directory is a real git worktree for one configured repository
- there is no path indirection, no symlink maintenance, no inferred grouping
- task deletion is a single lifecycle operation rooted at one directory

## Persistence

Task-local metadata becomes the source of truth.

Each task root contains a manifest:

```text
~/.grove/tasks/<task-slug>/.grove/task.toml
```

Suggested manifest shape:

```toml
name = "flohome-launch"
slug = "flohome-launch"
branch = "flohome-launch"
created_at_unix_secs = 1772841600

[parent_agent]
agent = "codex"
session_name = "grove-task-flohome-launch"

[[worktrees]]
repository_name = "infra-base-services"
repository_path = "/repos/infra-base-services"
path = "/Users/michael/.grove/tasks/flohome-launch/infra-base-services"
branch = "flohome-launch"
base_branch = "main"
agent = "codex"

[[worktrees]]
repository_name = "terraform-fastly"
repository_path = "/repos/terraform-fastly"
path = "/Users/michael/.grove/tasks/flohome-launch/terraform-fastly"
branch = "flohome-launch"
base_branch = "main"
agent = "codex"
```

Per-worktree marker files may remain as runtime hints if they simplify local detection, but they are no longer the primary data model. The canonical record is the task manifest.

## Discovery

Discovery becomes task-first, not repository-first.

New startup model:

1. Scan `~/.grove/tasks/`
2. Read each `.grove/task.toml`
3. Build in-memory `Task` values
4. Reconcile declared worktrees against git state
5. Reconcile task and worktree sessions against tmux state
6. Surface drift explicitly

This changes the role of git discovery:

- before, git worktree enumeration was the primary model
- after, git worktree enumeration is a validation and repair input

Examples of drift Grove should detect:

- declared worktree path is missing
- path exists but is not a git worktree
- worktree points at the wrong repository
- worktree is on the wrong branch
- task session is missing
- worktree session is missing or orphaned

## Lifecycle

### Create Task

Creation flow:

1. User selects one or more configured repositories
2. User enters a task name
3. Grove derives a shared branch slug from the task name
4. Grove creates task root under `~/.grove/tasks/<slug>/`
5. Grove creates one git worktree per selected repository under the task root
6. Grove writes `.grove/task.toml`
7. Grove optionally starts the parent agent

Creation is uniform for single-repository and multi-repository tasks.

### Start Parent Agent

The parent agent runs in the task root. The task root is not a git repository, and that is intentional. Its purpose is coordination, planning, and cross-repository file access.

### Start Worktree Agent

Each worktree can still run its own repository-scoped agent in the worktree directory. Repository-specific git operations and setup remain worktree-scoped.

### Delete Task

Delete flow:

1. stop parent session
2. stop worktree sessions
3. remove each git worktree from its source repository
4. remove task root
5. remove task manifest

### Merge and Update

Merge and update-from-base remain worktree-scoped operations. There is no synthetic "merge task" git operation because the task root is not a repository.

## Runtime Model

The runtime now has two scopes:

- task scope
- worktree scope

Implications:

- status must exist at both scopes
- session naming must distinguish task-root sessions from worktree sessions
- polling must capture parent task output separately from worktree output
- attention state may come from either the parent agent or a worktree agent

Recommended status model:

- `TaskStatus` derived from parent agent presence plus aggregate worktree attention
- `WorktreeStatus` derived from repository-scoped agent state

The task should not pretend to be a git checkout. The task status is orchestration status, not repository status.

## UI Model

The sidebar should list tasks, not raw worktrees.

Suggested interaction model:

- sidebar rows represent tasks
- selected task expands or reveals its worktrees
- single-repository task still renders as a task with one worktree
- preview pane defaults to task-root parent agent output
- commands act on either selected task or selected worktree, depending on focus

Required vocabulary shift:

- "workspace" becomes "task" in UI copy
- "project" becomes "repository" where possible
- worktree-specific operations must say "worktree"

The UI should make scope explicit so the user always knows whether a command affects:

- the task root
- one repository worktree

## Configuration

Configured repositories remain global app configuration. Existing `ProjectConfig` can be renamed to `RepositoryConfig`.

Task state should not be stored in `~/.config/grove/projects.toml`. Tasks live under `~/.grove/tasks/` because tasks are runtime working state, not static repository configuration.

## Migration

Breaking migration is acceptable and preferred over prolonged dual-mode support.

Migration strategy:

- convert configured "projects" terminology to "repositories"
- convert every existing Grove-managed single-repository workspace into a one-worktree task
- write a migration guide in `docs/migrations/`
- optionally provide a one-shot importer for legacy workspaces if it materially reduces manual cleanup

Explicit non-goal:

- no long-term compatibility layer where old workspace mode and new task mode coexist indefinitely

## Why Not a Collection or Symlink Model

A collection or symlink layer keeps the old model intact and adds an aggregate shell around it. That looks cheaper initially, but it creates the wrong long-term boundary:

- the top-level user object still does not exist as a first-class domain object
- discovery remains repo-first
- lifecycle stays fragmented
- parent agent cwd becomes synthetic
- drift handling becomes more complex because grouping is inferred

That is exactly the class of hack this design should avoid.

## Risks

- large rename and vocabulary migration through domain, UI, logging, and docs
- replay/debug snapshots and task capture models will need schema changes
- session naming needs a careful transition to avoid collisions between task and worktree sessions
- merge/update/delete flows must stay worktree-scoped even as the UI becomes task-scoped

## Non-Goals

- no support for main worktrees inside tasks
- no compatibility mode where tasks can be virtual references to old worktree buckets
- no attempt to make the task root behave like a git repository

## Recommendation

Implement the full task-first architecture:

- `Task` becomes the primary object
- `Worktree` remains the git primitive beneath it
- task manifests become the source of truth
- discovery becomes task-first
- UI becomes task-first
- single-repository work uses the same model as multi-repository work

# Grove Config Split Migration Prompt (One-Time)

Copy this prompt into your AI coding assistant (Codex/Claude), run once.

```text
Migrate my Grove config from legacy single-file format to split files.

Repository: grove
Goal:
- Keep global settings in ~/.config/grove/config.toml
- Move mutable project state to ~/.config/grove/projects.toml

Do exactly this:
1) Read ~/.config/grove/config.toml (legacy source).
2) If file is missing, stop and report.
3) Create a timestamped backup:
   - ~/.config/grove/config.toml.bak-<timestamp>
   - if ~/.config/grove/projects.toml exists, back it up too
4) Parse legacy config and write new ~/.config/grove/config.toml with only:
   - sidebar_width_pct (default 33 if missing)
   - launch_skip_permissions (default false if missing)
5) Write ~/.config/grove/projects.toml with:
   - projects array/table content from legacy config
   - attention_acks array/table content from legacy config
   - if missing in legacy config, write empty defaults
6) Do not modify any other Grove files.
7) Print a short summary:
   - values written to config.toml
   - number of projects migrated
   - number of attention_acks migrated
   - backup file paths
```

Expected result:
- `~/.config/grove/config.toml` is stable/declarative-friendly.
- `~/.config/grove/projects.toml` is Grove-owned mutable state.
- `sidebar_width_pct` and `launch_skip_permissions` are managed by file edits,
  not runtime UI writes.

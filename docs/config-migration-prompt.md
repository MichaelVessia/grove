# Grove Config Split Migration Prompt (One-Time)

Copy this prompt into your AI coding assistant (Codex/Claude), run once.

## Config directory

Grove uses the Rust `dirs::config_dir()` crate to resolve its config directory.
The actual path depends on the platform:

| Platform | Config directory |
|----------|-----------------|
| macOS | `~/Library/Application Support/grove/` |
| Linux | `~/.config/grove/` |

Throughout this prompt, `<CONFIG_DIR>` refers to the platform-appropriate path
above. **Check both locations** if unsure which one your system uses.

## Migration prompt

```text
Migrate my Grove config from legacy single-file format to split files.

Repository: grove
Goal:
- Keep global settings in <CONFIG_DIR>/config.toml
- Move mutable project state to <CONFIG_DIR>/projects.toml

The config directory depends on the platform:
- macOS: ~/Library/Application Support/grove/
- Linux: ~/.config/grove/

Do exactly this:
1) Determine the config directory for this platform.
2) Read <CONFIG_DIR>/config.toml (legacy source).
3) If file is missing, stop and report.
4) Create a timestamped backup:
   - <CONFIG_DIR>/config.toml.bak-<timestamp>
   - if <CONFIG_DIR>/projects.toml exists, back it up too
5) Parse legacy config and write new <CONFIG_DIR>/config.toml with only:
   - sidebar_width_pct (default 33 if missing)
   - launch_skip_permissions (default false if missing)
6) Write <CONFIG_DIR>/projects.toml with:
   - projects array/table content from legacy config
   - attention_acks array/table content from legacy config
   - if missing in legacy config, write empty defaults
7) Do not modify any other Grove files.
8) Print a short summary:
   - values written to config.toml
   - number of projects migrated
   - number of attention_acks migrated
   - backup file paths
```

## Expected result

- `<CONFIG_DIR>/config.toml` is stable/declarative-friendly.
- `<CONFIG_DIR>/projects.toml` is Grove-owned mutable state.
- `sidebar_width_pct` and `launch_skip_permissions` are managed by file edits,
  not runtime UI writes.

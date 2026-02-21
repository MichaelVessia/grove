use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilitySnapshot {
    pub local_lifecycle: bool,
    pub in_process_service: bool,
    pub daemon_transport: bool,
    pub remote_transport: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandDescriptor {
    pub command: String,
    pub description: String,
    pub usage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RootCommandTree {
    pub command: String,
    pub summary: String,
    pub capabilities: CapabilitySnapshot,
    pub commands: Vec<CommandDescriptor>,
}

pub fn root_command_tree() -> RootCommandTree {
    RootCommandTree {
        command: "grove".to_string(),
        summary: "Agent-first lifecycle command tree".to_string(),
        capabilities: CapabilitySnapshot {
            local_lifecycle: true,
            in_process_service: true,
            daemon_transport: false,
            remote_transport: false,
        },
        commands: vec![
            command("grove tui", "Launch Grove TUI", "grove tui [--repo <path>]"),
            command(
                "grove workspace list",
                "List workspaces in a repository",
                "grove workspace list [--repo <path>]",
            ),
            command(
                "grove workspace create",
                "Create a workspace",
                "grove workspace create --name <name> [--base <branch> | --existing-branch <branch>] [--agent <claude|codex>] [--start] [--dry-run] [--repo <path>]",
            ),
            command(
                "grove workspace edit",
                "Edit workspace metadata",
                "grove workspace edit [--workspace <name> | --workspace-path <path>] [--agent <claude|codex>] [--base <branch>] [--repo <path>]",
            ),
            command(
                "grove workspace delete",
                "Delete a workspace",
                "grove workspace delete [--workspace <name> | --workspace-path <path>] [--delete-branch] [--force-stop] [--dry-run] [--repo <path>]",
            ),
            command(
                "grove workspace merge",
                "Merge a workspace branch",
                "grove workspace merge [--workspace <name> | --workspace-path <path>] [--cleanup-workspace] [--cleanup-branch] [--dry-run] [--repo <path>]",
            ),
            command(
                "grove workspace update",
                "Update workspace from base branch",
                "grove workspace update [--workspace <name> | --workspace-path <path>] [--dry-run] [--repo <path>]",
            ),
            command(
                "grove agent start",
                "Start an agent in a workspace",
                "grove agent start [--workspace <name> | --workspace-path <path>] [--prompt <text>] [--pre-launch <cmd>] [--skip-permissions] [--dry-run] [--repo <path>]",
            ),
            command(
                "grove agent stop",
                "Stop an agent in a workspace",
                "grove agent stop [--workspace <name> | --workspace-path <path>] [--dry-run] [--repo <path>]",
            ),
        ],
    }
}

fn command(command: &str, description: &str, usage: &str) -> CommandDescriptor {
    CommandDescriptor {
        command: command.to_string(),
        description: description.to_string(),
        usage: usage.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::root_command_tree;
    use serde_json::json;
    use std::collections::BTreeSet;

    fn expected_command_set() -> BTreeSet<&'static str> {
        BTreeSet::from([
            "grove tui",
            "grove workspace list",
            "grove workspace create",
            "grove workspace edit",
            "grove workspace delete",
            "grove workspace merge",
            "grove workspace update",
            "grove agent start",
            "grove agent stop",
        ])
    }

    #[test]
    fn root_command_tree_lists_all_v1_lifecycle_commands() {
        let tree = root_command_tree();
        let listed: BTreeSet<&str> = tree
            .commands
            .iter()
            .map(|entry| entry.command.as_str())
            .collect();
        assert_eq!(listed, expected_command_set());
    }

    #[test]
    fn root_command_tree_provides_usage_template_for_each_command() {
        let tree = root_command_tree();
        for descriptor in tree.commands {
            assert!(!descriptor.usage.trim().is_empty());
            assert!(descriptor.usage.starts_with(&descriptor.command));
        }
    }

    #[test]
    fn root_command_tree_serializes_capabilities_and_usage_templates() {
        let value =
            serde_json::to_value(root_command_tree()).expect("root command tree should serialize");
        assert_eq!(
            value["capabilities"],
            json!({
                "local_lifecycle": true,
                "in_process_service": true,
                "daemon_transport": false,
                "remote_transport": false
            })
        );
        assert_eq!(
            value["commands"][0],
            json!({
                "command": "grove tui",
                "description": "Launch Grove TUI",
                "usage": "grove tui [--repo <path>]"
            })
        );
    }
}

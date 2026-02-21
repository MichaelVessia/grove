use crate::interface::cli_contract::NextAction;

#[derive(Debug, Default)]
pub struct NextActionsBuilder {
    actions: Vec<NextAction>,
}

impl NextActionsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(mut self, command: impl Into<String>, description: impl Into<String>) -> Self {
        let command = command.into().trim().to_string();
        let description = description.into().trim().to_string();
        if command.is_empty() || description.is_empty() {
            return self;
        }
        self.actions.push(NextAction {
            command,
            description,
        });
        self
    }

    pub fn build(self) -> Vec<NextAction> {
        self.actions
    }
}

pub fn after_workspace_create(workspace_name: &str, started: bool) -> Vec<NextAction> {
    let workspace = workspace_name.trim();
    let selector = format!("--workspace {workspace}");
    let builder = NextActionsBuilder::new().push("workspace list", "Inspect workspace inventory");
    let builder = if started {
        builder
    } else {
        builder.push(
            format!("agent start {selector}"),
            "Start the agent in the new workspace",
        )
    };
    builder
        .push(
            format!("workspace update {selector}"),
            "Update workspace from base branch",
        )
        .push(
            format!("workspace merge {selector}"),
            "Merge workspace branch into base branch",
        )
        .build()
}

pub fn after_workspace_merge(workspace_name: &str) -> Vec<NextAction> {
    let workspace = workspace_name.trim();
    let selector = format!("--workspace {workspace}");
    NextActionsBuilder::new()
        .push("workspace list", "Inspect remaining workspaces")
        .push(
            format!("workspace delete {selector}"),
            "Delete merged workspace if no longer needed",
        )
        .push(
            format!("workspace create --name {workspace}-followup"),
            "Create a follow-up workspace",
        )
        .build()
}

pub fn after_agent_stop(workspace_name: &str) -> Vec<NextAction> {
    let workspace = workspace_name.trim();
    let selector = format!("--workspace {workspace}");
    NextActionsBuilder::new()
        .push(
            format!("agent start {selector}"),
            "Restart the workspace agent",
        )
        .push(
            format!("workspace delete {selector} --force-stop"),
            "Delete workspace when the agent should stay stopped",
        )
        .push("workspace list", "Inspect workspace inventory")
        .build()
}

#[cfg(test)]
mod tests {
    use super::{
        NextActionsBuilder, after_agent_stop, after_workspace_create, after_workspace_merge,
    };
    use serde_json::json;

    #[test]
    fn builder_skips_entries_with_blank_command_or_description() {
        let actions = NextActionsBuilder::new()
            .push("workspace list", "Inspect workspaces")
            .push("   ", "should be ignored")
            .push("workspace merge --workspace a", "   ")
            .build();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].command, "workspace list");
        assert_eq!(actions[0].description, "Inspect workspaces");
    }

    #[test]
    fn workspace_create_followups_include_expected_first_steps() {
        let actions = after_workspace_create("feature-auth", false);
        assert_eq!(actions[0].command, "workspace list");
        assert_eq!(actions[1].command, "agent start --workspace feature-auth");
    }

    #[test]
    fn workspace_create_followups_skip_agent_start_when_workspace_already_started() {
        let actions = after_workspace_create("feature-auth", true);
        let has_agent_start = actions
            .iter()
            .any(|action| action.command == "agent start --workspace feature-auth");
        assert!(!has_agent_start);
    }

    #[test]
    fn workspace_merge_followups_include_cleanup_and_followup_create() {
        let actions = after_workspace_merge("feature-auth");
        assert_eq!(
            actions[1].command,
            "workspace delete --workspace feature-auth"
        );
        assert_eq!(
            actions[2].command,
            "workspace create --name feature-auth-followup"
        );
    }

    #[test]
    fn agent_stop_followups_include_restart_and_delete_options() {
        let actions = after_agent_stop("feature-auth");
        assert_eq!(actions[0].command, "agent start --workspace feature-auth");
        assert_eq!(
            actions[1].command,
            "workspace delete --workspace feature-auth --force-stop"
        );
    }

    #[test]
    fn next_actions_serialize_to_machine_usable_shape() {
        let value =
            serde_json::to_value(after_workspace_create("feature-auth", false)).expect("serialize");
        assert_eq!(
            value,
            json!([
                {
                    "command": "workspace list",
                    "description": "Inspect workspace inventory"
                },
                {
                    "command": "agent start --workspace feature-auth",
                    "description": "Start the agent in the new workspace"
                },
                {
                    "command": "workspace update --workspace feature-auth",
                    "description": "Update workspace from base branch"
                },
                {
                    "command": "workspace merge --workspace feature-auth",
                    "description": "Merge workspace branch into base branch"
                }
            ])
        );
    }
}

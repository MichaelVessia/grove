use super::view_prelude::*;

impl GroveApp {
    #[cfg(test)]
    pub(super) fn shell_lines(&self, preview_height: usize) -> Vec<String> {
        let mut lines = vec![
            format!("Grove Shell | Repo: {}", self.repo_name),
            format!(
                "Mode: {} | Focus: {}",
                self.mode_label(),
                self.focus_label()
            ),
            "Workspaces (j/k, arrows, Tab/h/l focus, Enter preview, n create, o open repo, e edit, m merge, u update, s/x/r start-stop-restart, comma rename tab, D delete, S settings, M mouse, ? help, ! unsafe, Esc list)"
                .to_string(),
        ];

        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                lines.push(format!("! discovery failed: {message}"));
            }
            DiscoveryState::Empty => {
                lines.push("No workspaces discovered".to_string());
            }
            DiscoveryState::Ready => {
                for (idx, workspace) in self.state.workspaces.iter().enumerate() {
                    let selected = if idx == self.state.selected_index {
                        "▸"
                    } else {
                        " "
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    lines.push(format!(
                        "{} {} | {} | {} | {}{}",
                        selected,
                        workspace_name,
                        workspace.branch,
                        workspace.agent.label(),
                        workspace.path.display(),
                        if workspace.is_orphaned {
                            " | session ended"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }

        if let Some(dialog) = self.launch_dialog() {
            lines.push(String::new());
            lines.push("Start Agent Dialog".to_string());
            lines.push(format!(
                "Field: {}",
                self.current_launch_dialog_focus_field()
                    .map(LaunchDialogField::label)
                    .unwrap_or("unknown")
            ));
            lines.push(format!(
                "Name: {}",
                if dialog.start_config.name.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.start_config.name.clone()
                }
            ));
            lines.push(format!(
                "Prompt: {}",
                if dialog.start_config.prompt.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.start_config.prompt.clone()
                }
            ));
            lines.push(format!(
                "Init command: {}",
                if dialog.start_config.init_command.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.start_config.init_command.clone()
                }
            ));
            lines.push(format!(
                "Permission mode: {}",
                dialog.start_config.permission_mode.label()
            ));
        }
        if let Some(dialog) = self.delete_dialog() {
            lines.push(String::new());
            lines.push("Delete Task Dialog".to_string());
            lines.push(format!("Task: {}", dialog.task.name));
            lines.push(format!("Branch: {}", dialog.task.branch));
            lines.push(format!("Worktrees: {}", dialog.task.worktrees.len()));
            lines.push(format!(
                "Delete local branch: {}",
                if dialog.delete_local_branch {
                    "on"
                } else {
                    "off"
                }
            ));
            lines.push(format!(
                "Kill tmux sessions: {}",
                if dialog.kill_tmux_sessions {
                    "on"
                } else {
                    "off"
                }
            ));
        }
        if let Some(dialog) = self.merge_dialog() {
            lines.push(String::new());
            lines.push("Merge Workspace Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.workspace_branch));
            lines.push(format!("Base branch: {}", dialog.base_branch));
            lines.push(format!(
                "Cleanup worktree: {}",
                if dialog.cleanup_workspace {
                    "on"
                } else {
                    "off"
                }
            ));
            lines.push(format!(
                "Cleanup local branch: {}",
                if dialog.cleanup_local_branch {
                    "on"
                } else {
                    "off"
                }
            ));
        }
        if let Some(dialog) = self.update_from_base_dialog() {
            lines.push(String::new());
            lines.push("Update From Base Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.workspace_branch));
            lines.push(format!("Base branch: {}", dialog.base_branch));
        }
        if let Some(dialog) = self.pull_upstream_dialog() {
            lines.push(String::new());
            lines.push("Pull Upstream Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Base branch: {}", dialog.base_branch));
            lines.push(format!(
                "Propagate targets: {}",
                dialog.propagate_target_count
            ));
        }

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "{} ({}, {})",
                    Self::workspace_display_name(workspace),
                    workspace.branch,
                    workspace.path.display()
                )
            })
            .unwrap_or_else(|| "none".to_string());

        lines.push(String::new());
        lines.push("Preview Pane".to_string());
        lines.push(format!("Selected workspace: {}", selected_workspace));
        let (visible_start, visible_end) = self.preview_visible_range_for_height(preview_height);
        let visible_lines = self.preview_plain_lines_range(visible_start, visible_end);
        if visible_lines.is_empty() {
            lines.push("(no preview output)".to_string());
        } else {
            lines.extend(visible_lines);
        }
        lines.push(self.status_bar_line());

        lines
    }
}

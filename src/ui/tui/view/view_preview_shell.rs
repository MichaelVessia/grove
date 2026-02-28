use super::*;

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
            "Workspaces (j/k, arrows, Tab/h/l focus, Enter preview, n create, e edit, m merge, u update, s/x/r start-stop-restart, D delete, S settings, M mouse, ? help, ! unsafe, Esc list)"
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
            lines.push(format!("Field: {}", dialog.focused_field.label()));
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
                "Unsafe launch: {}",
                if dialog.start_config.skip_permissions {
                    "on"
                } else {
                    "off"
                }
            ));
        }
        if let Some(dialog) = self.delete_dialog() {
            lines.push(String::new());
            lines.push("Delete Workspace Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.branch));
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
        let mut visible_lines = self.preview.visible_lines(preview_height);
        self.apply_interactive_cursor_overlay(&mut visible_lines, preview_height);
        if visible_lines.is_empty() {
            lines.push("(no preview output)".to_string());
        } else {
            lines.extend(visible_lines);
        }
        lines.push(self.status_bar_line());

        lines
    }
}

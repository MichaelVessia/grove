use super::*;

impl GroveApp {
    fn shell_session_status_summary(&self, workspace: &Workspace) -> Option<String> {
        let shell_session_name = shell_session_name_for_workspace(workspace);
        if self.shell_sessions.is_in_flight(&shell_session_name) {
            return Some(format!("Starting shell session for {}...", workspace.name));
        }
        if self.shell_sessions.is_failed(&shell_session_name) {
            return Some(format!(
                "Shell session failed for {}.\nPress Enter to retry session launch.",
                workspace.name
            ));
        }
        if workspace.is_orphaned {
            return Some(format!("Reconnecting session for {}...", workspace.name));
        }
        None
    }

    fn selected_workspace_summary(&self) -> String {
        self.state
            .selected_workspace()
            .map(|workspace| {
                if self.preview_tab == PreviewTab::Shell {
                    return self
                        .shell_session_status_summary(workspace)
                        .unwrap_or_else(|| {
                            format!("Preparing shell session for {}...", workspace.name)
                        });
                }

                self.shell_session_status_summary(workspace)
                    .unwrap_or_else(|| format!("Preparing session for {}...", workspace.name))
            })
            .unwrap_or_else(|| "No workspace selected".to_string())
    }

    pub(super) fn refresh_preview_summary(&mut self) {
        self.preview
            .apply_capture(&self.selected_workspace_summary());
    }
}

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
                if workspace.is_main && !workspace.status.has_session() {
                    return self.main_worktree_splash();
                }
                if workspace.is_main {
                    return "Connecting to main workspace session...".to_string();
                }

                self.shell_session_status_summary(workspace)
                    .unwrap_or_else(|| format!("Preparing session for {}...", workspace.name))
            })
            .unwrap_or_else(|| "No workspace selected".to_string())
    }

    fn main_worktree_splash(&self) -> String {
        const G: &str = "\x1b[38;2;166;227;161m";
        const T: &str = "\x1b[38;2;250;179;135m";
        const R: &str = "\x1b[0m";

        [
            String::new(),
            format!("{G}                    .@@@.{R}"),
            format!("{G}                 .@@@@@@@@@.{R}"),
            format!("{G}               .@@@@@@@@@@@@@.{R}"),
            format!("{G}    .@@@.     @@@@@@@@@@@@@@@@@        .@@.{R}"),
            format!("{G}  .@@@@@@@.  @@@@@@@@@@@@@@@@@@@    .@@@@@@@@.{R}"),
            format!("{G} @@@@@@@@@@@ @@@@@@@@@@@@@@@@@@@@  @@@@@@@@@@@@@{R}"),
            format!("{G} @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@{R}"),
            format!("{G}  '@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@'{R}"),
            format!("{G}    '@@@@@@@@  '@@@@@@@@@@@@@@@' @@@@@@@@@@@@@@'{R}"),
            format!("{G}      '@@@@'     '@@@@@@@@@@@'    '@@@@@@@@@@'{R}"),
            format!("         {T}||{R}        {G}'@@@@@@@'{R}        {G}'@@@@'{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("         {T}||{R}           {T}|||{R}              {T}||{R}"),
            format!("        {T}/||\\{R}         {T}/|||\\{R}            {T}/||\\{R}"),
            String::new(),
            "Base Worktree".to_string(),
            String::new(),
            "This is your repo root.".to_string(),
            "Create focused workspaces from here when you start new work.".to_string(),
            String::new(),
            "--------------------------------------------------".to_string(),
            String::new(),
            "Press 'n' to create a workspace, hit 's' to open agent".to_string(),
            String::new(),
            "Each workspace has its own directory and branch.".to_string(),
            "Run agents in parallel without branch hopping.".to_string(),
        ]
        .join("\n")
    }

    pub(super) fn refresh_preview_summary(&mut self) {
        self.preview
            .apply_capture(&self.selected_workspace_summary());
    }
}

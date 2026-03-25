use std::collections::HashSet;

use crate::domain::{Workspace, WorkspaceStatus};

use super::{LivePreviewTarget, TMUX_SESSION_PREFIX};

const TASK_SESSION_PREFIX: &str = "grove-task-";
const TASK_WORKTREE_SESSION_PREFIX: &str = "grove-wt-";

pub(crate) fn sanitize_workspace_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for character in name.chars() {
        let mapped = if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            character
        } else {
            '-'
        };

        if mapped == '-' {
            if !last_dash {
                out.push('-');
            }
            last_dash = true;
            continue;
        }

        out.push(mapped);
        last_dash = false;
    }

    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        return "workspace".to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
pub fn session_name_for_workspace(workspace_name: &str) -> String {
    session_name_for_workspace_in_project(None, workspace_name)
}

pub fn session_name_for_workspace_ref(workspace: &Workspace) -> String {
    if let Some(task_slug) = workspace.task_slug.as_deref() {
        return session_name_for_task_worktree(
            task_slug,
            workspace
                .project_name
                .as_deref()
                .unwrap_or(workspace.name.as_str()),
        );
    }

    session_name_for_workspace_in_project(workspace.project_name.as_deref(), &workspace.name)
}

pub fn session_name_for_task(task_slug: &str) -> String {
    format!(
        "{TASK_SESSION_PREFIX}{}",
        sanitize_workspace_name(task_slug)
    )
}

pub fn session_name_for_task_worktree(task_slug: &str, repository_name: &str) -> String {
    format!(
        "{TASK_WORKTREE_SESSION_PREFIX}{}-{}",
        sanitize_workspace_name(task_slug),
        sanitize_workspace_name(repository_name)
    )
}

pub fn git_session_name_for_workspace(workspace: &Workspace) -> String {
    format!("{}-git", session_name_for_workspace_ref(workspace))
}

pub fn shell_session_name_for_workspace(workspace: &Workspace) -> String {
    format!("{}-shell", session_name_for_workspace_ref(workspace))
}

pub fn session_name_for_workspace_in_project(
    project_name: Option<&str>,
    workspace_name: &str,
) -> String {
    if let Some(project_name) = project_name {
        let project = sanitize_workspace_name(project_name);
        return format!(
            "{TMUX_SESSION_PREFIX}{project}-{}",
            sanitize_workspace_name(workspace_name)
        );
    }

    format!(
        "{TMUX_SESSION_PREFIX}{}",
        sanitize_workspace_name(workspace_name)
    )
}

pub fn live_preview_agent_session(workspace: Option<&Workspace>) -> Option<String> {
    let workspace = workspace?;
    if !workspace.status.has_session() {
        return None;
    }

    Some(session_name_for_workspace_ref(workspace))
}

pub fn workspace_can_enter_interactive(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
) -> bool {
    if preview_tab_is_git {
        return workspace.is_some();
    }

    live_preview_agent_session(workspace).is_some()
}

pub fn workspace_can_start_agent(workspace: Option<&Workspace>) -> bool {
    let Some(workspace) = workspace else {
        return false;
    };
    if !workspace.supported_agent {
        return false;
    }

    matches!(
        workspace.status,
        WorkspaceStatus::Main
            | WorkspaceStatus::Idle
            | WorkspaceStatus::Done
            | WorkspaceStatus::Error
            | WorkspaceStatus::Unknown
    )
}

pub fn workspace_can_stop_agent(workspace: Option<&Workspace>) -> bool {
    let Some(workspace) = workspace else {
        return false;
    };

    workspace.status.has_session()
}

pub fn workspace_session_for_preview_tab(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
    git_preview_session: Option<&str>,
) -> Option<String> {
    if preview_tab_is_git {
        workspace?;
        return git_preview_session.map(str::to_string);
    }

    live_preview_agent_session(workspace)
}

pub fn git_preview_session_if_ready(
    workspace: Option<&Workspace>,
    ready_sessions: &HashSet<String>,
) -> Option<String> {
    let workspace = workspace?;
    let session_name = git_session_name_for_workspace(workspace);
    if !ready_sessions.contains(&session_name) {
        return None;
    }

    Some(session_name)
}

pub fn live_preview_session_for_tab(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
    ready_sessions: &HashSet<String>,
) -> Option<String> {
    if preview_tab_is_git {
        return git_preview_session_if_ready(workspace, ready_sessions);
    }

    live_preview_agent_session(workspace)
}

pub fn live_preview_capture_target_for_tab(
    workspace: Option<&Workspace>,
    preview_tab_is_git: bool,
    ready_sessions: &HashSet<String>,
) -> Option<LivePreviewTarget> {
    let session_name = live_preview_session_for_tab(workspace, preview_tab_is_git, ready_sessions)?;
    Some(LivePreviewTarget {
        session_name,
        include_escape_sequences: true,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use crate::domain::{AgentType, WorkspaceStatus};

    use super::super::LivePreviewTarget;
    use super::super::launch_plan::{
        build_shell_launch_plan, launch_request_for_workspace, shell_launch_request_for_workspace,
    };
    use super::{
        git_preview_session_if_ready, git_session_name_for_workspace, live_preview_agent_session,
        live_preview_capture_target_for_tab, live_preview_session_for_tab, sanitize_workspace_name,
        session_name_for_task, session_name_for_task_worktree, session_name_for_workspace,
        session_name_for_workspace_ref, shell_session_name_for_workspace,
        workspace_can_enter_interactive, workspace_can_start_agent, workspace_can_stop_agent,
        workspace_session_for_preview_tab,
    };

    fn fixture_workspace(name: &str, is_main: bool) -> crate::domain::Workspace {
        crate::domain::Workspace::try_new(
            name.to_string(),
            PathBuf::from(format!("/repos/grove-{name}")),
            if is_main {
                "main".to_string()
            } else {
                name.to_string()
            },
            Some(1_700_000_100),
            AgentType::Claude,
            if is_main {
                WorkspaceStatus::Main
            } else {
                WorkspaceStatus::Idle
            },
            is_main,
        )
        .expect("workspace should be valid")
    }

    #[test]
    fn session_names_distinguish_task_root_and_worktree_scope() {
        assert_eq!(
            session_name_for_task("flohome-launch"),
            "grove-task-flohome-launch"
        );
        assert_eq!(
            session_name_for_task_worktree("flohome-launch", "flohome"),
            "grove-wt-flohome-launch-flohome"
        );
    }

    #[test]
    fn task_session_names_reuse_workspace_sanitization() {
        assert_eq!(sanitize_workspace_name(" infra/base "), "infra-base");
        assert_eq!(
            session_name_for_task(" infra/base "),
            "grove-task-infra-base"
        );
        assert_eq!(
            session_name_for_task_worktree(" infra/base ", "terraform.fastly"),
            "grove-wt-infra-base-terraform-fastly"
        );
    }

    #[test]
    fn session_name_sanitizes_workspace_label() {
        assert_eq!(
            sanitize_workspace_name("feature/auth.v2"),
            "feature-auth-v2"
        );
        assert_eq!(
            session_name_for_workspace("feature/auth.v2"),
            "grove-ws-feature-auth-v2"
        );
        assert_eq!(sanitize_workspace_name("///"), "workspace");
    }

    #[test]
    fn session_name_for_workspace_ref_uses_project_context_when_present() {
        let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
            "project.one".to_string(),
            PathBuf::from("/repos/project.one"),
        );
        assert_eq!(
            session_name_for_workspace_ref(&workspace),
            "grove-ws-project-one-feature-auth-v2"
        );
    }

    #[test]
    fn git_session_name_uses_project_context_when_present() {
        let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
            "project.one".to_string(),
            PathBuf::from("/repos/project.one"),
        );
        assert_eq!(
            git_session_name_for_workspace(&workspace),
            "grove-ws-project-one-feature-auth-v2-git"
        );
    }

    #[test]
    fn shell_session_name_uses_project_context_when_present() {
        let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
            "project.one".to_string(),
            PathBuf::from("/repos/project.one"),
        );
        assert_eq!(
            shell_session_name_for_workspace(&workspace),
            "grove-ws-project-one-feature-auth-v2-shell"
        );
    }

    #[test]
    fn launch_request_for_workspace_copies_workspace_context_and_options() {
        let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
            "project.one".to_string(),
            PathBuf::from("/repos/project.one"),
        );
        let request = launch_request_for_workspace(
            &workspace,
            Some("run checks".to_string()),
            crate::infrastructure::config::ThemeName::default(),
            Some("direnv allow".to_string()),
            true,
            vec![(
                "CLAUDE_CONFIG_DIR".to_string(),
                "~/.claude-work".to_string(),
            )],
            Some((132, 44)),
        );

        assert_eq!(request.project_name.as_deref(), Some("project.one"));
        assert_eq!(request.workspace_name, "feature/auth.v2");
        assert_eq!(
            request.workspace_path,
            PathBuf::from("/repos/grove-feature/auth.v2")
        );
        assert_eq!(request.agent, AgentType::Claude);
        assert_eq!(request.prompt.as_deref(), Some("run checks"));
        assert_eq!(
            request.workspace_init_command.as_deref(),
            Some("direnv allow")
        );
        assert!(request.skip_permissions);
        assert_eq!(
            request.agent_env,
            vec![(
                "CLAUDE_CONFIG_DIR".to_string(),
                "~/.claude-work".to_string()
            )]
        );
        assert_eq!(request.capture_cols, Some(132));
        assert_eq!(request.capture_rows, Some(44));
    }

    #[test]
    fn shell_launch_request_for_workspace_uses_workspace_path_and_options() {
        let workspace = fixture_workspace("feature", false);
        let request = shell_launch_request_for_workspace(
            &workspace,
            "grove-ws-feature-git".to_string(),
            "lazygit".to_string(),
            crate::infrastructure::config::ThemeName::default(),
            Some("direnv allow".to_string()),
            Some(120),
            Some(40),
        );

        assert_eq!(request.session_name, "grove-ws-feature-git");
        assert_eq!(
            request.workspace_path,
            PathBuf::from("/repos/grove-feature")
        );
        assert_eq!(request.command, "lazygit");
        assert_eq!(
            request.workspace_init_command.as_deref(),
            Some("direnv allow")
        );
        assert_eq!(request.capture_cols, Some(120));
        assert_eq!(request.capture_rows, Some(40));
    }

    #[test]
    fn build_shell_launch_plan_skips_send_keys_when_command_is_empty() {
        let request = shell_launch_request_for_workspace(
            &fixture_workspace("feature", false),
            "grove-ws-feature-shell".to_string(),
            String::new(),
            crate::infrastructure::config::ThemeName::default(),
            None,
            Some(120),
            Some(40),
        );
        let plan = build_shell_launch_plan(&request);

        assert!(plan.launch_cmd.is_empty());
    }

    #[test]
    fn build_shell_launch_plan_with_workspace_init_runs_before_empty_command() {
        let request = shell_launch_request_for_workspace(
            &fixture_workspace("feature", false),
            "grove-ws-feature-shell".to_string(),
            String::new(),
            crate::infrastructure::config::ThemeName::default(),
            Some("direnv allow".to_string()),
            Some(120),
            Some(40),
        );
        let plan = build_shell_launch_plan(&request);

        assert_eq!(plan.launch_cmd.len(), 6);
        assert!(
            plan.launch_cmd[4].contains("bash -lc"),
            "expected init wrapper command, got {}",
            plan.launch_cmd[4]
        );
        assert!(
            plan.launch_cmd[4].contains("direnv allow"),
            "expected init command in wrapper, got {}",
            plan.launch_cmd[4]
        );
    }

    #[test]
    fn workspace_init_runs_directly_without_lock_wrapper() {
        let request = shell_launch_request_for_workspace(
            &fixture_workspace("feature", false),
            "grove-ws-feature-shell".to_string(),
            String::new(),
            crate::infrastructure::config::ThemeName::default(),
            Some("echo init".to_string()),
            Some(120),
            Some(40),
        );
        let plan = build_shell_launch_plan(&request);
        let command = &plan.launch_cmd[4];

        assert!(
            command.contains("bash -lc"),
            "expected shell wrapper, got {command}"
        );
        assert!(
            command.contains("echo init"),
            "expected init command, got {command}"
        );
        assert!(
            !command.contains("workspace-init-"),
            "lock wrapper should be removed, got {command}"
        );
    }

    #[test]
    fn build_shell_launch_plan_with_direnv_init_wraps_run_command_in_direnv_exec() {
        let request = shell_launch_request_for_workspace(
            &fixture_workspace("feature", false),
            "grove-ws-feature-shell".to_string(),
            "yarn test".to_string(),
            crate::infrastructure::config::ThemeName::default(),
            Some("direnv allow".to_string()),
            Some(120),
            Some(40),
        );
        let plan = build_shell_launch_plan(&request);

        assert!(
            plan.launch_cmd[4].contains("direnv exec . bash -lc"),
            "expected direnv exec wrapper, got {}",
            plan.launch_cmd[4]
        );
        assert!(
            plan.launch_cmd[4].contains("yarn test"),
            "expected shell command in wrapped launch, got {}",
            plan.launch_cmd[4]
        );
    }

    #[test]
    fn build_shell_launch_plan_with_capture_dimensions_resizes_before_send_keys() {
        let request = shell_launch_request_for_workspace(
            &fixture_workspace("feature", false),
            "grove-ws-feature-shell".to_string(),
            "bash".to_string(),
            crate::infrastructure::config::ThemeName::default(),
            None,
            Some(120),
            Some(40),
        );
        let plan = build_shell_launch_plan(&request);

        assert_eq!(
            plan.pre_launch_cmds.last(),
            Some(&vec![
                "tmux".to_string(),
                "resize-window".to_string(),
                "-t".to_string(),
                "grove-ws-feature-shell".to_string(),
                "-x".to_string(),
                "120".to_string(),
                "-y".to_string(),
                "40".to_string(),
            ])
        );
    }

    #[test]
    fn live_preview_agent_session_requires_live_workspace_session() {
        let idle_workspace = fixture_workspace("feature", false);
        assert_eq!(live_preview_agent_session(Some(&idle_workspace)), None);

        let mut active_workspace = fixture_workspace("feature", false);
        active_workspace.status = WorkspaceStatus::Active;
        assert_eq!(
            live_preview_agent_session(Some(&active_workspace)),
            Some("grove-ws-feature".to_string())
        );
    }

    #[test]
    fn workspace_can_enter_interactive_depends_on_preview_tab_mode() {
        let idle_workspace = fixture_workspace("feature", false);
        assert!(!workspace_can_enter_interactive(
            Some(&idle_workspace),
            false
        ));
        assert!(workspace_can_enter_interactive(Some(&idle_workspace), true));
        assert!(!workspace_can_enter_interactive(None, false));

        let mut active_workspace = fixture_workspace("feature", false);
        active_workspace.status = WorkspaceStatus::Active;
        assert!(workspace_can_enter_interactive(
            Some(&active_workspace),
            false
        ));
    }

    #[test]
    fn workspace_can_start_agent_depends_on_status_and_support() {
        let idle_workspace = fixture_workspace("feature", false);
        assert!(workspace_can_start_agent(Some(&idle_workspace)));
        assert!(!workspace_can_start_agent(None));

        let unsupported_workspace = fixture_workspace("feature", false).with_supported_agent(false);
        assert!(!workspace_can_start_agent(Some(&unsupported_workspace)));

        let mut waiting_workspace = fixture_workspace("feature", false);
        waiting_workspace.status = WorkspaceStatus::Waiting;
        assert!(!workspace_can_start_agent(Some(&waiting_workspace)));

        let mut done_workspace = fixture_workspace("feature", false);
        done_workspace.status = WorkspaceStatus::Done;
        assert!(workspace_can_start_agent(Some(&done_workspace)));
    }

    #[test]
    fn workspace_can_stop_agent_depends_on_session_status() {
        let idle_workspace = fixture_workspace("feature", false);
        assert!(!workspace_can_stop_agent(Some(&idle_workspace)));
        assert!(!workspace_can_stop_agent(None));

        let mut active_workspace = fixture_workspace("feature", false);
        active_workspace.status = WorkspaceStatus::Active;
        assert!(workspace_can_stop_agent(Some(&active_workspace)));
    }

    #[test]
    fn workspace_session_for_preview_tab_respects_preview_tab_mode() {
        let idle_workspace = fixture_workspace("feature", false);
        assert_eq!(
            workspace_session_for_preview_tab(
                Some(&idle_workspace),
                true,
                Some("grove-ws-feature-git"),
            ),
            Some("grove-ws-feature-git".to_string())
        );
        assert_eq!(
            workspace_session_for_preview_tab(Some(&idle_workspace), true, None),
            None
        );
        assert_eq!(
            workspace_session_for_preview_tab(None, true, Some("grove-ws-feature-git")),
            None
        );
        assert_eq!(
            workspace_session_for_preview_tab(
                Some(&idle_workspace),
                false,
                Some("grove-ws-feature-git"),
            ),
            None
        );

        let mut active_workspace = fixture_workspace("feature", false);
        active_workspace.status = WorkspaceStatus::Active;
        assert_eq!(
            workspace_session_for_preview_tab(Some(&active_workspace), false, None),
            Some("grove-ws-feature".to_string())
        );
    }

    #[test]
    fn git_preview_session_if_ready_requires_matching_ready_session() {
        let workspace = fixture_workspace("feature", false);
        let mut ready_sessions = HashSet::new();
        assert_eq!(
            git_preview_session_if_ready(Some(&workspace), &ready_sessions),
            None
        );
        ready_sessions.insert("grove-ws-feature-git".to_string());
        assert_eq!(
            git_preview_session_if_ready(Some(&workspace), &ready_sessions),
            Some("grove-ws-feature-git".to_string())
        );
        assert_eq!(git_preview_session_if_ready(None, &ready_sessions), None);
    }

    #[test]
    fn live_preview_session_for_tab_uses_git_or_agent_policy() {
        let idle_workspace = fixture_workspace("feature", false);
        let mut active_workspace = fixture_workspace("feature", false);
        active_workspace.status = WorkspaceStatus::Active;
        let mut ready_sessions = HashSet::new();
        ready_sessions.insert("grove-ws-feature-git".to_string());

        assert_eq!(
            live_preview_session_for_tab(Some(&idle_workspace), true, &ready_sessions),
            Some("grove-ws-feature-git".to_string())
        );
        assert_eq!(
            live_preview_session_for_tab(Some(&idle_workspace), false, &ready_sessions),
            None
        );
        assert_eq!(
            live_preview_session_for_tab(Some(&active_workspace), false, &ready_sessions),
            Some("grove-ws-feature".to_string())
        );
    }

    #[test]
    fn live_preview_capture_target_for_tab_sets_capture_mode() {
        let mut active_workspace = fixture_workspace("feature", false);
        active_workspace.status = WorkspaceStatus::Active;
        let mut ready_sessions = HashSet::new();
        ready_sessions.insert("grove-ws-feature-git".to_string());

        assert_eq!(
            live_preview_capture_target_for_tab(Some(&active_workspace), false, &ready_sessions),
            Some(LivePreviewTarget {
                session_name: "grove-ws-feature".to_string(),
                include_escape_sequences: true,
            })
        );
        assert_eq!(
            live_preview_capture_target_for_tab(Some(&active_workspace), true, &ready_sessions),
            Some(LivePreviewTarget {
                session_name: "grove-ws-feature-git".to_string(),
                include_escape_sequences: true,
            })
        );
    }
}

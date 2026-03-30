use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::domain::{Task, Workspace};

use super::launch_plan::{
    build_launch_plan, build_shell_launch_plan, build_task_launch_plan, stop_plan,
};
use super::sessions::{
    session_name_for_task, session_name_for_task_worktree, session_name_for_workspace_in_project,
    session_name_for_workspace_ref,
};
use super::{
    LaunchPlan, LaunchRequest, LauncherScript, SessionExecutionResult, ShellLaunchRequest,
    TaskLaunchRequest,
};

#[cfg(test)]
pub fn execute_launch_plan(launch_plan: LaunchPlan) -> std::io::Result<()> {
    let mut executor = ProcessCommandExecutor;
    execute_launch_plan_with_executor(&launch_plan, &mut executor)
}

pub enum CommandExecutionMode<'a> {
    Process,
    Delegating(&'a mut dyn FnMut(&[String]) -> std::io::Result<()>),
}

pub fn execute_launch_request_with_result_for_mode(
    request: &LaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    let workspace_name = request.workspace_name.clone();
    let workspace_path = request.workspace_path.clone();
    let launch_plan = build_launch_plan(request);
    let session_name = launch_plan.session_name.clone();
    let result = execute_launch_plan_for_mode(&launch_plan, mode);
    SessionExecutionResult {
        workspace_name,
        workspace_path,
        session_name,
        result,
    }
}

pub fn execute_task_launch_request_with_result_for_mode(
    request: &TaskLaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    let workspace_name = request.task_slug.clone();
    let workspace_path = request.task_root.clone();
    let launch_plan = build_task_launch_plan(request);
    let session_name = launch_plan.session_name.clone();
    let result = execute_launch_plan_for_mode(&launch_plan, mode);
    SessionExecutionResult {
        workspace_name,
        workspace_path,
        session_name,
        result,
    }
}

pub fn execute_shell_launch_request_for_mode(
    request: &ShellLaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> (String, Result<(), String>) {
    let launch_plan = build_shell_launch_plan(request);
    let session_name = launch_plan.session_name.clone();
    let result = execute_launch_plan_for_mode(&launch_plan, mode);
    (session_name, result)
}

pub fn execute_stop_workspace_with_result_for_mode(
    workspace: &Workspace,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    let workspace_name = workspace.name.clone();
    let workspace_path = workspace.path.clone();
    let session_name = session_name_for_workspace_ref(workspace);
    let commands = stop_plan(&session_name);
    let result = execute_commands_for_mode(&commands, mode);
    SessionExecutionResult {
        workspace_name,
        workspace_path,
        session_name,
        result,
    }
}

pub fn execute_stop_task_with_result_for_mode(
    task_name: &str,
    task_root: &Path,
    task_slug: &str,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    let workspace_name = task_name.to_string();
    let workspace_path = task_root.to_path_buf();
    let session_name = session_name_for_task(task_slug);
    let commands = stop_plan(&session_name);
    let result = execute_commands_for_mode(&commands, mode);
    SessionExecutionResult {
        workspace_name,
        workspace_path,
        session_name,
        result,
    }
}

pub fn execute_launch_plan_for_mode(
    launch_plan: &LaunchPlan,
    mode: CommandExecutionMode<'_>,
) -> Result<(), String> {
    match mode {
        CommandExecutionMode::Process => {
            let mut executor = ProcessCommandExecutor;
            execute_launch_plan_with_executor(launch_plan, &mut executor)
                .map_err(|error| error.to_string())
        }
        CommandExecutionMode::Delegating(execute) => {
            let mut executor = DelegatingCommandExecutor::new(execute)
                .with_script_write_error_prefix("launcher script write failed: ");
            execute_launch_plan_with_executor(launch_plan, &mut executor)
                .map_err(|error| error.to_string())
        }
    }
}

#[cfg(test)]
pub fn execute_commands(commands: &[Vec<String>]) -> std::io::Result<()> {
    let mut executor = ProcessCommandExecutor;
    execute_commands_with_executor(commands, &mut executor)
}

pub fn execute_commands_for_mode(
    commands: &[Vec<String>],
    mode: CommandExecutionMode<'_>,
) -> Result<(), String> {
    match mode {
        CommandExecutionMode::Process => {
            let mut executor = ProcessCommandExecutor;
            execute_commands_with_executor(commands, &mut executor)
                .map_err(|error| error.to_string())
        }
        CommandExecutionMode::Delegating(execute) => {
            let mut executor = DelegatingCommandExecutor::new(execute);
            execute_commands_with_executor(commands, &mut executor)
                .map_err(|error| error.to_string())
        }
    }
}

pub fn execute_command_with(
    command: &[String],
    execute: impl FnOnce(&[String]) -> std::io::Result<()>,
) -> std::io::Result<()> {
    if command.is_empty() {
        return Ok(());
    }

    execute(command)
}

pub fn execute_commands_with(
    commands: &[Vec<String>],
    mut execute: impl FnMut(&[String]) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let mut executor = DelegatingCommandExecutor::new(&mut execute);
    execute_commands_with_executor(commands, &mut executor)
}

pub fn execute_launch_plan_with(
    launch_plan: &LaunchPlan,
    mut execute: impl FnMut(&[String]) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let mut executor = DelegatingCommandExecutor::new(&mut execute)
        .with_script_write_error_prefix("launcher script write failed: ");
    execute_launch_plan_with_executor(launch_plan, &mut executor)
}

pub trait CommandExecutor {
    fn execute(&mut self, command: &[String]) -> std::io::Result<()>;

    fn write_launcher_script(&mut self, script: &LauncherScript) -> std::io::Result<()> {
        write_launcher_script_to_disk(script)
    }
}

pub struct ProcessCommandExecutor;

impl CommandExecutor for ProcessCommandExecutor {
    fn execute(&mut self, command: &[String]) -> std::io::Result<()> {
        crate::infrastructure::process::execute_command(command)
    }
}

pub struct DelegatingCommandExecutor<'a> {
    execute: &'a mut dyn FnMut(&[String]) -> std::io::Result<()>,
    script_write_error_prefix: Option<&'a str>,
}

impl<'a> DelegatingCommandExecutor<'a> {
    pub fn new(execute: &'a mut dyn FnMut(&[String]) -> std::io::Result<()>) -> Self {
        Self {
            execute,
            script_write_error_prefix: None,
        }
    }

    pub fn with_script_write_error_prefix(mut self, prefix: &'a str) -> Self {
        self.script_write_error_prefix = Some(prefix);
        self
    }
}

impl CommandExecutor for DelegatingCommandExecutor<'_> {
    fn execute(&mut self, command: &[String]) -> std::io::Result<()> {
        (self.execute)(command)
    }

    fn write_launcher_script(&mut self, script: &LauncherScript) -> std::io::Result<()> {
        match self.script_write_error_prefix {
            Some(prefix) => write_launcher_script_to_disk(script)
                .map_err(|error| std::io::Error::other(format!("{prefix}{error}"))),
            None => write_launcher_script_to_disk(script),
        }
    }
}

fn write_launcher_script_to_disk(script: &LauncherScript) -> std::io::Result<()> {
    if let Some(parent) = script.path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&script.path, &script.contents)
}

pub fn execute_commands_with_executor(
    commands: &[Vec<String>],
    executor: &mut impl CommandExecutor,
) -> std::io::Result<()> {
    for command in commands {
        execute_command_with(command, |command| executor.execute(command))?;
    }
    Ok(())
}

pub fn execute_launch_plan_with_executor(
    launch_plan: &LaunchPlan,
    executor: &mut impl CommandExecutor,
) -> std::io::Result<()> {
    for cmd in &launch_plan.cleanup_cmds {
        let _ = execute_command_with(cmd, |command| executor.execute(command));
    }

    if let Some(script) = &launch_plan.launcher_script {
        executor.write_launcher_script(script)?;
    }

    execute_commands_with_executor(&launch_plan.pre_launch_cmds, executor)?;
    execute_command_with(&launch_plan.launch_cmd, |command| executor.execute(command))
}

fn base_session_name_for_workspace_identity(
    task_slug: Option<&str>,
    project_name: Option<&str>,
    workspace_name: &str,
) -> String {
    if let Some(task_slug) = task_slug {
        return session_name_for_task_worktree(task_slug, project_name.unwrap_or(workspace_name));
    }

    session_name_for_workspace_in_project(project_name, workspace_name)
}

pub fn kill_workspace_session_command(
    task_slug: Option<&str>,
    project_name: Option<&str>,
    workspace_name: &str,
) -> Vec<String> {
    let session_name =
        base_session_name_for_workspace_identity(task_slug, project_name, workspace_name);
    kill_tmux_session_command(&session_name)
}

pub fn kill_workspace_session_commands(
    task_slug: Option<&str>,
    project_name: Option<&str>,
    workspace_name: &str,
) -> Vec<Vec<String>> {
    let session_name =
        base_session_name_for_workspace_identity(task_slug, project_name, workspace_name);
    vec![
        kill_tmux_session_command(&session_name),
        kill_tmux_session_command(&format!("{session_name}-git")),
        kill_tmux_session_command(&format!("{session_name}-shell")),
    ]
}

fn session_name_matches_base_session(base_session_name: &str, session_name: &str) -> bool {
    let Some(suffix) = session_name.strip_prefix(base_session_name) else {
        return false;
    };
    matches!(suffix, "" | "-git" | "-shell")
        || suffix
            .strip_prefix("-agent-")
            .or_else(|| suffix.strip_prefix("-shell-"))
            .is_some_and(|ordinal| {
                !ordinal.is_empty() && ordinal.bytes().all(|b| b.is_ascii_digit())
            })
}

pub fn workspace_session_name_matches(
    task_slug: Option<&str>,
    project_name: Option<&str>,
    workspace_name: &str,
    session_name: &str,
) -> bool {
    let base_session_name =
        base_session_name_for_workspace_identity(task_slug, project_name, workspace_name);
    session_name_matches_base_session(base_session_name.as_str(), session_name)
}

pub fn workspace_session_names_for_cleanup(
    task_slug: Option<&str>,
    project_name: Option<&str>,
    workspace_name: &str,
    existing_sessions: &[String],
) -> Vec<String> {
    existing_sessions
        .iter()
        .filter(|s| workspace_session_name_matches(task_slug, project_name, workspace_name, s))
        .cloned()
        .collect()
}

pub fn kill_workspace_session_commands_for_existing_sessions(
    task_slug: Option<&str>,
    project_name: Option<&str>,
    workspace_name: &str,
    existing_sessions: &[String],
) -> Vec<Vec<String>> {
    workspace_session_names_for_cleanup(task_slug, project_name, workspace_name, existing_sessions)
        .into_iter()
        .map(|session_name| kill_tmux_session_command(&session_name))
        .collect()
}

pub fn task_session_names_for_cleanup(task: &Task, existing_sessions: &[String]) -> Vec<String> {
    let mut matched = Vec::new();
    let mut seen = HashSet::new();
    let task_session = session_name_for_task(task.slug.as_str());
    let worktree_sessions = task
        .worktrees
        .iter()
        .map(|worktree| {
            session_name_for_task_worktree(task.slug.as_str(), worktree.repository_name.as_str())
        })
        .collect::<Vec<String>>();

    for session_name in existing_sessions {
        if (session_name_matches_base_session(task_session.as_str(), session_name)
            || worktree_sessions.iter().any(|base_session_name| {
                session_name_matches_base_session(base_session_name.as_str(), session_name)
            }))
            && seen.insert(session_name.clone())
        {
            matched.push(session_name.clone());
        }
    }

    matched
}

pub fn kill_task_session_commands(task: &Task) -> Vec<Vec<String>> {
    let mut commands = Vec::new();
    commands.push(kill_tmux_session_command(
        session_name_for_task(task.slug.as_str()).as_str(),
    ));
    for worktree in &task.worktrees {
        commands.push(kill_tmux_session_command(
            session_name_for_task_worktree(task.slug.as_str(), worktree.repository_name.as_str())
                .as_str(),
        ));
    }
    commands
}

pub fn kill_task_session_commands_for_existing_sessions(
    task: &Task,
    existing_sessions: &[String],
) -> Vec<Vec<String>> {
    task_session_names_for_cleanup(task, existing_sessions)
        .into_iter()
        .map(|session_name| kill_tmux_session_command(&session_name))
        .collect()
}

pub(super) fn kill_tmux_session_command(session_name: &str) -> Vec<String> {
    vec![
        "tmux".to_string(),
        "kill-session".to_string(),
        "-t".to_string(),
        session_name.to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::domain::{AgentType, PermissionMode, WorkspaceStatus};
    use crate::test_support::unique_test_dir;

    use super::super::{LaunchPlan, LaunchRequest, LauncherScript};
    use super::{
        CommandExecutionMode, CommandExecutor, execute_command_with, execute_commands,
        execute_commands_for_mode, execute_commands_with, execute_commands_with_executor,
        execute_launch_plan, execute_launch_plan_for_mode, execute_launch_plan_with,
        execute_launch_plan_with_executor, execute_launch_request_with_result_for_mode,
        execute_stop_workspace_with_result_for_mode, kill_workspace_session_command,
        kill_workspace_session_commands, kill_workspace_session_commands_for_existing_sessions,
        workspace_session_name_matches, workspace_session_names_for_cleanup,
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

    #[derive(Default)]
    struct RecordingCommandExecutor {
        commands: Vec<Vec<String>>,
        launcher_scripts: Vec<(PathBuf, String)>,
    }

    impl CommandExecutor for RecordingCommandExecutor {
        fn execute(&mut self, command: &[String]) -> std::io::Result<()> {
            self.commands.push(command.to_vec());
            Ok(())
        }

        fn write_launcher_script(&mut self, script: &LauncherScript) -> std::io::Result<()> {
            self.launcher_scripts
                .push((script.path.clone(), script.contents.clone()));
            Ok(())
        }
    }

    #[test]
    fn execute_commands_runs_successful_command_sequence() {
        let commands = vec![
            vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
            Vec::new(),
        ];
        assert!(execute_commands(&commands).is_ok());
    }

    #[test]
    fn execute_commands_returns_error_for_missing_program() {
        let commands = vec![vec![
            "grove-this-command-does-not-exist".to_string(),
            "arg".to_string(),
        ]];
        assert!(execute_commands(&commands).is_err());
    }

    #[test]
    fn execute_commands_for_mode_process_returns_string_errors() {
        let commands = vec![vec![
            "grove-this-command-does-not-exist".to_string(),
            "arg".to_string(),
        ]];
        let result = execute_commands_for_mode(&commands, CommandExecutionMode::Process);
        let error_text = result.expect_err("missing program should error");

        assert!(!error_text.is_empty());
    }

    #[test]
    fn execute_launch_request_with_result_for_mode_includes_workspace_context() {
        let request = LaunchRequest {
            session_name: None,
            task_slug: None,
            project_name: Some("project.one".to_string()),
            workspace_name: "auth-flow".to_string(),
            workspace_path: PathBuf::from("/repos/project.one/worktrees/auth-flow"),
            agent: AgentType::Claude,
            theme_name: crate::infrastructure::config::ThemeName::default(),
            prompt: None,
            workspace_init_command: None,
            permission_mode: PermissionMode::Default,
            agent_env: Vec::new(),
            capture_cols: Some(120),
            capture_rows: Some(40),
        };
        let result = execute_launch_request_with_result_for_mode(
            &request,
            CommandExecutionMode::Delegating(&mut |_command| {
                Err(std::io::Error::other("synthetic execution failure"))
            }),
        );

        assert_eq!(result.workspace_name, "auth-flow");
        assert_eq!(
            result.workspace_path,
            PathBuf::from("/repos/project.one/worktrees/auth-flow")
        );
        assert_eq!(result.session_name, "grove-ws-project-one-auth-flow");
        assert!(result.result.is_err());
    }

    #[test]
    fn execute_stop_workspace_with_result_for_mode_includes_workspace_context() {
        let workspace = fixture_workspace("feature/auth.v2", false).with_project_context(
            "project.one".to_string(),
            PathBuf::from("/repos/project.one"),
        );
        let mut commands = Vec::new();
        let result = execute_stop_workspace_with_result_for_mode(
            &workspace,
            CommandExecutionMode::Delegating(&mut |command| {
                commands.push(command.to_vec());
                Ok(())
            }),
        );

        assert_eq!(result.workspace_name, "feature/auth.v2");
        assert_eq!(
            result.workspace_path,
            PathBuf::from("/repos/grove-feature/auth.v2")
        );
        assert_eq!(result.session_name, "grove-ws-project-one-feature-auth-v2");
        assert!(result.result.is_ok());
        assert_eq!(
            commands,
            vec![
                vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2".to_string(),
                    "C-c".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2".to_string(),
                ],
                vec![
                    "rm".to_string(),
                    "-f".to_string(),
                    "/tmp/grove-exit-grove-ws-project-one-feature-auth-v2".to_string(),
                ],
            ]
        );
    }

    #[test]
    fn execute_launch_plan_writes_launcher_script_and_executes_commands() {
        let temp_dir = unique_test_dir("execute-launch-plan");
        let script_path = temp_dir.join(".grove/start.sh");
        let launch_plan = LaunchPlan {
            session_name: "grove-ws-test".to_string(),
            pane_lookup_cmd: Vec::new(),
            cleanup_cmds: Vec::new(),
            pre_launch_cmds: vec![vec![
                "sh".to_string(),
                "-lc".to_string(),
                "true".to_string(),
            ]],
            launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
            launcher_script: Some(LauncherScript {
                path: script_path.clone(),
                contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
            }),
        };

        let result = execute_launch_plan(launch_plan);
        assert!(result.is_ok());
        assert_eq!(
            fs::read_to_string(script_path).expect("launcher script should be written"),
            "#!/usr/bin/env bash\necho hi\n"
        );
    }

    #[test]
    fn execute_commands_with_uses_supplied_executor() {
        let commands = vec![
            vec!["echo".to_string(), "first".to_string()],
            vec!["echo".to_string(), "second".to_string()],
        ];
        let mut observed = Vec::new();

        let result = execute_commands_with(&commands, |command| {
            observed.push(command.join(" "));
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(observed, vec!["echo first", "echo second"]);
    }

    #[test]
    fn execute_commands_with_executor_skips_empty_commands() {
        let commands = vec![
            Vec::new(),
            vec!["echo".to_string(), "ran".to_string()],
            Vec::new(),
        ];
        let mut executor = RecordingCommandExecutor::default();

        let result = execute_commands_with_executor(&commands, &mut executor);

        assert!(result.is_ok());
        assert_eq!(
            executor.commands,
            vec![vec!["echo".to_string(), "ran".to_string()]]
        );
    }

    #[test]
    fn execute_launch_plan_with_executor_runs_cleanup_then_prelaunch_then_launch() {
        let launch_plan = LaunchPlan {
            session_name: "grove-ws-test".to_string(),
            pane_lookup_cmd: Vec::new(),
            cleanup_cmds: vec![vec![
                "tmux".to_string(),
                "kill-session".to_string(),
                "-t".to_string(),
                "grove-ws-test".to_string(),
            ]],
            pre_launch_cmds: vec![
                vec!["echo".to_string(), "one".to_string()],
                vec!["echo".to_string(), "two".to_string()],
            ],
            launch_cmd: vec!["echo".to_string(), "launch".to_string()],
            launcher_script: Some(LauncherScript {
                path: PathBuf::from("/tmp/.grove/start.sh"),
                contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
            }),
        };
        let mut executor = RecordingCommandExecutor::default();

        let result = execute_launch_plan_with_executor(&launch_plan, &mut executor);

        assert!(result.is_ok());
        assert_eq!(
            executor.commands,
            vec![
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-test".to_string(),
                ],
                vec!["echo".to_string(), "one".to_string()],
                vec!["echo".to_string(), "two".to_string()],
                vec!["echo".to_string(), "launch".to_string()],
            ]
        );
        assert_eq!(executor.launcher_scripts.len(), 1);
    }

    #[test]
    fn execute_launch_plan_with_executor_tolerates_cleanup_cmd_failure() {
        let launch_plan = LaunchPlan {
            session_name: "grove-ws-test".to_string(),
            pane_lookup_cmd: Vec::new(),
            cleanup_cmds: vec![vec!["false".to_string()]],
            pre_launch_cmds: vec![vec!["echo".to_string(), "one".to_string()]],
            launch_cmd: vec!["echo".to_string(), "launch".to_string()],
            launcher_script: None,
        };

        let result = execute_launch_plan(launch_plan);

        assert!(result.is_ok());
    }

    #[test]
    fn execute_command_with_skips_empty_commands() {
        let mut executed = false;

        let result = execute_command_with(&Vec::new(), |_command| {
            executed = true;
            Ok(())
        });

        assert!(result.is_ok());
        assert!(!executed);
    }

    #[test]
    fn execute_command_with_invokes_executor_for_non_empty_commands() {
        let command = vec!["echo".to_string(), "ok".to_string()];
        let mut observed = String::new();

        let result = execute_command_with(&command, |command| {
            observed = command.join(" ");
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(observed, "echo ok");
    }

    #[test]
    fn execute_launch_plan_with_prefixes_script_write_errors() {
        let temp_dir = unique_test_dir("execute-launch-plan-sync");
        let blocked_path = temp_dir.join("blocked");
        fs::write(&blocked_path, "not a directory").expect("blocked path should be writable");
        let launch_plan = LaunchPlan {
            session_name: "grove-ws-test".to_string(),
            pane_lookup_cmd: Vec::new(),
            cleanup_cmds: Vec::new(),
            pre_launch_cmds: Vec::new(),
            launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
            launcher_script: Some(LauncherScript {
                path: blocked_path.join(".grove/start.sh"),
                contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
            }),
        };

        let result = execute_launch_plan_with(&launch_plan, |_command| Ok(()));
        let error_text = result.expect_err("script write should fail").to_string();

        assert!(error_text.starts_with("launcher script write failed: "));
    }

    #[test]
    fn execute_launch_plan_for_mode_delegating_prefixes_script_write_errors() {
        let temp_dir = unique_test_dir("execute-launch-plan-sync-mode");
        let blocked_path = temp_dir.join("blocked");
        fs::write(&blocked_path, "not a directory").expect("blocked path should be writable");
        let launch_plan = LaunchPlan {
            session_name: "grove-ws-test".to_string(),
            pane_lookup_cmd: Vec::new(),
            cleanup_cmds: Vec::new(),
            pre_launch_cmds: Vec::new(),
            launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
            launcher_script: Some(LauncherScript {
                path: blocked_path.join(".grove/start.sh"),
                contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
            }),
        };

        let result = execute_launch_plan_for_mode(
            &launch_plan,
            CommandExecutionMode::Delegating(&mut |_command| Ok(())),
        );
        let error_text = result.expect_err("script write should fail");

        assert!(error_text.starts_with("launcher script write failed: "));
    }

    #[test]
    fn execute_launch_plan_keeps_unprefixed_script_write_errors() {
        let temp_dir = unique_test_dir("execute-launch-plan");
        let blocked_path = temp_dir.join("blocked");
        fs::write(&blocked_path, "not a directory").expect("blocked path should be writable");
        let launch_plan = LaunchPlan {
            session_name: "grove-ws-test".to_string(),
            pane_lookup_cmd: Vec::new(),
            cleanup_cmds: Vec::new(),
            pre_launch_cmds: Vec::new(),
            launch_cmd: vec!["sh".to_string(), "-lc".to_string(), "true".to_string()],
            launcher_script: Some(LauncherScript {
                path: blocked_path.join(".grove/start.sh"),
                contents: "#!/usr/bin/env bash\necho hi\n".to_string(),
            }),
        };

        let result = execute_launch_plan(launch_plan);
        let error_text = result.expect_err("script write should fail").to_string();

        assert!(!error_text.starts_with("launcher script write failed: "));
    }

    #[test]
    fn kill_workspace_session_command_uses_project_scoped_tmux_session_name() {
        assert_eq!(
            kill_workspace_session_command(None, Some("project.one"), "feature/auth.v2"),
            vec![
                "tmux".to_string(),
                "kill-session".to_string(),
                "-t".to_string(),
                "grove-ws-project-one-feature-auth-v2".to_string(),
            ]
        );
    }

    #[test]
    fn kill_workspace_session_commands_include_agent_git_and_shell_sessions() {
        assert_eq!(
            kill_workspace_session_commands(None, Some("project.one"), "feature/auth.v2"),
            vec![
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2-git".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2-shell".to_string(),
                ],
            ]
        );
    }

    #[test]
    fn workspace_session_name_matches_accepts_numbered_agent_and_shell_tabs() {
        assert!(workspace_session_name_matches(
            None,
            Some("project.one"),
            "feature/auth.v2",
            "grove-ws-project-one-feature-auth-v2-agent-1",
        ));
        assert!(workspace_session_name_matches(
            None,
            Some("project.one"),
            "feature/auth.v2",
            "grove-ws-project-one-feature-auth-v2-shell-2",
        ));
        assert!(workspace_session_name_matches(
            None,
            Some("project.one"),
            "feature/auth.v2",
            "grove-ws-project-one-feature-auth-v2-git",
        ));
        assert!(!workspace_session_name_matches(
            None,
            Some("project.one"),
            "feature/auth.v2",
            "grove-ws-project-one-feature-auth-v2-agent-x",
        ));
        assert!(!workspace_session_name_matches(
            None,
            Some("project.one"),
            "feature/auth.v2",
            "grove-ws-project-one-other-agent-1",
        ));
    }

    #[test]
    fn workspace_session_names_for_cleanup_filters_to_workspace_tabs() {
        let sessions = vec![
            "grove-ws-project-one-feature-auth-v2".to_string(),
            "grove-ws-project-one-feature-auth-v2-git".to_string(),
            "grove-ws-project-one-feature-auth-v2-shell-1".to_string(),
            "grove-ws-project-one-feature-auth-v2-agent-1".to_string(),
            "grove-ws-project-one-other-agent-1".to_string(),
        ];
        assert_eq!(
            workspace_session_names_for_cleanup(
                None,
                Some("project.one"),
                "feature/auth.v2",
                sessions.as_slice(),
            ),
            vec![
                "grove-ws-project-one-feature-auth-v2".to_string(),
                "grove-ws-project-one-feature-auth-v2-git".to_string(),
                "grove-ws-project-one-feature-auth-v2-shell-1".to_string(),
                "grove-ws-project-one-feature-auth-v2-agent-1".to_string(),
            ],
        );
        assert_eq!(
            kill_workspace_session_commands_for_existing_sessions(
                None,
                Some("project.one"),
                "feature/auth.v2",
                sessions.as_slice(),
            ),
            vec![
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2-git".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2-shell-1".to_string(),
                ],
                vec![
                    "tmux".to_string(),
                    "kill-session".to_string(),
                    "-t".to_string(),
                    "grove-ws-project-one-feature-auth-v2-agent-1".to_string(),
                ],
            ],
        );
    }
}

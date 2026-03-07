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

pub fn start_workspace_with_mode(
    request: &LaunchRequest,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    execute_launch_request_with_result_for_mode(request, mode)
}

pub fn stop_workspace_with_mode(
    workspace: &Workspace,
    mode: CommandExecutionMode<'_>,
) -> SessionExecutionResult {
    execute_stop_workspace_with_result_for_mode(workspace, mode)
}

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
    if let Some(script) = &launch_plan.launcher_script {
        executor.write_launcher_script(script)?;
    }

    execute_commands_with_executor(&launch_plan.pre_launch_cmds, executor)?;
    execute_command_with(&launch_plan.launch_cmd, |command| executor.execute(command))
}

pub fn kill_workspace_session_command(
    project_name: Option<&str>,
    workspace_name: &str,
) -> Vec<String> {
    let session_name = session_name_for_workspace_in_project(project_name, workspace_name);
    kill_tmux_session_command(&session_name)
}

pub fn kill_workspace_session_commands(
    project_name: Option<&str>,
    workspace_name: &str,
) -> Vec<Vec<String>> {
    let session_name = session_name_for_workspace_in_project(project_name, workspace_name);
    vec![
        kill_tmux_session_command(&session_name),
        kill_tmux_session_command(&format!("{session_name}-git")),
        kill_tmux_session_command(&format!("{session_name}-shell")),
    ]
}

fn has_numbered_suffix(session_name: &str, prefix: &str) -> bool {
    let Some(ordinal) = session_name.strip_prefix(prefix) else {
        return false;
    };
    !ordinal.is_empty() && ordinal.chars().all(|character| character.is_ascii_digit())
}

fn session_name_matches_base_session(base_session_name: &str, session_name: &str) -> bool {
    if session_name == base_session_name {
        return true;
    }

    let git_session_name = format!("{base_session_name}-git");
    if session_name == git_session_name {
        return true;
    }

    let shell_session_name = format!("{base_session_name}-shell");
    if session_name == shell_session_name {
        return true;
    }

    let agent_prefix = format!("{base_session_name}-agent-");
    if has_numbered_suffix(session_name, agent_prefix.as_str()) {
        return true;
    }

    let shell_prefix = format!("{base_session_name}-shell-");
    has_numbered_suffix(session_name, shell_prefix.as_str())
}

pub fn workspace_session_name_matches(
    project_name: Option<&str>,
    workspace_name: &str,
    session_name: &str,
) -> bool {
    let base_session_name = session_name_for_workspace_in_project(project_name, workspace_name);
    session_name_matches_base_session(base_session_name.as_str(), session_name)
}

pub fn workspace_session_names_for_cleanup(
    project_name: Option<&str>,
    workspace_name: &str,
    existing_sessions: &[String],
) -> Vec<String> {
    let mut matched = Vec::new();
    let mut seen = HashSet::new();
    for session_name in existing_sessions {
        if !workspace_session_name_matches(project_name, workspace_name, session_name) {
            continue;
        }
        if !seen.insert(session_name.clone()) {
            continue;
        }
        matched.push(session_name.clone());
    }
    matched
}

pub fn kill_workspace_session_commands_for_existing_sessions(
    project_name: Option<&str>,
    workspace_name: &str,
    existing_sessions: &[String],
) -> Vec<Vec<String>> {
    workspace_session_names_for_cleanup(project_name, workspace_name, existing_sessions)
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

fn kill_tmux_session_command(session_name: &str) -> Vec<String> {
    vec![
        "tmux".to_string(),
        "kill-session".to_string(),
        "-t".to_string(),
        session_name.to_string(),
    ]
}

use std::fs;

use crate::domain::Workspace;

use super::launch_plan::{build_launch_plan, build_shell_launch_plan, stop_plan};
use super::sessions::{session_name_for_workspace_in_project, session_name_for_workspace_ref};
use super::{
    LaunchPlan, LaunchRequest, LauncherScript, SessionExecutionResult, ShellLaunchRequest,
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

fn kill_tmux_session_command(session_name: &str) -> Vec<String> {
    vec![
        "tmux".to_string(),
        "kill-session".to_string(),
        "-t".to_string(),
        session_name.to_string(),
    ]
}

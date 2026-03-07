use std::path::Path;

use crate::domain::{AgentType, Workspace};

use super::sessions::{session_name_for_task, session_name_for_workspace_in_project};
use super::{
    GROVE_LAUNCHER_SCRIPT_PATH, LaunchPlan, LaunchRequest, LauncherScript,
    OPENCODE_UNSAFE_PERMISSION_JSON, ShellLaunchRequest,
};

pub fn launch_request_for_workspace(
    workspace: &Workspace,
    prompt: Option<String>,
    workspace_init_command: Option<String>,
    skip_permissions: bool,
    agent_env: Vec<(String, String)>,
    capture_cols: Option<u16>,
    capture_rows: Option<u16>,
) -> LaunchRequest {
    LaunchRequest {
        project_name: workspace.project_name.clone(),
        workspace_name: workspace.name.clone(),
        workspace_path: workspace.path.clone(),
        agent: workspace.agent,
        prompt,
        workspace_init_command,
        skip_permissions,
        agent_env,
        capture_cols,
        capture_rows,
    }
}

pub fn shell_launch_request_for_workspace(
    workspace: &Workspace,
    session_name: String,
    command: String,
    workspace_init_command: Option<String>,
    capture_cols: Option<u16>,
    capture_rows: Option<u16>,
) -> ShellLaunchRequest {
    ShellLaunchRequest {
        session_name,
        workspace_path: workspace.path.clone(),
        command,
        workspace_init_command,
        capture_cols,
        capture_rows,
    }
}

pub fn tmux_launch_error_indicates_duplicate_session(error: &str) -> bool {
    error.to_ascii_lowercase().contains("duplicate session")
}

pub fn build_launch_plan(request: &LaunchRequest) -> LaunchPlan {
    let session_name = session_name_for_workspace_in_project(
        request.project_name.as_deref(),
        &request.workspace_name,
    );
    let agent_cmd = build_agent_command(request.agent, request.skip_permissions);
    let launch_agent_cmd = launch_command_with_workspace_init(
        &request.workspace_path,
        agent_cmd,
        request.workspace_init_command.as_deref(),
    );
    let mut plan = tmux_launch_plan(request, session_name, launch_agent_cmd);
    if let Some(resize_cmd) = launch_resize_window_command(
        &plan.session_name,
        request.capture_cols,
        request.capture_rows,
    ) {
        plan.pre_launch_cmds.push(resize_cmd);
    }
    plan
}

pub fn build_task_launch_plan(request: &super::TaskLaunchRequest) -> LaunchPlan {
    let session_name = session_name_for_task(request.task_slug.as_str());
    let agent_cmd = build_agent_command(request.agent, request.skip_permissions);
    let launch_agent_cmd = launch_command_with_workspace_init(
        &request.task_root,
        agent_cmd,
        request.workspace_init_command.as_deref(),
    );
    let shared = LaunchRequest {
        project_name: None,
        workspace_name: request.task_slug.clone(),
        workspace_path: request.task_root.clone(),
        agent: request.agent,
        prompt: request.prompt.clone(),
        workspace_init_command: request.workspace_init_command.clone(),
        skip_permissions: request.skip_permissions,
        agent_env: request.agent_env.clone(),
        capture_cols: request.capture_cols,
        capture_rows: request.capture_rows,
    };
    let mut plan = tmux_launch_plan(&shared, session_name, launch_agent_cmd);
    if let Some(resize_cmd) = launch_resize_window_command(
        &plan.session_name,
        request.capture_cols,
        request.capture_rows,
    ) {
        plan.pre_launch_cmds.push(resize_cmd);
    }
    plan
}

pub fn build_shell_launch_plan(request: &ShellLaunchRequest) -> LaunchPlan {
    let wrapped_command = launch_command_with_workspace_init(
        &request.workspace_path,
        request.command.clone(),
        request.workspace_init_command.as_deref(),
    );
    let shared = LaunchRequest {
        project_name: None,
        workspace_name: request.session_name.clone(),
        workspace_path: request.workspace_path.clone(),
        agent: AgentType::Codex,
        prompt: None,
        workspace_init_command: request.workspace_init_command.clone(),
        skip_permissions: false,
        agent_env: Vec::new(),
        capture_cols: request.capture_cols,
        capture_rows: request.capture_rows,
    };
    let mut plan = tmux_launch_plan(
        &shared,
        request.session_name.clone(),
        wrapped_command.clone(),
    );
    if let Some(resize_cmd) = launch_resize_window_command(
        &plan.session_name,
        request.capture_cols,
        request.capture_rows,
    ) {
        plan.pre_launch_cmds.push(resize_cmd);
    }
    if wrapped_command.trim().is_empty() {
        plan.launch_cmd = Vec::new();
    }
    plan
}

fn tmux_launch_plan(
    request: &LaunchRequest,
    session_name: String,
    launch_agent_cmd: String,
) -> LaunchPlan {
    let session_target = session_name.clone();
    let mut pre_launch_cmds = vec![
        vec![
            "tmux".to_string(),
            "new-session".to_string(),
            "-d".to_string(),
            "-s".to_string(),
            session_name.clone(),
            "-c".to_string(),
            request.workspace_path.to_string_lossy().to_string(),
        ],
        vec![
            "tmux".to_string(),
            "set-option".to_string(),
            "-t".to_string(),
            session_name.clone(),
            "history-limit".to_string(),
            "10000".to_string(),
        ],
    ];
    if let Some(agent_env_cmd) = build_agent_env_command(&request.agent_env) {
        pre_launch_cmds.push(vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            session_name.clone(),
            agent_env_cmd,
            "Enter".to_string(),
        ]);
    }
    let pane_lookup_cmd = vec![
        "tmux".to_string(),
        "list-panes".to_string(),
        "-t".to_string(),
        session_name.clone(),
        "-F".to_string(),
        "#{pane_id}".to_string(),
    ];

    match &request.prompt {
        None => LaunchPlan {
            session_name,
            pane_lookup_cmd,
            pre_launch_cmds,
            launch_cmd: vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                session_target,
                launch_agent_cmd,
                "Enter".to_string(),
            ],
            launcher_script: None,
        },
        Some(prompt) => {
            let launcher_path = request.workspace_path.join(GROVE_LAUNCHER_SCRIPT_PATH);
            let launcher_contents =
                build_launcher_script(&launch_agent_cmd, prompt, &launcher_path);
            LaunchPlan {
                session_name,
                pane_lookup_cmd,
                pre_launch_cmds,
                launch_cmd: vec![
                    "tmux".to_string(),
                    "send-keys".to_string(),
                    "-t".to_string(),
                    session_target,
                    format!("bash {}", launcher_path.to_string_lossy()),
                    "Enter".to_string(),
                ],
                launcher_script: Some(LauncherScript {
                    path: launcher_path,
                    contents: launcher_contents,
                }),
            }
        }
    }
}

pub(super) fn build_agent_env_command(agent_env: &[(String, String)]) -> Option<String> {
    if agent_env.is_empty() {
        return None;
    }
    let exports = agent_env
        .iter()
        .map(|(key, value)| format!("{key}={}", shell_quote(value)))
        .collect::<Vec<String>>()
        .join(" ");
    Some(format!("export {exports}"))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn launch_resize_window_command(
    session_name: &str,
    capture_cols: Option<u16>,
    capture_rows: Option<u16>,
) -> Option<Vec<String>> {
    let cols = capture_cols.filter(|value| *value > 0)?;
    let rows = capture_rows.filter(|value| *value > 0)?;
    Some(vec![
        "tmux".to_string(),
        "resize-window".to_string(),
        "-t".to_string(),
        session_name.to_string(),
        "-x".to_string(),
        cols.to_string(),
        "-y".to_string(),
        rows.to_string(),
    ])
}

pub fn stop_plan(session_name: &str) -> Vec<Vec<String>> {
    vec![
        vec![
            "tmux".to_string(),
            "send-keys".to_string(),
            "-t".to_string(),
            session_name.to_string(),
            "C-c".to_string(),
        ],
        vec![
            "tmux".to_string(),
            "kill-session".to_string(),
            "-t".to_string(),
            session_name.to_string(),
        ],
    ]
}

pub(crate) fn build_agent_command(agent: AgentType, skip_permissions: bool) -> String {
    if let Some(command_override) = env_agent_command_override(agent) {
        return command_override;
    }

    default_agent_command(agent, skip_permissions)
}

pub(super) fn default_agent_command(agent: AgentType, skip_permissions: bool) -> String {
    match (agent, skip_permissions) {
        (AgentType::Claude, true) => "claude --dangerously-skip-permissions".to_string(),
        (AgentType::Claude, false) => "claude".to_string(),
        (AgentType::Codex, true) => "codex --dangerously-bypass-approvals-and-sandbox".to_string(),
        (AgentType::Codex, false) => "codex".to_string(),
        (AgentType::OpenCode, true) => {
            format!("OPENCODE_PERMISSION='{OPENCODE_UNSAFE_PERMISSION_JSON}' opencode")
        }
        (AgentType::OpenCode, false) => "opencode".to_string(),
    }
}

fn env_agent_command_override(agent: AgentType) -> Option<String> {
    let variable = agent.command_override_env_var();
    let override_value = std::env::var(variable).ok()?;
    trimmed_nonempty(&override_value)
}

pub(crate) fn trimmed_nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn launch_command_with_workspace_init(
    _workspace_path: &Path,
    command: String,
    workspace_init_command: Option<&str>,
) -> String {
    let Some(init_command) = workspace_init_command.and_then(trimmed_nonempty) else {
        return command;
    };
    let run_command = trimmed_nonempty(&command).map(|command| {
        if init_command_mentions_direnv(init_command.as_str()) {
            return direnv_exec_wrapped_command(command.as_str());
        }

        command
    });
    let mut script = init_command;
    if let Some(run_command) = run_command {
        script.push('\n');
        script.push_str(run_command.as_str());
    }
    format!("bash -lc {}", shell_quote(script.as_str()))
}

fn init_command_mentions_direnv(command: &str) -> bool {
    command
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '_' || character == '-')
        })
        .any(|token| token.eq_ignore_ascii_case("direnv"))
}

fn direnv_exec_wrapped_command(command: &str) -> String {
    format!("direnv exec . bash -lc {}", shell_quote(command))
}

fn build_launcher_script(agent_cmd: &str, prompt: &str, launcher_path: &Path) -> String {
    format!(
        "#!/bin/bash\nexport NVM_DIR=\"${{NVM_DIR:-$HOME/.nvm}}\"\n[ -s \"$NVM_DIR/nvm.sh\" ] && source \"$NVM_DIR/nvm.sh\" 2>/dev/null\nif ! command -v node &>/dev/null; then\n  [ -f \"$HOME/.zshrc\" ] && source \"$HOME/.zshrc\" 2>/dev/null\n  [ -f \"$HOME/.bashrc\" ] && source \"$HOME/.bashrc\" 2>/dev/null\nfi\n{agent_cmd} \"$(cat <<'GROVE_PROMPT_EOF'\n{prompt}\nGROVE_PROMPT_EOF\n)\"\nrm -f {}\n",
        launcher_path.to_string_lossy()
    )
}

#[cfg(test)]
mod tests {
    use super::build_task_launch_plan;
    use crate::application::agent_runtime::TaskLaunchRequest;
    use crate::domain::AgentType;
    use std::path::PathBuf;

    #[test]
    fn build_task_launch_plan_targets_task_root_session() {
        let request = TaskLaunchRequest {
            task_slug: "flohome-launch".to_string(),
            task_root: PathBuf::from("/tmp/.grove/tasks/flohome-launch"),
            agent: AgentType::Codex,
            prompt: None,
            workspace_init_command: None,
            skip_permissions: false,
            agent_env: Vec::new(),
            capture_cols: None,
            capture_rows: None,
        };

        let plan = build_task_launch_plan(&request);

        assert_eq!(plan.session_name, "grove-task-flohome-launch");
        assert_eq!(
            plan.pre_launch_cmds[0],
            vec![
                "tmux".to_string(),
                "new-session".to_string(),
                "-d".to_string(),
                "-s".to_string(),
                "grove-task-flohome-launch".to_string(),
                "-c".to_string(),
                "/tmp/.grove/tasks/flohome-launch".to_string(),
            ]
        );
        assert_eq!(
            plan.launch_cmd,
            vec![
                "tmux".to_string(),
                "send-keys".to_string(),
                "-t".to_string(),
                "grove-task-flohome-launch".to_string(),
                "codex".to_string(),
                "Enter".to_string(),
            ]
        );
    }
}

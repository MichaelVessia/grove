use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::domain::{AgentType, Workspace, WorkspaceStatus};

pub const TMUX_SESSION_PREFIX: &str = "grove-ws-";
const WAITING_PATTERNS: [&str; 6] = [
    "[y/n]",
    "(y/n)",
    "allow edit",
    "allow bash",
    "approve",
    "confirm",
];
const THINKING_PATTERNS: [&str; 2] = ["<thinking>", "thinking..."];
const DONE_PATTERNS: [&str; 3] = ["task completed", "finished", "exited with code 0"];
const ERROR_PATTERNS: [&str; 4] = ["error:", "failed", "panic:", "traceback"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionActivity {
    Idle,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub workspace_name: String,
    pub workspace_path: PathBuf,
    pub agent: AgentType,
    pub prompt: Option<String>,
    pub skip_permissions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherScript {
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPlan {
    pub session_name: String,
    pub pane_lookup_cmd: Vec<String>,
    pub pre_launch_cmds: Vec<Vec<String>>,
    pub launch_cmd: Vec<String>,
    pub launcher_script: Option<LauncherScript>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationResult {
    pub workspaces: Vec<Workspace>,
    pub orphaned_sessions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputDigest {
    pub raw_hash: u64,
    pub raw_len: usize,
    pub cleaned_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureChange {
    pub digest: OutputDigest,
    pub changed_raw: bool,
    pub changed_cleaned: bool,
    pub cleaned_output: String,
    pub render_output: String,
}

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

pub fn session_name_for_workspace(workspace_name: &str) -> String {
    format!(
        "{TMUX_SESSION_PREFIX}{}",
        sanitize_workspace_name(workspace_name)
    )
}

pub fn build_launch_plan(request: &LaunchRequest) -> LaunchPlan {
    let session_name = session_name_for_workspace(&request.workspace_name);
    let session_target = session_name.clone();

    let pre_launch_cmds = vec![
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

    let pane_lookup_cmd = vec![
        "tmux".to_string(),
        "list-panes".to_string(),
        "-t".to_string(),
        session_name.clone(),
        "-F".to_string(),
        "#{pane_id}".to_string(),
    ];

    let agent_cmd = build_agent_command(request.agent, request.skip_permissions);

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
                agent_cmd,
                "Enter".to_string(),
            ],
            launcher_script: None,
        },
        Some(prompt) => {
            let launcher_path = request.workspace_path.join(".grove-start.sh");
            let launcher_contents = build_launcher_script(&agent_cmd, prompt, &launcher_path);
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

fn default_agent_command(agent: AgentType, skip_permissions: bool) -> String {
    match (agent, skip_permissions) {
        (AgentType::Claude, true) => "claude --dangerously-skip-permissions".to_string(),
        (AgentType::Claude, false) => "claude".to_string(),
        (AgentType::Codex, true) => "codex --dangerously-bypass-approvals-and-sandbox".to_string(),
        (AgentType::Codex, false) => "codex".to_string(),
    }
}

fn env_agent_command_override(agent: AgentType) -> Option<String> {
    let variable = match agent {
        AgentType::Claude => "GROVE_CLAUDE_CMD",
        AgentType::Codex => "GROVE_CODEX_CMD",
    };
    let override_value = std::env::var(variable).ok()?;
    normalized_agent_command_override(&override_value)
}

fn normalized_agent_command_override(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

pub(crate) fn detect_waiting_prompt(output: &str) -> Option<String> {
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(5);

    for line in &lines[start..] {
        let lower = line.to_ascii_lowercase();
        if WAITING_PATTERNS
            .iter()
            .any(|pattern| lower.contains(pattern))
        {
            return Some(line.trim().to_string());
        }
    }

    None
}

pub(crate) fn detect_status(
    output: &str,
    session_activity: SessionActivity,
    is_main: bool,
    has_live_session: bool,
    supported_agent: bool,
) -> WorkspaceStatus {
    if is_main {
        return WorkspaceStatus::Main;
    }

    if !supported_agent {
        return WorkspaceStatus::Unsupported;
    }

    if !has_live_session {
        return WorkspaceStatus::Idle;
    }

    let lower_output = output.to_ascii_lowercase();

    if ERROR_PATTERNS
        .iter()
        .any(|pattern| lower_output.contains(pattern))
    {
        return WorkspaceStatus::Error;
    }

    if DONE_PATTERNS
        .iter()
        .any(|pattern| lower_output.contains(pattern))
    {
        return WorkspaceStatus::Done;
    }

    if THINKING_PATTERNS
        .iter()
        .any(|pattern| lower_output.contains(pattern))
    {
        return WorkspaceStatus::Thinking;
    }

    if detect_waiting_prompt(output).is_some() {
        return WorkspaceStatus::Waiting;
    }

    match session_activity {
        SessionActivity::Active => WorkspaceStatus::Active,
        SessionActivity::Idle => WorkspaceStatus::Idle,
    }
}

pub fn reconcile_with_sessions(
    workspaces: &[Workspace],
    running_sessions: &HashSet<String>,
    previously_running_workspace_names: &HashSet<String>,
) -> ReconciliationResult {
    let mut mapped_workspaces = Vec::with_capacity(workspaces.len());
    let mut matched_sessions = HashSet::new();

    for workspace in workspaces {
        let mut updated = workspace.clone();
        if workspace.is_main {
            mapped_workspaces.push(updated);
            continue;
        }

        let session_name = session_name_for_workspace(&workspace.name);
        let has_live_session = running_sessions.contains(&session_name);
        if has_live_session {
            matched_sessions.insert(session_name);
            updated.status = detect_status(
                "",
                SessionActivity::Active,
                false,
                true,
                updated.supported_agent,
            );
            updated.is_orphaned = false;
        } else {
            updated.status = detect_status(
                "",
                SessionActivity::Idle,
                false,
                false,
                updated.supported_agent,
            );
            updated.is_orphaned = previously_running_workspace_names.contains(&workspace.name);
        }

        mapped_workspaces.push(updated);
    }

    let mut orphaned_sessions: Vec<String> = running_sessions
        .iter()
        .filter(|session_name| !matched_sessions.contains(*session_name))
        .cloned()
        .collect();
    orphaned_sessions.sort();

    ReconciliationResult {
        workspaces: mapped_workspaces,
        orphaned_sessions,
    }
}

pub fn poll_interval(
    status: WorkspaceStatus,
    is_selected: bool,
    is_preview_focused: bool,
    interactive_mode: bool,
    since_last_key: Duration,
    output_changing: bool,
) -> Duration {
    if interactive_mode && is_selected {
        if since_last_key < Duration::from_secs(2) {
            return Duration::from_millis(50);
        }
        if since_last_key < Duration::from_secs(10) {
            return Duration::from_millis(200);
        }
        return Duration::from_millis(500);
    }

    if !is_selected {
        return Duration::from_secs(10);
    }

    if output_changing {
        return Duration::from_millis(200);
    }

    if is_preview_focused {
        return Duration::from_millis(500);
    }

    match status {
        WorkspaceStatus::Active | WorkspaceStatus::Thinking => Duration::from_millis(200),
        WorkspaceStatus::Waiting | WorkspaceStatus::Idle => Duration::from_secs(2),
        WorkspaceStatus::Done | WorkspaceStatus::Error => Duration::from_secs(20),
        WorkspaceStatus::Main | WorkspaceStatus::Unknown | WorkspaceStatus::Unsupported => {
            Duration::from_secs(2)
        }
    }
}

pub(crate) fn evaluate_capture_change(
    previous: Option<&OutputDigest>,
    raw_output: &str,
) -> CaptureChange {
    let render_output = strip_non_sgr_control_sequences(raw_output);
    let cleaned_output = strip_mouse_fragments(&strip_sgr_sequences(&render_output));
    let digest = OutputDigest {
        raw_hash: content_hash(raw_output),
        raw_len: raw_output.len(),
        cleaned_hash: content_hash(&cleaned_output),
    };

    match previous {
        None => CaptureChange {
            digest,
            changed_raw: true,
            changed_cleaned: true,
            cleaned_output,
            render_output,
        },
        Some(previous_digest) => CaptureChange {
            changed_raw: previous_digest.raw_hash != digest.raw_hash
                || previous_digest.raw_len != digest.raw_len,
            changed_cleaned: previous_digest.cleaned_hash != digest.cleaned_hash,
            digest,
            cleaned_output,
            render_output,
        },
    }
}

fn is_safe_text_character(character: char) -> bool {
    matches!(character, '\n' | '\t') || !character.is_control()
}

pub(crate) fn strip_mouse_fragments(input: &str) -> String {
    let mut cleaned = input.to_string();

    for mode in [1000u16, 1002, 1003, 1005, 1006, 1015, 2004] {
        cleaned = cleaned.replace(&format!("\u{1b}[?{mode}h"), "");
        cleaned = cleaned.replace(&format!("\u{1b}[?{mode}l"), "");
        cleaned = cleaned.replace(&format!("[?{mode}h"), "");
        cleaned = cleaned.replace(&format!("[?{mode}l"), "");
    }

    strip_partial_mouse_sequences(&cleaned)
}

fn strip_non_sgr_control_sequences(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        if character != '\u{1b}' {
            if is_safe_text_character(character) {
                cleaned.push(character);
            }
            continue;
        }

        let Some(next) = chars.next() else {
            break;
        };

        match next {
            '[' => {
                let mut csi = String::from("\u{1b}[");
                if let Some(final_char) = consume_csi_sequence(&mut chars, &mut csi)
                    && final_char == 'm'
                {
                    cleaned.push_str(&csi);
                }
            }
            ']' => consume_osc_sequence(&mut chars),
            'P' | 'X' | '^' | '_' => consume_st_sequence(&mut chars),
            '(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' => {
                let _ = chars.next();
            }
            _ => {}
        }
    }

    cleaned
}

fn strip_sgr_sequences(input: &str) -> String {
    let mut cleaned = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            if chars.next_if_eq(&'[').is_some() {
                let mut did_end = false;
                while let Some(value) = chars.next() {
                    if ('\u{40}'..='\u{7e}').contains(&value) {
                        did_end = true;
                        break;
                    }
                }
                if did_end {
                    continue;
                }
            }
            continue;
        }

        if is_safe_text_character(character) {
            cleaned.push(character);
        }
    }

    cleaned
}

fn consume_csi_sequence<I>(chars: &mut std::iter::Peekable<I>, buffer: &mut String) -> Option<char>
where
    I: Iterator<Item = char>,
{
    while let Some(character) = chars.next() {
        buffer.push(character);
        if ('\u{40}'..='\u{7e}').contains(&character) {
            return Some(character);
        }
    }

    None
}

fn consume_osc_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(character) = chars.next() {
        if character == '\u{7}' {
            return;
        }

        if character == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
            return;
        }
    }
}

fn consume_st_sequence<I>(chars: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.next_if_eq(&'\\').is_some() {
            return;
        }
    }
}

fn strip_partial_mouse_sequences(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    while index < bytes.len() {
        if let Some(end) = parse_mouse_fragment_end(bytes, index) {
            index = end;
            continue;
        }

        output.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(output).unwrap_or_default()
}

fn parse_mouse_fragment_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) == Some(&b'[') && bytes.get(start.saturating_add(1)) == Some(&b'<') {
        return parse_sgr_mouse_tail(bytes, start.saturating_add(2));
    }
    if matches!(bytes.get(start), Some(b'M' | b'm'))
        && bytes.get(start.saturating_add(1)) == Some(&b'[')
        && bytes.get(start.saturating_add(2)) == Some(&b'<')
    {
        return parse_sgr_mouse_tail(bytes, start.saturating_add(3));
    }

    None
}

fn parse_sgr_mouse_tail(bytes: &[u8], mut index: usize) -> Option<usize> {
    index = consume_ascii_digits(bytes, index)?;

    if bytes.get(index) != Some(&b';') {
        return None;
    }
    index = index.saturating_add(1);
    index = consume_ascii_digits(bytes, index)?;

    if bytes.get(index) != Some(&b';') {
        return None;
    }
    index = index.saturating_add(1);
    index = consume_ascii_digits(bytes, index)?;

    if matches!(bytes.get(index), Some(b'M' | b'm')) {
        index = index.saturating_add(1);
    }

    Some(index)
}

fn consume_ascii_digits(bytes: &[u8], mut start: usize) -> Option<usize> {
    let initial = start;
    while matches!(bytes.get(start), Some(b'0'..=b'9')) {
        start = start.saturating_add(1);
    }

    if start == initial { None } else { Some(start) }
}

fn build_launcher_script(agent_cmd: &str, prompt: &str, launcher_path: &Path) -> String {
    format!(
        "#!/bin/bash\nexport NVM_DIR=\"${{NVM_DIR:-$HOME/.nvm}}\"\n[ -s \"$NVM_DIR/nvm.sh\" ] && source \"$NVM_DIR/nvm.sh\" 2>/dev/null\nif ! command -v node &>/dev/null; then\n  [ -f \"$HOME/.zshrc\" ] && source \"$HOME/.zshrc\" 2>/dev/null\n  [ -f \"$HOME/.bashrc\" ] && source \"$HOME/.bashrc\" 2>/dev/null\nfi\n{agent_cmd} \"$(cat <<'GROVE_PROMPT_EOF'\n{prompt}\nGROVE_PROMPT_EOF\n)\"\nrm -f {}\n",
        launcher_path.to_string_lossy()
    )
}

fn content_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{
        CaptureChange, LaunchRequest, SessionActivity, build_launch_plan, default_agent_command,
        detect_status, detect_waiting_prompt, evaluate_capture_change,
        normalized_agent_command_override, poll_interval, reconcile_with_sessions,
        sanitize_workspace_name, session_name_for_workspace, stop_plan, strip_mouse_fragments,
    };
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};

    fn fixture_workspace(name: &str, is_main: bool) -> Workspace {
        Workspace::try_new(
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
    fn codex_launch_command_matches_prd_flags() {
        assert_eq!(default_agent_command(AgentType::Codex, false), "codex");
        assert_eq!(
            default_agent_command(AgentType::Codex, true),
            "codex --dangerously-bypass-approvals-and-sandbox"
        );
    }

    #[test]
    fn agent_command_override_normalization_trims_whitespace() {
        assert_eq!(
            normalized_agent_command_override("  /tmp/fake-codex --flag  "),
            Some("/tmp/fake-codex --flag".to_string())
        );
    }

    #[test]
    fn agent_command_override_normalization_ignores_empty_values() {
        assert_eq!(normalized_agent_command_override(""), None);
        assert_eq!(normalized_agent_command_override("   "), None);
    }

    #[test]
    fn launch_plan_without_prompt_sends_agent_directly() {
        let request = LaunchRequest {
            workspace_name: "auth-flow".to_string(),
            workspace_path: PathBuf::from("/repos/grove-auth-flow"),
            agent: AgentType::Claude,
            prompt: None,
            skip_permissions: true,
        };

        let plan = build_launch_plan(&request);

        assert_eq!(plan.session_name, "grove-ws-auth-flow");
        assert!(plan.launcher_script.is_none());
        assert_eq!(
            plan.launch_cmd,
            vec![
                "tmux",
                "send-keys",
                "-t",
                "grove-ws-auth-flow",
                "claude --dangerously-skip-permissions",
                "Enter"
            ]
        );
    }

    #[test]
    fn launch_plan_with_prompt_writes_launcher_script() {
        let request = LaunchRequest {
            workspace_name: "db_migration".to_string(),
            workspace_path: PathBuf::from("/repos/grove-db_migration"),
            agent: AgentType::Codex,
            prompt: Some("fix migration".to_string()),
            skip_permissions: false,
        };

        let plan = build_launch_plan(&request);

        let script = plan.launcher_script.expect("script should be present");
        assert!(script.contents.contains("codex"));
        assert!(script.contents.contains("fix migration"));
        assert!(script.contents.contains("GROVE_PROMPT_EOF"));
        assert_eq!(
            plan.launch_cmd,
            vec![
                "tmux",
                "send-keys",
                "-t",
                "grove-ws-db_migration",
                "bash /repos/grove-db_migration/.grove-start.sh",
                "Enter"
            ]
        );
    }

    #[test]
    fn stop_plan_uses_ctrl_c_then_kill_session() {
        let plan = stop_plan("grove-ws-auth-flow");
        assert_eq!(plan.len(), 2);
        assert_eq!(
            plan[0],
            vec!["tmux", "send-keys", "-t", "grove-ws-auth-flow", "C-c"]
        );
        assert_eq!(
            plan[1],
            vec!["tmux", "kill-session", "-t", "grove-ws-auth-flow"]
        );
    }

    #[test]
    fn waiting_prompt_checks_tail_lines_only() {
        let output = "approve earlier\nline\nline\nline\nline\nline\n";
        assert_eq!(detect_waiting_prompt(output), None);

        let tail_output = "line\nline\nline\nline\nallow edit? [y/n]\n";
        assert_eq!(
            detect_waiting_prompt(tail_output),
            Some("allow edit? [y/n]".to_string())
        );
    }

    #[test]
    fn status_resolution_prioritizes_error_done_thinking_before_session_activity() {
        assert_eq!(
            detect_status("panic: bad", SessionActivity::Active, false, true, true),
            WorkspaceStatus::Error
        );
        assert_eq!(
            detect_status(
                "task completed successfully",
                SessionActivity::Active,
                false,
                true,
                true
            ),
            WorkspaceStatus::Done
        );
        assert_eq!(
            detect_status("thinking...", SessionActivity::Active, false, true, true),
            WorkspaceStatus::Thinking
        );
        assert_eq!(
            detect_status("allow edit? [y/n]", SessionActivity::Active, false, true, true),
            WorkspaceStatus::Waiting
        );
        assert_eq!(
            detect_status("", SessionActivity::Active, false, true, true),
            WorkspaceStatus::Active
        );
        assert_eq!(
            detect_status("", SessionActivity::Idle, false, false, true),
            WorkspaceStatus::Idle
        );
        assert_eq!(
            detect_status("", SessionActivity::Active, false, true, false),
            WorkspaceStatus::Unsupported
        );
    }

    #[test]
    fn reconciliation_marks_active_orphaned_and_orphan_sessions() {
        let workspaces = vec![
            fixture_workspace("grove", true),
            fixture_workspace("feature-a", false),
            fixture_workspace("feature-b", false),
        ];

        let running_sessions = HashSet::from([
            "grove-ws-feature-a".to_string(),
            "grove-ws-zombie".to_string(),
        ]);
        let previously_running = HashSet::from(["feature-b".to_string()]);

        let result = reconcile_with_sessions(&workspaces, &running_sessions, &previously_running);
        assert_eq!(result.workspaces[1].status, WorkspaceStatus::Active);
        assert_eq!(result.workspaces[2].status, WorkspaceStatus::Idle);
        assert!(result.workspaces[2].is_orphaned);
        assert_eq!(
            result.orphaned_sessions,
            vec!["grove-ws-zombie".to_string()]
        );
    }

    #[test]
    fn poll_intervals_follow_preview_and_interactive_rules() {
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_millis(100),
                true
            ),
            Duration::from_millis(50)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_secs(5),
                true
            ),
            Duration::from_millis(200)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                true,
                false,
                true,
                Duration::from_secs(15),
                false
            ),
            Duration::from_millis(500)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Active,
                false,
                false,
                false,
                Duration::from_secs(30),
                true
            ),
            Duration::from_secs(10)
        );
        assert_eq!(
            poll_interval(
                WorkspaceStatus::Done,
                true,
                false,
                false,
                Duration::from_secs(30),
                false
            ),
            Duration::from_secs(20)
        );
    }

    #[test]
    fn capture_change_detects_mouse_fragment_noise() {
        let first = evaluate_capture_change(None, "hello\u{1b}[?1000h\u{1b}[<35;192;47M");
        assert!(first.changed_raw);
        assert!(first.changed_cleaned);

        let second = evaluate_capture_change(Some(&first.digest), "hello\u{1b}[?1000l");
        assert!(second.changed_raw);
        assert!(!second.changed_cleaned);
        assert_eq!(second.cleaned_output, "hello");

        let third = evaluate_capture_change(Some(&second.digest), "hello world");
        assert!(third.changed_cleaned);
    }

    #[test]
    fn capture_change_first_capture_marks_changed() {
        let change: CaptureChange = evaluate_capture_change(None, "one");
        assert!(change.changed_raw);
        assert!(change.changed_cleaned);
    }

    #[test]
    fn capture_change_strips_ansi_control_sequences() {
        let raw = "A\u{1b}[31mB\u{1b}[39m C\u{1b}]0;title\u{7}\n";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "AB C\n");
    }

    #[test]
    fn capture_change_strips_terminal_control_bytes() {
        let raw = "A\u{000e}B\u{000f}C\r\n";
        let change = evaluate_capture_change(None, raw);
        assert_eq!(change.cleaned_output, "ABC\n");
        assert_eq!(change.render_output, "ABC\n");
    }

    #[test]
    fn capture_change_ignores_truncated_partial_mouse_fragments() {
        let first = evaluate_capture_change(None, "prompt [<65;103;31");
        assert_eq!(first.cleaned_output, "prompt ");

        let second = evaluate_capture_change(Some(&first.digest), "prompt [<65;103;32");
        assert!(!second.changed_cleaned);
        assert_eq!(second.cleaned_output, "prompt ");
    }

    #[test]
    fn strip_mouse_fragments_removes_terminal_modes_and_preserves_normal_brackets() {
        assert_eq!(strip_mouse_fragments("value[?1002h"), "value");
        assert_eq!(strip_mouse_fragments("keep [test]"), "keep [test]");
    }

    #[test]
    fn strip_mouse_fragments_removes_boundary_prefixed_partial_sequences() {
        assert_eq!(strip_mouse_fragments("prompt M[<64;107;16M"), "prompt ");
        assert_eq!(strip_mouse_fragments("prompt m[<65;107;14"), "prompt ");
    }
}

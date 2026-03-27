use super::*;
use crate::domain::{PermissionMode, Worktree};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartAgentConfigState {
    pub(super) name: String,
    pub(super) prompt: String,
    pub(super) init_command: String,
    pub(super) permission_mode: PermissionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartOptions {
    pub(super) name: Option<String>,
    pub(super) prompt: Option<String>,
    pub(super) init_command: Option<String>,
    pub(super) permission_mode: PermissionMode,
}

impl StartAgentConfigState {
    pub(super) fn new(
        name: String,
        prompt: String,
        init_command: String,
        permission_mode: PermissionMode,
    ) -> Self {
        Self {
            name,
            prompt,
            init_command,
            permission_mode,
        }
    }

    pub(super) fn is_input_nonempty(&self) -> bool {
        !self.name.is_empty() || !self.prompt.is_empty() || !self.init_command.is_empty()
    }

    pub(super) fn parse_start_options(&self) -> StartOptions {
        StartOptions {
            name: trimmed_nonempty(&self.name),
            prompt: trimmed_nonempty(&self.prompt),
            init_command: trimmed_nonempty(&self.init_command),
            permission_mode: self.permission_mode,
        }
    }

    fn text_field_mut(&mut self, field: StartAgentConfigField) -> Option<&mut String> {
        match field {
            StartAgentConfigField::Name => Some(&mut self.name),
            StartAgentConfigField::Prompt => Some(&mut self.prompt),
            StartAgentConfigField::InitCommand => Some(&mut self.init_command),
            StartAgentConfigField::Unsafe => None,
        }
    }

    pub(super) fn backspace(&mut self, field: StartAgentConfigField) {
        if let Some(text) = self.text_field_mut(field) {
            text.pop();
        }
    }

    pub(super) fn clear(&mut self, field: StartAgentConfigField) {
        if let Some(text) = self.text_field_mut(field) {
            text.clear();
        }
    }

    pub(super) fn push_char(&mut self, field: StartAgentConfigField, character: char) {
        if let Some(text) = self.text_field_mut(field) {
            text.push(character);
        }
    }

    pub(super) fn cycle_permission_mode(&mut self, agent: AgentType) {
        self.permission_mode = self.permission_mode.next_for_agent(agent);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LaunchDialogTarget {
    WorkspaceTab,
    ParentTask(Task),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LaunchDialogState {
    pub(super) target: LaunchDialogTarget,
    pub(super) agent: AgentType,
    pub(super) start_config: StartAgentConfigState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StopDialogState {
    pub(super) workspace: Workspace,
    pub(super) session_name: String,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ConfirmDialogAction {
    CloseActiveTab {
        workspace_path: PathBuf,
        tab_id: u64,
        session_name: String,
    },
    QuitApp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfirmDialogState {
    pub(super) action: ConfirmDialogAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionCleanupDialogState {
    pub(super) options: SessionCleanupOptions,
    pub(super) plan: SessionCleanupPlan,
    pub(super) last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeleteDialogState {
    pub(super) task: Task,
    pub(super) target: DeleteDialogTarget,
    pub(super) is_base_task: bool,
    pub(super) is_missing: bool,
    pub(super) delete_local_branch: bool,
    pub(super) kill_tmux_sessions: bool,
}

impl DeleteDialogState {
    pub(super) fn delete_local_branch_enabled(&self) -> bool {
        match &self.target {
            DeleteDialogTarget::Task => !self.is_base_task,
            DeleteDialogTarget::Worktree {
                is_main_worktree, ..
            } => !is_main_worktree,
        }
    }

    pub(super) fn deletes_task(&self) -> bool {
        match &self.target {
            DeleteDialogTarget::Task => true,
            DeleteDialogTarget::Worktree { deletes_task, .. } => *deletes_task,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DeleteDialogTarget {
    Task,
    Worktree {
        worktree: Worktree,
        deletes_task: bool,
        is_main_worktree: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MergeDialogState {
    pub(super) task_slug: Option<String>,
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) cleanup_workspace: bool,
    pub(super) cleanup_local_branch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UpdateFromBaseDialogState {
    pub(super) task_slug: Option<String>,
    pub(super) project_name: Option<String>,
    pub(super) project_path: Option<PathBuf>,
    pub(super) is_main_workspace: bool,
    pub(super) workspace_name: String,
    pub(super) workspace_branch: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeleteDialogField {
    DeleteLocalBranch,
    KillTmuxSessions,
    DeleteButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StopDialogField {
    StopButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfirmDialogField {
    ConfirmButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionCleanupDialogField {
    IncludeStale,
    IncludeAttached,
    ApplyButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MergeDialogField {
    CleanupWorkspace,
    CleanupLocalBranch,
    MergeButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UpdateFromBaseDialogField {
    UpdateButton,
    CancelButton,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PullUpstreamDialogState {
    pub(super) task_slug: Option<String>,
    pub(super) project_name: String,
    pub(super) project_path: PathBuf,
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) base_branch: String,
    pub(super) propagate_target_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PullUpstreamDialogField {
    PullButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartAgentConfigField {
    Name,
    Prompt,
    InitCommand,
    Unsafe,
}

impl StartAgentConfigField {
    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Prompt => "prompt",
            Self::InitCommand => "init_command",
            Self::Unsafe => "unsafe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LaunchDialogField {
    Agent,
    StartConfig(StartAgentConfigField),
    StartButton,
    CancelButton,
}

#[cfg(test)]
impl LaunchDialogField {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Agent => Self::StartConfig(StartAgentConfigField::Name),
            Self::StartConfig(StartAgentConfigField::Name) => {
                Self::StartConfig(StartAgentConfigField::Prompt)
            }
            Self::StartConfig(StartAgentConfigField::Prompt) => {
                Self::StartConfig(StartAgentConfigField::InitCommand)
            }
            Self::StartConfig(StartAgentConfigField::InitCommand) => {
                Self::StartConfig(StartAgentConfigField::Unsafe)
            }
            Self::StartConfig(StartAgentConfigField::Unsafe) => Self::StartButton,
            Self::StartButton => Self::CancelButton,
            Self::CancelButton => Self::Agent,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::StartConfig(StartAgentConfigField::Name) => Self::Agent,
            Self::StartConfig(StartAgentConfigField::Prompt) => {
                Self::StartConfig(StartAgentConfigField::Name)
            }
            Self::StartConfig(StartAgentConfigField::InitCommand) => {
                Self::StartConfig(StartAgentConfigField::Prompt)
            }
            Self::StartConfig(StartAgentConfigField::Unsafe) => {
                Self::StartConfig(StartAgentConfigField::InitCommand)
            }
            Self::StartButton => Self::StartConfig(StartAgentConfigField::Unsafe),
            Self::CancelButton => Self::StartButton,
            Self::Agent => Self::CancelButton,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::StartConfig(field) => field.label(),
            Self::StartButton => "start",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CreateDialogState {
    pub(super) mode: CreateDialogMode,
    pub(super) tab: CreateDialogTab,
    pub(super) task_name: String,
    pub(super) pr_url: String,
    pub(super) register_as_base: bool,
    pub(super) project_index: usize,
    pub(super) selected_repository_indices: Vec<usize>,
    pub(super) project_picker: Option<CreateProjectPickerState>,
}

impl CreateDialogState {
    pub(super) fn is_add_worktree_mode(&self) -> bool {
        matches!(self.mode, CreateDialogMode::AddWorktree { .. })
    }

    pub(super) fn target_task(&self) -> Option<&Task> {
        match &self.mode {
            CreateDialogMode::NewTask => None,
            CreateDialogMode::AddWorktree { task } => Some(task),
        }
    }

    pub(super) fn first_field(&self) -> CreateDialogField {
        if self.is_add_worktree_mode() {
            return CreateDialogField::Project;
        }
        CreateDialogField::first_for_tab(self.tab)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CreateDialogMode {
    NewTask,
    AddWorktree { task: Task },
}

#[derive(Debug, Clone)]
pub(super) struct CreateProjectPickerState {
    pub(super) filter: String,
    pub(super) filtered_project_indices: Vec<usize>,
    pub(super) project_list: ListState,
}

impl CreateProjectPickerState {
    pub(super) fn selected_filtered_index(&self) -> usize {
        self.project_list.selected().unwrap_or(0)
    }

    pub(super) fn set_selected_filtered_index(&mut self, index: usize) {
        self.project_list.select(Some(index));
    }
}

impl PartialEq for CreateProjectPickerState {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.filtered_project_indices == other.filtered_project_indices
            && self.selected_filtered_index() == other.selected_filtered_index()
            && self.project_list.offset == other.project_list.offset
    }
}

impl Eq for CreateProjectPickerState {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditDialogState {
    pub(super) workspace_name: String,
    pub(super) workspace_path: PathBuf,
    pub(super) is_main: bool,
    pub(super) branch: String,
    pub(super) base_branch: String,
    pub(super) was_running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RenameTabDialogState {
    pub(super) workspace_path: PathBuf,
    pub(super) tab_id: u64,
    pub(super) current_title: String,
    pub(super) title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogField {
    WorkspaceName,
    RegisterAsBase,
    PullRequestUrl,
    Project,
    CreateButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CreateDialogTab {
    Manual,
    PullRequest,
}

impl CreateDialogTab {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::PullRequest => "From GitHub PR",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Manual => Self::PullRequest,
            Self::PullRequest => Self::Manual,
        }
    }

    pub(super) fn previous(self) -> Self {
        match self {
            Self::Manual => Self::PullRequest,
            Self::PullRequest => Self::Manual,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditDialogField {
    BaseBranch,
    SaveButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RenameTabDialogField {
    Title,
    RenameButton,
    CancelButton,
}

impl CreateDialogField {
    pub(super) fn first_for_tab(tab: CreateDialogTab) -> Self {
        match tab {
            CreateDialogTab::Manual => Self::WorkspaceName,
            CreateDialogTab::PullRequest => Self::Project,
        }
    }

    #[cfg(test)]
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::WorkspaceName => "name",
            Self::RegisterAsBase => "register_as_base",
            Self::PullRequestUrl => "pr_url",
            Self::Project => "project",
            Self::CreateButton => "create",
            Self::CancelButton => "cancel",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ProjectDialogState {
    pub(super) filter_input: TextInput,
    pub(super) filtered_project_indices: Vec<usize>,
    pub(super) project_list: ListState,
    pub(super) add_dialog: Option<ProjectAddDialogState>,
    pub(super) defaults_dialog: Option<ProjectDefaultsDialogState>,
}

impl ProjectDialogState {
    pub(super) fn filter(&self) -> &str {
        self.filter_input.value()
    }

    pub(super) fn selected_filtered_index(&self) -> usize {
        self.project_list.selected().unwrap_or(0)
    }

    pub(super) fn set_selected_filtered_index(&mut self, index: usize) {
        self.project_list.select(Some(index));
    }
}

#[derive(Debug, Clone)]
pub(super) struct ProjectAddDialogState {
    pub(super) path_input: TextInput,
    pub(super) name_input: TextInput,
    pub(super) path_matches: Vec<ProjectPathMatch>,
    pub(super) path_match_list: ListState,
    pub(super) cached_search_root: Option<PathBuf>,
    pub(super) cached_repo_roots: Vec<PathBuf>,
}

impl ProjectAddDialogState {
    pub(super) fn sync_focus(&mut self, focused_field: Option<ProjectAddDialogField>) {
        self.path_input
            .set_focused(focused_field == Some(ProjectAddDialogField::Path));
        self.name_input
            .set_focused(focused_field == Some(ProjectAddDialogField::Name));
    }

    pub(super) fn selected_path_match_index(&self) -> usize {
        self.path_match_list.selected().unwrap_or(0)
    }

    pub(super) fn set_selected_path_match_index(&mut self, index: usize) {
        self.path_match_list.select(Some(index));
    }

    pub(super) fn selected_path_match(&self) -> Option<&ProjectPathMatch> {
        self.path_matches.get(self.selected_path_match_index())
    }
}

#[derive(Debug, Clone)]
pub(super) struct ProjectDefaultsDialogState {
    pub(super) project_index: usize,
    pub(super) base_branch_input: TextInput,
    pub(super) workspace_init_command_input: TextInput,
    pub(super) claude_env_input: TextInput,
    pub(super) codex_env_input: TextInput,
    pub(super) opencode_env_input: TextInput,
}

impl ProjectDefaultsDialogState {
    pub(super) fn sync_focus(&mut self, focused_field: Option<ProjectDefaultsDialogField>) {
        self.base_branch_input
            .set_focused(focused_field == Some(ProjectDefaultsDialogField::BaseBranch));
        self.workspace_init_command_input
            .set_focused(focused_field == Some(ProjectDefaultsDialogField::WorkspaceInitCommand));
        self.claude_env_input
            .set_focused(focused_field == Some(ProjectDefaultsDialogField::ClaudeEnv));
        self.codex_env_input
            .set_focused(focused_field == Some(ProjectDefaultsDialogField::CodexEnv));
        self.opencode_env_input
            .set_focused(focused_field == Some(ProjectDefaultsDialogField::OpenCodeEnv));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectPathMatch {
    pub(super) path: PathBuf,
    pub(super) score: i64,
    pub(super) already_added: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectAddDialogField {
    Path,
    Name,
    AddButton,
    CancelButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectDefaultsDialogField {
    BaseBranch,
    WorkspaceInitCommand,
    ClaudeEnv,
    CodexEnv,
    OpenCodeEnv,
    SaveButton,
    CancelButton,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SettingsDialogState {
    pub(super) initial_theme: ThemeName,
    pub(super) theme: ThemeName,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct PerformanceDialogState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsDialogField {
    Theme,
    SaveButton,
    CancelButton,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_dialog_field_next_cycles_through_all_fields() {
        let expected = [
            "agent",
            "name",
            "prompt",
            "init_command",
            "unsafe",
            "start",
            "cancel",
        ];
        let mut field = LaunchDialogField::Agent;
        for label in &expected {
            assert_eq!(field.label(), *label);
            field = field.next();
        }
        assert_eq!(
            field,
            LaunchDialogField::Agent,
            "next must wrap back to Agent"
        );
    }

    #[test]
    fn launch_dialog_field_previous_cycles_through_all_fields() {
        let expected = [
            "agent",
            "cancel",
            "start",
            "unsafe",
            "init_command",
            "prompt",
            "name",
        ];
        let mut field = LaunchDialogField::Agent;
        for label in &expected {
            assert_eq!(field.label(), *label);
            field = field.previous();
        }
        assert_eq!(
            field,
            LaunchDialogField::Agent,
            "previous must wrap back to Agent"
        );
    }
}

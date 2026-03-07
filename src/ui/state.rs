use crate::domain::{Task, Workspace, WorkspaceStatus, Worktree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneFocus {
    WorkspaceList,
    Preview,
}

impl PaneFocus {
    pub fn label(self) -> &'static str {
        match self {
            Self::WorkspaceList => "WorkspaceList",
            Self::Preview => "Preview",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::WorkspaceList => "workspace_list",
            Self::Preview => "preview",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    List,
    Preview,
}

impl UiMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::List => "List",
            Self::Preview => "Preview",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Preview => "preview",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    MoveSelectionUp,
    MoveSelectionDown,
    ToggleFocus,
    EnterPreviewMode,
    EnterListMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub tasks: Vec<Task>,
    pub workspaces: Vec<Workspace>,
    pub selected_task_index: usize,
    pub selected_worktree_index: usize,
    pub selected_index: usize,
    pub focus: PaneFocus,
    pub mode: UiMode,
}

impl AppState {
    pub fn new(tasks: Vec<Task>) -> Self {
        let workspaces = flatten_tasks_as_workspaces(tasks.as_slice());
        let mut state = Self {
            tasks,
            workspaces,
            selected_task_index: 0,
            selected_worktree_index: 0,
            selected_index: 0,
            focus: PaneFocus::WorkspaceList,
            mode: UiMode::List,
        };
        state.sync_selection_fields();
        state
    }

    pub fn from_workspaces(workspaces: Vec<Workspace>) -> Self {
        let tasks = tasks_from_workspaces(workspaces.as_slice());
        let mut state = Self {
            tasks,
            workspaces,
            selected_task_index: 0,
            selected_worktree_index: 0,
            selected_index: 0,
            focus: PaneFocus::WorkspaceList,
            mode: UiMode::List,
        };
        state.sync_selection_fields();
        state
    }

    pub fn selected_task(&self) -> Option<&Task> {
        selection_for_flat_index(self.tasks.as_slice(), self.selected_index)
            .and_then(|(task_index, _)| self.tasks.get(task_index))
    }

    pub fn selected_worktree(&self) -> Option<&Worktree> {
        selection_for_flat_index(self.tasks.as_slice(), self.selected_index).and_then(
            |(task_index, worktree_index)| {
                self.tasks
                    .get(task_index)
                    .and_then(|task| task.worktrees.get(worktree_index))
            },
        )
    }

    pub fn selected_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(self.selected_index)
    }

    #[cfg(test)]
    pub fn selected_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.selected_index)
    }

    fn sync_selection_fields(&mut self) {
        if self.workspaces.is_empty() {
            self.selected_index = 0;
            self.selected_task_index = 0;
            self.selected_worktree_index = 0;
            return;
        }

        let last = self.workspaces.len().saturating_sub(1);
        self.selected_index = self.selected_index.min(last);

        if let Some((task_index, worktree_index)) =
            selection_for_flat_index(self.tasks.as_slice(), self.selected_index)
        {
            self.selected_task_index = task_index;
            self.selected_worktree_index = worktree_index;
        }
    }
}

fn flatten_tasks_as_workspaces(tasks: &[Task]) -> Vec<Workspace> {
    tasks
        .iter()
        .flat_map(|task| {
            task.worktrees
                .iter()
                .map(|worktree| workspace_from_task_worktree(task, worktree))
        })
        .collect()
}

fn workspace_from_task_worktree(_task: &Task, worktree: &Worktree) -> Workspace {
    Workspace {
        name: worktree.repository_name.clone(),
        path: worktree.path.clone(),
        project_name: Some(worktree.repository_name.clone()),
        project_path: Some(worktree.repository_path.clone()),
        branch: worktree.branch.clone(),
        base_branch: worktree.base_branch.clone(),
        last_activity_unix_secs: worktree.last_activity_unix_secs,
        agent: worktree.agent,
        status: if worktree.status == WorkspaceStatus::Main {
            WorkspaceStatus::Idle
        } else {
            worktree.status
        },
        is_main: false,
        is_orphaned: worktree.is_orphaned,
        supported_agent: worktree.supported_agent,
        pull_requests: worktree.pull_requests.clone(),
    }
}

fn selection_for_flat_index(tasks: &[Task], selected_index: usize) -> Option<(usize, usize)> {
    let mut flat_index = 0usize;

    for (task_index, task) in tasks.iter().enumerate() {
        for (worktree_index, _) in task.worktrees.iter().enumerate() {
            if flat_index == selected_index {
                return Some((task_index, worktree_index));
            }
            flat_index = flat_index.saturating_add(1);
        }
    }

    None
}

fn tasks_from_workspaces(workspaces: &[Workspace]) -> Vec<Task> {
    workspaces
        .iter()
        .map(|workspace| {
            let repository_path = workspace
                .project_path
                .clone()
                .unwrap_or_else(|| workspace.path.clone());
            let repository_name = workspace.project_name.clone().unwrap_or_else(|| {
                repository_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map_or_else(|| workspace.name.clone(), ToString::to_string)
            });

            Task {
                name: workspace.name.clone(),
                slug: workspace.name.clone(),
                root_path: workspace.path.clone(),
                branch: workspace.branch.clone(),
                worktrees: vec![Worktree {
                    repository_name,
                    repository_path,
                    path: workspace.path.clone(),
                    branch: workspace.branch.clone(),
                    base_branch: workspace.base_branch.clone(),
                    last_activity_unix_secs: workspace.last_activity_unix_secs,
                    agent: workspace.agent,
                    status: workspace.status,
                    is_orphaned: workspace.is_orphaned,
                    supported_agent: workspace.supported_agent,
                    pull_requests: workspace.pull_requests.clone(),
                }],
            }
        })
        .collect()
}

pub fn reduce(state: &mut AppState, action: Action) {
    match action {
        Action::MoveSelectionUp => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
            state.sync_selection_fields();
        }
        Action::MoveSelectionDown => {
            let last = state.workspaces.len().saturating_sub(1);
            if state.selected_index < last {
                state.selected_index += 1;
            }
            state.sync_selection_fields();
        }
        Action::ToggleFocus => match state.focus {
            PaneFocus::WorkspaceList => {
                if state.selected_workspace().is_some() {
                    state.mode = UiMode::Preview;
                    state.focus = PaneFocus::Preview;
                }
            }
            PaneFocus::Preview => {
                state.focus = PaneFocus::WorkspaceList;
            }
        },
        Action::EnterPreviewMode => {
            if state.selected_workspace().is_some() {
                state.mode = UiMode::Preview;
                state.focus = PaneFocus::Preview;
            }
        }
        Action::EnterListMode => {
            state.mode = UiMode::List;
            state.focus = PaneFocus::WorkspaceList;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, AppState, PaneFocus, UiMode, reduce};
    use crate::domain::{AgentType, Task, WorkspaceStatus, Worktree};
    use std::path::PathBuf;

    fn fixture_task(slug: &str, repository_names: &[&str]) -> Task {
        let worktrees = repository_names
            .iter()
            .map(|repository_name| {
                Worktree::try_new(
                    (*repository_name).to_string(),
                    PathBuf::from(format!("/repos/{repository_name}")),
                    PathBuf::from(format!("/tasks/{slug}/{repository_name}")),
                    slug.to_string(),
                    AgentType::Codex,
                    WorkspaceStatus::Idle,
                )
                .expect("worktree should be valid")
            })
            .collect();
        Task::try_new(
            slug.to_string(),
            slug.to_string(),
            PathBuf::from(format!("/tasks/{slug}")),
            slug.to_string(),
            worktrees,
        )
        .expect("task should be valid")
    }

    fn fixture_state() -> AppState {
        AppState::new(vec![
            fixture_task("grove-maintenance", &["grove"]),
            fixture_task("flohome-launch", &["flohome", "terraform-fastly"]),
            fixture_task("infra-rollout", &["infra-base-services"]),
        ])
    }

    #[test]
    fn app_state_tracks_selected_task_and_selected_worktree() {
        let state = fixture_state();

        assert_eq!(state.focus, PaneFocus::WorkspaceList);
        assert_eq!(state.mode, UiMode::List);
        assert_eq!(
            state.selected_task().map(|task| task.slug.as_str()),
            Some("grove-maintenance")
        );
        assert_eq!(
            state
                .selected_worktree()
                .map(|worktree| worktree.repository_name.as_str()),
            Some("grove")
        );
    }

    #[test]
    fn reducer_moves_selection_with_bounds() {
        let mut state = fixture_state();

        reduce(&mut state, Action::MoveSelectionDown);
        assert_eq!(state.selected_index, 1);

        reduce(&mut state, Action::MoveSelectionDown);
        reduce(&mut state, Action::MoveSelectionDown);
        assert_eq!(state.selected_index, 2);

        reduce(&mut state, Action::MoveSelectionUp);
        reduce(&mut state, Action::MoveSelectionUp);
        reduce(&mut state, Action::MoveSelectionUp);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn reducer_toggles_focus_and_switches_modes() {
        let mut state = fixture_state();

        reduce(&mut state, Action::ToggleFocus);
        assert_eq!(state.focus, PaneFocus::Preview);
        assert_eq!(state.mode, UiMode::Preview);

        reduce(&mut state, Action::EnterPreviewMode);
        assert_eq!(state.mode, UiMode::Preview);
        assert_eq!(state.focus, PaneFocus::Preview);

        reduce(&mut state, Action::EnterListMode);
        assert_eq!(state.mode, UiMode::List);
        assert_eq!(state.focus, PaneFocus::WorkspaceList);
    }
}

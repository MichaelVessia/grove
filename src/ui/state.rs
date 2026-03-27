use crate::domain::{Task, Workspace, WorkspaceStatus, Worktree};
use crate::infrastructure::paths::refer_to_same_location;
use std::path::Path;

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
    #[cfg(test)]
    EnterPreviewMode,
    #[cfg(test)]
    EnterListMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub tasks: Vec<Task>,
    pub workspaces: Vec<Workspace>,
    pub selected_task_index: usize,
    pub selected_worktree_index: usize,
    pub selected_index: usize,
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

    pub fn select_index(&mut self, index: usize) -> bool {
        if self.workspaces.is_empty() {
            self.selected_index = 0;
            self.selected_task_index = 0;
            self.selected_worktree_index = 0;
            return false;
        }

        let bounded = index.min(self.workspaces.len().saturating_sub(1));
        let changed = bounded != self.selected_index;
        self.selected_index = bounded;
        self.sync_selection_fields();
        changed
    }

    pub fn select_workspace_path(&mut self, workspace_path: impl AsRef<Path>) -> bool {
        let Some(index) = self.workspaces.iter().position(|workspace| {
            refer_to_same_location(workspace.path.as_path(), workspace_path.as_ref())
        }) else {
            return false;
        };

        self.select_index(index);
        true
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

fn workspace_from_task_worktree(task: &Task, worktree: &Worktree) -> Workspace {
    let is_main = worktree.is_main_checkout();
    Workspace {
        name: if task.worktrees.len() == 1 {
            task.name.clone()
        } else {
            worktree.repository_name.clone()
        },
        task_slug: Some(task.slug.clone()),
        path: worktree.path.clone(),
        project_name: Some(worktree.repository_name.clone()),
        project_path: Some(worktree.repository_path.clone()),
        branch: worktree.branch.clone(),
        base_branch: worktree.base_branch.clone(),
        last_activity_unix_secs: worktree.last_activity_unix_secs,
        agent: worktree.agent,
        status: if is_main {
            WorkspaceStatus::Main
        } else {
            worktree.status
        },
        is_main,
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

#[cfg(test)]
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
        #[cfg(test)]
        Action::EnterPreviewMode => {
            if state.selected_workspace().is_some() {
                state.mode = UiMode::Preview;
            }
        }
        #[cfg(test)]
        Action::EnterListMode => {
            state.mode = UiMode::List;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, AppState, UiMode, reduce};
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
        assert_eq!(state.selected_task_index, 1);
        assert_eq!(state.selected_worktree_index, 0);

        reduce(&mut state, Action::MoveSelectionDown);
        reduce(&mut state, Action::MoveSelectionDown);
        assert_eq!(state.selected_index, 3);
        assert_eq!(state.selected_task_index, 2);
        assert_eq!(state.selected_worktree_index, 0);

        reduce(&mut state, Action::MoveSelectionUp);
        reduce(&mut state, Action::MoveSelectionUp);
        reduce(&mut state, Action::MoveSelectionUp);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.selected_task_index, 0);
        assert_eq!(state.selected_worktree_index, 0);
    }

    #[test]
    fn select_index_syncs_task_and_worktree_indices() {
        let mut state = fixture_state();

        state.select_index(2);

        assert_eq!(state.selected_index, 2);
        assert_eq!(state.selected_task_index, 1);
        assert_eq!(state.selected_worktree_index, 1);
        assert_eq!(
            state.selected_task().map(|task| task.slug.as_str()),
            Some("flohome-launch")
        );
        assert_eq!(
            state
                .selected_worktree()
                .map(|worktree| worktree.repository_name.as_str()),
            Some("terraform-fastly")
        );
    }

    #[test]
    fn select_workspace_path_restores_selection_fields() {
        let mut state = fixture_state();

        let selected =
            state.select_workspace_path(PathBuf::from("/tasks/flohome-launch/terraform-fastly"));

        assert!(selected);
        assert_eq!(state.selected_index, 2);
        assert_eq!(state.selected_task_index, 1);
        assert_eq!(state.selected_worktree_index, 1);
    }

    #[test]
    fn reducer_switches_modes() {
        let mut state = fixture_state();

        reduce(&mut state, Action::EnterPreviewMode);
        assert_eq!(state.mode, UiMode::Preview);

        reduce(&mut state, Action::EnterListMode);
        assert_eq!(state.mode, UiMode::List);
    }
}

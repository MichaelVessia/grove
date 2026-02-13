use crate::domain::Workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneFocus {
    WorkspaceList,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    List,
    Preview,
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
    pub workspaces: Vec<Workspace>,
    pub selected_index: usize,
    pub focus: PaneFocus,
    pub mode: UiMode,
}

impl AppState {
    pub fn new(workspaces: Vec<Workspace>) -> Self {
        Self {
            workspaces,
            selected_index: 0,
            focus: PaneFocus::WorkspaceList,
            mode: UiMode::List,
        }
    }

    pub fn selected_workspace(&self) -> Option<&Workspace> {
        self.workspaces.get(self.selected_index)
    }

    pub fn selected_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.workspaces.get_mut(self.selected_index)
    }
}

pub fn reduce(state: &mut AppState, action: Action) {
    match action {
        Action::MoveSelectionUp => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }
        Action::MoveSelectionDown => {
            let last = state.workspaces.len().saturating_sub(1);
            if state.selected_index < last {
                state.selected_index += 1;
            }
        }
        Action::ToggleFocus => {
            state.focus = match state.focus {
                PaneFocus::WorkspaceList => PaneFocus::Preview,
                PaneFocus::Preview => PaneFocus::WorkspaceList,
            };
        }
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
    use crate::domain::{AgentType, Workspace, WorkspaceStatus};
    use std::path::PathBuf;

    fn fixture_state() -> AppState {
        AppState::new(vec![
            Workspace::try_new(
                "grove".to_string(),
                PathBuf::from("/repos/grove"),
                "main".to_string(),
                Some(1_700_000_300),
                AgentType::Claude,
                WorkspaceStatus::Main,
                true,
            )
            .expect("main workspace should be valid"),
            Workspace::try_new(
                "feature-a".to_string(),
                PathBuf::from("/repos/grove-feature-a"),
                "feature-a".to_string(),
                Some(1_700_000_200),
                AgentType::Codex,
                WorkspaceStatus::Idle,
                false,
            )
            .expect("workspace should be valid"),
            Workspace::try_new(
                "feature-b".to_string(),
                PathBuf::from("/repos/grove-feature-b"),
                "feature-b".to_string(),
                Some(1_700_000_100),
                AgentType::Claude,
                WorkspaceStatus::Unknown,
                false,
            )
            .expect("workspace should be valid"),
        ])
    }

    #[test]
    fn default_state_selects_first_workspace_and_list_mode() {
        let state = fixture_state();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.focus, PaneFocus::WorkspaceList);
        assert_eq!(state.mode, UiMode::List);
        assert_eq!(
            state
                .selected_workspace()
                .map(|workspace| workspace.name.clone()),
            Some("grove".to_string())
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
        assert_eq!(state.mode, UiMode::List);

        reduce(&mut state, Action::EnterPreviewMode);
        assert_eq!(state.mode, UiMode::Preview);
        assert_eq!(state.focus, PaneFocus::Preview);

        reduce(&mut state, Action::EnterListMode);
        assert_eq!(state.mode, UiMode::List);
        assert_eq!(state.focus, PaneFocus::WorkspaceList);
    }
}

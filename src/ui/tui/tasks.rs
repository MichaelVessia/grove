use std::collections::HashSet;
use std::path::Path;

use super::*;
use crate::ui::state::AppState;

impl GroveApp {
    pub(super) fn reconcile_task_order(&mut self) {
        let valid_slugs = self
            .state
            .tasks
            .iter()
            .map(|task| task.slug.clone())
            .collect::<HashSet<String>>();
        let mut ordered = self
            .task_order
            .iter()
            .filter(|slug| valid_slugs.contains(slug.as_str()))
            .fold(Vec::<String>::new(), |mut acc, slug| {
                if !acc.iter().any(|entry| entry == slug) {
                    acc.push(slug.clone());
                }
                acc
            });
        for task in &self.state.tasks {
            if !ordered.iter().any(|slug| slug == &task.slug) {
                ordered.push(task.slug.clone());
            }
        }
        self.task_order = ordered;
    }

    fn task_order_index(&self, task_slug: &str) -> usize {
        self.task_order
            .iter()
            .position(|slug| slug == task_slug)
            .unwrap_or(usize::MAX)
    }

    fn ordered_tasks(&self) -> Vec<Task> {
        let mut tasks = self.state.tasks.clone();
        tasks.sort_by(|left, right| {
            self.task_order_index(left.slug.as_str())
                .cmp(&self.task_order_index(right.slug.as_str()))
                .then_with(|| left.slug.cmp(&right.slug))
        });
        tasks
    }

    pub(super) fn reorder_tasks_for_task_order(&mut self) {
        if self.state.tasks.is_empty() {
            return;
        }

        let selected_workspace_path = self.selected_workspace_path();
        let previous_mode = self.state.mode;
        let previous_selected_index = self.state.selected_index;
        let mut state = AppState::new(self.ordered_tasks());

        if let Some(path) = selected_workspace_path.as_deref() {
            restore_state_selection_for_workspace_path(&mut state, path, previous_selected_index);
        } else {
            restore_state_selection_for_flat_index(&mut state, previous_selected_index);
        }

        state.mode = previous_mode;
        self.state = state;
    }

    fn selected_task_slug(&self) -> Option<String> {
        self.state.selected_task().map(|task| task.slug.clone())
    }

    pub(super) fn task_reorder_active(&self) -> bool {
        self.task_reorder.is_some()
    }

    pub(super) fn open_task_reorder_mode(&mut self) {
        if self.modal_open() || self.task_reorder_active() {
            return;
        }
        let Some(task_slug) = self.selected_task_slug() else {
            self.show_info_toast("selected workspace is not part of a task");
            return;
        };
        self.reconcile_task_order();
        self.task_reorder = Some(TaskReorderState {
            original_task_order: self.task_order.clone(),
            moving_task_slug: task_slug,
        });
        self.show_info_toast("task reorder mode, j/k or Up/Down move, Enter save, Esc cancel");
    }

    pub(super) fn move_selected_task_in_reorder(&mut self, direction: i8) {
        let Some(reorder) = self.task_reorder.as_ref() else {
            return;
        };
        let Some(current_index) = self
            .task_order
            .iter()
            .position(|slug| slug == &reorder.moving_task_slug)
        else {
            return;
        };
        let next_index = if direction.is_negative() {
            current_index.saturating_sub(1)
        } else {
            current_index
                .saturating_add(1)
                .min(self.task_order.len().saturating_sub(1))
        };
        if next_index == current_index {
            return;
        }

        self.task_order.swap(current_index, next_index);
        self.reorder_tasks_for_task_order();
    }

    pub(super) fn save_task_reorder(&mut self) {
        if !self.task_reorder_active() {
            return;
        }
        self.reconcile_task_order();
        if let Err(error) = self.save_projects_config() {
            self.show_error_toast(format!("task order save failed: {error}"));
            return;
        }
        self.task_reorder = None;
        self.show_success_toast("task order saved");
    }

    pub(super) fn cancel_task_reorder(&mut self) {
        let Some(reorder) = self.task_reorder.take() else {
            return;
        };
        self.task_order = reorder.original_task_order;
        self.reconcile_task_order();
        self.reorder_tasks_for_task_order();
        self.show_info_toast("task reorder cancelled");
    }

    pub(super) fn task_header_marker(&self, task: &Task) -> &'static str {
        if self
            .task_reorder
            .as_ref()
            .is_some_and(|reorder| reorder.moving_task_slug == task.slug)
        {
            "↕"
        } else {
            "▾"
        }
    }

    #[cfg(test)]
    pub(super) fn set_tasks_for_test(&mut self, tasks: Vec<Task>) {
        self.state = AppState::new(tasks);
        self.reconcile_task_order();
        self.reorder_tasks_for_task_order();
    }

    #[cfg(test)]
    pub(super) fn set_task_order_for_test(&mut self, task_order: Vec<String>) {
        self.task_order = task_order;
        self.reconcile_task_order();
        self.reorder_tasks_for_task_order();
    }
}

fn restore_state_selection_for_workspace_path(
    state: &mut AppState,
    workspace_path: &Path,
    fallback_flat_index: usize,
) {
    let Some((_, _, flat_index)) =
        selection_for_workspace_path(state.tasks.as_slice(), workspace_path)
    else {
        restore_state_selection_for_flat_index(state, fallback_flat_index);
        return;
    };

    state.select_index(flat_index);
}

fn restore_state_selection_for_flat_index(state: &mut AppState, flat_index: usize) {
    state.select_index(flat_index);
}

fn selection_for_workspace_path(
    tasks: &[Task],
    workspace_path: &Path,
) -> Option<(usize, usize, usize)> {
    let mut flat_index = 0usize;

    for (task_index, task) in tasks.iter().enumerate() {
        for (worktree_index, worktree) in task.worktrees.iter().enumerate() {
            if refer_to_same_location(worktree.path.as_path(), workspace_path) {
                return Some((task_index, worktree_index, flat_index));
            }
            flat_index = flat_index.saturating_add(1);
        }
    }

    None
}

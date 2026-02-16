use super::*;

impl GroveApp {
    fn filtered_project_indices(&self, query: &str) -> Vec<usize> {
        if query.trim().is_empty() {
            return (0..self.projects.len()).collect();
        }

        let query_lower = query.to_ascii_lowercase();
        self.projects
            .iter()
            .enumerate()
            .filter(|(_, project)| {
                project.name.to_ascii_lowercase().contains(&query_lower)
                    || project
                        .path
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains(&query_lower)
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub(super) fn refresh_project_dialog_filtered(&mut self) {
        let query = match self.project_dialog.as_ref() {
            Some(dialog) => dialog.filter.clone(),
            None => return,
        };
        let filtered = self.filtered_project_indices(&query);
        let Some(dialog) = self.project_dialog.as_mut() else {
            return;
        };

        dialog.filtered_project_indices = filtered;
        if dialog.filtered_project_indices.is_empty() {
            dialog.selected_filtered_index = 0;
            return;
        }
        if dialog.selected_filtered_index >= dialog.filtered_project_indices.len() {
            dialog.selected_filtered_index =
                dialog.filtered_project_indices.len().saturating_sub(1);
        }
    }

    pub(super) fn selected_project_dialog_project_index(&self) -> Option<usize> {
        let dialog = self.project_dialog.as_ref()?;
        if dialog.filtered_project_indices.is_empty() {
            return None;
        }
        dialog
            .filtered_project_indices
            .get(dialog.selected_filtered_index)
            .copied()
    }

    pub(super) fn focus_project_by_index(&mut self, project_index: usize) {
        let Some(project) = self.projects.get(project_index) else {
            return;
        };

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace.is_main
                        && workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| project_paths_equal(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
            return;
        }

        if let Some((workspace_index, _)) =
            self.state
                .workspaces
                .iter()
                .enumerate()
                .find(|(_, workspace)| {
                    workspace
                        .project_path
                        .as_ref()
                        .is_some_and(|path| project_paths_equal(path, &project.path))
                })
        {
            self.select_workspace_by_index(workspace_index);
        }
    }

    pub(super) fn open_project_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let selected_project_index = self.selected_project_index();
        let filtered_project_indices: Vec<usize> = (0..self.projects.len()).collect();
        let selected_filtered_index = filtered_project_indices
            .iter()
            .position(|index| *index == selected_project_index)
            .unwrap_or(0);
        self.project_dialog = Some(ProjectDialogState {
            filter: String::new(),
            filtered_project_indices,
            selected_filtered_index,
            add_dialog: None,
        });
    }

    pub(super) fn open_project_add_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog.as_mut() else {
            return;
        };
        project_dialog.add_dialog = Some(ProjectAddDialogState {
            name: String::new(),
            path: String::new(),
            focused_field: ProjectAddDialogField::Name,
        });
    }

    fn normalized_project_path(raw: &str) -> PathBuf {
        if let Some(stripped) = raw.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(stripped);
        }
        PathBuf::from(raw)
    }

    fn save_projects_config(&self) -> Result<(), String> {
        let config = GroveConfig {
            multiplexer: self.multiplexer,
            projects: self.projects.clone(),
        };
        crate::infrastructure::config::save_to_path(&self.config_path, &config)
    }

    pub(super) fn add_project_from_dialog(&mut self) {
        let Some(project_dialog) = self.project_dialog.as_ref() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_ref() else {
            return;
        };

        let path_input = add_dialog.path.trim();
        if path_input.is_empty() {
            self.show_toast("project path is required", true);
            return;
        }
        let normalized = Self::normalized_project_path(path_input);
        let canonical = match normalized.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                self.show_toast(format!("invalid project path: {error}"), true);
                return;
            }
        };

        let repo_root_output = Command::new("git")
            .current_dir(&canonical)
            .args(["rev-parse", "--show-toplevel"])
            .output();
        let repo_root = match repo_root_output {
            Ok(output) if output.status.success() => {
                let raw = String::from_utf8(output.stdout).unwrap_or_default();
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    canonical.clone()
                } else {
                    PathBuf::from(trimmed)
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                self.show_toast(format!("not a git repository: {stderr}"), true);
                return;
            }
            Err(error) => {
                self.show_toast(format!("git check failed: {error}"), true);
                return;
            }
        };
        let repo_root = repo_root.canonicalize().unwrap_or(repo_root);

        if self
            .projects
            .iter()
            .any(|project| project_paths_equal(&project.path, &repo_root))
        {
            self.show_toast("project already exists", true);
            return;
        }

        let project_name = if add_dialog.name.trim().is_empty() {
            project_display_name(&repo_root)
        } else {
            add_dialog.name.trim().to_string()
        };
        self.projects.push(ProjectConfig {
            name: project_name.clone(),
            path: repo_root.clone(),
        });
        if let Err(error) = self.save_projects_config() {
            self.show_toast(format!("project save failed: {error}"), true);
            return;
        }

        if let Some(dialog) = self.project_dialog.as_mut() {
            dialog.add_dialog = None;
        }
        self.refresh_project_dialog_filtered();
        self.refresh_workspaces(None);
        self.show_toast(format!("project '{}' added", project_name), false);
    }

    pub(super) fn handle_project_add_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(project_dialog) = self.project_dialog.as_mut() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_mut() else {
            return;
        };

        match key_event.code {
            KeyCode::Escape => {
                project_dialog.add_dialog = None;
            }
            KeyCode::Tab => {
                add_dialog.focused_field = add_dialog.focused_field.next();
            }
            KeyCode::BackTab => {
                add_dialog.focused_field = add_dialog.focused_field.previous();
            }
            KeyCode::Enter => match add_dialog.focused_field {
                ProjectAddDialogField::AddButton => self.add_project_from_dialog(),
                ProjectAddDialogField::CancelButton => project_dialog.add_dialog = None,
                ProjectAddDialogField::Name | ProjectAddDialogField::Path => {
                    add_dialog.focused_field = add_dialog.focused_field.next();
                }
            },
            KeyCode::Backspace => match add_dialog.focused_field {
                ProjectAddDialogField::Name => {
                    add_dialog.name.pop();
                }
                ProjectAddDialogField::Path => {
                    add_dialog.path.pop();
                }
                ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
            },
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                match add_dialog.focused_field {
                    ProjectAddDialogField::Name => add_dialog.name.push(character),
                    ProjectAddDialogField::Path => add_dialog.path.push(character),
                    ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_project_dialog_key(&mut self, key_event: KeyEvent) {
        if self
            .project_dialog
            .as_ref()
            .and_then(|dialog| dialog.add_dialog.as_ref())
            .is_some()
        {
            self.handle_project_add_dialog_key(key_event);
            return;
        }

        match key_event.code {
            KeyCode::Escape => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && !dialog.filter.is_empty()
                {
                    dialog.filter.clear();
                    self.refresh_project_dialog_filtered();
                    return;
                }
                self.project_dialog = None;
            }
            KeyCode::Enter => {
                if let Some(project_index) = self.selected_project_dialog_project_index() {
                    self.focus_project_by_index(project_index);
                    self.project_dialog = None;
                }
            }
            KeyCode::Up => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index > 0
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index.saturating_add(1)
                        < dialog.filtered_project_indices.len()
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_add(1);
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        dialog.selected_filtered_index =
                            dialog.selected_filtered_index.saturating_add(1) % len;
                    }
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        dialog.selected_filtered_index = if dialog.selected_filtered_index == 0 {
                            len.saturating_sub(1)
                        } else {
                            dialog.selected_filtered_index.saturating_sub(1)
                        };
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.filter.pop();
                }
                self.refresh_project_dialog_filtered();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'a' || character == 'A') =>
            {
                self.open_project_add_dialog();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'n' || character == 'N') =>
            {
                if let Some(dialog) = self.project_dialog.as_mut()
                    && dialog.selected_filtered_index.saturating_add(1)
                        < dialog.filtered_project_indices.len()
                {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_add(1);
                }
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'p' || character == 'P') =>
            {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.selected_filtered_index =
                        dialog.selected_filtered_index.saturating_sub(1);
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if let Some(dialog) = self.project_dialog.as_mut() {
                    dialog.filter.push(character);
                }
                self.refresh_project_dialog_filtered();
            }
            _ => {}
        }
    }

    pub(super) fn handle_settings_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.settings_dialog.as_mut() else {
            return;
        };

        enum PostAction {
            None,
            Save,
            Cancel,
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.previous();
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
            }
            KeyCode::Char(' ') => {
                if dialog.focused_field == SettingsDialogField::Multiplexer {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                SettingsDialogField::Multiplexer => {
                    dialog.multiplexer = dialog.multiplexer.next();
                }
                SettingsDialogField::SaveButton => post_action = PostAction::Save,
                SettingsDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.apply_settings_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("settings", "dialog_cancelled");
                self.settings_dialog = None;
            }
        }
    }

    pub(super) fn open_settings_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        self.settings_dialog = Some(SettingsDialogState {
            multiplexer: self.multiplexer,
            focused_field: SettingsDialogField::Multiplexer,
        });
    }

    fn has_running_workspace_sessions(&self) -> bool {
        self.state
            .workspaces
            .iter()
            .any(|workspace| workspace.status.has_session())
    }

    pub(super) fn apply_settings_dialog_save(&mut self) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };

        if dialog.multiplexer != self.multiplexer && self.has_running_workspace_sessions() {
            self.show_toast(
                "restart running workspaces before switching multiplexer",
                true,
            );
            return;
        }

        let selected = dialog.multiplexer;
        self.multiplexer = selected;
        self.tmux_input = input_for_multiplexer(selected);
        let config = GroveConfig {
            multiplexer: selected,
            projects: self.projects.clone(),
        };
        if let Err(error) = crate::infrastructure::config::save_to_path(&self.config_path, &config)
        {
            self.show_toast(format!("settings save failed: {error}"), true);
            return;
        }

        self.settings_dialog = None;
        self.interactive = None;
        self.lazygit_ready_sessions.clear();
        self.lazygit_failed_sessions.clear();
        self.refresh_workspaces(None);
        self.poll_preview();
        self.show_toast(format!("multiplexer set to {}", selected.label()), false);
    }
}

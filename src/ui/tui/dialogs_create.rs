use super::*;

impl GroveApp {
    pub(super) fn handle_create_dialog_key(&mut self, key_event: KeyEvent) {
        if self.create_in_flight {
            return;
        }

        let ctrl_n = key_event.code == KeyCode::Char('n') && key_event.modifiers == Modifiers::CTRL;
        let ctrl_p = key_event.code == KeyCode::Char('p') && key_event.modifiers == Modifiers::CTRL;

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("create", "dialog_cancelled");
                self.create_dialog = None;
                self.clear_create_branch_picker();
            }
            KeyCode::Enter => {
                if self.select_create_base_branch_from_dropdown() {
                    if let Some(dialog) = self.create_dialog.as_mut() {
                        dialog.focused_field = dialog.focused_field.next();
                    }
                    self.refresh_create_branch_filtered();
                    return;
                }

                enum EnterAction {
                    ConfirmCreate,
                    CancelDialog,
                    AdvanceField,
                }

                let action = self
                    .create_dialog
                    .as_ref()
                    .map(|dialog| match dialog.focused_field {
                        CreateDialogField::CreateButton => EnterAction::ConfirmCreate,
                        CreateDialogField::CancelButton => EnterAction::CancelDialog,
                        CreateDialogField::WorkspaceName
                        | CreateDialogField::Project
                        | CreateDialogField::BaseBranch
                        | CreateDialogField::Agent => EnterAction::AdvanceField,
                    });

                match action {
                    Some(EnterAction::ConfirmCreate) => self.confirm_create_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("create", "dialog_cancelled");
                        self.create_dialog = None;
                        self.clear_create_branch_picker();
                    }
                    Some(EnterAction::AdvanceField) => {
                        if let Some(dialog) = self.create_dialog.as_mut() {
                            dialog.focused_field = dialog.focused_field.next();
                        }
                    }
                    None => {}
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.next();
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.create_dialog.as_mut() {
                    dialog.focused_field = dialog.focused_field.previous();
                }
            }
            KeyCode::Left | KeyCode::Right => {}
            KeyCode::Up => {
                if self.create_base_branch_dropdown_visible() && self.create_branch_index > 0 {
                    self.create_branch_index = self.create_branch_index.saturating_sub(1);
                    return;
                }
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(-1);
                    return;
                }
                if let Some(dialog) = self.create_dialog.as_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Down => {
                if self.create_base_branch_dropdown_visible()
                    && self.create_branch_index.saturating_add(1)
                        < self.create_branch_filtered.len()
                {
                    self.create_branch_index = self.create_branch_index.saturating_add(1);
                    return;
                }
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(1);
                    return;
                }
                if let Some(dialog) = self.create_dialog.as_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Char(_) if ctrl_n || ctrl_p => {
                let focused_field = self
                    .create_dialog
                    .as_ref()
                    .map(|dialog| dialog.focused_field);
                if focused_field == Some(CreateDialogField::BaseBranch)
                    && !self.create_branch_filtered.is_empty()
                {
                    if ctrl_n
                        && self.create_branch_index.saturating_add(1)
                            < self.create_branch_filtered.len()
                    {
                        self.create_branch_index = self.create_branch_index.saturating_add(1);
                    }
                    if ctrl_p && self.create_branch_index > 0 {
                        self.create_branch_index = self.create_branch_index.saturating_sub(1);
                    }
                } else if focused_field == Some(CreateDialogField::Project) {
                    if ctrl_n {
                        self.shift_create_dialog_project(1);
                    }
                    if ctrl_p {
                        self.shift_create_dialog_project(-1);
                    }
                } else if focused_field == Some(CreateDialogField::Agent)
                    && let Some(dialog) = self.create_dialog.as_mut()
                {
                    Self::toggle_create_dialog_agent(dialog);
                }
            }
            KeyCode::Backspace => {
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog.as_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            dialog.workspace_name.pop();
                        }
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.pop();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::Project
                        | CreateDialogField::Agent
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                }
                if refresh_base_branch {
                    self.refresh_create_branch_filtered();
                }
            }
            KeyCode::Char(character) if key_event.modifiers.is_empty() => {
                if self
                    .create_dialog
                    .as_ref()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    if character == 'j' {
                        self.shift_create_dialog_project(1);
                        return;
                    }
                    if character == 'k' {
                        self.shift_create_dialog_project(-1);
                        return;
                    }
                }
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog.as_mut() {
                    if dialog.focused_field == CreateDialogField::Agent
                        && (character == 'j' || character == 'k' || character == ' ')
                    {
                        Self::toggle_create_dialog_agent(dialog);
                        return;
                    }
                    if (dialog.focused_field == CreateDialogField::CreateButton
                        || dialog.focused_field == CreateDialogField::CancelButton)
                        && (character == 'h' || character == 'l')
                    {
                        dialog.focused_field =
                            if dialog.focused_field == CreateDialogField::CreateButton {
                                CreateDialogField::CancelButton
                            } else {
                                CreateDialogField::CreateButton
                            };
                        return;
                    }
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            if character.is_ascii_alphanumeric()
                                || character == '-'
                                || character == '_'
                            {
                                dialog.workspace_name.push(character);
                            }
                        }
                        CreateDialogField::Project => {}
                        CreateDialogField::BaseBranch => {
                            if character == 'j'
                                && self.create_branch_index.saturating_add(1)
                                    < self.create_branch_filtered.len()
                            {
                                self.create_branch_index =
                                    self.create_branch_index.saturating_add(1);
                                return;
                            }
                            if character == 'k' && self.create_branch_index > 0 {
                                self.create_branch_index =
                                    self.create_branch_index.saturating_sub(1);
                                return;
                            }
                            if !character.is_control() {
                                dialog.base_branch.push(character);
                                refresh_base_branch = true;
                            }
                        }
                        CreateDialogField::Agent => {}
                        CreateDialogField::CreateButton | CreateDialogField::CancelButton => {}
                    }
                }
                if refresh_base_branch {
                    self.refresh_create_branch_filtered();
                }
            }
            _ => {}
        }
    }
    fn selected_base_branch(&self) -> String {
        let selected = self.state.selected_workspace();
        if let Some(workspace) = selected
            && let Some(base_branch) = workspace.base_branch.as_ref()
            && !base_branch.trim().is_empty()
        {
            return base_branch.clone();
        }

        if let Some(workspace) = selected
            && !workspace.branch.trim().is_empty()
            && workspace.branch != "(detached)"
        {
            return workspace.branch.clone();
        }

        "main".to_string()
    }

    pub(super) fn selected_project_index(&self) -> usize {
        let Some(workspace) = self.state.selected_workspace() else {
            return 0;
        };
        let Some(workspace_project_path) = workspace.project_path.as_ref() else {
            return 0;
        };
        self.projects
            .iter()
            .position(|project| project_paths_equal(&project.path, workspace_project_path))
            .unwrap_or(0)
    }

    fn create_dialog_selected_project(&self) -> Option<&ProjectConfig> {
        let dialog = self.create_dialog.as_ref()?;
        self.projects.get(dialog.project_index)
    }

    fn refresh_create_dialog_branch_candidates(&mut self, selected_base_branch: String) {
        let branches = self
            .create_dialog_selected_project()
            .map(|project| load_local_branches(&project.path).unwrap_or_default())
            .unwrap_or_default();
        self.create_branch_all = branches;
        if !self
            .create_branch_all
            .iter()
            .any(|branch| branch == &selected_base_branch)
        {
            self.create_branch_all.insert(0, selected_base_branch);
        }
        self.refresh_create_branch_filtered();
    }

    pub(super) fn open_create_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        if self.projects.is_empty() {
            self.show_toast("no projects configured, press p to add one", true);
            return;
        }

        let selected_base_branch = self.selected_base_branch();
        let default_agent = self
            .state
            .selected_workspace()
            .map_or(AgentType::Claude, |workspace| workspace.agent);
        let project_index = self.selected_project_index();
        self.create_dialog = Some(CreateDialogState {
            workspace_name: String::new(),
            project_index,
            agent: default_agent,
            base_branch: selected_base_branch.clone(),
            focused_field: CreateDialogField::WorkspaceName,
        });
        self.refresh_create_dialog_branch_candidates(selected_base_branch);
        self.log_dialog_event_with_fields(
            "create",
            "dialog_opened",
            [("agent".to_string(), Value::from(default_agent.label()))],
        );
        self.state.mode = UiMode::List;
        self.state.focus = PaneFocus::WorkspaceList;
        self.last_tmux_error = None;
    }

    pub(super) fn toggle_agent(agent: AgentType) -> AgentType {
        match agent {
            AgentType::Claude => AgentType::Codex,
            AgentType::Codex => AgentType::Claude,
        }
    }

    fn toggle_create_dialog_agent(dialog: &mut CreateDialogState) {
        dialog.agent = Self::toggle_agent(dialog.agent);
    }
    fn shift_create_dialog_project(&mut self, delta: isize) {
        let Some(dialog) = self.create_dialog.as_mut() else {
            return;
        };
        if self.projects.is_empty() {
            return;
        }

        let len = self.projects.len();
        let current = dialog.project_index.min(len.saturating_sub(1));
        let mut next = current;
        if delta < 0 {
            next = current.saturating_sub(1);
        } else if delta > 0 {
            next = (current.saturating_add(1)).min(len.saturating_sub(1));
        }

        if next == dialog.project_index {
            return;
        }

        dialog.project_index = next;
        let selected_base_branch = dialog.base_branch.clone();
        self.refresh_create_dialog_branch_candidates(selected_base_branch);
    }

    pub(super) fn clear_create_branch_picker(&mut self) {
        self.create_branch_all.clear();
        self.create_branch_filtered.clear();
        self.create_branch_index = 0;
    }

    pub(super) fn refresh_create_branch_filtered(&mut self) {
        let query = self
            .create_dialog
            .as_ref()
            .map(|dialog| dialog.base_branch.clone())
            .unwrap_or_default();
        self.create_branch_filtered = filter_branches(&query, &self.create_branch_all);
        if self.create_branch_filtered.is_empty() {
            self.create_branch_index = 0;
            return;
        }
        if self.create_branch_index >= self.create_branch_filtered.len() {
            self.create_branch_index = self.create_branch_filtered.len().saturating_sub(1);
        }
    }

    fn create_base_branch_dropdown_visible(&self) -> bool {
        self.create_dialog.as_ref().is_some_and(|dialog| {
            dialog.focused_field == CreateDialogField::BaseBranch
                && !self.create_branch_filtered.is_empty()
        })
    }

    fn select_create_base_branch_from_dropdown(&mut self) -> bool {
        if !self.create_base_branch_dropdown_visible() {
            return false;
        }
        let Some(selected_branch) = self
            .create_branch_filtered
            .get(self.create_branch_index)
            .cloned()
        else {
            return false;
        };
        if let Some(dialog) = self.create_dialog.as_mut() {
            dialog.base_branch = selected_branch;
            return true;
        }
        false
    }
}

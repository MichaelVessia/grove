use super::*;

impl GroveApp {
    pub(super) fn handle_create_dialog_key(&mut self, key_event: KeyEvent) {
        if self.create_in_flight {
            return;
        }

        let alt_previous_tab =
            key_event.code == KeyCode::Char('[') && key_event.modifiers == Modifiers::ALT;
        let alt_next_tab =
            key_event.code == KeyCode::Char(']') && key_event.modifiers == Modifiers::ALT;
        if alt_previous_tab {
            self.switch_create_dialog_tab(false);
            return;
        }
        if alt_next_tab {
            self.switch_create_dialog_tab(true);
            return;
        }

        let ctrl_n = key_event.code == KeyCode::Char('n') && key_event.modifiers == Modifiers::CTRL;
        let ctrl_p = key_event.code == KeyCode::Char('p') && key_event.modifiers == Modifiers::CTRL;

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("create", "dialog_cancelled");
                self.close_active_dialog();
                self.clear_create_branch_picker();
            }
            KeyCode::Enter => {
                if self.select_create_base_branch_from_dropdown() {
                    self.create_dialog_focus_next();
                    self.refresh_create_branch_filtered();
                    return;
                }

                enum EnterAction {
                    ConfirmCreate,
                    CancelDialog,
                    AdvanceField,
                }

                let action = self
                    .create_dialog()
                    .map(|dialog| match dialog.focused_field {
                        CreateDialogField::CreateButton => EnterAction::ConfirmCreate,
                        CreateDialogField::CancelButton => EnterAction::CancelDialog,
                        CreateDialogField::WorkspaceName
                        | CreateDialogField::PullRequestUrl
                        | CreateDialogField::Project
                        | CreateDialogField::BaseBranch
                        | CreateDialogField::Agent
                        | CreateDialogField::StartConfig(_) => EnterAction::AdvanceField,
                    });

                match action {
                    Some(EnterAction::ConfirmCreate) => self.confirm_create_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("create", "dialog_cancelled");
                        self.close_active_dialog();
                        self.clear_create_branch_picker();
                    }
                    Some(EnterAction::AdvanceField) => {
                        self.create_dialog_focus_next();
                    }
                    None => {}
                }
            }
            KeyCode::Tab => {
                self.create_dialog_focus_next();
            }
            KeyCode::BackTab => {
                self.create_dialog_focus_previous();
            }
            KeyCode::Left | KeyCode::Right => {}
            KeyCode::Up => {
                if self.create_base_branch_dropdown_visible() && self.create_branch_index > 0 {
                    self.create_branch_index = self.create_branch_index.saturating_sub(1);
                    return;
                }
                if self
                    .create_dialog()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(-1);
                    return;
                }
                if let Some(dialog) = self.create_dialog_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::select_previous_create_dialog_agent(dialog);
                }
                if let Some(dialog) = self.create_dialog_mut()
                    && dialog.focused_field
                        == CreateDialogField::StartConfig(StartAgentConfigField::Unsafe)
                {
                    dialog.start_config.toggle_unsafe();
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
                    .create_dialog()
                    .is_some_and(|dialog| dialog.focused_field == CreateDialogField::Project)
                {
                    self.shift_create_dialog_project(1);
                    return;
                }
                if let Some(dialog) = self.create_dialog_mut()
                    && dialog.focused_field == CreateDialogField::Agent
                {
                    Self::select_next_create_dialog_agent(dialog);
                }
                if let Some(dialog) = self.create_dialog_mut()
                    && dialog.focused_field
                        == CreateDialogField::StartConfig(StartAgentConfigField::Unsafe)
                {
                    dialog.start_config.toggle_unsafe();
                }
            }
            KeyCode::Char(_) if ctrl_n || ctrl_p => {
                if ctrl_n {
                    self.create_dialog_focus_next();
                } else {
                    self.create_dialog_focus_previous();
                }
            }
            KeyCode::Backspace => {
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            dialog.workspace_name.pop();
                        }
                        CreateDialogField::PullRequestUrl => {
                            dialog.pr_url.pop();
                        }
                        CreateDialogField::BaseBranch => {
                            dialog.base_branch.pop();
                            refresh_base_branch = true;
                        }
                        CreateDialogField::StartConfig(field) => {
                            dialog.start_config.backspace(field);
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
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                let focused_field = self.create_dialog().map(|dialog| dialog.focused_field);
                if self
                    .create_dialog()
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
                if focused_field == Some(CreateDialogField::BaseBranch) {
                    if character == 'j'
                        && self.create_branch_index.saturating_add(1)
                            < self.create_branch_filtered.len()
                    {
                        self.create_branch_index = self.create_branch_index.saturating_add(1);
                        return;
                    }
                    if character == 'k' && self.create_branch_index > 0 {
                        self.create_branch_index = self.create_branch_index.saturating_sub(1);
                        return;
                    }
                }
                let mut refresh_base_branch = false;
                if let Some(dialog) = self.create_dialog_mut() {
                    if dialog.focused_field == CreateDialogField::Agent
                        && (character == 'j' || character == 'k' || character == ' ')
                    {
                        if character == 'k' {
                            Self::select_previous_create_dialog_agent(dialog);
                        } else {
                            Self::select_next_create_dialog_agent(dialog);
                        }
                        return;
                    }
                    if dialog.focused_field
                        == CreateDialogField::StartConfig(StartAgentConfigField::Unsafe)
                        && (character == 'j' || character == 'k' || character == ' ')
                    {
                        dialog.start_config.toggle_unsafe();
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
                        CreateDialogField::PullRequestUrl => {
                            if !character.is_control() {
                                dialog.pr_url.push(character);
                            }
                        }
                        CreateDialogField::Project => {}
                        CreateDialogField::BaseBranch => {
                            if !character.is_control() {
                                dialog.base_branch.push(character);
                                refresh_base_branch = true;
                            }
                        }
                        CreateDialogField::StartConfig(field) => match field {
                            StartAgentConfigField::Prompt | StartAgentConfigField::InitCommand => {
                                if !character.is_control() {
                                    dialog.start_config.push_char(field, character);
                                }
                            }
                            StartAgentConfigField::Unsafe => {}
                        },
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
    fn select_next_create_dialog_agent(dialog: &mut CreateDialogState) {
        dialog.agent = Self::next_agent(dialog.agent);
    }

    fn select_previous_create_dialog_agent(dialog: &mut CreateDialogState) {
        dialog.agent = Self::previous_agent(dialog.agent);
    }
    fn shift_create_dialog_project(&mut self, delta: isize) {
        let Some(current_index) = self.create_dialog().map(|dialog| dialog.project_index) else {
            return;
        };
        if self.projects.is_empty() {
            return;
        }

        let len = self.projects.len();
        let current = current_index.min(len.saturating_sub(1));
        let mut next = current;
        if delta < 0 {
            next = current.saturating_sub(1);
        } else if delta > 0 {
            next = (current.saturating_add(1)).min(len.saturating_sub(1));
        }

        if next == current_index {
            return;
        }

        self.apply_create_dialog_project_defaults(next);
    }

    fn create_base_branch_dropdown_visible(&self) -> bool {
        self.create_dialog().is_some_and(|dialog| {
            dialog.tab == CreateDialogTab::Manual
                && dialog.focused_field == CreateDialogField::BaseBranch
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
        if let Some(dialog) = self.create_dialog_mut() {
            dialog.base_branch = selected_branch;
            return true;
        }
        false
    }

    fn create_dialog_focus_next(&mut self) {
        if let Some(dialog) = self.create_dialog_mut() {
            dialog.focused_field = dialog.focused_field.next(dialog.tab);
        }
    }

    fn create_dialog_focus_previous(&mut self) {
        if let Some(dialog) = self.create_dialog_mut() {
            dialog.focused_field = dialog.focused_field.previous(dialog.tab);
        }
    }

    fn switch_create_dialog_tab(&mut self, forward: bool) {
        if let Some(dialog) = self.create_dialog_mut() {
            dialog.tab = if forward {
                dialog.tab.next()
            } else {
                dialog.tab.previous()
            };
            dialog.focused_field = CreateDialogField::first_for_tab(dialog.tab);
        }
    }
}

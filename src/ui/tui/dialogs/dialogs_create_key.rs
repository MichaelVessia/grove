use super::*;

impl GroveApp {
    fn handle_create_project_picker_key(&mut self, key_event: KeyEvent) {
        let ctrl_n = key_event.code == KeyCode::Char('n') && key_event.modifiers == Modifiers::CTRL;
        let ctrl_p = key_event.code == KeyCode::Char('p') && key_event.modifiers == Modifiers::CTRL;

        match key_event.code {
            KeyCode::Escape => {
                let should_clear_filter = self
                    .create_dialog()
                    .and_then(|dialog| dialog.project_picker.as_ref())
                    .is_some_and(|picker| !picker.filter.is_empty());
                if should_clear_filter {
                    if let Some(dialog) = self.create_dialog_mut()
                        && let Some(picker) = dialog.project_picker.as_mut()
                    {
                        picker.filter.clear();
                    }
                    self.refresh_create_project_picker_filtered();
                    return;
                }

                self.close_create_project_picker();
            }
            KeyCode::Enter => {
                let Some(project_index) = self.selected_create_project_picker_project_index()
                else {
                    return;
                };
                self.apply_create_dialog_project_defaults(project_index);
                self.close_create_project_picker();
                self.create_dialog_focus_next();
            }
            KeyCode::Up => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                    && picker.selected_filtered_index() > 0
                {
                    picker.project_list.select_previous();
                }
            }
            KeyCode::Down => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                    && picker.selected_filtered_index().saturating_add(1)
                        < picker.filtered_project_indices.len()
                {
                    picker
                        .project_list
                        .select_next(picker.filtered_project_indices.len());
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.set_selected_filtered_index(
                            picker.selected_filtered_index().saturating_add(1) % len,
                        );
                    }
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.set_selected_filtered_index(
                            if picker.selected_filtered_index() == 0 {
                                len.saturating_sub(1)
                            } else {
                                picker.selected_filtered_index().saturating_sub(1)
                            },
                        );
                    }
                }
            }
            KeyCode::Char(_) if ctrl_n => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.set_selected_filtered_index(
                            picker.selected_filtered_index().saturating_add(1) % len,
                        );
                    }
                }
            }
            KeyCode::Char(_) if ctrl_p => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.set_selected_filtered_index(
                            if picker.selected_filtered_index() == 0 {
                                len.saturating_sub(1)
                            } else {
                                picker.selected_filtered_index().saturating_sub(1)
                            },
                        );
                    }
                }
            }
            KeyCode::Char(' ') if key_event.modifiers.is_empty() => {
                let Some(project_index) = self.selected_create_project_picker_project_index()
                else {
                    return;
                };
                if let Some(dialog) = self.create_dialog_mut() {
                    dialog.project_index = project_index;
                    if dialog.tab == CreateDialogTab::PullRequest || dialog.register_as_base {
                        return;
                    }
                }
                self.toggle_create_dialog_project_selection();
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    picker.filter.pop();
                }
                self.refresh_create_project_picker_filtered();
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                    && !character.is_control()
                {
                    picker.filter.push(character);
                }
                self.refresh_create_project_picker_filtered();
            }
            _ => {}
        }
    }

    pub(super) fn handle_create_dialog_key(&mut self, key_event: KeyEvent) {
        if self.dialogs.create_in_flight {
            return;
        }
        if self.create_project_picker_open() {
            self.handle_create_project_picker_key(key_event);
            return;
        }

        let alt_previous_tab =
            key_event.code == KeyCode::Char('[') && key_event.modifiers == Modifiers::ALT;
        let alt_next_tab =
            key_event.code == KeyCode::Char(']') && key_event.modifiers == Modifiers::ALT;

        self.sync_active_dialog_focus_field();
        let Some(focused_field) = self.create_dialog().map(|dialog| dialog.focused_field) else {
            return;
        };

        if alt_previous_tab
            && self
                .create_dialog()
                .is_some_and(|dialog| !dialog.is_add_worktree_mode())
        {
            self.switch_create_dialog_tab(false);
            return;
        }
        if alt_next_tab
            && self
                .create_dialog()
                .is_some_and(|dialog| !dialog.is_add_worktree_mode())
        {
            self.switch_create_dialog_tab(true);
            return;
        }

        let ctrl_n = key_event.code == KeyCode::Char('n') && key_event.modifiers == Modifiers::CTRL;
        let ctrl_p = key_event.code == KeyCode::Char('p') && key_event.modifiers == Modifiers::CTRL;

        match key_event.code {
            KeyCode::Escape => {
                self.log_dialog_event("create", "dialog_cancelled");
                self.close_active_dialog();
            }
            KeyCode::Enter => {
                enum EnterAction {
                    ConfirmCreate,
                    CancelDialog,
                    AdvanceField,
                }

                if focused_field == CreateDialogField::Project {
                    self.open_create_project_picker();
                    return;
                }
                let action = match focused_field {
                    CreateDialogField::CreateButton => EnterAction::ConfirmCreate,
                    CreateDialogField::CancelButton => EnterAction::CancelDialog,
                    CreateDialogField::WorkspaceName
                    | CreateDialogField::RegisterAsBase
                    | CreateDialogField::PullRequestUrl
                    | CreateDialogField::Project => EnterAction::AdvanceField,
                };

                match action {
                    EnterAction::ConfirmCreate => self.confirm_create_dialog(),
                    EnterAction::CancelDialog => {
                        self.log_dialog_event("create", "dialog_cancelled");
                        self.close_active_dialog();
                    }
                    EnterAction::AdvanceField => {
                        self.create_dialog_focus_next();
                    }
                }
            }
            KeyCode::Tab => {
                self.create_dialog_focus_next();
            }
            KeyCode::BackTab => {
                self.create_dialog_focus_previous();
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {}
            KeyCode::Char(' ') if key_event.modifiers.is_empty() => {
                let mut refresh_focus = false;
                if let Some(dialog) = self.create_dialog_mut()
                    && focused_field == CreateDialogField::RegisterAsBase
                {
                    dialog.register_as_base = !dialog.register_as_base;
                    if dialog.register_as_base {
                        dialog.task_name.clear();
                        dialog.selected_repository_indices = vec![dialog.project_index];
                    }
                    refresh_focus = true;
                }
                if refresh_focus {
                    self.refresh_create_dialog_focus_trap();
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
                if let Some(dialog) = self.create_dialog_mut() {
                    match focused_field {
                        CreateDialogField::WorkspaceName if !dialog.register_as_base => {
                            dialog.task_name.pop();
                        }
                        CreateDialogField::PullRequestUrl => {
                            dialog.pr_url.pop();
                        }
                        CreateDialogField::WorkspaceName
                        | CreateDialogField::RegisterAsBase
                        | CreateDialogField::Project
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if (focused_field == CreateDialogField::CreateButton
                    || focused_field == CreateDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    self.focus_dialog_field(if focused_field == CreateDialogField::CreateButton {
                        FOCUS_ID_CREATE_CANCEL_BUTTON
                    } else {
                        FOCUS_ID_CREATE_CREATE_BUTTON
                    });
                    return;
                }

                if let Some(dialog) = self.create_dialog_mut() {
                    match focused_field {
                        CreateDialogField::WorkspaceName if !dialog.register_as_base => {
                            if character.is_ascii_alphanumeric()
                                || character == '-'
                                || character == '_'
                            {
                                dialog.task_name.push(character);
                            }
                        }
                        CreateDialogField::PullRequestUrl => {
                            if !character.is_control() {
                                dialog.pr_url.push(character);
                            }
                        }
                        CreateDialogField::WorkspaceName
                        | CreateDialogField::RegisterAsBase
                        | CreateDialogField::Project => {}
                        CreateDialogField::CreateButton | CreateDialogField::CancelButton => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn create_dialog_focus_next(&mut self) {
        if self.create_dialog().is_some() {
            self.focus_next_dialog_field();
        }
    }

    fn create_dialog_focus_previous(&mut self) {
        if self.create_dialog().is_some() {
            self.focus_prev_dialog_field();
        }
    }

    fn refresh_create_dialog_focus_trap(&mut self) {
        self.refresh_active_dialog_focus_trap();
    }

    fn switch_create_dialog_tab(&mut self, forward: bool) {
        let mut refresh_focus = false;
        if let Some(dialog) = self.create_dialog_mut() {
            if dialog.is_add_worktree_mode() {
                return;
            }
            dialog.tab = if forward {
                dialog.tab.next()
            } else {
                dialog.tab.previous()
            };
            dialog.project_picker = None;
            dialog.focused_field = dialog.first_field();
            refresh_focus = true;
        }
        if refresh_focus {
            self.refresh_create_dialog_focus_trap();
        }
    }
}

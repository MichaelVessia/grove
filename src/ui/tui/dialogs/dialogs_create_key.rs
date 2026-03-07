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
                    && picker.selected_filtered_index > 0
                {
                    picker.selected_filtered_index =
                        picker.selected_filtered_index.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                    && picker.selected_filtered_index.saturating_add(1)
                        < picker.filtered_project_indices.len()
                {
                    picker.selected_filtered_index =
                        picker.selected_filtered_index.saturating_add(1);
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.selected_filtered_index =
                            picker.selected_filtered_index.saturating_add(1) % len;
                    }
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.selected_filtered_index = if picker.selected_filtered_index == 0 {
                            len.saturating_sub(1)
                        } else {
                            picker.selected_filtered_index.saturating_sub(1)
                        };
                    }
                }
            }
            KeyCode::Char(_) if ctrl_n => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.selected_filtered_index =
                            picker.selected_filtered_index.saturating_add(1) % len;
                    }
                }
            }
            KeyCode::Char(_) if ctrl_p => {
                if let Some(dialog) = self.create_dialog_mut()
                    && let Some(picker) = dialog.project_picker.as_mut()
                {
                    let len = picker.filtered_project_indices.len();
                    if len > 0 {
                        picker.selected_filtered_index = if picker.selected_filtered_index == 0 {
                            len.saturating_sub(1)
                        } else {
                            picker.selected_filtered_index.saturating_sub(1)
                        };
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
                    if dialog.tab == CreateDialogTab::PullRequest {
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
            }
            KeyCode::Enter => {
                enum EnterAction {
                    ConfirmCreate,
                    CancelDialog,
                    AdvanceField,
                }

                let focused_field = self.create_dialog().map(|dialog| dialog.focused_field);
                if focused_field == Some(CreateDialogField::Project) {
                    self.open_create_project_picker();
                    return;
                }
                let action = focused_field.map(|field| match field {
                    CreateDialogField::CreateButton => EnterAction::ConfirmCreate,
                    CreateDialogField::CancelButton => EnterAction::CancelDialog,
                    CreateDialogField::WorkspaceName
                    | CreateDialogField::PullRequestUrl
                    | CreateDialogField::Project => EnterAction::AdvanceField,
                });

                match action {
                    Some(EnterAction::ConfirmCreate) => self.confirm_create_dialog(),
                    Some(EnterAction::CancelDialog) => {
                        self.log_dialog_event("create", "dialog_cancelled");
                        self.close_active_dialog();
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
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {}
            KeyCode::Char(_) if ctrl_n || ctrl_p => {
                if ctrl_n {
                    self.create_dialog_focus_next();
                } else {
                    self.create_dialog_focus_previous();
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.create_dialog_mut() {
                    match dialog.focused_field {
                        CreateDialogField::WorkspaceName => {
                            dialog.task_name.pop();
                        }
                        CreateDialogField::PullRequestUrl => {
                            dialog.pr_url.pop();
                        }
                        CreateDialogField::Project
                        | CreateDialogField::CreateButton
                        | CreateDialogField::CancelButton => {}
                    }
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if let Some(dialog) = self.create_dialog_mut() {
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
                                dialog.task_name.push(character);
                            }
                        }
                        CreateDialogField::PullRequestUrl => {
                            if !character.is_control() {
                                dialog.pr_url.push(character);
                            }
                        }
                        CreateDialogField::Project => {}
                        CreateDialogField::CreateButton | CreateDialogField::CancelButton => {}
                    }
                }
            }
            _ => {}
        }
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
            dialog.project_picker = None;
            dialog.focused_field = CreateDialogField::first_for_tab(dialog.tab);
        }
    }
}

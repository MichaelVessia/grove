use super::*;

impl GroveApp {
    fn handle_project_defaults_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(project_dialog) = self.project_dialog_mut() else {
            return;
        };
        let Some(defaults_dialog) = project_dialog.defaults_dialog.as_mut() else {
            return;
        };

        enum PostAction {
            None,
            Save,
            Close,
        }
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Close;
            }
            KeyCode::Tab => {
                defaults_dialog.focused_field = defaults_dialog.focused_field.next();
                defaults_dialog.sync_focus();
            }
            KeyCode::BackTab => {
                defaults_dialog.focused_field = defaults_dialog.focused_field.previous();
                defaults_dialog.sync_focus();
            }
            KeyCode::Char(_) if ctrl_n => {
                defaults_dialog.focused_field = defaults_dialog.focused_field.next();
                defaults_dialog.sync_focus();
            }
            KeyCode::Char(_) if ctrl_p => {
                defaults_dialog.focused_field = defaults_dialog.focused_field.previous();
                defaults_dialog.sync_focus();
            }
            KeyCode::Enter => match defaults_dialog.focused_field {
                ProjectDefaultsDialogField::SaveButton => post_action = PostAction::Save,
                ProjectDefaultsDialogField::CancelButton => post_action = PostAction::Close,
                ProjectDefaultsDialogField::BaseBranch
                | ProjectDefaultsDialogField::WorkspaceInitCommand
                | ProjectDefaultsDialogField::ClaudeEnv
                | ProjectDefaultsDialogField::CodexEnv
                | ProjectDefaultsDialogField::OpenCodeEnv => {
                    defaults_dialog.focused_field = defaults_dialog.focused_field.next();
                    defaults_dialog.sync_focus();
                }
            },
            KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => match defaults_dialog.focused_field {
                ProjectDefaultsDialogField::BaseBranch => {
                    let _ = defaults_dialog
                        .base_branch_input
                        .handle_event(&Event::Key(key_event));
                }
                ProjectDefaultsDialogField::WorkspaceInitCommand => {
                    let _ = defaults_dialog
                        .workspace_init_command_input
                        .handle_event(&Event::Key(key_event));
                }
                ProjectDefaultsDialogField::ClaudeEnv => {
                    let _ = defaults_dialog
                        .claude_env_input
                        .handle_event(&Event::Key(key_event));
                }
                ProjectDefaultsDialogField::CodexEnv => {
                    let _ = defaults_dialog
                        .codex_env_input
                        .handle_event(&Event::Key(key_event));
                }
                ProjectDefaultsDialogField::OpenCodeEnv => {
                    let _ = defaults_dialog
                        .opencode_env_input
                        .handle_event(&Event::Key(key_event));
                }
                ProjectDefaultsDialogField::SaveButton
                | ProjectDefaultsDialogField::CancelButton => {}
            },
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if (defaults_dialog.focused_field == ProjectDefaultsDialogField::SaveButton
                    || defaults_dialog.focused_field == ProjectDefaultsDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    defaults_dialog.focused_field = if defaults_dialog.focused_field
                        == ProjectDefaultsDialogField::SaveButton
                    {
                        ProjectDefaultsDialogField::CancelButton
                    } else {
                        ProjectDefaultsDialogField::SaveButton
                    };
                    defaults_dialog.sync_focus();
                    return;
                }
                match defaults_dialog.focused_field {
                    ProjectDefaultsDialogField::BaseBranch => {
                        let _ = defaults_dialog
                            .base_branch_input
                            .handle_event(&Event::Key(key_event));
                    }
                    ProjectDefaultsDialogField::WorkspaceInitCommand => {
                        let _ = defaults_dialog
                            .workspace_init_command_input
                            .handle_event(&Event::Key(key_event));
                    }
                    ProjectDefaultsDialogField::ClaudeEnv => {
                        let _ = defaults_dialog
                            .claude_env_input
                            .handle_event(&Event::Key(key_event));
                    }
                    ProjectDefaultsDialogField::CodexEnv => {
                        let _ = defaults_dialog
                            .codex_env_input
                            .handle_event(&Event::Key(key_event));
                    }
                    ProjectDefaultsDialogField::OpenCodeEnv => {
                        let _ = defaults_dialog
                            .opencode_env_input
                            .handle_event(&Event::Key(key_event));
                    }
                    ProjectDefaultsDialogField::SaveButton
                    | ProjectDefaultsDialogField::CancelButton => {}
                }
            }
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.save_project_defaults_from_dialog(),
            PostAction::Close => {
                if let Some(project_dialog) = self.project_dialog_mut() {
                    project_dialog.defaults_dialog = None;
                }
            }
        }
    }

    pub(super) fn handle_project_add_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(project_dialog) = self.project_dialog_mut() else {
            return;
        };
        let Some(add_dialog) = project_dialog.add_dialog.as_mut() else {
            return;
        };

        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Escape => {
                project_dialog.add_dialog = None;
            }
            KeyCode::Tab => {
                add_dialog.focused_field = add_dialog.focused_field.next();
                add_dialog.sync_focus();
            }
            KeyCode::BackTab => {
                add_dialog.focused_field = add_dialog.focused_field.previous();
                add_dialog.sync_focus();
            }
            KeyCode::Char(_) if ctrl_n => {
                add_dialog.focused_field = add_dialog.focused_field.next();
                add_dialog.sync_focus();
            }
            KeyCode::Char(_) if ctrl_p => {
                add_dialog.focused_field = add_dialog.focused_field.previous();
                add_dialog.sync_focus();
            }
            KeyCode::Enter => match add_dialog.focused_field {
                ProjectAddDialogField::AddButton => self.add_project_from_dialog(),
                ProjectAddDialogField::CancelButton => project_dialog.add_dialog = None,
                ProjectAddDialogField::Name | ProjectAddDialogField::Path => {
                    add_dialog.focused_field = add_dialog.focused_field.next();
                    add_dialog.sync_focus();
                }
            },
            KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => match add_dialog.focused_field {
                ProjectAddDialogField::Name => {
                    let _ = add_dialog.name_input.handle_event(&Event::Key(key_event));
                }
                ProjectAddDialogField::Path => {
                    let _ = add_dialog.path_input.handle_event(&Event::Key(key_event));
                }
                ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
            },
            KeyCode::Char(_) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                match add_dialog.focused_field {
                    ProjectAddDialogField::Name => {
                        let _ = add_dialog.name_input.handle_event(&Event::Key(key_event));
                    }
                    ProjectAddDialogField::Path => {
                        let _ = add_dialog.path_input.handle_event(&Event::Key(key_event));
                    }
                    ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_project_dialog_key(&mut self, key_event: KeyEvent) {
        if self
            .project_dialog()
            .and_then(|dialog| dialog.add_dialog.as_ref())
            .is_some()
        {
            self.handle_project_add_dialog_key(key_event);
            return;
        }
        if self
            .project_dialog()
            .and_then(|dialog| dialog.defaults_dialog.as_ref())
            .is_some()
        {
            self.handle_project_defaults_dialog_key(key_event);
            return;
        }
        if self.dialogs.project_delete_in_flight {
            return;
        }
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        match key_event.code {
            KeyCode::Escape => {
                if let Some(dialog) = self.project_dialog_mut()
                    && !dialog.filter().is_empty()
                {
                    dialog.filter_input.clear();
                    self.refresh_project_dialog_filtered();
                    return;
                }
                self.close_active_dialog();
            }
            KeyCode::Enter => {
                if let Some(project_index) = self.selected_project_dialog_project_index() {
                    self.focus_project_by_index(project_index);
                    self.close_active_dialog();
                }
            }
            KeyCode::Up => {
                if let Some(dialog) = self.project_dialog_mut()
                    && dialog.project_list.selected().is_some()
                {
                    dialog.project_list.select_previous();
                }
            }
            KeyCode::Down => {
                if let Some(dialog) = self.project_dialog_mut()
                    && !dialog.filtered_project_indices.is_empty()
                {
                    dialog
                        .project_list
                        .select_next(dialog.filtered_project_indices.len());
                }
            }
            KeyCode::Tab => {
                if let Some(dialog) = self.project_dialog_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        let next = dialog.selected_filtered_index().saturating_add(1) % len;
                        dialog.set_selected_filtered_index(next);
                    }
                }
            }
            KeyCode::BackTab => {
                if let Some(dialog) = self.project_dialog_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        let selected = dialog.selected_filtered_index();
                        dialog.set_selected_filtered_index(if selected == 0 {
                            len.saturating_sub(1)
                        } else {
                            selected.saturating_sub(1)
                        });
                    }
                }
            }
            KeyCode::Char(_) if ctrl_n => {
                if let Some(dialog) = self.project_dialog_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        let next = dialog.selected_filtered_index().saturating_add(1) % len;
                        dialog.set_selected_filtered_index(next);
                    }
                }
            }
            KeyCode::Char(_) if ctrl_p => {
                if let Some(dialog) = self.project_dialog_mut() {
                    let len = dialog.filtered_project_indices.len();
                    if len > 0 {
                        let selected = dialog.selected_filtered_index();
                        dialog.set_selected_filtered_index(if selected == 0 {
                            len.saturating_sub(1)
                        } else {
                            selected.saturating_sub(1)
                        });
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(dialog) = self.project_dialog_mut() {
                    let _ = dialog.filter_input.handle_event(&Event::Key(key_event));
                }
                self.refresh_project_dialog_filtered();
            }
            KeyCode::Delete => {
                self.delete_selected_project_from_dialog();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'x' || character == 'X') =>
            {
                self.delete_selected_project_from_dialog();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'a' || character == 'A') =>
            {
                self.open_project_add_dialog();
            }
            KeyCode::Char(character)
                if key_event.modifiers == Modifiers::CTRL
                    && (character == 'e' || character == 'E') =>
            {
                self.open_selected_project_defaults_dialog();
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if let Some(dialog) = self.project_dialog_mut()
                    && !character.is_control()
                {
                    let _ = dialog.filter_input.handle_event(&Event::Key(key_event));
                }
                self.refresh_project_dialog_filtered();
            }
            _ => {}
        }
    }
}

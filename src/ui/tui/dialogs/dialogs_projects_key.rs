use super::*;

impl GroveApp {
    fn handle_project_defaults_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(focused_field) = self.project_dialog().and_then(|dialog| {
            dialog.defaults_dialog.as_ref().map(|defaults_dialog| {
                match self.focus_manager.current() {
                    Some(FOCUS_ID_PROJECT_DEFAULTS_BASE_BRANCH_INPUT) => {
                        ProjectDefaultsDialogField::BaseBranch
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_INIT_COMMAND_INPUT) => {
                        ProjectDefaultsDialogField::WorkspaceInitCommand
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_CLAUDE_ENV_INPUT) => {
                        ProjectDefaultsDialogField::ClaudeEnv
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_CODEX_ENV_INPUT) => {
                        ProjectDefaultsDialogField::CodexEnv
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_OPENCODE_ENV_INPUT) => {
                        ProjectDefaultsDialogField::OpenCodeEnv
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_SAVE_BUTTON) => {
                        ProjectDefaultsDialogField::SaveButton
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_CANCEL_BUTTON) => {
                        ProjectDefaultsDialogField::CancelButton
                    }
                    _ => defaults_dialog.focused_field,
                }
            })
        }) else {
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
                self.focus_next_dialog_field();
            }
            KeyCode::BackTab => {
                self.focus_prev_dialog_field();
            }
            KeyCode::Char(_) if ctrl_n => {
                self.focus_next_dialog_field();
            }
            KeyCode::Char(_) if ctrl_p => {
                self.focus_prev_dialog_field();
            }
            KeyCode::Enter => match focused_field {
                ProjectDefaultsDialogField::SaveButton => post_action = PostAction::Save,
                ProjectDefaultsDialogField::CancelButton => post_action = PostAction::Close,
                ProjectDefaultsDialogField::BaseBranch
                | ProjectDefaultsDialogField::WorkspaceInitCommand
                | ProjectDefaultsDialogField::ClaudeEnv
                | ProjectDefaultsDialogField::CodexEnv
                | ProjectDefaultsDialogField::OpenCodeEnv => {
                    self.focus_next_dialog_field();
                }
            },
            KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => {
                if let Some(project_dialog) = self.project_dialog_mut()
                    && let Some(defaults_dialog) = project_dialog.defaults_dialog.as_mut()
                {
                    match focused_field {
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
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if (focused_field == ProjectDefaultsDialogField::SaveButton
                    || focused_field == ProjectDefaultsDialogField::CancelButton)
                    && (character == 'h' || character == 'l')
                {
                    self.focus_dialog_field(
                        if focused_field == ProjectDefaultsDialogField::SaveButton {
                            FOCUS_ID_PROJECT_DEFAULTS_CANCEL_BUTTON
                        } else {
                            FOCUS_ID_PROJECT_DEFAULTS_SAVE_BUTTON
                        },
                    );
                    return;
                }
                if let Some(project_dialog) = self.project_dialog_mut()
                    && let Some(defaults_dialog) = project_dialog.defaults_dialog.as_mut()
                {
                    match focused_field {
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
            }
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Save => self.save_project_defaults_from_dialog(),
            PostAction::Close => self.close_project_defaults_dialog(),
        }
    }

    pub(super) fn handle_project_add_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(focused_field) = self.project_dialog().and_then(|dialog| {
            dialog
                .add_dialog
                .as_ref()
                .map(|add_dialog| match self.focus_manager.current() {
                    Some(FOCUS_ID_PROJECT_ADD_PATH_INPUT) => ProjectAddDialogField::Path,
                    Some(FOCUS_ID_PROJECT_ADD_NAME_INPUT) => ProjectAddDialogField::Name,
                    Some(FOCUS_ID_PROJECT_ADD_ADD_BUTTON) => ProjectAddDialogField::AddButton,
                    Some(FOCUS_ID_PROJECT_ADD_CANCEL_BUTTON) => ProjectAddDialogField::CancelButton,
                    _ => add_dialog.focused_field,
                })
        }) else {
            return;
        };

        enum PostAction {
            None,
            Add,
            Close,
            RefreshMatches,
            AcceptSelectedPathMatch,
        }

        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));
        let move_down = key_event.code == KeyCode::Down || ctrl_n;
        let move_up = key_event.code == KeyCode::Up || ctrl_p;
        let mut post_action = PostAction::None;

        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Close;
            }
            KeyCode::Tab => {
                self.focus_next_dialog_field();
            }
            KeyCode::BackTab => {
                self.focus_prev_dialog_field();
            }
            _ if move_up => {
                if focused_field == ProjectAddDialogField::Path
                    && let Some(project_dialog) = self.project_dialog_mut()
                    && let Some(add_dialog) = project_dialog.add_dialog.as_mut()
                    && add_dialog.path_match_list.selected().is_some()
                {
                    add_dialog.path_match_list.select_previous();
                }
            }
            _ if move_down => {
                if focused_field == ProjectAddDialogField::Path
                    && let Some(project_dialog) = self.project_dialog_mut()
                    && let Some(add_dialog) = project_dialog.add_dialog.as_mut()
                    && !add_dialog.path_matches.is_empty()
                {
                    add_dialog
                        .path_match_list
                        .select_next(add_dialog.path_matches.len());
                }
            }
            KeyCode::Enter => match focused_field {
                ProjectAddDialogField::AddButton => post_action = PostAction::Add,
                ProjectAddDialogField::CancelButton => post_action = PostAction::Close,
                ProjectAddDialogField::Path => {
                    if self
                        .project_dialog()
                        .and_then(|dialog| dialog.add_dialog.as_ref())
                        .and_then(ProjectAddDialogState::selected_path_match)
                        .is_some()
                    {
                        post_action = PostAction::AcceptSelectedPathMatch;
                    } else {
                        self.focus_next_dialog_field();
                    }
                }
                ProjectAddDialogField::Name => {
                    self.focus_next_dialog_field();
                }
            },
            KeyCode::Left | KeyCode::Right
                if matches!(
                    focused_field,
                    ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton
                ) =>
            {
                self.focus_dialog_field(if focused_field == ProjectAddDialogField::AddButton {
                    FOCUS_ID_PROJECT_ADD_CANCEL_BUTTON
                } else {
                    FOCUS_ID_PROJECT_ADD_ADD_BUTTON
                });
            }
            KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => {
                if let Some(project_dialog) = self.project_dialog_mut()
                    && let Some(add_dialog) = project_dialog.add_dialog.as_mut()
                {
                    match focused_field {
                        ProjectAddDialogField::Path => {
                            if add_dialog.path_input.handle_event(&Event::Key(key_event)) {
                                post_action = PostAction::RefreshMatches;
                            }
                        }
                        ProjectAddDialogField::Name => {
                            let _ = add_dialog.name_input.handle_event(&Event::Key(key_event));
                        }
                        ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
                    }
                }
            }
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                if matches!(
                    focused_field,
                    ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton
                ) && matches!(character, 'h' | 'H' | 'l' | 'L')
                {
                    self.focus_dialog_field(if matches!(character, 'h' | 'H') {
                        FOCUS_ID_PROJECT_ADD_ADD_BUTTON
                    } else {
                        FOCUS_ID_PROJECT_ADD_CANCEL_BUTTON
                    });
                    return;
                }
                if let Some(project_dialog) = self.project_dialog_mut()
                    && let Some(add_dialog) = project_dialog.add_dialog.as_mut()
                {
                    match focused_field {
                        ProjectAddDialogField::Path => {
                            if add_dialog.path_input.handle_event(&Event::Key(key_event)) {
                                post_action = PostAction::RefreshMatches;
                            }
                        }
                        ProjectAddDialogField::Name => {
                            let _ = add_dialog.name_input.handle_event(&Event::Key(key_event));
                        }
                        ProjectAddDialogField::AddButton | ProjectAddDialogField::CancelButton => {}
                    }
                }
            }
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Add => self.add_project_from_dialog(),
            PostAction::Close => self.close_project_add_dialog(),
            PostAction::RefreshMatches => self.refresh_project_add_dialog_matches(),
            PostAction::AcceptSelectedPathMatch => self.accept_selected_project_add_path_match(),
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

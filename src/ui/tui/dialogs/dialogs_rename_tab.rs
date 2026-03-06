use super::*;

impl GroveApp {
    pub(super) fn open_rename_tab_dialog(&mut self) {
        if self.modal_open() {
            return;
        }

        let Some(workspace) = self.state.selected_workspace().cloned() else {
            self.show_info_toast("no workspace selected");
            return;
        };
        let Some(tab) = self.selected_active_tab().cloned() else {
            self.show_info_toast("no active tab");
            return;
        };
        if tab.kind == WorkspaceTabKind::Home {
            self.show_info_toast("home tab title is fixed");
            return;
        }

        self.set_rename_tab_dialog(RenameTabDialogState {
            workspace_path: workspace.path.clone(),
            tab_id: tab.id,
            current_title: tab.title.clone(),
            title: tab.title,
            focused_field: RenameTabDialogField::Title,
        });
        self.log_dialog_event_with_fields(
            "rename_tab",
            "dialog_opened",
            [
                ("workspace".to_string(), Value::from(workspace.name)),
                ("tab_id".to_string(), Value::from(tab.id)),
            ],
        );
    }

    fn apply_rename_tab_dialog_save(&mut self) {
        let Some(dialog) = self.rename_tab_dialog().cloned() else {
            return;
        };
        let next_title = dialog.title.trim().to_string();
        if next_title.is_empty() {
            self.show_info_toast("tab title cannot be empty");
            return;
        }

        if let Err(error) = self.rename_workspace_tab_title(
            dialog.workspace_path.as_path(),
            dialog.tab_id,
            next_title.clone(),
        ) {
            self.show_error_toast(format!("tab rename failed: {error}"));
            return;
        }

        self.log_dialog_event_with_fields(
            "rename_tab",
            "dialog_confirmed",
            [
                (
                    "workspace_path".to_string(),
                    Value::from(dialog.workspace_path.display().to_string()),
                ),
                ("tab_id".to_string(), Value::from(dialog.tab_id)),
                (
                    "previous_title".to_string(),
                    Value::from(dialog.current_title),
                ),
                ("title".to_string(), Value::from(next_title.clone())),
            ],
        );
        self.close_active_dialog();
        self.show_success_toast(format!("renamed tab to '{next_title}'"));
    }

    pub(super) fn handle_rename_tab_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.rename_tab_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        enum PostAction {
            None,
            Rename,
            Cancel,
        }

        if dialog.focused_field == RenameTabDialogField::Title
            && Self::allows_text_input_modifiers(key_event.modifiers)
        {
            match key_event.code {
                KeyCode::Backspace => {
                    dialog.title.pop();
                    return;
                }
                KeyCode::Char(character) if !character.is_control() => {
                    dialog.title.push(character);
                    return;
                }
                _ => {}
            }
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Char(_) if ctrl_n => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(_) if ctrl_p => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left => {
                if dialog.focused_field == RenameTabDialogField::CancelButton {
                    dialog.focused_field = RenameTabDialogField::RenameButton;
                }
            }
            KeyCode::Right => {
                if dialog.focused_field == RenameTabDialogField::RenameButton {
                    dialog.focused_field = RenameTabDialogField::CancelButton;
                }
            }
            KeyCode::Char('h')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != RenameTabDialogField::Title =>
            {
                if dialog.focused_field == RenameTabDialogField::CancelButton {
                    dialog.focused_field = RenameTabDialogField::RenameButton;
                }
            }
            KeyCode::Char('l')
                if key_event.modifiers.is_empty()
                    && dialog.focused_field != RenameTabDialogField::Title =>
            {
                if dialog.focused_field == RenameTabDialogField::RenameButton {
                    dialog.focused_field = RenameTabDialogField::CancelButton;
                }
            }
            KeyCode::Enter => match dialog.focused_field {
                RenameTabDialogField::Title => {
                    dialog.focused_field = dialog.focused_field.next();
                }
                RenameTabDialogField::RenameButton => post_action = PostAction::Rename,
                RenameTabDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::Rename => self.apply_rename_tab_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("rename_tab", "dialog_cancelled");
                self.close_active_dialog();
            }
        }
    }
}

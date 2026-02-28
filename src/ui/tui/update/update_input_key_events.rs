use super::*;

impl GroveApp {
    fn is_ctrl_modal_nav_key(key_event: &KeyEvent) -> bool {
        key_event.modifiers == Modifiers::CTRL
            && matches!(
                key_event.code,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('p') | KeyCode::Char('P')
            )
    }

    fn remap_command_palette_nav_key(key_event: KeyEvent) -> KeyEvent {
        if key_event.modifiers != Modifiers::CTRL {
            return key_event;
        }

        match key_event.code {
            KeyCode::Char('n') | KeyCode::Char('N') => KeyEvent::new(KeyCode::Down)
                .with_modifiers(key_event.modifiers)
                .with_kind(key_event.kind),
            KeyCode::Char('p') | KeyCode::Char('P') => KeyEvent::new(KeyCode::Up)
                .with_modifiers(key_event.modifiers)
                .with_kind(key_event.kind),
            _ => key_event,
        }
    }

    fn normalize_command_palette_key_event(mut key_event: KeyEvent) -> KeyEvent {
        if key_event.kind == KeyEventKind::Repeat {
            key_event.kind = KeyEventKind::Press;
        }
        key_event
    }

    fn global_workspace_navigation_command(key_event: &KeyEvent) -> Option<UiCommand> {
        if !key_event.modifiers.contains(Modifiers::ALT) {
            return None;
        }

        match key_event.code {
            KeyCode::Char('j') | KeyCode::Char('J') => Some(UiCommand::MoveSelectionDown),
            KeyCode::Char('k') | KeyCode::Char('K') => Some(UiCommand::MoveSelectionUp),
            KeyCode::Char('[') => Some(UiCommand::PreviousTab),
            KeyCode::Char(']') => Some(UiCommand::NextTab),
            KeyCode::Char('b') | KeyCode::Char('B') => Some(UiCommand::ResizeSidebarNarrower),
            KeyCode::Char('f') | KeyCode::Char('F') => Some(UiCommand::ResizeSidebarWider),
            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Left => {
                Some(UiCommand::ResizeSidebarNarrower)
            }
            KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Right => {
                Some(UiCommand::ResizeSidebarWider)
            }
            _ => None,
        }
    }

    fn apply_paste_to_create_dialog(&mut self, text: &str) -> bool {
        let mut refresh_base_branch = false;
        let mut handled = false;
        if let Some(dialog) = self.create_dialog_mut() {
            match dialog.focused_field {
                CreateDialogField::WorkspaceName => {
                    handled = true;
                    for character in text.chars() {
                        if character.is_ascii_alphanumeric() || character == '-' || character == '_'
                        {
                            dialog.workspace_name.push(character);
                        }
                    }
                }
                CreateDialogField::PullRequestUrl => {
                    handled = true;
                    for character in text.chars() {
                        if !character.is_control() {
                            dialog.pr_url.push(character);
                        }
                    }
                }
                CreateDialogField::BaseBranch => {
                    handled = true;
                    for character in text.chars() {
                        if !character.is_control() {
                            dialog.base_branch.push(character);
                            refresh_base_branch = true;
                        }
                    }
                }
                CreateDialogField::StartConfig(field) => {
                    if matches!(
                        field,
                        StartAgentConfigField::Prompt | StartAgentConfigField::InitCommand
                    ) {
                        handled = true;
                        for character in text.chars() {
                            if !character.is_control() {
                                dialog.start_config.push_char(field, character);
                            }
                        }
                    }
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
        handled
    }

    pub(super) fn handle_paste_event(&mut self, paste_event: PasteEvent) -> Cmd<Msg> {
        if self.apply_paste_to_create_dialog(&paste_event.text) {
            return Cmd::None;
        }

        let input_seq = self.next_input_seq();
        let received_at = Instant::now();
        let (target_session, bracketed) = {
            let Some(state) = self.interactive.as_mut() else {
                return Cmd::None;
            };
            state.bracketed_paste = paste_event.bracketed;
            (state.target_session.clone(), state.bracketed_paste)
        };
        self.log_input_event_with_fields(
            "interactive_paste_received",
            input_seq,
            vec![
                (
                    "chars".to_string(),
                    Value::from(usize_to_u64(paste_event.text.chars().count())),
                ),
                ("bracketed".to_string(), Value::from(paste_event.bracketed)),
                ("session".to_string(), Value::from(target_session.clone())),
            ],
        );

        let payload = encode_paste_payload(&paste_event.text, bracketed || paste_event.bracketed);
        let send_cmd = self.send_interactive_action(
            &InteractiveAction::SendLiteral(payload),
            &target_session,
            Some(InputTraceContext {
                seq: input_seq,
                received_at,
            }),
        );
        self.schedule_interactive_debounced_poll(received_at);
        send_cmd
    }

    pub(super) fn enter_preview_or_interactive(&mut self) {
        if matches!(self.preview_tab, PreviewTab::Agent | PreviewTab::Shell)
            && let Some(workspace) = self.state.selected_workspace()
        {
            let session_name = shell_session_name_for_workspace(workspace);
            self.shell_sessions.retry_failed(&session_name);
        }
        if !self.enter_interactive(Instant::now()) {
            reduce(&mut self.state, Action::EnterPreviewMode);
            self.poll_preview();
        }
    }

    fn non_interactive_command_for_key(&self, key_event: &KeyEvent) -> Option<UiCommand> {
        let in_preview_focus =
            self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview;
        let in_preview_scroll =
            in_preview_focus && matches!(self.preview_tab, PreviewTab::Agent | PreviewTab::Shell);
        let can_enter_interactive = self.can_enter_interactive_session();

        for command in UiCommand::all() {
            let matches = match command {
                UiCommand::ToggleFocus => {
                    matches!(
                        key_event.code,
                        KeyCode::Tab | KeyCode::Char('h') | KeyCode::Char('l')
                    )
                }
                UiCommand::ToggleSidebar => matches!(key_event.code, KeyCode::Char('\\')),
                UiCommand::OpenPreview => match key_event.code {
                    KeyCode::Enter => !in_preview_focus || !can_enter_interactive,
                    _ => false,
                },
                UiCommand::EnterInteractive => {
                    matches!(key_event.code, KeyCode::Enter)
                        && in_preview_focus
                        && can_enter_interactive
                }
                UiCommand::FocusPreview => false,
                UiCommand::FocusList => matches!(key_event.code, KeyCode::Escape),
                UiCommand::MoveSelectionUp => {
                    matches!(key_event.code, KeyCode::Char('k') | KeyCode::Up) && !in_preview_focus
                }
                UiCommand::MoveSelectionDown => {
                    matches!(key_event.code, KeyCode::Char('j') | KeyCode::Down)
                        && !in_preview_focus
                }
                UiCommand::ScrollUp => {
                    matches!(key_event.code, KeyCode::Char('k') | KeyCode::Up) && in_preview_scroll
                }
                UiCommand::ScrollDown => {
                    matches!(key_event.code, KeyCode::Char('j') | KeyCode::Down)
                        && in_preview_scroll
                }
                UiCommand::PageUp => matches!(key_event.code, KeyCode::PageUp) && in_preview_scroll,
                UiCommand::PageDown => {
                    matches!(key_event.code, KeyCode::PageDown) && in_preview_scroll
                }
                UiCommand::ScrollBottom => {
                    matches!(key_event.code, KeyCode::Char('G') | KeyCode::End) && in_preview_scroll
                }
                UiCommand::PreviousTab => {
                    matches!(key_event.code, KeyCode::Char('[')) && in_preview_focus
                }
                UiCommand::NextTab => {
                    matches!(key_event.code, KeyCode::Char(']')) && in_preview_focus
                }
                UiCommand::ResizeSidebarNarrower | UiCommand::ResizeSidebarWider => false,
                UiCommand::NewWorkspace => {
                    matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'))
                }
                UiCommand::EditWorkspace => {
                    matches!(key_event.code, KeyCode::Char('e') | KeyCode::Char('E'))
                }
                UiCommand::StartAgent => matches!(key_event.code, KeyCode::Char('s')),
                UiCommand::StopAgent => {
                    matches!(key_event.code, KeyCode::Char('x'))
                        && (self.state.mode == UiMode::Preview
                            || self.state.focus == PaneFocus::Preview)
                        && self.preview_tab == PreviewTab::Agent
                }
                UiCommand::RestartAgent => {
                    matches!(key_event.code, KeyCode::Char('r'))
                        && (self.state.mode == UiMode::Preview
                            || self.state.focus == PaneFocus::Preview)
                        && self.preview_tab == PreviewTab::Agent
                }
                UiCommand::DeleteWorkspace => matches!(key_event.code, KeyCode::Char('D')),
                UiCommand::MergeWorkspace => matches!(key_event.code, KeyCode::Char('m')),
                UiCommand::UpdateFromBase => matches!(key_event.code, KeyCode::Char('u')),
                UiCommand::RefreshWorkspaces => matches!(key_event.code, KeyCode::Char('R')),
                UiCommand::OpenProjects => {
                    matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'))
                }
                UiCommand::ReorderProjects => false,
                UiCommand::DeleteProject => false,
                UiCommand::OpenSettings => matches!(key_event.code, KeyCode::Char('S')),
                UiCommand::ToggleMouseCapture => matches!(key_event.code, KeyCode::Char('M')),
                UiCommand::ToggleUnsafe => matches!(key_event.code, KeyCode::Char('!')),
                UiCommand::OpenHelp => matches!(key_event.code, KeyCode::Char('?')),
                UiCommand::OpenCommandPalette => Self::is_ctrl_char_key(key_event, 'k'),
                UiCommand::Quit => {
                    matches!(key_event.code, KeyCode::Char('q')) && key_event.modifiers.is_empty()
                }
            };
            if matches {
                return Some(*command);
            }
        }

        None
    }

    fn handle_non_interactive_key(&mut self, key_event: KeyEvent) -> bool {
        let Some(command) = self.non_interactive_command_for_key(&key_event) else {
            return false;
        };
        self.execute_ui_command(command)
    }

    fn dispatch_dialog_key(&mut self, key_event: &KeyEvent) -> bool {
        if self.create_dialog().is_some() {
            self.handle_create_dialog_key(*key_event);
            return true;
        }
        if self.edit_dialog().is_some() {
            self.handle_edit_dialog_key(*key_event);
            return true;
        }
        if self.launch_dialog().is_some() {
            self.handle_launch_dialog_key(*key_event);
            return true;
        }
        if self.stop_dialog().is_some() {
            self.handle_stop_dialog_key(*key_event);
            return true;
        }
        if self.confirm_dialog().is_some() {
            self.handle_confirm_dialog_key(*key_event);
            return true;
        }
        if self.delete_dialog().is_some() {
            self.handle_delete_dialog_key(*key_event);
            return true;
        }
        if self.merge_dialog().is_some() {
            self.handle_merge_dialog_key(*key_event);
            return true;
        }
        if self.update_from_base_dialog().is_some() {
            self.handle_update_from_base_dialog_key(*key_event);
            return true;
        }
        if self.project_dialog().is_some() {
            self.handle_project_dialog_key(*key_event);
            return true;
        }
        if self.settings_dialog().is_some() {
            self.handle_settings_dialog_key(*key_event);
            return true;
        }

        false
    }

    pub(super) fn handle_key(&mut self, key_event: KeyEvent) -> (bool, Cmd<Msg>) {
        if !matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return (false, Cmd::None);
        }

        if Self::is_ctrl_char_key(&key_event, 'k') {
            return (
                self.execute_ui_command(UiCommand::OpenCommandPalette),
                Cmd::None,
            );
        }

        if self.command_palette.is_visible() {
            let event = Event::Key(Self::normalize_command_palette_key_event(
                Self::remap_command_palette_nav_key(key_event),
            ));
            if let Some(action) = self.command_palette.handle_event(&event) {
                return match action {
                    PaletteAction::Dismiss => (false, Cmd::None),
                    PaletteAction::Execute(id) => {
                        (self.execute_command_palette_action(id.as_str()), Cmd::None)
                    }
                };
            }
            return (false, Cmd::None);
        }

        if !self.modal_open()
            && let Some(command) = Self::global_workspace_navigation_command(&key_event)
        {
            if self.interactive.is_some()
                && matches!(
                    command,
                    UiCommand::MoveSelectionDown
                        | UiCommand::MoveSelectionUp
                        | UiCommand::PreviousTab
                        | UiCommand::NextTab
                        | UiCommand::StopAgent
                )
            {
                self.exit_interactive_to_list();
            }
            return (self.execute_ui_command(command), Cmd::None);
        }

        if self.interactive.is_some() {
            return (false, self.handle_interactive_key(key_event));
        }

        if Self::is_ctrl_char_key(&key_event, 'c') && !self.modal_open() {
            self.open_quit_dialog();
            return (false, Cmd::None);
        }

        if Self::is_ctrl_modal_nav_key(&key_event) && self.dispatch_dialog_key(&key_event) {
            return (false, Cmd::None);
        }

        let keybinding_state = self.keybinding_state();
        if let Some(action) = self
            .action_mapper
            .map(&key_event, &keybinding_state, Instant::now())
        {
            if !matches!(action, KeybindingAction::PassThrough) {
                return (self.apply_keybinding_action(action), Cmd::None);
            }
        } else {
            return (false, Cmd::None);
        }

        if self.dispatch_dialog_key(&key_event) {
            return (false, Cmd::None);
        }
        if self.keybind_help_open {
            self.handle_keybind_help_key(key_event);
            return (false, Cmd::None);
        }

        (self.handle_non_interactive_key(key_event), Cmd::None)
    }
}

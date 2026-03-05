use super::update_prelude::*;

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
        for command in UiCommand::all() {
            if command.matches_keybinding(key_event, KeybindingScope::GlobalNavigation) {
                return Some(*command);
            }
        }
        None
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
                CreateDialogField::Project
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
            let Some(state) = self.session.interactive.as_mut() else {
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
        if !self.enter_interactive(Instant::now()) {
            reduce(&mut self.state, Action::EnterPreviewMode);
            self.poll_preview();
        }
    }

    fn non_interactive_command_for_key(&self, key_event: &KeyEvent) -> Option<UiCommand> {
        let in_preview_focus =
            self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview;
        let in_preview_scroll = in_preview_focus && self.active_tab_is_scrollable();
        let can_enter_interactive = self.can_enter_interactive_session();

        for command in UiCommand::all() {
            if !command.matches_keybinding(key_event, KeybindingScope::NonInteractive) {
                continue;
            }
            if self.non_interactive_command_enabled(
                *command,
                in_preview_focus,
                in_preview_scroll,
                can_enter_interactive,
            ) {
                return Some(*command);
            }
        }

        None
    }

    fn non_interactive_command_enabled(
        &self,
        command: UiCommand,
        in_preview_focus: bool,
        in_preview_scroll: bool,
        can_enter_interactive: bool,
    ) -> bool {
        match command {
            UiCommand::OpenPreview => !in_preview_focus || !can_enter_interactive,
            UiCommand::EnterInteractive => in_preview_focus && can_enter_interactive,
            UiCommand::MoveSelectionUp | UiCommand::MoveSelectionDown => !in_preview_focus,
            UiCommand::ScrollUp
            | UiCommand::ScrollDown
            | UiCommand::PageUp
            | UiCommand::PageDown
            | UiCommand::ScrollBottom => in_preview_scroll,
            UiCommand::PreviousTab | UiCommand::NextTab => in_preview_focus,
            UiCommand::StopAgent | UiCommand::RestartAgent => {
                self.state.mode == UiMode::Preview
                    && self.state.focus == PaneFocus::Preview
                    && match command {
                        UiCommand::StopAgent => self.active_tab_session_name().is_some(),
                        UiCommand::RestartAgent => self
                            .selected_active_tab()
                            .is_some_and(|tab| tab.kind != WorkspaceTabKind::Home),
                        _ => false,
                    }
            }
            UiCommand::ResizeSidebarNarrower
            | UiCommand::ResizeSidebarWider
            | UiCommand::FocusPreview
            | UiCommand::ReorderProjects
            | UiCommand::DeleteProject => false,
            _ => true,
        }
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
        if self.session_cleanup_dialog().is_some() {
            self.handle_session_cleanup_dialog_key(*key_event);
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

        if UiCommand::OpenCommandPalette
            .matches_keybinding(&key_event, KeybindingScope::NonInteractive)
        {
            return (
                self.execute_ui_command(UiCommand::OpenCommandPalette),
                Cmd::None,
            );
        }

        if self.dialogs.command_palette.is_visible() {
            let event = Event::Key(Self::normalize_command_palette_key_event(
                Self::remap_command_palette_nav_key(key_event),
            ));
            if let Some(action) = self.dialogs.command_palette.handle_event(&event) {
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
            if self.session.interactive.is_some()
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

        if self.session.interactive.is_some() {
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
        if self.dialogs.keybind_help_open {
            self.handle_keybind_help_key(key_event);
            return (false, Cmd::None);
        }

        (self.handle_non_interactive_key(key_event), Cmd::None)
    }

    fn preview_is_focused(&self) -> bool {
        self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview
    }

    pub(super) fn preview_scroll_tab_is_focused(&self) -> bool {
        self.preview_is_focused() && self.active_tab_is_scrollable()
    }

    pub(super) fn open_keybind_help(&mut self) {
        if self.modal_open() {
            return;
        }
        self.dialogs.keybind_help_open = true;
    }

    pub(super) fn move_selection(&mut self, action: Action) {
        let before = self.state.selected_index;
        reduce(&mut self.state, action);
        if self.state.selected_index != before {
            self.handle_workspace_selection_changed();
        }
    }

    pub(super) fn handle_workspace_selection_changed(&mut self) {
        if self.session.interactive.is_some() {
            self.exit_interactive_to_list();
        }
        self.sync_preview_tab_from_active_workspace_tab();
        self.preview.jump_to_bottom();
        self.clear_agent_activity_tracking();
        self.clear_preview_selection();
        if self.selected_live_preview_session_if_ready().is_none() {
            self.refresh_preview_summary();
        }
        self.poll_preview_prioritized();
    }
}

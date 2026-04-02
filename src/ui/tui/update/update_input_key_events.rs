use super::update_prelude::*;

impl GroveApp {
    fn is_ctrl_modal_nav_key(key_event: &KeyEvent) -> bool {
        key_event.modifiers == Modifiers::CTRL
            && matches!(
                key_event.code,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('p') | KeyCode::Char('P')
            )
    }

    fn attention_ack_key_pressed(&self, key_event: &KeyEvent) -> bool {
        self.selected_attention_item.is_some()
            && key_event.modifiers.is_empty()
            && matches!(key_event.code, KeyCode::Char('a'))
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
        let mut handled = false;
        let focused_field = self.current_create_dialog_focus_field();
        if let Some(dialog) = self.create_dialog_mut() {
            match focused_field {
                Some(CreateDialogField::WorkspaceName) if !dialog.register_as_base => {
                    handled = true;
                    for character in text.chars() {
                        if character.is_ascii_alphanumeric() || character == '-' || character == '_'
                        {
                            dialog.task_name.push(character);
                        }
                    }
                }
                Some(CreateDialogField::PullRequestUrl) => {
                    handled = true;
                    for character in text.chars() {
                        if !character.is_control() {
                            dialog.pr_url.push(character);
                        }
                    }
                }
                Some(
                    CreateDialogField::WorkspaceName
                    | CreateDialogField::RegisterAsBase
                    | CreateDialogField::Project
                    | CreateDialogField::CreateButton
                    | CreateDialogField::CancelButton,
                )
                | None => {}
            }
        }
        handled
    }

    fn paste_text_input(input: &mut TextInput, text: &str) -> bool {
        input.handle_event(&Event::Paste(PasteEvent::new(text.to_string(), true)))
    }

    fn apply_paste_to_project_dialog(&mut self, text: &str) -> bool {
        enum PostAction {
            None,
            RefreshProjectFilter,
            RefreshProjectAddMatches,
        }

        let mut handled = false;
        let mut post_action = PostAction::None;
        let current_focus_id = self.focus_manager.current();
        if let Some(project_dialog) = self.project_dialog_mut() {
            if let Some(add_dialog) = project_dialog.add_dialog.as_mut() {
                handled = match current_focus_id {
                    Some(FOCUS_ID_PROJECT_ADD_PATH_INPUT) => {
                        let changed = Self::paste_text_input(&mut add_dialog.path_input, text);
                        if changed {
                            post_action = PostAction::RefreshProjectAddMatches;
                        }
                        changed
                    }
                    Some(FOCUS_ID_PROJECT_ADD_NAME_INPUT) => {
                        Self::paste_text_input(&mut add_dialog.name_input, text)
                    }
                    Some(FOCUS_ID_PROJECT_ADD_ADD_BUTTON)
                    | Some(FOCUS_ID_PROJECT_ADD_CANCEL_BUTTON) => false,
                    _ => false,
                };
            } else if let Some(defaults_dialog) = project_dialog.defaults_dialog.as_mut() {
                handled = match current_focus_id {
                    Some(FOCUS_ID_PROJECT_DEFAULTS_BASE_BRANCH_INPUT) => {
                        Self::paste_text_input(&mut defaults_dialog.base_branch_input, text)
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_INIT_COMMAND_INPUT) => Self::paste_text_input(
                        &mut defaults_dialog.workspace_init_command_input,
                        text,
                    ),
                    Some(FOCUS_ID_PROJECT_DEFAULTS_CLAUDE_ENV_INPUT) => {
                        Self::paste_text_input(&mut defaults_dialog.claude_env_input, text)
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_CODEX_ENV_INPUT) => {
                        Self::paste_text_input(&mut defaults_dialog.codex_env_input, text)
                    }
                    Some(FOCUS_ID_PROJECT_DEFAULTS_SAVE_BUTTON)
                    | Some(FOCUS_ID_PROJECT_DEFAULTS_CANCEL_BUTTON) => false,
                    _ => false,
                };
            } else {
                handled = Self::paste_text_input(&mut project_dialog.filter_input, text);
                if handled {
                    post_action = PostAction::RefreshProjectFilter;
                }
            }
        }

        match post_action {
            PostAction::None => {}
            PostAction::RefreshProjectFilter => self.refresh_project_dialog_filtered(),
            PostAction::RefreshProjectAddMatches => self.refresh_project_add_dialog_matches(),
        }

        handled
    }

    pub(super) fn handle_paste_event(&mut self, paste_event: PasteEvent) -> Cmd<Msg> {
        if self.apply_paste_to_create_dialog(&paste_event.text) {
            return Cmd::None;
        }
        if self.apply_paste_to_project_dialog(&paste_event.text) {
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
        if self.selected_attention_item.is_some() {
            let _ = self.focus_main_pane(FOCUS_ID_PREVIEW);
            self.focus_selected_workspace_attention_tab();
            self.selected_attention_item = None;
            self.poll_preview();
            return;
        }
        if !self.enter_interactive(Instant::now()) {
            let _ = self.focus_main_pane(FOCUS_ID_PREVIEW);
            self.acknowledge_selected_workspace_attention_for_preview_focus();
            self.poll_preview();
        }
    }

    fn non_interactive_command_for_key(&self, key_event: &KeyEvent) -> Option<UiCommand> {
        let in_preview_focus = self.preview_focused();
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
            UiCommand::MoveTabLeft | UiCommand::MoveTabRight => {
                in_preview_focus
                    && self
                        .selected_active_tab()
                        .is_some_and(|tab| tab.kind != WorkspaceTabKind::Home)
            }
            UiCommand::AddWorktree | UiCommand::DeleteWorkspace | UiCommand::DeleteWorktree => {
                self.workspace_list_focused()
            }
            UiCommand::OpenRepository => {
                self.workspace_list_focused() && self.state.selected_workspace().is_some()
            }
            UiCommand::OpenDiffTab => in_preview_focus,
            UiCommand::RenameActiveTab
            | UiCommand::StopAgent
            | UiCommand::RestartAgent
            | UiCommand::StartParentAgent
            | UiCommand::StartAgent => {
                in_preview_focus
                    && match command {
                        UiCommand::StartAgent => self.state.selected_workspace().is_some(),
                        UiCommand::RenameActiveTab => self
                            .selected_active_tab()
                            .is_some_and(|tab| tab.kind != WorkspaceTabKind::Home),
                        UiCommand::StopAgent => self.active_tab_session_name().is_some(),
                        UiCommand::RestartAgent => self
                            .selected_active_tab()
                            .is_some_and(|tab| tab.kind != WorkspaceTabKind::Home),
                        UiCommand::StartParentAgent => self.selected_home_tab_targets_task_root(),
                        _ => false,
                    }
            }
            UiCommand::ResizeSidebarNarrower
            | UiCommand::ResizeSidebarWider
            | UiCommand::FocusPreview
            | UiCommand::DeleteProject
            | UiCommand::OpenCommandPalette => false,
            UiCommand::FocusAttentionInbox => !in_preview_focus,
            UiCommand::ReorderTasks => self.workspace_list_focused(),
            _ => true,
        }
    }

    fn handle_main_pane_arrow_key(&mut self, key_event: &KeyEvent) -> bool {
        if !key_event.modifiers.is_empty() || self.modal_open() {
            return false;
        }

        let dir = match key_event.code {
            KeyCode::Left => NavDirection::Left,
            KeyCode::Right => NavDirection::Right,
            _ => return false,
        };
        let mode_before = self.state.mode;
        let focus_before = self.focus_manager.current();
        let moved = self.navigate_main_panes(dir);
        if moved
            && (self.state.mode != mode_before || self.focus_manager.current() != focus_before)
            && self.preview_focused()
        {
            self.acknowledge_selected_workspace_attention_for_preview_focus();
            self.poll_preview();
        }
        moved
    }

    fn handle_non_interactive_key(&mut self, key_event: KeyEvent) -> bool {
        if self.attention_ack_key_pressed(&key_event) {
            self.acknowledge_selected_attention_item();
            return false;
        }
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
        if self.rename_tab_dialog().is_some() {
            self.handle_rename_tab_dialog_key(*key_event);
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
        if self.pull_upstream_dialog().is_some() {
            self.handle_pull_upstream_dialog_key(*key_event);
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
            let palette_mode = self.dialogs.palette_mode;
            let workspace_jump_query_is_empty = self.dialogs.command_palette.query().trim().is_empty();
            let event = Event::Key(Self::normalize_command_palette_key_event(
                Self::remap_command_palette_nav_key(key_event),
            ));
            if let Some(action) = self.dialogs.command_palette.handle_event(&event) {
                return match action {
                    PaletteAction::Dismiss => {
                        self.dialogs.palette_mode = None;
                        (false, Cmd::None)
                    }
                    PaletteAction::Execute(id) => {
                        let handled = self.execute_visible_palette_action(id.as_str());
                        if matches!(palette_mode, Some(PaletteMode::WorkspaceJump))
                            && !workspace_jump_query_is_empty
                        {
                            self.selected_attention_item = None;
                        }
                        self.dialogs.palette_mode = None;
                        (handled, Cmd::None)
                    }
                };
            }
            return (false, Cmd::None);
        }

        if self.task_reorder_active() {
            match key_event.code {
                KeyCode::Escape => self.cancel_task_reorder(),
                KeyCode::Enter => self.save_task_reorder(),
                KeyCode::Char('k') if key_event.modifiers.is_empty() => {
                    self.move_selected_task_in_reorder(-1);
                }
                KeyCode::Char('j') if key_event.modifiers.is_empty() => {
                    self.move_selected_task_in_reorder(1);
                }
                KeyCode::Up | KeyCode::BackTab => self.move_selected_task_in_reorder(-1),
                KeyCode::Down | KeyCode::Tab => self.move_selected_task_in_reorder(1),
                KeyCode::Char(_) if Self::is_ctrl_modal_nav_key(&key_event) => {
                    if matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N')) {
                        self.move_selected_task_in_reorder(1);
                    } else {
                        self.move_selected_task_in_reorder(-1);
                    }
                }
                _ => {}
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

        if self.attention_ack_key_pressed(&key_event) {
            self.acknowledge_selected_attention_item();
            return (false, Cmd::None);
        }

        if self.session.interactive.is_some() {
            return (false, self.handle_interactive_key(key_event));
        }

        if self.handle_main_pane_arrow_key(&key_event) {
            return (false, Cmd::None);
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
        self.preview_focused()
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
        let row_map = self.sidebar_selectable_row_map();
        if row_map.is_empty() {
            return;
        }

        let current_target = self
            .selected_attention_item
            .map(SidebarSelectable::Attention)
            .unwrap_or(SidebarSelectable::Workspace(self.state.selected_index));
        let Some(current_line) = row_map
            .iter()
            .position(|entry| entry.is_some_and(|target| target == current_target))
        else {
            return;
        };

        let direction: isize = match action {
            Action::MoveSelectionUp => -1,
            Action::MoveSelectionDown => 1,
            #[cfg(test)]
            Action::EnterPreviewMode | Action::EnterListMode => return,
        };
        let len = row_map.len();
        let mut candidate = current_line;
        loop {
            candidate = if direction > 0 {
                if candidate + 1 >= len {
                    0
                } else {
                    candidate + 1
                }
            } else if candidate == 0 {
                len - 1
            } else {
                candidate - 1
            };
            if candidate == current_line {
                return;
            }
            if let Some(target) = row_map[candidate] {
                self.select_sidebar_target(target);
                return;
            }
        }
    }

    pub(super) fn handle_workspace_selection_changed(&mut self) {
        self.record_selected_workspace_visit();
        if self.session.interactive.is_some() {
            self.exit_interactive_to_list();
        }
        self.sync_preview_tab_from_active_workspace_tab();
        let preview_height = self
            .preview_output_dimensions()
            .map_or(1, |(_, height)| usize::from(height));
        self.preview_scroll_to_bottom(preview_height);
        self.clear_agent_activity_tracking();
        self.clear_preview_selection();
        if self.selected_live_preview_session_if_ready().is_none() {
            self.refresh_preview_summary();
        }
        self.poll_preview_prioritized();
    }
}

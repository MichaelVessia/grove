use super::*;

impl GroveApp {
    pub(super) fn handle_paste_event(&mut self, paste_event: PasteEvent) -> Cmd<Msg> {
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
                    Value::from(
                        u64::try_from(paste_event.text.chars().count()).unwrap_or(u64::MAX),
                    ),
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
        let in_preview_agent = in_preview_focus && self.preview_tab == PreviewTab::Agent;
        let can_enter_interactive = workspace_can_enter_interactive(
            self.state.selected_workspace(),
            self.preview_tab == PreviewTab::Git,
        );

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
                    matches!(key_event.code, KeyCode::Char('k') | KeyCode::Up) && in_preview_agent
                }
                UiCommand::ScrollDown => {
                    matches!(key_event.code, KeyCode::Char('j') | KeyCode::Down) && in_preview_agent
                }
                UiCommand::PageUp => matches!(key_event.code, KeyCode::PageUp) && in_preview_agent,
                UiCommand::PageDown => {
                    matches!(key_event.code, KeyCode::PageDown) && in_preview_agent
                }
                UiCommand::ScrollBottom => {
                    matches!(key_event.code, KeyCode::Char('G')) && in_preview_agent
                }
                UiCommand::PreviousTab => {
                    matches!(key_event.code, KeyCode::Char('[')) && in_preview_focus
                }
                UiCommand::NextTab => {
                    matches!(key_event.code, KeyCode::Char(']')) && in_preview_focus
                }
                UiCommand::NewWorkspace => {
                    matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'))
                }
                UiCommand::EditWorkspace => {
                    matches!(key_event.code, KeyCode::Char('e') | KeyCode::Char('E'))
                }
                UiCommand::StartAgent => matches!(key_event.code, KeyCode::Char('s')),
                UiCommand::StopAgent => matches!(key_event.code, KeyCode::Char('x')),
                UiCommand::DeleteWorkspace => matches!(key_event.code, KeyCode::Char('D')),
                UiCommand::MergeWorkspace => matches!(key_event.code, KeyCode::Char('m')),
                UiCommand::UpdateFromBase => matches!(key_event.code, KeyCode::Char('u')),
                UiCommand::OpenProjects => {
                    matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'))
                }
                UiCommand::DeleteProject => false,
                UiCommand::OpenSettings => matches!(key_event.code, KeyCode::Char('S')),
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
            let event = Event::Key(key_event);
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

        if self.interactive.is_some() {
            return (false, self.handle_interactive_key(key_event));
        }

        if self.create_dialog.is_some()
            && key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('p'))
        {
            self.handle_create_dialog_key(key_event);
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

        if self.create_dialog.is_some() {
            self.handle_create_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.edit_dialog.is_some() {
            self.handle_edit_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.launch_dialog.is_some() {
            self.handle_launch_dialog_key(key_event);
            return (false, Cmd::None);
        }

        if self.delete_dialog.is_some() {
            self.handle_delete_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.merge_dialog.is_some() {
            self.handle_merge_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.update_from_base_dialog.is_some() {
            self.handle_update_from_base_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.project_dialog.is_some() {
            self.handle_project_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.settings_dialog.is_some() {
            self.handle_settings_dialog_key(key_event);
            return (false, Cmd::None);
        }
        if self.keybind_help_open {
            self.handle_keybind_help_key(key_event);
            return (false, Cmd::None);
        }

        (self.handle_non_interactive_key(key_event), Cmd::None)
    }
}

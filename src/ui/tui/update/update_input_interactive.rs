use super::update_prelude::*;

impl GroveApp {
    pub(super) fn exit_interactive_to_list(&mut self) {
        self.session.interactive = None;
        self.begin_interactive_preview_reset();
        let _ = self.focus_main_pane(FOCUS_ID_WORKSPACE_LIST);
        self.clear_preview_selection();
    }

    pub(super) fn exit_interactive_to_preview(&mut self) {
        self.session.interactive = None;
        self.begin_interactive_preview_reset();
        let _ = self.focus_main_pane(FOCUS_ID_PREVIEW);
        self.clear_preview_selection();
    }

    fn map_interactive_key(key_event: KeyEvent) -> Option<InteractiveKey> {
        let ctrl = key_event.modifiers.contains(Modifiers::CTRL);
        let alt = key_event.modifiers.contains(Modifiers::ALT);
        let shift = key_event.modifiers.contains(Modifiers::SHIFT);

        match key_event.code {
            KeyCode::Enter => {
                if ctrl || alt || shift {
                    return Some(InteractiveKey::ModifiedEnter { shift, alt, ctrl });
                }
                Some(InteractiveKey::Enter)
            }
            KeyCode::Tab => Some(InteractiveKey::Tab),
            KeyCode::BackTab => Some(InteractiveKey::BackTab),
            KeyCode::Backspace => Some(InteractiveKey::Backspace),
            KeyCode::Delete => Some(InteractiveKey::Delete),
            KeyCode::Up => Some(InteractiveKey::Up),
            KeyCode::Down => Some(InteractiveKey::Down),
            KeyCode::Left => Some(InteractiveKey::Left),
            KeyCode::Right => Some(InteractiveKey::Right),
            KeyCode::Home => Some(InteractiveKey::Home),
            KeyCode::End => Some(InteractiveKey::End),
            KeyCode::PageUp => Some(InteractiveKey::PageUp),
            KeyCode::PageDown => Some(InteractiveKey::PageDown),
            KeyCode::Escape => Some(InteractiveKey::Escape),
            KeyCode::F(index) => Some(InteractiveKey::Function(index)),
            KeyCode::Char(character) => {
                if (ctrl && matches!(character, '\\' | '|' | '4')) || character == '\u{1c}' {
                    return Some(InteractiveKey::CtrlBackslash);
                }
                if alt && matches!(character, 'c' | 'C') {
                    return Some(InteractiveKey::AltC);
                }
                if alt && matches!(character, 'v' | 'V') {
                    return Some(InteractiveKey::AltV);
                }
                if ctrl {
                    return Some(InteractiveKey::Ctrl(character));
                }
                Some(InteractiveKey::Char(character))
            }
            _ => None,
        }
    }
    pub(super) fn handle_interactive_key(&mut self, key_event: KeyEvent) -> Cmd<Msg> {
        let now = Instant::now();
        let input_seq = self.next_input_seq();

        let Some(interactive_key) = Self::map_interactive_key(key_event) else {
            self.log_input_event_with_fields(
                "interactive_key_unmapped",
                input_seq,
                vec![(
                    "code".to_string(),
                    Value::from(format!("{:?}", key_event.code)),
                )],
            );
            return Cmd::None;
        };
        self.log_input_event_with_fields(
            "interactive_key_received",
            input_seq,
            vec![
                (
                    "key".to_string(),
                    Value::from(Self::interactive_key_kind(&interactive_key)),
                ),
                (
                    "repeat".to_string(),
                    Value::from(matches!(key_event.kind, KeyEventKind::Repeat)),
                ),
            ],
        );

        let (action, target_session, bracketed_paste) = {
            let Some(state) = self.session.interactive.as_mut() else {
                return Cmd::None;
            };
            let action = state.handle_key(interactive_key, now);
            let session = state.target_session.clone();
            let bracketed_paste = state.bracketed_paste;
            (action, session, bracketed_paste)
        };
        self.log_input_event_with_fields(
            "interactive_action_selected",
            input_seq,
            vec![
                (
                    "action".to_string(),
                    Value::from(Self::interactive_action_kind(&action)),
                ),
                ("session".to_string(), Value::from(target_session.clone())),
            ],
        );
        let trace_context = InputTraceContext {
            seq: input_seq,
            received_at: now,
        };

        match action {
            InteractiveAction::ExitInteractive => {
                self.exit_interactive_to_preview();
                Cmd::None
            }
            InteractiveAction::CopySelection => {
                self.copy_interactive_capture();
                Cmd::None
            }
            InteractiveAction::PasteClipboard => {
                let preview_height = self
                    .preview_output_dimensions()
                    .map_or(1, |(_, height)| usize::from(height));
                if !self.preview_auto_scroll_for_height(preview_height) {
                    self.jump_preview_to_bottom();
                }
                let send_cmd = self.paste_clipboard_text(
                    &target_session,
                    bracketed_paste,
                    Some(trace_context),
                );
                self.schedule_interactive_debounced_poll(now);
                send_cmd
            }
            InteractiveAction::Noop
            | InteractiveAction::SendNamed(_)
            | InteractiveAction::SendLiteral(_) => {
                let send_cmd =
                    self.send_interactive_action(&action, &target_session, Some(trace_context));
                self.schedule_interactive_debounced_poll(now);
                send_cmd
            }
        }
    }

    fn control_character_for(character: char) -> Option<char> {
        let normalized = character.to_ascii_lowercase();
        if !normalized.is_ascii_lowercase() {
            return None;
        }

        let normalized_code = u32::from(normalized);
        let a_code = u32::from('a');
        let offset = normalized_code.checked_sub(a_code)?;
        let control_code = offset.checked_add(1)?;
        char::from_u32(control_code)
    }

    pub(super) fn is_ctrl_char_key(key_event: &KeyEvent, character: char) -> bool {
        if key_event.kind != KeyEventKind::Press {
            return false;
        }

        let KeyCode::Char(value) = key_event.code else {
            return false;
        };
        if value.eq_ignore_ascii_case(&character) && key_event.modifiers == Modifiers::CTRL {
            return true;
        }

        let Some(control_character) = Self::control_character_for(character) else {
            return false;
        };
        value == control_character
            && (key_event.modifiers.is_empty() || key_event.modifiers == Modifiers::CTRL)
    }

    pub(super) fn enter_interactive(&mut self, now: Instant) -> bool {
        let session_name = match self.preview_tab {
            PreviewTab::Home => {
                let Some(session_name) = self.ensure_agent_preview_session_for_interactive() else {
                    return false;
                };
                session_name
            }
            PreviewTab::Git => {
                let Some(target) = self.prepare_live_preview_session() else {
                    return false;
                };
                target.session_name
            }
            PreviewTab::Shell => {
                let Some(session_name) = self.ensure_shell_preview_session_for_interactive() else {
                    return false;
                };
                session_name
            }
            PreviewTab::Agent => {
                let Some(session_name) = self.ensure_agent_preview_session_for_interactive() else {
                    return false;
                };
                session_name
            }
            PreviewTab::Diff => return false,
        };

        self.session.interactive = Some(InteractiveState::new(
            "%0".to_string(),
            session_name,
            now,
            self.viewport_height,
            self.viewport_width,
        ));
        self.polling.interactive_poll_due_at = None;
        self.session.last_tmux_error = None;
        let _ = self.focus_main_pane(FOCUS_ID_PREVIEW);
        self.clear_preview_selection();
        self.sync_interactive_session_geometry();
        self.poll_preview();
        true
    }
}

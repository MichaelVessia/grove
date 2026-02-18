use super::*;

impl GroveApp {
    pub(super) fn render_launch_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.launch_dialog.as_ref() else {
            return;
        };
        if area.width < 20 || area.height < 11 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(100);
        let dialog_height = 11u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let unsafe_focused = focused(LaunchDialogField::Unsafe);
        let unsafe_state = if dialog.skip_permissions {
            "on, bypass approvals and sandbox"
        } else {
            "off, standard safety checks"
        };
        let start_focused = focused(LaunchDialogField::StartButton);
        let cancel_focused = focused(LaunchDialogField::CancelButton);
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Launch profile", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Prompt",
                dialog.prompt.as_str(),
                "Describe initial task for the agent",
                focused(LaunchDialogField::Prompt),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "PreLaunch",
                dialog.pre_launch_command.as_str(),
                "Optional command to run before launch",
                focused(LaunchDialogField::PreLaunchCommand),
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "Unsafe",
                unsafe_state,
                unsafe_focused,
                theme.peach,
                if dialog.skip_permissions {
                    theme.red
                } else {
                    theme.text
                },
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Start",
                "Cancel",
                start_focused,
                cancel_focused,
            ),
        ];
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, Space toggle unsafe, Enter start, Esc cancel",
        ));
        let body = FtText::from_lines(lines);
        let content = OverlayModalContent {
            title: "Start Agent",
            body,
            theme,
            border_color: theme.mauve,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(theme.crust, 0.55))
            .hit_id(HitId::new(HIT_ID_LAUNCH_DIALOG))
            .render(area, frame);
    }
}

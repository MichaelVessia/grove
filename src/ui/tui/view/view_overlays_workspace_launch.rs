use super::*;

impl GroveApp {
    pub(super) fn render_launch_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.launch_dialog() else {
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
        let start_focused = focused(LaunchDialogField::StartButton);
        let cancel_focused = focused(LaunchDialogField::CancelButton);
        let config_rows =
            modal_start_agent_config_rows(content_width, theme, &dialog.start_config, |field| {
                focused(LaunchDialogField::StartConfig(field))
            });
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Launch profile", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            config_rows[0].clone(),
            config_rows[1].clone(),
            config_rows[2].clone(),
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
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Start Agent",
                theme,
                border_color: theme.mauve,
                hit_id: HIT_ID_LAUNCH_DIALOG,
            },
        );
    }
}

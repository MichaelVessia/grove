use super::*;

impl GroveApp {
    pub(super) fn render_settings_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.settings_dialog.as_ref() else {
            return;
        };
        if area.width < 40 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(12).min(72);
        let dialog_height = 12u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let current = dialog.multiplexer.label();
        let multiplexer_focused = focused(SettingsDialogField::Multiplexer);
        let save_focused = focused(SettingsDialogField::SaveButton);
        let cancel_focused = focused(SettingsDialogField::CancelButton);
        let body = FtText::from_lines(vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("Global settings", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_focus_badged_row(
                content_width,
                theme,
                "Multiplexer",
                current,
                multiplexer_focused,
                theme.blue,
                theme.text,
            ),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width("  h/l, Left/Right, Space cycles", content_width),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "  Switching requires restarting running workspaces",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "Save",
                "Cancel",
                save_focused,
                cancel_focused,
            ),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Saved to ~/.config/grove/config.toml",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
        ]);

        let content = OverlayModalContent {
            title: "Settings",
            body,
            theme,
            border_color: theme.teal,
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
            .hit_id(HitId::new(HIT_ID_SETTINGS_DIALOG))
            .render(area, frame);
    }
}

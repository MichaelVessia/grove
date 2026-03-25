use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_settings_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.settings_dialog() else {
            return;
        };
        if area.width < 40 || area.height < 15 {
            return;
        }

        let dialog_width = area.width.saturating_sub(12).min(72);
        let dialog_height = 15u16;
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let theme_focused = focused(SettingsDialogField::Theme);
        let save_focused = focused(SettingsDialogField::SaveButton);
        let cancel_focused = focused(SettingsDialogField::CancelButton);
        let theme_value = format!(
            "{} ({})",
            theme_display_name(dialog.theme),
            dialog.theme.config_key()
        );
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Global settings"),
                Style::new().fg(packed(theme.border)),
            )]),
            FtLine::raw(""),
        ];
        lines.push(modal_focus_badged_row(
            content_width,
            theme,
            "Theme",
            theme_value.as_str(),
            theme_focused,
            packed(theme.primary),
            packed(theme.text),
        ));
        lines.push(FtLine::raw(""));
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Save",
            "Cancel",
            save_focused,
            cancel_focused,
        ));
        lines.push(FtLine::raw(""));
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Use left/right or space to cycle built-in themes.",
        ));
        let body = FtText::from_lines(lines);

        let content = OverlayModalContent {
            title: "Settings",
            body,
            theme,
            border_color: packed(theme.info),
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(packed(theme.background), 0.55))
            .hit_id(HitId::new(HIT_ID_SETTINGS_DIALOG))
            .render(area, frame);
    }
}

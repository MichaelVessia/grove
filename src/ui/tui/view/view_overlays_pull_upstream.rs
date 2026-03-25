use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_pull_upstream_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.pull_upstream_dialog() else {
            return;
        };
        if area.width < 28 || area.height < 12 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(96);
        let has_propagate = dialog.propagate_target_count > 0;
        let dialog_height = if has_propagate { 15u16 } else { 14u16 };
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let pull_focused = focused(PullUpstreamDialogField::PullButton);
        let cancel_focused = focused(PullUpstreamDialogField::CancelButton);
        let path = dialog.workspace_path.display().to_string();
        let strategy = format!(
            "  Strategy: git pull --ff-only origin {}",
            dialog.base_branch
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
                fit("Pull plan"),
                Style::new().fg(packed(theme.border)),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Name",
                dialog.workspace_name.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Branch",
                dialog.base_branch.as_str(),
                packed(theme.primary),
                packed(theme.text),
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "Path",
                path.as_str(),
                packed(theme.primary),
                packed(theme.border),
            ),
        ];
        lines.extend(modal_wrapped_rows(
            content_width,
            strategy.as_str(),
            Style::new()
                .fg(packed(theme.text_subtle))
                .bg(packed(theme.background)),
        ));
        if has_propagate {
            let propagate_text = format!(
                "  After pull, {} workspace(s) can be updated from base",
                dialog.propagate_target_count
            );
            lines.extend(modal_wrapped_rows(
                content_width,
                propagate_text.as_str(),
                Style::new()
                    .fg(packed(theme.warning))
                    .bg(packed(theme.background)),
            ));
        }
        lines.push(FtLine::raw(""));
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Pull",
            "Cancel",
            pull_focused,
            cancel_focused,
        ));
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, h/l switch buttons, Enter or p pull, Esc cancel",
        ));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Pull Upstream",
                theme,
                border_color: packed(theme.info),
                hit_id: HIT_ID_PULL_UPSTREAM_DIALOG,
            },
        );
    }
}

use super::view_prelude::*;

impl GroveApp {
    pub(super) fn render_launch_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.launch_dialog() else {
            return;
        };
        if area.width < 20 || area.height < 16 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(100);
        let dialog_height = 16u16;
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let title = match dialog.target {
            LaunchDialogTarget::WorkspaceTab => "New Agent Tab",
            LaunchDialogTarget::ParentTask(_) => "Start Parent Agent",
        };
        let focused = |field| dialog.focused_field == field;
        let start_focused = focused(LaunchDialogField::StartButton);
        let cancel_focused = focused(LaunchDialogField::CancelButton);
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };
        let agent_row = |agent: AgentType| {
            let mut style = Style::new().fg(packed(theme.border));
            if dialog.focused_field == LaunchDialogField::Agent {
                style = style.bg(packed(theme.surface));
            }
            let marker = if dialog.agent == agent { "●" } else { "○" };
            let label = format!("{marker} {}", agent.label());
            FtLine::from_spans(vec![FtSpan::styled(
                fit(label.as_str()),
                if dialog.agent == agent {
                    style.fg(self.workspace_agent_color(agent)).bold()
                } else {
                    style
                },
            )])
        };
        let config_rows =
            modal_start_agent_config_rows(content_width, theme, &dialog.start_config, |field| {
                focused(LaunchDialogField::StartConfig(field))
            });
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Launch profile"),
                Style::new().fg(packed(theme.border)),
            )]),
            FtLine::raw(""),
            FtLine::from_spans(vec![FtSpan::styled(
                fit("Agent"),
                Style::new().fg(packed(theme.border)),
            )]),
            agent_row(AgentType::Claude),
            agent_row(AgentType::Codex),
            agent_row(AgentType::OpenCode),
            FtLine::raw(""),
            config_rows[0].clone(),
            config_rows[1].clone(),
            config_rows[2].clone(),
            config_rows[3].clone(),
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
            "Tab/C-n next, S-Tab/C-p prev, j/k or h/l choose agent, type Name/Prompt/InitCmd, Space toggle unsafe, Enter start, Esc cancel",
        ));
        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title,
                theme,
                border_color: packed(theme.secondary),
                hit_id: HIT_ID_LAUNCH_DIALOG,
            },
        );
    }
}

use super::view_prelude::*;

fn cleanup_reason_label(reason: SessionCleanupReason) -> &'static str {
    match reason {
        SessionCleanupReason::Orphaned => "orphan",
        SessionCleanupReason::StaleAuxiliary => "stale",
    }
}

fn cleanup_age_label(age_secs: Option<u64>) -> String {
    let Some(age_secs) = age_secs else {
        return "unknown".to_string();
    };
    if age_secs < 60 {
        return format!("{age_secs}s");
    }
    if age_secs < 60 * 60 {
        return format!("{}m", age_secs / 60);
    }
    if age_secs < 24 * 60 * 60 {
        return format!("{}h", age_secs / (60 * 60));
    }
    format!("{}d", age_secs / (24 * 60 * 60))
}

impl GroveApp {
    pub(super) fn render_session_cleanup_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.session_cleanup_dialog() else {
            return;
        };
        if area.width < 36 || area.height < 14 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).clamp(70, 116);
        let dialog_height = area.height.saturating_sub(4).clamp(16, 26);
        let theme = self.active_ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let include_stale_focused = focused(SessionCleanupDialogField::IncludeStale);
        let include_attached_focused = focused(SessionCleanupDialogField::IncludeAttached);
        let apply_focused = focused(SessionCleanupDialogField::ApplyButton);
        let cancel_focused = focused(SessionCleanupDialogField::CancelButton);
        let include_stale_state = if dialog.options.include_stale {
            "enabled, include stale -git/-shell sessions".to_string()
        } else {
            "disabled, orphaned sessions only".to_string()
        };
        let include_attached_state = if dialog.options.include_attached {
            "enabled, include attached sessions".to_string()
        } else {
            "disabled, protect attached sessions".to_string()
        };
        let fit = |text: &str| {
            let text = ftui::text::truncate_with_ellipsis(text, content_width, "…");
            format!(
                "{text}{}",
                " ".repeat(content_width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };
        let fit_nested = |text: &str| {
            let width = content_width.saturating_sub(2);
            let text = ftui::text::truncate_with_ellipsis(text, width, "…");
            format!(
                "{text}{}",
                " ".repeat(width.saturating_sub(ftui::text::display_width(text.as_str())))
            )
        };

        let mut lines = vec![FtLine::from_spans(vec![FtSpan::styled(
            fit(format!(
                "Candidates: {} · Skipped attached: {}",
                dialog.plan.candidates.len(),
                dialog.plan.skipped_attached.len()
            )
            .as_str()),
            Style::new().fg(packed(theme.border)),
        )])];
        if let Some(error) = dialog.last_error.as_ref() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                fit(format!("Error: {error}").as_str()),
                Style::new().fg(packed(theme.error)).bold(),
            )]));
        }
        lines.push(FtLine::raw(""));
        lines.push(FtLine::from_spans(vec![FtSpan::styled(
            fit("Planned session cleanup"),
            Style::new().fg(packed(theme.text_muted)).bold(),
        )]));

        let max_list_rows = usize::from(dialog_height).saturating_sub(13).max(3);
        if dialog.plan.candidates.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                fit("  none"),
                Style::new().fg(packed(theme.text_subtle)),
            )]));
        } else {
            for entry in dialog.plan.candidates.iter().take(max_list_rows) {
                let reason = cleanup_reason_label(entry.reason);
                let age = cleanup_age_label(entry.age_secs);
                let line = format!(
                    "  {} [{reason}] age={age} attached={}",
                    entry.session_name, entry.attached_clients
                );
                let reason_color = if entry.reason == SessionCleanupReason::Orphaned {
                    packed(theme.error)
                } else {
                    packed(theme.accent)
                };
                lines.push(FtLine::from_spans(vec![
                    FtSpan::styled("  ", Style::new().fg(packed(theme.text_subtle))),
                    FtSpan::styled(fit_nested(line.trim_start()), Style::new().fg(reason_color)),
                ]));
            }
            if dialog.plan.candidates.len() > max_list_rows {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    fit(format!(
                        "  ... +{} more",
                        dialog.plan.candidates.len().saturating_sub(max_list_rows)
                    )
                    .as_str()),
                    Style::new().fg(packed(theme.border)),
                )]));
            }
        }

        lines.push(FtLine::raw(""));
        lines.push(modal_focus_badged_row(
            content_width,
            theme,
            "IncludeStale",
            include_stale_state.as_str(),
            include_stale_focused,
            packed(theme.accent),
            if dialog.options.include_stale {
                packed(theme.error)
            } else {
                packed(theme.text)
            },
        ));
        lines.push(modal_focus_badged_row(
            content_width,
            theme,
            "IncludeAttached",
            include_attached_state.as_str(),
            include_attached_focused,
            packed(theme.accent),
            if dialog.options.include_attached {
                packed(theme.error)
            } else {
                packed(theme.text)
            },
        ));
        lines.push(FtLine::raw(""));

        let apply_label = if dialog.plan.candidates.is_empty() {
            "Apply"
        } else {
            "Kill Sessions"
        };
        lines.push(modal_actions_row(
            content_width,
            theme,
            apply_label,
            "Cancel",
            apply_focused,
            cancel_focused,
        ));
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, Space toggle option, Enter or D apply, Esc cancel",
        ));

        let body = FtText::from_lines(lines);
        render_modal_dialog(
            frame,
            area,
            body,
            ModalDialogSpec {
                dialog_width,
                dialog_height,
                title: "Cleanup Sessions",
                theme,
                border_color: packed(theme.warning),
                hit_id: HIT_ID_SESSION_CLEANUP_DIALOG,
            },
        );
    }
}

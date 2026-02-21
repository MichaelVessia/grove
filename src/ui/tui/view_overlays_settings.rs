use super::*;

impl GroveApp {
    pub(super) fn render_settings_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.settings_dialog() else {
            return;
        };
        if area.width < 56 || area.height < 22 {
            return;
        }

        let dialog_width = area.width.saturating_sub(8).min(96);
        let dialog_height = 24u16;
        let theme = ui_theme();
        let content_width = usize::from(dialog_width.saturating_sub(2));
        let focused = |field| dialog.focused_field == field;
        let selected_name = dialog.remote_name.trim();
        let selected_status = if selected_name.is_empty() {
            "offline".to_string()
        } else {
            self.remote_status_for(selected_name).label().to_string()
        };
        let active_remote = self
            .active_remote_profile
            .clone()
            .unwrap_or_else(|| "(none)".to_string());
        let mut lines = vec![
            FtLine::from_spans(vec![FtSpan::styled(
                pad_or_truncate_to_display_width(
                    "Global settings, remotes via SSH tunnel socket path",
                    content_width,
                ),
                Style::new().fg(theme.overlay0),
            )]),
            FtLine::raw(""),
            modal_static_badged_row(
                content_width,
                theme,
                "Profiles",
                format!("{}", self.remote_profiles.len()).as_str(),
                theme.teal,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "ActiveRemote",
                active_remote.as_str(),
                theme.mauve,
                theme.text,
            ),
            modal_static_badged_row(
                content_width,
                theme,
                "ProfileStatus",
                selected_status.as_str(),
                theme.peach,
                theme.text,
            ),
            FtLine::raw(""),
            modal_labeled_input_row(
                content_width,
                theme,
                "Name",
                dialog.remote_name.as_str(),
                "Remote profile name",
                focused(SettingsDialogField::RemoteName),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "Host",
                dialog.remote_host.as_str(),
                "SSH host",
                focused(SettingsDialogField::RemoteHost),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "User",
                dialog.remote_user.as_str(),
                "SSH user",
                focused(SettingsDialogField::RemoteUser),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "SocketPath",
                dialog.remote_socket_path.as_str(),
                "Optional local tunnel socket path, blank = infer from user/host",
                focused(SettingsDialogField::RemoteSocketPath),
            ),
            modal_labeled_input_row(
                content_width,
                theme,
                "DefaultRepo",
                dialog.remote_default_repo_path.as_str(),
                "Optional default repo path on remote",
                focused(SettingsDialogField::RemoteDefaultRepoPath),
            ),
            FtLine::raw(""),
            modal_actions_row(
                content_width,
                theme,
                "SaveProfile",
                "DeleteProfile",
                focused(SettingsDialogField::SaveProfileButton),
                focused(SettingsDialogField::DeleteProfileButton),
            ),
            modal_actions_row(
                content_width,
                theme,
                "Test",
                "Connect",
                focused(SettingsDialogField::TestProfileButton),
                focused(SettingsDialogField::ConnectProfileButton),
            ),
            modal_focus_badged_row(
                content_width,
                theme,
                "Disconnect",
                "Enter to disconnect selected/active profile",
                focused(SettingsDialogField::DisconnectProfileButton),
                theme.peach,
                theme.text,
            ),
            FtLine::raw(""),
        ];
        lines.push(modal_actions_row(
            content_width,
            theme,
            "Save",
            "Cancel",
            focused(SettingsDialogField::SaveButton),
            focused(SettingsDialogField::CancelButton),
        ));
        lines.push(FtLine::raw(""));
        lines.extend(modal_wrapped_hint_rows(
            content_width,
            theme,
            "Tab/C-n next, S-Tab/C-p prev, Enter applies action, Esc closes, saved in ~/.config/grove/config.toml",
        ));
        let body = FtText::from_lines(lines);

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

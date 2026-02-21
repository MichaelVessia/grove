use super::*;
use crate::infrastructure::process::stderr_or_status;

impl GroveApp {
    fn default_remote_repo_path_for_user(user: &str) -> Option<String> {
        let user = user.trim();
        if user.is_empty() {
            return None;
        }
        Some(format!("/home/{user}/"))
    }

    fn remote_default_repo_path(profile: &RemoteProfileConfig) -> Option<String> {
        trimmed_nonempty(profile.default_repo_path.as_deref().unwrap_or(""))
            .or_else(|| Self::default_remote_repo_path_for_user(profile.user.as_str()))
    }

    fn sync_default_repo_path_with_remote_user(
        dialog: &mut SettingsDialogState,
        previous_user: &str,
    ) {
        let current_value = dialog.remote_default_repo_path.trim();
        let previous_auto_value = Self::default_remote_repo_path_for_user(previous_user);
        let should_update = current_value.is_empty()
            || previous_auto_value
                .as_deref()
                .is_some_and(|value| current_value == value);
        if !should_update {
            return;
        }

        dialog.remote_default_repo_path =
            Self::default_remote_repo_path_for_user(dialog.remote_user.as_str())
                .unwrap_or_default();
    }

    fn normalized_socket_path(raw: &str) -> PathBuf {
        if let Some(stripped) = raw.strip_prefix("~/")
            && let Some(home) = dirs::home_dir()
        {
            return home.join(stripped);
        }
        PathBuf::from(raw)
    }

    fn settings_remote_profile_from_dialog(&self) -> Result<RemoteProfileConfig, String> {
        let Some(dialog) = self.settings_dialog() else {
            return Err("settings dialog is not open".to_string());
        };
        let Some(name) = trimmed_nonempty(&dialog.remote_name) else {
            return Err("remote profile name is required".to_string());
        };
        let Some(host) = trimmed_nonempty(&dialog.remote_host) else {
            return Err("remote host is required".to_string());
        };
        let Some(user) = trimmed_nonempty(&dialog.remote_user) else {
            return Err("remote user is required".to_string());
        };
        let remote_socket_path =
            trimmed_nonempty(&dialog.remote_socket_path).unwrap_or_else(|| {
                Self::default_local_tunnel_socket_path_for_user_host(user.as_str(), host.as_str())
                    .display()
                    .to_string()
            });
        let default_repo_path = trimmed_nonempty(&dialog.remote_default_repo_path)
            .or_else(|| Self::default_remote_repo_path_for_user(user.as_str()));

        Ok(RemoteProfileConfig {
            name,
            host,
            user,
            remote_socket_path,
            default_repo_path,
        })
    }

    fn settings_selected_remote_profile_name(&self) -> Option<String> {
        self.settings_dialog()
            .and_then(|dialog| trimmed_nonempty(&dialog.remote_name))
    }

    fn remote_profile_by_name(&self, name: &str) -> Option<RemoteProfileConfig> {
        self.remote_profiles
            .iter()
            .find(|profile| profile.name == name)
            .cloned()
    }

    fn tunnel_target(profile: &RemoteProfileConfig) -> String {
        format!("{}@{}", profile.user, profile.host)
    }

    fn tunnel_key_for_user_host(user: &str, host: &str) -> String {
        format!("{user}-{host}")
            .replace('/', "-")
            .replace([':', '@'], "-")
    }

    fn tunnel_key(profile: &RemoteProfileConfig) -> String {
        Self::tunnel_key_for_user_host(profile.user.as_str(), profile.host.as_str())
    }

    fn default_local_tunnel_socket_path_for_user_host(user: &str, host: &str) -> PathBuf {
        let file_name = format!("groved-{}.sock", Self::tunnel_key_for_user_host(user, host));
        if let Some(home) = dirs::home_dir() {
            return home.join(".grove").join(file_name);
        }

        std::env::temp_dir().join(file_name)
    }

    fn local_tunnel_socket_path(profile: &RemoteProfileConfig) -> PathBuf {
        if let Some(socket_path) = trimmed_nonempty(profile.remote_socket_path.as_str()) {
            return Self::normalized_socket_path(socket_path.as_str());
        }

        Self::default_local_tunnel_socket_path_for_user_host(
            profile.user.as_str(),
            profile.host.as_str(),
        )
    }

    fn tunnel_control_path(profile: &RemoteProfileConfig) -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            return home
                .join(".grove")
                .join(format!("ssh-groved-{}.ctl", Self::tunnel_key(profile)));
        }

        std::env::temp_dir().join(format!("ssh-groved-{}.ctl", Self::tunnel_key(profile)))
    }

    fn remote_daemon_socket_path(profile: &RemoteProfileConfig) -> String {
        format!("/home/{}/.grove/groved.sock", profile.user)
    }

    fn tunnel_is_up(profile: &RemoteProfileConfig, control_path: &Path) -> bool {
        if !control_path.exists() {
            return false;
        }

        let target = Self::tunnel_target(profile);
        let output = Command::new("ssh")
            .args([
                "-S",
                control_path.to_string_lossy().as_ref(),
                "-O",
                "check",
                target.as_str(),
            ])
            .output();
        output.is_ok_and(|output| output.status.success())
    }

    fn ensure_profile_tunnel(
        &self,
        profile: &RemoteProfileConfig,
        local_socket_path: &Path,
    ) -> Result<(), String> {
        let control_path = Self::tunnel_control_path(profile);
        if Self::tunnel_is_up(profile, control_path.as_path()) {
            return Ok(());
        }

        if let Some(parent) = local_socket_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("local tunnel socket dir create failed: {error}"))?;
        }
        if let Some(parent) = control_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("ssh control dir create failed: {error}"))?;
        }

        if control_path.exists() {
            let _ = std::fs::remove_file(control_path.as_path());
        }

        let target = Self::tunnel_target(profile);
        let local_socket = local_socket_path.to_string_lossy();
        let remote_socket = Self::remote_daemon_socket_path(profile);
        let forward = format!("{local_socket}:{remote_socket}");
        let output = Command::new("ssh")
            .args([
                "-fN",
                "-M",
                "-S",
                control_path.to_string_lossy().as_ref(),
                "-o",
                "ExitOnForwardFailure=yes",
                "-o",
                "ServerAliveInterval=30",
                "-o",
                "ServerAliveCountMax=3",
                "-o",
                "ConnectTimeout=2",
                "-o",
                "ConnectionAttempts=1",
                "-o",
                "BatchMode=yes",
                "-L",
                forward.as_str(),
                target.as_str(),
            ])
            .output()
            .map_err(|error| format!("tunnel start failed: {error}"))?;
        if output.status.success() {
            return Ok(());
        }

        Err(format!(
            "tunnel start failed: {}",
            stderr_or_status(&output)
        ))
    }

    fn stop_profile_tunnel(&self, profile: &RemoteProfileConfig) -> Result<(), String> {
        let control_path = Self::tunnel_control_path(profile);
        if !control_path.exists() {
            return Ok(());
        }

        let target = Self::tunnel_target(profile);
        let output = Command::new("ssh")
            .args([
                "-S",
                control_path.to_string_lossy().as_ref(),
                "-O",
                "exit",
                target.as_str(),
            ])
            .output()
            .map_err(|error| format!("tunnel stop failed: {error}"))?;
        let _ = std::fs::remove_file(control_path.as_path());
        if output.status.success() {
            return Ok(());
        }

        Err(format!("tunnel stop failed: {}", stderr_or_status(&output)))
    }

    fn set_remote_status(&mut self, profile_name: &str, status: RemoteConnectionState) {
        self.remote_connection_state
            .insert(profile_name.to_string(), status);
    }

    pub(super) fn remote_status_for(&self, profile_name: &str) -> RemoteConnectionState {
        self.remote_connection_state
            .get(profile_name)
            .copied()
            .unwrap_or(RemoteConnectionState::Offline)
    }

    pub(super) fn apply_settings_profile_save(&mut self) {
        let profile = match self.settings_remote_profile_from_dialog() {
            Ok(profile) => profile,
            Err(error) => {
                self.show_info_toast(error);
                return;
            }
        };

        let profile_name = profile.name.clone();
        let saved_socket_path = profile.remote_socket_path.clone();
        let saved_default_repo_path = profile.default_repo_path.clone().unwrap_or_default();
        if let Some(existing) = self
            .remote_profiles
            .iter_mut()
            .find(|existing| existing.name == profile_name)
        {
            *existing = profile;
        } else {
            self.remote_profiles.push(profile);
            self.remote_profiles
                .sort_by(|left, right| left.name.cmp(&right.name));
        }
        self.remote_connection_state
            .entry(profile_name.clone())
            .or_insert(RemoteConnectionState::Offline);

        if let Err(error) = self.save_runtime_config() {
            self.show_error_toast(format!("remote profile save failed: {error}"));
            return;
        }
        if let Some(dialog) = self.settings_dialog_mut()
            && dialog.remote_socket_path.trim().is_empty()
        {
            dialog.remote_socket_path = saved_socket_path;
        }
        if let Some(dialog) = self.settings_dialog_mut()
            && dialog.remote_default_repo_path.trim().is_empty()
        {
            dialog.remote_default_repo_path = saved_default_repo_path;
        }

        self.show_success_toast(format!("remote profile '{}' saved", profile_name));
    }

    pub(super) fn apply_settings_profile_delete(&mut self) {
        let Some(profile_name) = self.settings_selected_remote_profile_name() else {
            self.show_info_toast("remote profile name is required");
            return;
        };

        let Some(index) = self
            .remote_profiles
            .iter()
            .position(|profile| profile.name == profile_name)
        else {
            self.show_info_toast("remote profile not found");
            return;
        };

        self.remote_profiles.remove(index);
        self.remote_connection_state.remove(profile_name.as_str());
        if self.active_remote_profile.as_deref() == Some(profile_name.as_str()) {
            self.active_remote_profile = None;
        }

        if let Err(error) = self.save_runtime_config() {
            self.show_error_toast(format!("remote profile delete failed: {error}"));
            return;
        }

        self.show_success_toast(format!("remote profile '{}' deleted", profile_name));
    }

    pub(super) fn apply_settings_profile_test(&mut self) {
        let profile = match self.settings_remote_profile_from_dialog() {
            Ok(dialog_profile) => self
                .remote_profile_by_name(dialog_profile.name.as_str())
                .unwrap_or(dialog_profile),
            Err(error) => {
                self.show_info_toast(error);
                return;
            }
        };
        let socket_path = Self::local_tunnel_socket_path(&profile);

        match ping_via_socket(socket_path.as_path()) {
            Ok(_protocol_version) => {
                self.set_remote_status(profile.name.as_str(), RemoteConnectionState::Connected);
                self.show_success_toast(format!("remote '{}' test succeeded", profile.name));
            }
            Err(error) => {
                let degraded = self.active_remote_profile.as_deref() == Some(profile.name.as_str());
                self.set_remote_status(
                    profile.name.as_str(),
                    if degraded {
                        RemoteConnectionState::Degraded
                    } else {
                        RemoteConnectionState::Offline
                    },
                );
                self.show_error_toast(format!("remote '{}' test failed: {error}", profile.name));
            }
        }
    }

    pub(super) fn apply_settings_profile_connect(&mut self) {
        let Some(profile_name) = self.settings_selected_remote_profile_name() else {
            self.show_info_toast("remote profile name is required");
            return;
        };
        let Some(profile) = self.remote_profile_by_name(profile_name.as_str()) else {
            self.show_info_toast("save remote profile before connect");
            return;
        };
        let socket_path = Self::local_tunnel_socket_path(&profile);

        let ping_result = match ping_via_socket(socket_path.as_path()) {
            Ok(protocol_version) => Ok(protocol_version),
            Err(first_error) => {
                let tunnel_result = self.ensure_profile_tunnel(&profile, socket_path.as_path());
                if let Err(tunnel_error) = tunnel_result {
                    Err(format!("{first_error}; {tunnel_error}"))
                } else {
                    ping_via_socket(socket_path.as_path())
                        .map_err(|second_error| format!("{first_error}; {second_error}"))
                }
            }
        };

        match ping_result {
            Ok(_protocol_version) => {
                self.active_remote_profile = Some(profile_name.clone());
                self.set_remote_status(profile_name.as_str(), RemoteConnectionState::Connected);
                if let Err(error) = self.save_runtime_config() {
                    self.show_error_toast(format!("remote connect persist failed: {error}"));
                    return;
                }
                self.show_success_toast(format!("remote '{}' connected", profile_name));
            }
            Err(error) => {
                self.active_remote_profile = Some(profile_name.clone());
                self.set_remote_status(profile_name.as_str(), RemoteConnectionState::Degraded);
                if let Err(save_error) = self.save_runtime_config() {
                    self.show_error_toast(format!("remote connect persist failed: {save_error}"));
                    return;
                }
                self.show_error_toast(format!("remote '{}' connect failed: {error}", profile_name));
            }
        }
    }

    pub(super) fn apply_settings_profile_disconnect(&mut self) {
        let target_name = self
            .settings_selected_remote_profile_name()
            .or_else(|| self.active_remote_profile.clone());
        let Some(profile_name) = target_name else {
            self.show_info_toast("no remote profile selected");
            return;
        };

        if self.active_remote_profile.as_deref() == Some(profile_name.as_str()) {
            self.active_remote_profile = None;
        }
        if let Some(profile) = self.remote_profile_by_name(profile_name.as_str()) {
            let _ = self.stop_profile_tunnel(&profile);
        }
        self.set_remote_status(profile_name.as_str(), RemoteConnectionState::Offline);

        if let Err(error) = self.save_runtime_config() {
            self.show_error_toast(format!("remote disconnect persist failed: {error}"));
            return;
        }

        self.show_success_toast(format!("remote '{}' disconnected", profile_name));
    }

    pub(super) fn handle_settings_dialog_key(&mut self, key_event: KeyEvent) {
        let Some(dialog) = self.settings_dialog_mut() else {
            return;
        };
        let ctrl_n = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('n') | KeyCode::Char('N'));
        let ctrl_p = key_event.modifiers == Modifiers::CTRL
            && matches!(key_event.code, KeyCode::Char('p') | KeyCode::Char('P'));

        enum PostAction {
            None,
            SaveProfile,
            DeleteProfile,
            TestProfile,
            ConnectProfile,
            DisconnectProfile,
            Save,
            Cancel,
        }

        let mut post_action = PostAction::None;
        match key_event.code {
            KeyCode::Escape => {
                post_action = PostAction::Cancel;
            }
            KeyCode::Tab | KeyCode::Down => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::BackTab | KeyCode::Up => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Char(_) if ctrl_n => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Char(_) if ctrl_p => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Left => {
                dialog.focused_field = dialog.focused_field.previous();
            }
            KeyCode::Right => {
                dialog.focused_field = dialog.focused_field.next();
            }
            KeyCode::Enter => match dialog.focused_field {
                SettingsDialogField::RemoteName
                | SettingsDialogField::RemoteHost
                | SettingsDialogField::RemoteUser
                | SettingsDialogField::RemoteSocketPath
                | SettingsDialogField::RemoteDefaultRepoPath => {
                    dialog.focused_field = dialog.focused_field.next();
                }
                SettingsDialogField::SaveProfileButton => post_action = PostAction::SaveProfile,
                SettingsDialogField::DeleteProfileButton => post_action = PostAction::DeleteProfile,
                SettingsDialogField::TestProfileButton => post_action = PostAction::TestProfile,
                SettingsDialogField::ConnectProfileButton => {
                    post_action = PostAction::ConnectProfile
                }
                SettingsDialogField::DisconnectProfileButton => {
                    post_action = PostAction::DisconnectProfile
                }
                SettingsDialogField::SaveButton => post_action = PostAction::Save,
                SettingsDialogField::CancelButton => post_action = PostAction::Cancel,
            },
            KeyCode::Backspace => match dialog.focused_field {
                SettingsDialogField::RemoteName => {
                    dialog.remote_name.pop();
                }
                SettingsDialogField::RemoteHost => {
                    dialog.remote_host.pop();
                }
                SettingsDialogField::RemoteUser => {
                    let previous_user = dialog.remote_user.clone();
                    dialog.remote_user.pop();
                    Self::sync_default_repo_path_with_remote_user(dialog, previous_user.as_str());
                }
                SettingsDialogField::RemoteSocketPath => {
                    dialog.remote_socket_path.pop();
                }
                SettingsDialogField::RemoteDefaultRepoPath => {
                    dialog.remote_default_repo_path.pop();
                }
                SettingsDialogField::SaveProfileButton
                | SettingsDialogField::DeleteProfileButton
                | SettingsDialogField::TestProfileButton
                | SettingsDialogField::ConnectProfileButton
                | SettingsDialogField::DisconnectProfileButton
                | SettingsDialogField::SaveButton
                | SettingsDialogField::CancelButton => {}
            },
            KeyCode::Char(character) if Self::allows_text_input_modifiers(key_event.modifiers) => {
                match dialog.focused_field {
                    SettingsDialogField::RemoteName => {
                        dialog.remote_name.push(character);
                    }
                    SettingsDialogField::RemoteHost => {
                        dialog.remote_host.push(character);
                    }
                    SettingsDialogField::RemoteUser => {
                        let previous_user = dialog.remote_user.clone();
                        dialog.remote_user.push(character);
                        Self::sync_default_repo_path_with_remote_user(
                            dialog,
                            previous_user.as_str(),
                        );
                    }
                    SettingsDialogField::RemoteSocketPath => {
                        dialog.remote_socket_path.push(character);
                    }
                    SettingsDialogField::RemoteDefaultRepoPath => {
                        dialog.remote_default_repo_path.push(character);
                    }
                    SettingsDialogField::SaveProfileButton
                    | SettingsDialogField::DeleteProfileButton
                    | SettingsDialogField::TestProfileButton
                    | SettingsDialogField::ConnectProfileButton
                    | SettingsDialogField::DisconnectProfileButton
                    | SettingsDialogField::SaveButton
                    | SettingsDialogField::CancelButton => {}
                }
            }
            _ => {}
        }

        match post_action {
            PostAction::None => {}
            PostAction::SaveProfile => self.apply_settings_profile_save(),
            PostAction::DeleteProfile => self.apply_settings_profile_delete(),
            PostAction::TestProfile => self.apply_settings_profile_test(),
            PostAction::ConnectProfile => self.apply_settings_profile_connect(),
            PostAction::DisconnectProfile => self.apply_settings_profile_disconnect(),
            PostAction::Save => self.apply_settings_dialog_save(),
            PostAction::Cancel => {
                self.log_dialog_event("settings", "dialog_cancelled");
                self.close_active_dialog();
            }
        }
    }

    pub(super) fn open_settings_dialog(&mut self) {
        if self.modal_open() {
            return;
        }
        let selected_profile = self
            .active_remote_profile
            .as_ref()
            .and_then(|name| self.remote_profile_by_name(name.as_str()))
            .or_else(|| self.remote_profiles.first().cloned());

        self.set_settings_dialog(SettingsDialogState {
            focused_field: SettingsDialogField::RemoteName,
            remote_name: selected_profile
                .as_ref()
                .map(|profile| profile.name.clone())
                .unwrap_or_default(),
            remote_host: selected_profile
                .as_ref()
                .map(|profile| profile.host.clone())
                .unwrap_or_default(),
            remote_user: selected_profile
                .as_ref()
                .map(|profile| profile.user.clone())
                .unwrap_or_default(),
            remote_socket_path: selected_profile
                .as_ref()
                .map(|profile| {
                    Self::local_tunnel_socket_path(profile)
                        .display()
                        .to_string()
                })
                .unwrap_or_default(),
            remote_default_repo_path: selected_profile
                .as_ref()
                .and_then(Self::remote_default_repo_path)
                .unwrap_or_default(),
        });
    }

    pub(super) fn apply_settings_dialog_save(&mut self) {
        if self.settings_dialog().is_none() {
            return;
        }

        if let Err(error) = self.save_runtime_config() {
            self.show_error_toast(format!("settings save failed: {error}"));
            return;
        }

        self.close_active_dialog();
        self.show_success_toast("settings saved");
    }
}

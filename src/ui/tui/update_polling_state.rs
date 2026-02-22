use super::*;

impl GroveApp {
    fn acknowledge_workspace_attention_for_path(&mut self, workspace_path: &Path) -> bool {
        let Some(marker) = self.last_attention_markers.get(workspace_path).cloned() else {
            return false;
        };
        if self
            .workspace_attention_ack_markers
            .get(workspace_path)
            .is_some_and(|saved| saved == &marker)
        {
            return false;
        }

        self.workspace_attention_ack_markers
            .insert(workspace_path.to_path_buf(), marker);
        true
    }

    pub(super) fn workspace_attention_acks_for_config(&self) -> Vec<WorkspaceAttentionAckConfig> {
        let mut entries = self
            .workspace_attention_ack_markers
            .iter()
            .map(|(workspace_path, marker)| WorkspaceAttentionAckConfig {
                workspace_path: workspace_path.clone(),
                marker: marker.clone(),
            })
            .collect::<Vec<WorkspaceAttentionAckConfig>>();
        entries.sort_by(|left, right| left.workspace_path.cmp(&right.workspace_path));
        entries
    }

    pub(super) fn runtime_config_snapshot(&self) -> GroveConfig {
        GroveConfig {
            sidebar_width_pct: self.sidebar_width_pct,
            projects: self.projects.clone(),
            remote_profiles: self.remote_profiles.clone(),
            active_remote_profile: self.active_remote_profile.clone(),
            attention_acks: self.workspace_attention_acks_for_config(),
        }
    }

    pub(super) fn save_runtime_config(&self) -> Result<(), String> {
        crate::infrastructure::config::save_to_path(
            &self.config_path,
            &self.runtime_config_snapshot(),
        )
    }

    fn refresh_workspace_attention_for_path(&mut self, workspace_path: &Path) {
        let selected_workspace_matches = self
            .state
            .selected_workspace()
            .is_some_and(|workspace| workspace.path == workspace_path);
        if selected_workspace_matches {
            self.workspace_attention.remove(workspace_path);
            if self.acknowledge_workspace_attention_for_path(workspace_path)
                && let Err(error) = self.save_runtime_config()
            {
                self.last_tmux_error = Some(format!("attention ack persist failed: {error}"));
            }
            return;
        }

        let Some(marker) = self.last_attention_markers.get(workspace_path).cloned() else {
            self.workspace_attention.remove(workspace_path);
            return;
        };

        let acknowledged = self
            .workspace_attention_ack_markers
            .get(workspace_path)
            .is_some_and(|saved| saved == &marker);
        if acknowledged {
            self.workspace_attention.remove(workspace_path);
            return;
        }

        self.workspace_attention.insert(
            workspace_path.to_path_buf(),
            WorkspaceAttention::NeedsAttention,
        );
    }

    pub(super) fn workspace_attention(&self, workspace_path: &Path) -> Option<WorkspaceAttention> {
        self.workspace_attention.get(workspace_path).copied()
    }

    pub(super) fn clear_attention_for_workspace_path(&mut self, workspace_path: &Path) {
        self.workspace_attention.remove(workspace_path);
        if self.acknowledge_workspace_attention_for_path(workspace_path)
            && let Err(error) = self.save_runtime_config()
        {
            self.last_tmux_error = Some(format!("attention ack persist failed: {error}"));
        }
    }

    pub(super) fn clear_attention_for_selected_workspace(&mut self) {
        let Some(workspace_path) = self.selected_workspace_path() else {
            return;
        };
        self.clear_attention_for_workspace_path(&workspace_path);
    }

    pub(super) fn track_workspace_status_transition(
        &mut self,
        workspace_path: &Path,
        previous_status: WorkspaceStatus,
        next_status: WorkspaceStatus,
        previous_orphaned: bool,
        next_orphaned: bool,
    ) {
        if previous_status == next_status && previous_orphaned == next_orphaned {
            return;
        }
        self.log_event_with_fields(
            "workspace_status",
            "transition",
            [
                (
                    "workspace_path".to_string(),
                    Value::from(workspace_path.display().to_string()),
                ),
                (
                    "previous_status".to_string(),
                    Value::from(Self::workspace_status_name(previous_status)),
                ),
                (
                    "next_status".to_string(),
                    Value::from(Self::workspace_status_name(next_status)),
                ),
                (
                    "previous_orphaned".to_string(),
                    Value::from(previous_orphaned),
                ),
                ("next_orphaned".to_string(), Value::from(next_orphaned)),
                (
                    "status_changed".to_string(),
                    Value::from(previous_status != next_status),
                ),
                (
                    "orphaned_changed".to_string(),
                    Value::from(previous_orphaned != next_orphaned),
                ),
            ],
        );
    }

    pub(super) fn reconcile_workspace_attention_tracking(&mut self) {
        let current_workspace_paths = self
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.path.clone())
            .collect::<Vec<PathBuf>>();
        let valid_paths = current_workspace_paths
            .iter()
            .cloned()
            .collect::<HashSet<PathBuf>>();

        self.workspace_attention
            .retain(|path, _| valid_paths.contains(path));
        self.workspace_attention_ack_markers
            .retain(|path, _| valid_paths.contains(path));
        for path in current_workspace_paths {
            self.refresh_workspace_attention_for_path(path.as_path());
        }
    }

    pub(super) fn reconcile_workspace_attention_with_markers(
        &mut self,
        markers: HashMap<PathBuf, String>,
    ) {
        self.last_attention_markers = markers.clone();
        let current_workspace_paths = self
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.path.clone())
            .collect::<Vec<PathBuf>>();
        let valid_paths = current_workspace_paths
            .iter()
            .cloned()
            .collect::<HashSet<PathBuf>>();

        self.workspace_attention
            .retain(|path, _| valid_paths.contains(path));
        self.workspace_attention_ack_markers
            .retain(|path, _| valid_paths.contains(path));
        for path in current_workspace_paths {
            self.refresh_workspace_attention_for_path_with_marker(
                path.as_path(),
                markers.get(&path),
            );
        }
    }

    fn refresh_workspace_attention_for_path_with_marker(
        &mut self,
        workspace_path: &Path,
        precomputed_marker: Option<&String>,
    ) {
        let selected_workspace_matches = self
            .state
            .selected_workspace()
            .is_some_and(|workspace| workspace.path == workspace_path);
        if selected_workspace_matches {
            self.workspace_attention.remove(workspace_path);
            if let Some(marker) = precomputed_marker
                && self
                    .workspace_attention_ack_markers
                    .get(workspace_path)
                    .is_none_or(|saved| saved != marker)
            {
                self.workspace_attention_ack_markers
                    .insert(workspace_path.to_path_buf(), marker.clone());
                if let Err(error) = self.save_runtime_config() {
                    self.last_tmux_error = Some(format!("attention ack persist failed: {error}"));
                }
            }
            return;
        }

        let Some(marker) = precomputed_marker else {
            self.workspace_attention.remove(workspace_path);
            return;
        };

        let acknowledged = self
            .workspace_attention_ack_markers
            .get(workspace_path)
            .is_some_and(|saved| saved == marker);
        if acknowledged {
            self.workspace_attention.remove(workspace_path);
            return;
        }

        self.workspace_attention.insert(
            workspace_path.to_path_buf(),
            WorkspaceAttention::NeedsAttention,
        );
    }

    fn next_poll_interval(&self) -> Duration {
        let status = self.selected_workspace_status();

        let since_last_key = self
            .interactive
            .as_ref()
            .map_or(Duration::from_secs(60), |interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
            });

        poll_interval(
            status,
            true,
            self.state.focus == PaneFocus::Preview,
            self.interactive.is_some(),
            since_last_key,
            self.output_changing,
        )
    }

    fn selected_workspace_status(&self) -> WorkspaceStatus {
        self.state
            .selected_workspace()
            .map_or(WorkspaceStatus::Unknown, |workspace| workspace.status)
    }

    pub(super) fn clear_agent_activity_tracking(&mut self) {
        self.output_changing = false;
        self.agent_output_changing = false;
        self.agent_activity_frames.clear();
    }

    pub(super) fn workspace_status_tracking_key(workspace_path: &Path) -> String {
        workspace_path.to_string_lossy().to_string()
    }

    pub(super) fn clear_status_tracking_for_workspace_path(&mut self, workspace_path: &Path) {
        let key = Self::workspace_status_tracking_key(workspace_path);
        self.workspace_status_digests.remove(&key);
        self.workspace_output_changing.remove(&key);
    }

    pub(super) fn clear_status_tracking(&mut self) {
        self.workspace_status_digests.clear();
        self.workspace_output_changing.clear();
    }

    pub(super) fn workspace_status_name(status: WorkspaceStatus) -> &'static str {
        match status {
            WorkspaceStatus::Main => "main",
            WorkspaceStatus::Idle => "idle",
            WorkspaceStatus::Active => "active",
            WorkspaceStatus::Thinking => "thinking",
            WorkspaceStatus::Waiting => "waiting",
            WorkspaceStatus::Done => "done",
            WorkspaceStatus::Error => "error",
            WorkspaceStatus::Unknown => "unknown",
            WorkspaceStatus::Unsupported => "unsupported",
        }
    }

    fn workspace_output_changing(&self, workspace_path: &Path) -> bool {
        let key = Self::workspace_status_tracking_key(workspace_path);
        self.workspace_output_changing
            .get(&key)
            .copied()
            .unwrap_or(false)
    }

    pub(super) fn push_agent_activity_frame(&mut self, changed: bool) {
        if self.agent_activity_frames.len() >= AGENT_ACTIVITY_WINDOW_FRAMES {
            self.agent_activity_frames.pop_front();
        }
        self.agent_activity_frames.push_back(changed);
    }

    fn has_recent_agent_activity(&self) -> bool {
        self.agent_activity_frames.contains(&true)
    }

    fn visual_tick_interval(&self) -> Option<Duration> {
        let selected_workspace_path = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.as_path());
        if self.status_is_visually_working(
            selected_workspace_path,
            self.selected_workspace_status(),
            true,
        ) {
            return Some(Duration::from_millis(FAST_ANIMATION_INTERVAL_MS));
        }
        None
    }

    pub(super) fn advance_visual_animation(&mut self) {
        self.fast_animation_frame = self.fast_animation_frame.wrapping_add(1);
    }

    pub(super) fn status_is_visually_working(
        &self,
        workspace_path: Option<&Path>,
        status: WorkspaceStatus,
        is_selected: bool,
    ) -> bool {
        if is_selected
            && self.interactive.as_ref().is_some_and(|interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
                    < Duration::from_millis(LOCAL_TYPING_SUPPRESS_MS)
            })
        {
            return false;
        }
        match status {
            WorkspaceStatus::Thinking => true,
            WorkspaceStatus::Active => {
                if workspace_path.is_some_and(|path| self.workspace_output_changing(path)) {
                    return true;
                }
                if is_selected {
                    return self.agent_output_changing || self.has_recent_agent_activity();
                }
                false
            }
            _ => false,
        }
    }

    pub(super) fn is_due_with_tolerance(now: Instant, due_at: Instant) -> bool {
        let tolerance = Duration::from_millis(TICK_EARLY_TOLERANCE_MS);
        let now_with_tolerance = now.checked_add(tolerance).unwrap_or(now);
        now_with_tolerance >= due_at
    }

    pub(super) fn schedule_next_tick(&mut self) -> Cmd<Msg> {
        let scheduled_at = Instant::now();
        if self.next_workspace_refresh_due_at.is_none() {
            self.next_workspace_refresh_due_at =
                Some(scheduled_at + Duration::from_millis(WORKSPACE_REFRESH_INTERVAL_MS));
        }

        let mut poll_due_at = scheduled_at + self.next_poll_interval();
        let mut source = "adaptive_poll";
        if let Some(interactive_due_at) = self.interactive_poll_due_at
            && interactive_due_at < poll_due_at
        {
            poll_due_at = interactive_due_at;
            source = "interactive_debounce";
        }

        if let Some(existing_poll_due_at) = self.next_poll_due_at
            && existing_poll_due_at <= poll_due_at
        {
            if existing_poll_due_at > scheduled_at {
                poll_due_at = existing_poll_due_at;
                source = "retained_poll";
            } else {
                poll_due_at = scheduled_at;
                source = "overdue_poll";
            }
        }
        self.next_poll_due_at = Some(poll_due_at);

        self.next_visual_due_at = if let Some(interval) = self.visual_tick_interval() {
            let candidate = scheduled_at + interval;
            Some(
                if let Some(existing_visual_due_at) = self.next_visual_due_at {
                    if existing_visual_due_at <= candidate && existing_visual_due_at > scheduled_at
                    {
                        existing_visual_due_at
                    } else {
                        candidate
                    }
                } else {
                    candidate
                },
            )
        } else {
            None
        };

        let mut due_at = poll_due_at;
        let mut trigger = "poll";
        if let Some(workspace_refresh_due_at) = self.next_workspace_refresh_due_at
            && workspace_refresh_due_at < due_at
        {
            due_at = workspace_refresh_due_at;
            source = "workspace_refresh";
            trigger = "workspace_refresh";
        }
        if let Some(visual_due_at) = self.next_visual_due_at
            && visual_due_at < due_at
        {
            due_at = visual_due_at;
            trigger = "visual";
        }
        if self.preview_poll_in_flight {
            let in_flight_due_at =
                scheduled_at + Duration::from_millis(PREVIEW_POLL_IN_FLIGHT_TICK_MS);
            if in_flight_due_at < due_at {
                due_at = in_flight_due_at;
                source = "poll_in_flight";
                trigger = "task_result";
            }
        }

        if let Some(existing_due_at) = self.next_tick_due_at
            && existing_due_at <= due_at
            && existing_due_at > scheduled_at
        {
            self.emit_event(
                LogEvent::new("tick", "retained")
                    .with_data("source", Value::from(source))
                    .with_data("trigger", Value::from(trigger))
                    .with_data(
                        "interval_ms",
                        Value::from(Self::duration_millis(
                            existing_due_at.saturating_duration_since(scheduled_at),
                        )),
                    )
                    .with_data("pending_depth", Value::from(self.pending_input_depth()))
                    .with_data(
                        "oldest_pending_age_ms",
                        Value::from(self.oldest_pending_input_age_ms(scheduled_at)),
                    ),
            );
            return Cmd::None;
        }

        let interval = due_at.saturating_duration_since(scheduled_at);
        let interval_ms = Self::duration_millis(interval);
        self.next_tick_due_at = Some(due_at);
        self.next_tick_interval_ms = Some(interval_ms);
        self.emit_event(
            LogEvent::new("tick", "scheduled")
                .with_data("source", Value::from(source))
                .with_data("trigger", Value::from(trigger))
                .with_data("interval_ms", Value::from(interval_ms))
                .with_data("pending_depth", Value::from(self.pending_input_depth()))
                .with_data(
                    "oldest_pending_age_ms",
                    Value::from(self.oldest_pending_input_age_ms(scheduled_at)),
                ),
        );
        Cmd::tick(interval)
    }

    pub(super) fn tick_is_due(&self, now: Instant) -> bool {
        let Some(due_at) = self.next_tick_due_at else {
            return true;
        };

        Self::is_due_with_tolerance(now, due_at)
    }
}

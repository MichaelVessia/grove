use super::*;

impl GroveApp {
    fn current_attention_marker_for_workspace_path(&self, workspace_path: &Path) -> Option<String> {
        let workspace = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)?;
        if !workspace.supported_agent || !workspace.status.has_session() {
            return None;
        }

        latest_assistant_attention_marker(workspace.agent, workspace.path.as_path())
    }

    fn acknowledge_workspace_attention_for_path(&mut self, workspace_path: &Path) -> bool {
        let Some(marker) = self.current_attention_marker_for_workspace_path(workspace_path) else {
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

    pub(super) fn save_projects_config(&self) -> Result<(), String> {
        let projects_path = crate::infrastructure::config::projects_path_for(&self.config_path);
        crate::infrastructure::config::save_projects_to_path(
            &projects_path,
            &self.projects,
            &self.workspace_attention_acks_for_config(),
        )
    }

    fn refresh_workspace_attention_for_path(&mut self, workspace_path: &Path) {
        let Some(marker) = self.current_attention_marker_for_workspace_path(workspace_path) else {
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
            && let Err(error) = self.save_projects_config()
        {
            self.last_tmux_error = Some(format!("attention ack persist failed: {error}"));
        }
    }

    pub(super) fn track_workspace_status_transition(
        &mut self,
        workspace_path: &Path,
        _previous_status: WorkspaceStatus,
        _next_status: WorkspaceStatus,
        _previous_orphaned: bool,
        _next_orphaned: bool,
    ) {
        self.refresh_workspace_attention_for_path(workspace_path);
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

    fn workspace_status_tracking_key(workspace_path: &Path) -> String {
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

    pub(super) fn capture_changed_cleaned_for_workspace(
        &mut self,
        workspace_path: &Path,
        output: &str,
    ) -> (bool, String) {
        let key = Self::workspace_status_tracking_key(workspace_path);
        let previous_digest = self.workspace_status_digests.get(&key);
        let change = evaluate_capture_change(previous_digest, output);
        self.workspace_status_digests
            .insert(key.clone(), change.digest);
        self.workspace_output_changing
            .insert(key, change.changed_cleaned);
        (change.changed_cleaned, change.cleaned_output)
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
            self.event_log.log(
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
        self.event_log.log(
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

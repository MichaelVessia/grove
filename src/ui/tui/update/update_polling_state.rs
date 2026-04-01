use super::update_prelude::*;

impl GroveApp {
    const STALLED_IDLE_POLLS_TO_ATTENTION: u8 = 6;
    const ATTENTION_PROMOTION_POLLS: u8 = 2;
    const ATTENTION_REMOVAL_POLLS: u8 = 3;

    fn workspace_has_running_agent_tab(&self, workspace_path: &Path) -> bool {
        self.workspace_tabs.get(workspace_path).is_some_and(|tabs| {
            tabs.tabs.iter().any(|tab| {
                tab.kind == WorkspaceTabKind::Agent
                    && tab.state == WorkspaceTabRuntimeState::Running
            })
        })
    }

    fn current_attention_marker_for_workspace_path(&self, workspace_path: &Path) -> Option<String> {
        #[cfg(test)]
        if let Some(marker) = self.attention_marker_overrides.get(workspace_path) {
            return marker.clone();
        }

        let workspace = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)?;
        let has_running_agent_tab = self.workspace_has_running_agent_tab(workspace_path);
        if !workspace.supported_agent || !has_running_agent_tab {
            return None;
        }

        latest_assistant_attention_marker(workspace.agent, workspace.path.as_path())
    }

    fn permission_wall_prompt(prompt: &str) -> bool {
        let lower = prompt.to_ascii_lowercase();
        ["approve", "allow", "confirm", "do you want"]
            .iter()
            .any(|pattern| lower.contains(pattern))
    }

    fn attention_item_for_workspace(
        &self,
        workspace: &Workspace,
        now_ms: u64,
    ) -> Option<AttentionItem> {
        if workspace.is_orphaned && !workspace.is_main {
            return Some(AttentionItem {
                fingerprint: format!("session-ended:{}", workspace.path.display()),
                reason: AttentionReason::SessionEnded,
                summary: AttentionReason::SessionEnded.summary().to_string(),
                workspace_path: workspace.path.clone(),
                task_slug: workspace.task_slug.clone().unwrap_or_default(),
                first_seen_at_ms: now_ms,
                last_seen_at_ms: now_ms,
            });
        }

        if workspace.status == WorkspaceStatus::Waiting
            && let Some(marker) =
                self.current_attention_marker_for_workspace_path(workspace.path.as_path())
        {
            let waiting_prompt = self
                .polling
                .workspace_waiting_prompts
                .get(workspace.path.as_path())
                .cloned();
            let reason = waiting_prompt
                .as_deref()
                .filter(|prompt| Self::permission_wall_prompt(prompt))
                .map(|_| AttentionReason::PermissionWall)
                .unwrap_or(AttentionReason::BlockedOnQuestion);
            return Some(AttentionItem {
                fingerprint: format!("{}:{marker}", reason.summary()),
                reason,
                summary: reason.summary().to_string(),
                workspace_path: workspace.path.clone(),
                task_slug: workspace.task_slug.clone().unwrap_or_default(),
                first_seen_at_ms: now_ms,
                last_seen_at_ms: now_ms,
            });
        }

        if !workspace.supported_agent
            || !self.workspace_has_running_agent_tab(workspace.path.as_path())
        {
            return None;
        }

        if workspace.status == WorkspaceStatus::Done {
            return Some(AttentionItem {
                fingerprint: format!("finished:{}", workspace.path.display()),
                reason: AttentionReason::Finished,
                summary: AttentionReason::Finished.summary().to_string(),
                workspace_path: workspace.path.clone(),
                task_slug: workspace.task_slug.clone().unwrap_or_default(),
                first_seen_at_ms: now_ms,
                last_seen_at_ms: now_ms,
            });
        }

        let idle_polls = self
            .polling
            .workspace_idle_polls_since_output
            .get(workspace.path.as_path())
            .copied()
            .unwrap_or(0);
        if workspace.status == WorkspaceStatus::Idle
            && idle_polls >= Self::STALLED_IDLE_POLLS_TO_ATTENTION
        {
            return Some(AttentionItem {
                fingerprint: format!("stalled:{}", workspace.path.display()),
                reason: AttentionReason::Stalled,
                summary: AttentionReason::Stalled.summary().to_string(),
                workspace_path: workspace.path.clone(),
                task_slug: workspace.task_slug.clone().unwrap_or_default(),
                first_seen_at_ms: now_ms,
                last_seen_at_ms: now_ms,
            });
        }

        None
    }

    fn current_attention_fingerprint_for_workspace_path(
        &self,
        workspace_path: &Path,
    ) -> Option<String> {
        let workspace = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)?;
        let item = self.attention_item_for_workspace(workspace, now_millis())?;
        Some(item.fingerprint)
    }

    fn acknowledge_workspace_attention_for_path(&mut self, workspace_path: &Path) -> bool {
        let fingerprint = self
            .current_attention_fingerprint_for_workspace_path(workspace_path)
            .or_else(|| {
                self.attention_items
                    .iter()
                    .find(|item| item.workspace_path == workspace_path)
                    .map(|item| item.fingerprint.clone())
            });
        let Some(fingerprint) = fingerprint else {
            return false;
        };
        if self
            .workspace_attention_ack_markers
            .get(workspace_path)
            .is_some_and(|saved| saved == &fingerprint)
        {
            return false;
        }

        self.workspace_attention_ack_markers
            .insert(workspace_path.to_path_buf(), fingerprint);
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
            &self.task_order,
            &self.workspace_attention_acks_for_config(),
            &self.hidden_base_project_paths_for_config(),
        )
    }

    pub(super) fn hidden_base_project_paths_for_config(&self) -> Vec<PathBuf> {
        let mut paths = self
            .hidden_base_project_paths
            .iter()
            .cloned()
            .collect::<Vec<PathBuf>>();
        paths.sort();
        paths
    }

    fn refresh_attention_items(&mut self) {
        let now_ms = now_millis();
        let previous_items = self
            .attention_items
            .iter()
            .cloned()
            .map(|item| (item.workspace_path.clone(), item))
            .collect::<HashMap<PathBuf, AttentionItem>>();
        let previous_observations = std::mem::take(&mut self.attention_observations);
        let mut next_observations = HashMap::new();
        let mut next_items = Vec::new();
        self.workspace_attention.clear();

        for workspace in &self.state.workspaces {
            let workspace_path = workspace.path.clone();
            let previous_visible = previous_items.get(workspace_path.as_path());
            let previous_observation = previous_observations.get(workspace_path.as_path());
            let candidate = self.attention_item_for_workspace(workspace, now_ms);

            let visible_item = if let Some(candidate) = candidate {
                if self
                    .workspace_attention_ack_markers
                    .get(workspace_path.as_path())
                    .is_some_and(|saved| saved == &candidate.fingerprint)
                {
                    None
                } else {
                    let observation = match previous_observation {
                        Some(previous) if previous.item.fingerprint == candidate.fingerprint => {
                            AttentionObservation {
                                item: AttentionItem {
                                    first_seen_at_ms: previous.item.first_seen_at_ms,
                                    ..candidate
                                },
                                seen_polls: previous.seen_polls.saturating_add(1),
                                missing_polls: 0,
                            }
                        }
                        _ => AttentionObservation {
                            item: candidate,
                            seen_polls: 1,
                            missing_polls: 0,
                        },
                    };

                    let visible = if let Some(previous) = previous_visible {
                        if previous.fingerprint == observation.item.fingerprint {
                            Some(AttentionItem {
                                first_seen_at_ms: previous.first_seen_at_ms,
                                ..observation.item.clone()
                            })
                        } else if observation.seen_polls >= Self::ATTENTION_PROMOTION_POLLS {
                            Some(observation.item.clone())
                        } else {
                            Some(previous.clone())
                        }
                    } else if observation.seen_polls >= Self::ATTENTION_PROMOTION_POLLS {
                        Some(observation.item.clone())
                    } else {
                        None
                    };

                    next_observations.insert(workspace_path.clone(), observation);
                    visible
                }
            } else if let Some(previous) = previous_visible {
                let missing_polls = previous_observation
                    .map_or(1, |observation| observation.missing_polls.saturating_add(1));
                if missing_polls < Self::ATTENTION_REMOVAL_POLLS {
                    next_observations.insert(
                        workspace_path.clone(),
                        AttentionObservation {
                            item: previous.clone(),
                            seen_polls: 0,
                            missing_polls,
                        },
                    );
                    Some(previous.clone())
                } else {
                    self.workspace_attention_ack_markers
                        .remove(workspace_path.as_path());
                    None
                }
            } else {
                self.workspace_attention_ack_markers
                    .remove(workspace_path.as_path());
                None
            };

            if let Some(item) = visible_item {
                self.workspace_attention
                    .insert(workspace_path, WorkspaceAttention::NeedsAttention);
                next_items.push(item);
            }
        }

        next_items.sort_by(|left, right| {
            left.reason
                .rank()
                .cmp(&right.reason.rank())
                .then_with(|| left.first_seen_at_ms.cmp(&right.first_seen_at_ms))
                .then_with(|| left.workspace_path.cmp(&right.workspace_path))
        });
        self.attention_observations = next_observations;
        self.attention_items = next_items;
        if self
            .selected_attention_item
            .is_some_and(|index| index >= self.attention_items.len())
        {
            self.selected_attention_item = None;
        }
        self.maybe_focus_attention_inbox_on_startup();
    }

    pub(super) fn workspace_attention(&self, workspace_path: &Path) -> Option<WorkspaceAttention> {
        self.workspace_attention.get(workspace_path).copied()
    }

    pub(super) fn selected_attention_item(&self) -> Option<&AttentionItem> {
        self.selected_attention_item
            .and_then(|index| self.attention_items.get(index))
    }

    pub(super) fn clear_attention_for_workspace_path(&mut self, workspace_path: &Path) {
        self.workspace_attention.remove(workspace_path);
        if self.acknowledge_workspace_attention_for_path(workspace_path)
            && let Err(error) = self.save_projects_config()
        {
            self.session.last_tmux_error = Some(format!("attention ack persist failed: {error}"));
        }
        self.attention_observations.remove(workspace_path);
        self.attention_items
            .retain(|item| item.workspace_path != workspace_path);
        self.refresh_attention_items();
    }

    pub(super) fn acknowledge_selected_workspace_attention_for_preview_focus(&mut self) {
        let _ = self;
    }

    pub(super) fn track_workspace_status_transition(
        &mut self,
        workspace_path: &Path,
        _previous_status: WorkspaceStatus,
        _next_status: WorkspaceStatus,
        _previous_orphaned: bool,
        _next_orphaned: bool,
    ) {
        self.refresh_attention_items();
        if self
            .selected_attention_item
            .is_some_and(|index| index >= self.attention_items.len())
        {
            self.selected_attention_item = None;
        }
        let _ = workspace_path;
    }

    pub(super) fn track_workspace_status_transition_deferred(
        &mut self,
        _workspace_path: &Path,
        _previous_status: WorkspaceStatus,
        _next_status: WorkspaceStatus,
        _previous_orphaned: bool,
        _next_orphaned: bool,
    ) {
    }

    pub(super) fn flush_deferred_attention_refresh(&mut self) {
        self.refresh_attention_items();
        if self
            .selected_attention_item
            .is_some_and(|index| index >= self.attention_items.len())
        {
            self.selected_attention_item = None;
        }
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
        self.attention_observations
            .retain(|path, _| valid_paths.contains(path));
        self.attention_items
            .retain(|item| valid_paths.contains(item.workspace_path.as_path()));
        self.refresh_attention_items();
    }

    fn next_poll_interval(&self) -> Duration {
        let status = self.selected_workspace_status();

        let since_last_key = self
            .session
            .interactive
            .as_ref()
            .map_or(Duration::from_secs(60), |interactive| {
                Instant::now().saturating_duration_since(interactive.last_key_time)
            });

        poll_interval(
            status,
            true,
            self.preview_focused(),
            self.session.interactive.is_some(),
            since_last_key,
            self.polling.output_changing,
        )
    }

    fn selected_workspace_status(&self) -> WorkspaceStatus {
        self.state
            .selected_workspace()
            .map_or(WorkspaceStatus::Unknown, |workspace| workspace.status)
    }

    pub(super) fn clear_agent_activity_tracking(&mut self) {
        self.polling.output_changing = false;
        self.polling.agent_output_changing = false;
        self.polling.last_live_preview_session = None;
        self.polling.pending_selected_session_bootstrap = None;
        self.polling.recent_local_echo_session = None;
        self.polling.agent_working_until = None;
        self.polling.agent_idle_polls_since_output = 0;
    }

    pub(super) fn record_workspace_poll_state(
        &mut self,
        workspace_path: &Path,
        status: WorkspaceStatus,
        cleaned_output: &str,
        changed: bool,
    ) -> Option<WorkspaceStatus> {
        let semantic_observation = self.workspace_status_observation_for_path(workspace_path);
        if let Some(observation) = semantic_observation {
            self.record_workspace_status_observation(workspace_path, &observation);
            return Some(observation.status);
        }

        self.polling
            .workspace_recent_activity
            .remove(workspace_path);

        if status == WorkspaceStatus::Waiting {
            if let Some(prompt) = detect_waiting_prompt(cleaned_output) {
                self.polling
                    .workspace_waiting_prompts
                    .insert(workspace_path.to_path_buf(), prompt);
            } else {
                self.polling
                    .workspace_waiting_prompts
                    .remove(workspace_path);
            }
        } else {
            self.polling
                .workspace_waiting_prompts
                .remove(workspace_path);
        }

        if changed {
            self.polling
                .workspace_idle_polls_since_output
                .remove(workspace_path);
            return None;
        }

        if status == WorkspaceStatus::Idle {
            let idle_polls = self
                .polling
                .workspace_idle_polls_since_output
                .entry(workspace_path.to_path_buf())
                .or_insert(0);
            *idle_polls = idle_polls.saturating_add(1);
            return None;
        }

        self.polling
            .workspace_idle_polls_since_output
            .remove(workspace_path);
        None
    }

    fn workspace_status_observation_for_path(
        &self,
        workspace_path: &Path,
    ) -> Option<WorkspaceStatusObservation> {
        #[cfg(test)]
        if let Some(observation) = self
            .polling
            .workspace_status_observation_overrides
            .get(workspace_path)
        {
            return Some(observation.clone());
        }

        let workspace = self
            .state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)?;
        let home_dir = dirs::home_dir()?;
        let session_name =
            crate::application::agent_runtime::session_name_for_workspace_ref(workspace);

        crate::application::agent_runtime::status::workspace_status_observation_in_home(
            crate::application::agent_runtime::status::BackgroundStatusObservationContext {
                agent: workspace.agent,
                workspace_path,
                home_dir: Some(home_dir.as_path()),
                activity_threshold: Duration::from_secs(30),
                session_name: session_name.as_str(),
            },
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn record_workspace_status_observation(
        &mut self,
        workspace_path: &Path,
        observation: &WorkspaceStatusObservation,
    ) {
        if observation.status == WorkspaceStatus::Waiting {
            if let Some(prompt) = &observation.waiting_excerpt {
                self.polling
                    .workspace_waiting_prompts
                    .insert(workspace_path.to_path_buf(), prompt.clone());
            } else {
                self.polling
                    .workspace_waiting_prompts
                    .remove(workspace_path);
            }
        } else {
            self.polling
                .workspace_waiting_prompts
                .remove(workspace_path);
        }

        self.polling
            .workspace_recent_activity
            .insert(workspace_path.to_path_buf(), observation.recent_activity);
        if observation.recent_activity || observation.status != WorkspaceStatus::Idle {
            self.polling
                .workspace_idle_polls_since_output
                .remove(workspace_path);
            return;
        }

        let idle_polls = self
            .polling
            .workspace_idle_polls_since_output
            .entry(workspace_path.to_path_buf())
            .or_insert(0);
        *idle_polls = idle_polls.saturating_add(1);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn apply_workspace_status_observation(
        &mut self,
        workspace_path: &Path,
        observation: WorkspaceStatusObservation,
    ) {
        let Some(workspace_index) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.path == workspace_path)
        else {
            return;
        };

        let previous_status = self.state.workspaces[workspace_index].status;
        let previous_orphaned = self.state.workspaces[workspace_index].is_orphaned;
        self.record_workspace_status_observation(workspace_path, &observation);
        let workspace = &mut self.state.workspaces[workspace_index];
        workspace.status = observation.status;
        workspace.is_orphaned = false;
        self.track_workspace_status_transition(
            workspace_path,
            previous_status,
            observation.status,
            previous_orphaned,
            false,
        );
    }

    pub(super) fn clear_status_tracking_for_workspace_path(&mut self, workspace_path: &Path) {
        self.polling.workspace_status_digests.remove(workspace_path);
        self.polling
            .workspace_output_changing
            .remove(workspace_path);
        self.polling
            .workspace_recent_activity
            .remove(workspace_path);
        #[cfg(test)]
        self.polling
            .workspace_status_observation_overrides
            .remove(workspace_path);
        self.polling
            .workspace_waiting_prompts
            .remove(workspace_path);
        self.polling
            .workspace_idle_polls_since_output
            .remove(workspace_path);
        self.attention_observations.remove(workspace_path);
    }

    pub(super) fn clear_status_tracking(&mut self) {
        self.polling.workspace_status_digests.clear();
        self.polling.workspace_output_changing.clear();
        self.polling.workspace_recent_activity.clear();
        #[cfg(test)]
        self.polling.workspace_status_observation_overrides.clear();
        self.polling.workspace_waiting_prompts.clear();
        self.polling.workspace_idle_polls_since_output.clear();
        self.attention_observations.clear();
    }

    pub(super) fn capture_changed_cleaned_for_workspace(
        &mut self,
        workspace_path: &Path,
        output: &str,
    ) -> (bool, String) {
        let previous_digest = self.polling.workspace_status_digests.get(workspace_path);
        let change = evaluate_capture_change(previous_digest, output);
        self.polling
            .workspace_status_digests
            .insert(workspace_path.to_path_buf(), change.digest);
        self.polling
            .workspace_output_changing
            .insert(workspace_path.to_path_buf(), change.changed_cleaned);
        (change.changed_cleaned, change.cleaned_output)
    }

    fn workspace_recent_activity(&self, workspace_path: &Path) -> bool {
        self.polling
            .workspace_recent_activity
            .get(workspace_path)
            .copied()
            .unwrap_or(false)
    }

    fn workspace_status_for_path(&self, workspace_path: &Path) -> Option<WorkspaceStatus> {
        self.state
            .workspaces
            .iter()
            .find(|workspace| workspace.path == workspace_path)
            .map(|workspace| workspace.status)
    }

    pub(super) fn push_agent_activity_frame(&mut self, changed: bool) {
        if changed {
            self.polling.agent_working_until =
                Some(Instant::now() + Duration::from_millis(WORKING_STATUS_HOLD_MS));
            self.polling.agent_idle_polls_since_output = 0;
            return;
        }

        if self.polling.agent_working_until.is_some() {
            self.polling.agent_idle_polls_since_output =
                self.polling.agent_idle_polls_since_output.saturating_add(1);
        }
    }

    pub(super) fn has_recent_agent_activity(&self) -> bool {
        let Some(working_until) = self.polling.agent_working_until else {
            return false;
        };

        if Instant::now() < working_until {
            return true;
        }

        self.polling.agent_idle_polls_since_output < WORKING_IDLE_POLLS_TO_CLEAR
    }

    fn visual_tick_interval(&self) -> Option<Duration> {
        let selected_workspace_path = self
            .state
            .selected_workspace()
            .map(|workspace| workspace.path.as_path());
        if self.status_is_visually_working(selected_workspace_path, true) {
            return Some(Duration::from_millis(FAST_ANIMATION_INTERVAL_MS));
        }
        None
    }

    pub(super) fn advance_visual_animation(&mut self) {
        self.polling
            .activity_animation
            .tick_delta(Duration::from_millis(FAST_ANIMATION_INTERVAL_MS).as_secs_f64());
    }

    pub(super) fn activity_animation_time(&self) -> f64 {
        self.polling.activity_animation.time()
    }

    pub(super) fn status_is_visually_working(
        &self,
        workspace_path: Option<&Path>,
        is_selected: bool,
    ) -> bool {
        if workspace_path.is_some_and(|path| {
            self.workspace_status_for_path(path) == Some(WorkspaceStatus::Waiting)
        }) {
            return false;
        }

        if is_selected {
            return workspace_path.is_some_and(|path| self.workspace_recent_activity(path))
                || self.has_recent_agent_activity();
        }

        workspace_path.is_some_and(|path| self.workspace_recent_activity(path))
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
        if let Some(interactive_due_at) = self.polling.interactive_poll_due_at
            && interactive_due_at < poll_due_at
        {
            poll_due_at = interactive_due_at;
            source = "interactive_debounce";
        }

        if let Some(existing_poll_due_at) = self.polling.next_poll_due_at
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
        self.polling.next_poll_due_at = Some(poll_due_at);

        self.polling.next_visual_due_at = if let Some(interval) = self.visual_tick_interval() {
            let candidate = scheduled_at + interval;
            Some(
                if let Some(existing_visual_due_at) = self.polling.next_visual_due_at {
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
        if let Some(visual_due_at) = self.polling.next_visual_due_at
            && visual_due_at < due_at
        {
            due_at = visual_due_at;
            trigger = "visual";
        }
        if self.polling.preview_poll_in_flight {
            let in_flight_due_at =
                scheduled_at + Duration::from_millis(PREVIEW_POLL_IN_FLIGHT_TICK_MS);
            if in_flight_due_at < due_at {
                due_at = in_flight_due_at;
                source = "poll_in_flight";
                trigger = "task_result";
            }
        }

        if let Some(existing_due_at) = self.polling.next_tick_due_at
            && existing_due_at <= due_at
            && existing_due_at > scheduled_at
        {
            self.telemetry.event_log.log(
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
        self.polling.next_tick_due_at = Some(due_at);
        self.polling.next_tick_interval_ms = Some(interval_ms);
        self.polling.next_tick_source = Some(source.to_string());
        self.polling.next_tick_trigger = Some(trigger.to_string());
        self.telemetry.event_log.log(
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
        let Some(due_at) = self.polling.next_tick_due_at else {
            return true;
        };

        Self::is_due_with_tolerance(now, due_at)
    }
}

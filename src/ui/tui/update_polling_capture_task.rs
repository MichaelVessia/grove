use super::*;

impl GroveApp {
    pub(super) fn poll_preview_sync(&mut self) {
        let live_preview = self.prepare_live_preview_session();
        let has_live_preview = live_preview.is_some();
        let cursor_session = self.interactive_target_session();
        let status_poll_targets = workspace_status_targets_for_polling_with_live_preview(
            &self.state.workspaces,
            live_preview.as_ref(),
        );

        if let Some(live_preview_target) = live_preview {
            let previous_digest = self.preview.last_digest().cloned();
            let capture_started_at = Instant::now();
            let raw_result = if let Some(socket_path) = &live_preview_target.daemon_socket_path {
                let payload = DaemonSessionCapturePayload {
                    session_name: live_preview_target.session_name.clone(),
                    scrollback_lines: 600,
                    include_escape_sequences: live_preview_target.include_escape_sequences,
                };
                match session_capture_via_socket(socket_path, payload) {
                    Ok(Ok(output)) => Ok(output),
                    Ok(Err(daemon_error)) => Err(daemon_error.message),
                    Err(io_error) => Err(io_error.to_string()),
                }
            } else {
                self.tmux_input
                    .capture_output(
                        &live_preview_target.session_name,
                        600,
                        live_preview_target.include_escape_sequences,
                    )
                    .map_err(|error| error.to_string())
            };
            let capture_ms =
                Self::duration_millis(Instant::now().saturating_duration_since(capture_started_at));
            let result = match raw_result {
                Ok(raw_output) => {
                    let change = evaluate_capture_change(previous_digest.as_ref(), &raw_output);
                    Ok(LivePreviewCaptureOutput { raw_output, change })
                }
                Err(e) => Err(e),
            };
            self.apply_live_preview_capture(
                &live_preview_target.session_name,
                live_preview_target.include_escape_sequences,
                capture_ms,
                capture_ms,
                result,
            );
        } else {
            self.clear_agent_activity_tracking();
            self.refresh_preview_summary();
        }

        for target in status_poll_targets {
            let capture_started_at = Instant::now();
            let raw_result = self
                .tmux_input
                .capture_output(&target.session_name, 120, false)
                .map_err(|error| error.to_string());
            let capture_ms =
                Self::duration_millis(Instant::now().saturating_duration_since(capture_started_at));
            let result = match raw_result {
                Ok(raw_output) => {
                    let change = evaluate_capture_change(None, &raw_output);
                    let resolved_status = detect_status_with_session_override(
                        &change.cleaned_output,
                        SessionActivity::Active,
                        target.is_main,
                        true,
                        target.supported_agent,
                        target.agent,
                        &target.workspace_path,
                    );
                    Ok(WorkspaceStatusCaptureOutput {
                        cleaned_output: change.cleaned_output,
                        digest: change.digest,
                        resolved_status,
                    })
                }
                Err(e) => Err(e),
            };
            self.apply_workspace_status_capture(WorkspaceStatusCapture {
                workspace_name: target.workspace_name,
                workspace_path: target.workspace_path,
                session_name: target.session_name,
                supported_agent: target.supported_agent,
                capture_ms,
                result,
            });
        }
        if !has_live_preview {
            self.refresh_preview_summary();
        }

        if let Some(target_session) = cursor_session {
            self.poll_interactive_cursor_sync(&target_session);
        }
    }

    pub(super) fn schedule_async_preview_poll(
        &self,
        generation: u64,
        live_preview: Option<LivePreviewTarget>,
        previous_live_digest: Option<OutputDigest>,
        cursor_session: Option<String>,
        cursor_daemon_socket_path: Option<PathBuf>,
        status_poll_targets: Vec<WorkspaceStatusTarget>,
    ) -> Cmd<Msg> {
        Cmd::task(move || {
            let live_capture = live_preview.map(|target| {
                let capture_started_at = Instant::now();
                let raw_result = if let Some(socket_path) = &target.daemon_socket_path {
                    let payload = DaemonSessionCapturePayload {
                        session_name: target.session_name.clone(),
                        scrollback_lines: 600,
                        include_escape_sequences: target.include_escape_sequences,
                    };
                    match session_capture_via_socket(socket_path, payload) {
                        Ok(Ok(output)) => Ok(output),
                        Ok(Err(daemon_error)) => Err(daemon_error.message),
                        Err(io_error) => Err(io_error.to_string()),
                    }
                } else {
                    CommandTmuxInput::capture_session_output(
                        &target.session_name,
                        600,
                        target.include_escape_sequences,
                    )
                    .map_err(|error| error.to_string())
                };
                let capture_ms = GroveApp::duration_millis(
                    Instant::now().saturating_duration_since(capture_started_at),
                );
                let result = match raw_result {
                    Ok(raw_output) => {
                        let change =
                            evaluate_capture_change(previous_live_digest.as_ref(), &raw_output);
                        Ok(LivePreviewCaptureOutput { raw_output, change })
                    }
                    Err(e) => Err(e),
                };
                LivePreviewCapture {
                    session: target.session_name,
                    include_escape_sequences: target.include_escape_sequences,
                    capture_ms,
                    total_ms: capture_ms,
                    result,
                }
            });

            let cursor_capture = cursor_session.map(|session| {
                let started_at = Instant::now();
                let result = if let Some(socket_path) = &cursor_daemon_socket_path {
                    let payload = DaemonSessionCursorMetadataPayload {
                        session_name: session.clone(),
                    };
                    match session_cursor_metadata_via_socket(socket_path, payload) {
                        Ok(Ok(metadata)) => Ok(metadata),
                        Ok(Err(daemon_error)) => Err(daemon_error.message),
                        Err(io_error) => Err(io_error.to_string()),
                    }
                } else {
                    CommandTmuxInput::capture_session_cursor_metadata(&session)
                        .map_err(|error| error.to_string())
                };
                let capture_ms =
                    GroveApp::duration_millis(Instant::now().saturating_duration_since(started_at));
                CursorCapture {
                    session,
                    capture_ms,
                    result,
                }
            });

            let workspace_status_captures = status_poll_targets
                .into_iter()
                .map(|target| {
                    let capture_started_at = Instant::now();
                    let raw_result = if let Some(socket_path) = &target.daemon_socket_path {
                        let payload = DaemonSessionCapturePayload {
                            session_name: target.session_name.clone(),
                            scrollback_lines: 120,
                            include_escape_sequences: false,
                        };
                        match session_capture_via_socket(socket_path, payload) {
                            Ok(Ok(output)) => Ok(output),
                            Ok(Err(daemon_error)) => Err(daemon_error.message),
                            Err(io_error) => Err(io_error.to_string()),
                        }
                    } else {
                        CommandTmuxInput::capture_session_output(&target.session_name, 120, false)
                            .map_err(|error| error.to_string())
                    };
                    let capture_ms = GroveApp::duration_millis(
                        Instant::now().saturating_duration_since(capture_started_at),
                    );
                    let result = match raw_result {
                        Ok(raw_output) => {
                            let change = evaluate_capture_change(None, &raw_output);
                            let resolved_status = detect_status_with_session_override(
                                &change.cleaned_output,
                                SessionActivity::Active,
                                target.is_main,
                                true,
                                target.supported_agent,
                                target.agent,
                                &target.workspace_path,
                            );
                            Ok(WorkspaceStatusCaptureOutput {
                                cleaned_output: change.cleaned_output,
                                digest: change.digest,
                                resolved_status,
                            })
                        }
                        Err(e) => Err(e),
                    };
                    WorkspaceStatusCapture {
                        workspace_name: target.workspace_name,
                        workspace_path: target.workspace_path,
                        session_name: target.session_name,
                        supported_agent: target.supported_agent,
                        capture_ms,
                        result,
                    }
                })
                .collect();

            Msg::PreviewPollCompleted(PreviewPollCompletion {
                generation,
                live_capture,
                cursor_capture,
                workspace_status_captures,
            })
        })
    }
}

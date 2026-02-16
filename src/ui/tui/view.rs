use super::*;

impl GroveApp {
    pub(super) fn render_model(&self, frame: &mut Frame) {
        let view_started_at = Instant::now();
        frame.set_cursor(None);
        frame.set_cursor_visible(false);
        frame.enable_hit_testing();
        let area = Rect::from_size(frame.buffer.width(), frame.buffer.height());
        let layout = Self::view_layout_for_size_with_sidebar(
            frame.buffer.width(),
            frame.buffer.height(),
            self.sidebar_width_pct,
            self.sidebar_hidden,
        );

        self.render_header(frame, layout.header);
        self.render_sidebar(frame, layout.sidebar);
        self.render_divider(frame, layout.divider);
        self.render_preview_pane(frame, layout.preview);
        self.render_status_line(frame, layout.status);
        self.render_create_dialog_overlay(frame, area);
        self.render_edit_dialog_overlay(frame, area);
        self.render_launch_dialog_overlay(frame, area);
        self.render_delete_dialog_overlay(frame, area);
        self.render_merge_dialog_overlay(frame, area);
        self.render_update_from_base_dialog_overlay(frame, area);
        self.render_settings_dialog_overlay(frame, area);
        self.render_project_dialog_overlay(frame, area);
        self.render_keybind_help_overlay(frame, area);
        self.render_command_palette_overlay(frame, area);
        self.render_toasts(frame, area);
        let draw_completed_at = Instant::now();
        self.last_hit_grid.replace(frame.hit_grid.clone());
        let frame_log_started_at = Instant::now();
        self.log_frame_render(frame);
        let view_completed_at = Instant::now();
        self.event_log.log(
            LogEvent::new("frame", "timing")
                .with_data(
                    "draw_ms",
                    Value::from(Self::duration_millis(
                        draw_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data(
                    "frame_log_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(frame_log_started_at),
                    )),
                )
                .with_data(
                    "view_ms",
                    Value::from(Self::duration_millis(
                        view_completed_at.saturating_duration_since(view_started_at),
                    )),
                )
                .with_data("degradation", Value::from(frame.degradation.as_str()))
                .with_data("pending_depth", Value::from(self.pending_input_depth())),
        );
    }

    #[cfg(test)]
    fn unsafe_label(&self) -> &'static str {
        if self.launch_skip_permissions {
            "on"
        } else {
            "off"
        }
    }

    #[cfg(test)]
    pub(super) fn status_bar_line(&self) -> String {
        if let Some(toast) = self.notifications.visible().last() {
            if matches!(toast.config.style_variant, ToastStyle::Error) {
                return format!("Status: error: {}", toast.content.message);
            }
            return format!("Status: {}", toast.content.message);
        }

        match &self.discovery_state {
            DiscoveryState::Error(message) => format!("Status: discovery error ({message})"),
            DiscoveryState::Empty => "Status: no worktrees found".to_string(),
            DiscoveryState::Ready => {
                if let Some(dialog) = &self.create_dialog {
                    return format!(
                        "Status: new workspace, field={}, agent={}, base_branch=\"{}\", name=\"{}\"",
                        dialog.focused_field.label(),
                        dialog.agent.label(),
                        dialog.base_branch.replace('\n', "\\n"),
                        dialog.workspace_name
                    );
                }
                if let Some(dialog) = &self.launch_dialog {
                    return format!(
                        "Status: start agent, field={}, unsafe={}, prompt=\"{}\", pre=\"{}\"",
                        dialog.focused_field.label(),
                        if dialog.skip_permissions { "on" } else { "off" },
                        dialog.prompt.replace('\n', "\\n"),
                        dialog.pre_launch_command.replace('\n', "\\n"),
                    );
                }
                if self.interactive.is_some() {
                    if let Some(message) = &self.last_tmux_error {
                        return format!(
                            "Status: INSERT, unsafe={}, tmux error: {message}",
                            self.unsafe_label()
                        );
                    }
                    return format!("Status: INSERT, unsafe={}", self.unsafe_label());
                }

                match self.state.mode {
                    UiMode::List => format!("Status: list, unsafe={}", self.unsafe_label()),
                    UiMode::Preview => format!(
                        "Status: preview, autoscroll={}, offset={}, split={}%, unsafe={}",
                        if self.preview.auto_scroll {
                            "on"
                        } else {
                            "off"
                        },
                        self.preview.offset,
                        self.sidebar_width_pct,
                        self.unsafe_label(),
                    ),
                }
            }
        }
    }

    pub(super) fn keybind_help_line(&self, context: HelpHintContext) -> String {
        UiCommand::help_hints_for(context)
            .iter()
            .filter_map(|command| command.help_hint_label(context))
            .collect::<Vec<&str>>()
            .join(", ")
    }

    fn status_hints_line(&self, context: StatusHintContext) -> String {
        UiCommand::status_hints_for(context)
            .iter()
            .filter_map(|command| command.status_hint_label(context))
            .collect::<Vec<&str>>()
            .join(", ")
    }

    fn keybind_hints_line(&self) -> String {
        if self.command_palette.is_visible() {
            return "Type to search, Up/Down choose, Enter run, Esc close".to_string();
        }
        if self.keybind_help_open {
            return "Esc/? close help".to_string();
        }
        if self.create_dialog.is_some() {
            return "Tab/S-Tab field, j/k or C-n/C-p move, h/l buttons, Enter select/create, Esc cancel"
                .to_string();
        }
        if self.edit_dialog.is_some() {
            return "Tab/S-Tab field, h/l buttons, Space toggle agent, Enter save/select, Esc cancel"
                .to_string();
        }
        if self.launch_dialog.is_some() {
            return "Tab/S-Tab field, h/l buttons, Space toggle unsafe, Enter select/start, Esc cancel"
                .to_string();
        }
        if self.delete_dialog.is_some() {
            return "Tab/S-Tab field, j/k move, Space toggle branch delete, Enter select/delete, D confirm, Esc cancel"
                .to_string();
        }
        if self.merge_dialog.is_some() {
            return "Tab/S-Tab field, j/k move, Space toggle cleanup, Enter select/merge, m confirm, Esc cancel"
                .to_string();
        }
        if self.update_from_base_dialog.is_some() {
            return "Tab/S-Tab field, h/l buttons, Enter select/update, u confirm, Esc cancel"
                .to_string();
        }
        if self.settings_dialog.is_some() {
            return "Tab/S-Tab field, j/k or h/l change, Enter save/select, Esc cancel".to_string();
        }
        if self.project_dialog.is_some() {
            return "Type filter, Up/Down or Tab/S-Tab navigate, Enter focus project, Ctrl+A add, Esc close"
                .to_string();
        }
        if self.interactive.is_some() {
            return "Esc Esc / Ctrl+\\ exit, Alt+C copy, Alt+V paste".to_string();
        }
        if self.preview_agent_tab_is_focused() {
            return self.status_hints_line(StatusHintContext::PreviewAgent);
        }
        if self.preview_git_tab_is_focused() {
            return self.status_hints_line(StatusHintContext::PreviewGit);
        }

        self.status_hints_line(StatusHintContext::List)
    }

    fn pane_border_style(&self, focused: bool) -> Style {
        let theme = ui_theme();
        if focused {
            return Style::new().fg(theme.blue).bold();
        }

        Style::new().fg(theme.overlay0)
    }

    fn workspace_agent_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.peach,
            AgentType::Codex => theme.text,
        }
    }

    fn activity_effect_secondary_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.text,
            AgentType::Codex => theme.overlay0,
        }
    }

    fn activity_effect_gradient(&self, agent: AgentType) -> ColorGradient {
        let primary = self.workspace_agent_color(agent);
        let secondary = self.activity_effect_secondary_color(agent);
        ColorGradient::new(vec![(0.0, primary), (0.5, secondary), (1.0, primary)])
    }

    fn activity_effect_time(&self) -> f64 {
        self.fast_animation_frame as f64 * (FAST_ANIMATION_INTERVAL_MS as f64 / 1000.0)
    }

    fn render_activity_effect_label(
        &self,
        label: &str,
        agent: AgentType,
        area: Rect,
        frame: &mut Frame,
    ) {
        if area.is_empty() || label.is_empty() {
            return;
        }

        let primary = self.workspace_agent_color(agent);
        StyledText::new(label)
            .bold()
            .base_color(primary)
            .effect(TextEffect::AnimatedGradient {
                gradient: self.activity_effect_gradient(agent),
                speed: 1.8,
            })
            .time(self.activity_effect_time())
            .render(area, frame);
    }

    pub(super) fn relative_age_label(&self, unix_secs: Option<i64>) -> String {
        let Some(unix_secs) = unix_secs else {
            return String::new();
        };
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_secs()).ok());
        let Some(now_secs) = now_secs else {
            return String::new();
        };
        let age_secs = now_secs.saturating_sub(unix_secs).max(0);
        if age_secs < 60 {
            return "now".to_string();
        }
        if age_secs < 3_600 {
            return format!("{}m", age_secs / 60);
        }
        if age_secs < 86_400 {
            return format!("{}h", age_secs / 3_600);
        }
        format!("{}d", age_secs / 86_400)
    }

    fn workspace_display_name(workspace: &Workspace) -> String {
        if workspace.is_main {
            "base".to_string()
        } else {
            workspace.name.clone()
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let base_style = Style::new().bg(theme.crust).fg(theme.text);
        let left_style = Style::new().bg(theme.surface0).fg(theme.blue).bold();
        let repo_style = Style::new().bg(theme.mantle).fg(theme.subtext0);

        let mut left: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Grove ".to_string(), left_style),
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(format!(" {} ", self.repo_name), repo_style),
        ];
        if self.command_palette.is_visible() {
            left.push(FtSpan::styled(
                " [Palette] ".to_string(),
                Style::new().bg(theme.surface1).fg(theme.mauve).bold(),
            ));
        }

        let line = chrome_bar_line(
            usize::from(area.width),
            base_style,
            left,
            Vec::new(),
            Vec::new(),
        );
        Paragraph::new(FtText::from_line(line)).render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_HEADER));
    }

    fn render_sidebar(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let block = Block::new()
            .title("Workspaces")
            .borders(Borders::ALL)
            .border_style(self.pane_border_style(
                self.state.focus == PaneFocus::WorkspaceList && !self.modal_open(),
            ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_WORKSPACE_LIST));

        if inner.is_empty() {
            return;
        }

        let theme = ui_theme();
        let mut lines: Vec<FtLine> = Vec::new();
        let mut animated_labels: Vec<(String, AgentType, u16, u16)> = Vec::new();
        let max_lines = usize::from(inner.height);
        if self.projects.is_empty() {
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "No projects configured",
                Style::new().fg(theme.subtext0),
            )]));
            lines.push(FtLine::raw(""));
            lines.push(FtLine::from_spans(vec![FtSpan::styled(
                "Press 'p' to add a project",
                Style::new().fg(theme.text).bold(),
            )]));
        } else if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            if let DiscoveryState::Error(message) = &self.discovery_state {
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    "Discovery error",
                    Style::new().fg(theme.red).bold(),
                )]));
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    message.as_str(),
                    Style::new().fg(theme.peach),
                )]));
            }
        } else {
            for (project_index, project) in self.projects.iter().enumerate() {
                if lines.len() >= max_lines {
                    break;
                }
                if project_index > 0 && lines.len() < max_lines {
                    lines.push(FtLine::raw(""));
                }
                if lines.len() >= max_lines {
                    break;
                }
                lines.push(FtLine::from_spans(vec![FtSpan::styled(
                    format!("▾ {}", project.name),
                    Style::new().fg(theme.overlay0).bold(),
                )]));

                let project_workspaces: Vec<(usize, &Workspace)> = self
                    .state
                    .workspaces
                    .iter()
                    .enumerate()
                    .filter(|(_, workspace)| {
                        workspace
                            .project_path
                            .as_ref()
                            .is_some_and(|path| project_paths_equal(path, &project.path))
                    })
                    .collect();

                if project_workspaces.is_empty() {
                    if lines.len() < max_lines {
                        lines.push(FtLine::from_spans(vec![FtSpan::styled(
                            "  (no workspaces)",
                            Style::new().fg(theme.subtext0),
                        )]));
                    }
                    continue;
                }

                for (idx, workspace) in project_workspaces {
                    if lines
                        .len()
                        .saturating_add(usize::from(WORKSPACE_ITEM_HEIGHT))
                        > max_lines
                    {
                        break;
                    }

                    let row_y = inner
                        .y
                        .saturating_add(u16::try_from(lines.len()).unwrap_or(u16::MAX));
                    let is_selected = idx == self.state.selected_index;
                    let is_working = self.status_is_visually_working(
                        Some(workspace.path.as_path()),
                        workspace.status,
                        is_selected,
                    );
                    let selected = if is_selected { "▸" } else { " " };
                    let row_background = if is_selected {
                        if self.state.focus == PaneFocus::WorkspaceList && !self.modal_open() {
                            Some(theme.surface1)
                        } else {
                            Some(theme.surface0)
                        }
                    } else {
                        None
                    };

                    let mut primary_style = Style::new().fg(theme.text);
                    let mut secondary_style = Style::new().fg(theme.subtext0);
                    if let Some(bg) = row_background {
                        primary_style = primary_style.bg(bg);
                        secondary_style = secondary_style.bg(bg);
                    }
                    if is_selected {
                        primary_style = primary_style.bold();
                    }

                    let workspace_label_style = if is_working {
                        primary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold()
                    } else {
                        primary_style
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    let show_branch = workspace.branch != workspace_name;
                    let branch_text = if show_branch {
                        format!(" · {}", workspace.branch)
                    } else {
                        String::new()
                    };
                    let agent_separator = " · ";
                    let mut row_spans = vec![
                        FtSpan::styled(format!("{selected} "), primary_style),
                        FtSpan::styled(workspace_name.clone(), workspace_label_style),
                    ];
                    if !branch_text.is_empty() {
                        row_spans.push(FtSpan::styled(branch_text.clone(), secondary_style));
                    }
                    row_spans.push(FtSpan::styled(agent_separator, secondary_style));
                    row_spans.push(FtSpan::styled(
                        workspace.agent.label().to_string(),
                        secondary_style
                            .fg(self.workspace_agent_color(workspace.agent))
                            .bold(),
                    ));
                    if workspace.is_orphaned {
                        row_spans.push(FtSpan::styled(
                            " · session ended",
                            secondary_style.fg(theme.peach),
                        ));
                    }
                    lines.push(FtLine::from_spans(row_spans));

                    if is_working {
                        let primary_label_x = inner.x.saturating_add(
                            u16::try_from(text_display_width("▸ ")).unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace_name.clone(),
                            workspace.agent,
                            primary_label_x,
                            row_y,
                        ));
                        let agent_prefix =
                            format!("{workspace_name}{branch_text}{agent_separator}");
                        let secondary_label_x = inner.x.saturating_add(
                            u16::try_from(
                                text_display_width("▸ ")
                                    .saturating_add(text_display_width(&agent_prefix)),
                            )
                            .unwrap_or(u16::MAX),
                        );
                        animated_labels.push((
                            workspace.agent.label().to_string(),
                            workspace.agent,
                            secondary_label_x,
                            row_y,
                        ));
                    }

                    if let Ok(data) = u64::try_from(idx) {
                        let row_height =
                            WORKSPACE_ITEM_HEIGHT.min(inner.bottom().saturating_sub(row_y));
                        let row_rect = Rect::new(inner.x, row_y, inner.width, row_height);
                        let _ = frame.register_hit(
                            row_rect,
                            HitId::new(HIT_ID_WORKSPACE_ROW),
                            FrameHitRegion::Content,
                            data,
                        );
                    }
                }
            }
        }

        Paragraph::new(FtText::from_lines(lines)).render(inner, frame);
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
    }

    fn render_divider(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let glyph = if self.divider_drag_active {
            "█"
        } else {
            "│"
        };
        let divider = std::iter::repeat_n(glyph, usize::from(area.height))
            .collect::<Vec<&str>>()
            .join("\n");
        let theme = ui_theme();
        Paragraph::new(divider)
            .style(Style::new().fg(if self.divider_drag_active {
                theme.blue
            } else {
                theme.overlay0
            }))
            .render(area, frame);
        let _ = frame.register_hit_region(
            Self::divider_hit_area(area, frame.width()),
            HitId::new(HIT_ID_DIVIDER),
        );
    }

    fn render_preview_pane(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let title = "Preview";
        let block =
            Block::new()
                .title(title)
                .borders(Borders::ALL)
                .border_style(self.pane_border_style(
                    self.state.focus == PaneFocus::Preview && !self.modal_open(),
                ));
        let inner = block.inner(area);
        block.render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_PREVIEW));

        if inner.is_empty() {
            return;
        }

        let selected_workspace = self.state.selected_workspace();
        let selected_agent = selected_workspace.map(|workspace| workspace.agent);
        let allow_cursor_overlay =
            self.preview_tab == PreviewTab::Git || selected_agent != Some(AgentType::Codex);
        let theme = ui_theme();
        let mut animated_labels: Vec<(String, AgentType, u16, u16)> = Vec::new();
        let selected_workspace_header = selected_workspace.map(|workspace| {
            let workspace_name = Self::workspace_display_name(workspace);
            let is_working = self.status_is_visually_working(
                Some(workspace.path.as_path()),
                workspace.status,
                true,
            );
            let branch_label = if workspace.branch != workspace_name {
                Some(workspace.branch.clone())
            } else {
                None
            };
            let age_label = self.relative_age_label(workspace.last_activity_unix_secs);
            (
                workspace_name,
                branch_label,
                age_label,
                is_working,
                workspace.agent,
                workspace.is_orphaned,
            )
        });

        let metadata_rows = usize::from(PREVIEW_METADATA_ROWS);
        let preview_height = usize::from(inner.height)
            .saturating_sub(metadata_rows)
            .max(1);

        let mut text_lines = vec![if let Some((
            name_label,
            branch_label,
            age_label,
            is_working,
            agent,
            is_orphaned,
        )) = selected_workspace_header.as_ref()
        {
            let mut spans = vec![FtSpan::styled(
                name_label.clone(),
                if *is_working {
                    Style::new().fg(self.workspace_agent_color(*agent)).bold()
                } else {
                    Style::new().fg(theme.text).bold()
                },
            )];
            if let Some(branch_label) = branch_label {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    branch_label.clone(),
                    Style::new().fg(theme.subtext0),
                ));
            }
            spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            spans.push(FtSpan::styled(
                agent.label().to_string(),
                Style::new().fg(self.workspace_agent_color(*agent)).bold(),
            ));
            if !age_label.is_empty() {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    age_label.clone(),
                    Style::new().fg(theme.overlay0),
                ));
            }
            if *is_orphaned {
                spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
                spans.push(FtSpan::styled(
                    "session ended",
                    Style::new().fg(theme.peach),
                ));
            }
            FtLine::from_spans(spans)
        } else {
            FtLine::from_spans(vec![FtSpan::styled(
                "none selected",
                Style::new().fg(theme.subtext0),
            )])
        }];
        let tab_active_style = Style::new().fg(theme.base).bg(theme.blue).bold();
        let tab_inactive_style = Style::new().fg(theme.subtext0).bg(theme.surface0);
        let mut tab_spans = Vec::new();
        for (index, tab) in [PreviewTab::Agent, PreviewTab::Git]
            .iter()
            .copied()
            .enumerate()
        {
            if index > 0 {
                tab_spans.push(FtSpan::raw(" ".to_string()));
            }
            let style = if tab == self.preview_tab {
                tab_active_style
            } else {
                tab_inactive_style
            };
            tab_spans.push(FtSpan::styled(format!(" {} ", tab.label()), style));
        }
        if let Some(workspace) = selected_workspace {
            tab_spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            tab_spans.push(FtSpan::styled(
                workspace.path.display().to_string(),
                Style::new().fg(theme.overlay0),
            ));
        } else {
            tab_spans.push(FtSpan::styled(" · ", Style::new().fg(theme.subtext0)));
            tab_spans.push(FtSpan::styled(
                "no workspace",
                Style::new().fg(theme.overlay0),
            ));
        }
        text_lines.push(FtLine::from_spans(tab_spans));
        if let Some((name_label, branch_label, _, true, agent, _)) =
            selected_workspace_header.as_ref()
        {
            animated_labels.push((name_label.clone(), *agent, inner.x, inner.y));
            let branch_prefix = branch_label
                .as_ref()
                .map_or(String::new(), |branch| format!(" · {branch}"));
            let agent_prefix = format!("{name_label}{branch_prefix} · ");
            animated_labels.push((
                agent.label().to_string(),
                *agent,
                inner.x.saturating_add(
                    u16::try_from(text_display_width(&agent_prefix)).unwrap_or(u16::MAX),
                ),
                inner.y,
            ));
        }

        let visible_range = self.preview_visible_range_for_height(preview_height);
        let visible_start = visible_range.0;
        let visible_end = visible_range.1;
        let visible_plain_lines = self.preview_plain_lines_range(visible_start, visible_end);
        match self.preview_tab {
            PreviewTab::Agent => {
                let mut visible_render_lines = if self.preview.render_lines.is_empty() {
                    Vec::new()
                } else {
                    let render_start = visible_start.min(self.preview.render_lines.len());
                    let render_end = visible_end.min(self.preview.render_lines.len());
                    if render_start < render_end {
                        self.preview.render_lines[render_start..render_end].to_vec()
                    } else {
                        Vec::new()
                    }
                };
                if visible_render_lines.len() < visible_plain_lines.len() {
                    visible_render_lines.extend(
                        visible_plain_lines[visible_render_lines.len()..]
                            .iter()
                            .cloned(),
                    );
                }
                if visible_render_lines.is_empty() && !visible_plain_lines.is_empty() {
                    visible_render_lines = visible_plain_lines.clone();
                }
                if allow_cursor_overlay {
                    self.apply_interactive_cursor_overlay_render(
                        &visible_plain_lines,
                        &mut visible_render_lines,
                        preview_height,
                    );
                }

                if visible_render_lines.is_empty() {
                    text_lines.push(FtLine::raw("(no preview output)"));
                } else {
                    text_lines.extend(
                        visible_render_lines
                            .iter()
                            .map(|line| ansi_line_to_styled_line(line)),
                    );
                }
            }
            PreviewTab::Git => {
                let mut visible_render_lines = if self.preview.render_lines.is_empty() {
                    Vec::new()
                } else {
                    let render_start = visible_start.min(self.preview.render_lines.len());
                    let render_end = visible_end.min(self.preview.render_lines.len());
                    if render_start < render_end {
                        self.preview.render_lines[render_start..render_end].to_vec()
                    } else {
                        Vec::new()
                    }
                };
                if visible_render_lines.len() < visible_plain_lines.len() {
                    visible_render_lines.extend(
                        visible_plain_lines[visible_render_lines.len()..]
                            .iter()
                            .cloned(),
                    );
                }
                if visible_render_lines.is_empty() && !visible_plain_lines.is_empty() {
                    visible_render_lines = visible_plain_lines.clone();
                }
                if allow_cursor_overlay {
                    self.apply_interactive_cursor_overlay_render(
                        &visible_plain_lines,
                        &mut visible_render_lines,
                        preview_height,
                    );
                }

                if visible_render_lines.is_empty() {
                    let fallback = if let Some(workspace) = selected_workspace {
                        let session_name = git_session_name_for_workspace(workspace);
                        if self.lazygit_failed_sessions.contains(&session_name) {
                            "(lazygit launch failed)"
                        } else if self.lazygit_ready_sessions.contains(&session_name) {
                            "(no lazygit output yet)"
                        } else {
                            "(launching lazygit...)"
                        }
                    } else {
                        "(no workspace selected)"
                    };
                    text_lines.push(FtLine::raw(fallback.to_string()));
                } else {
                    text_lines.extend(
                        visible_render_lines
                            .iter()
                            .map(|line| ansi_line_to_styled_line(line)),
                    );
                }
            }
        }

        Paragraph::new(FtText::from_lines(text_lines)).render(inner, frame);
        for (label, agent, x, y) in animated_labels {
            if y >= inner.bottom() {
                continue;
            }
            let width = inner.right().saturating_sub(x);
            if width == 0 {
                continue;
            }
            self.render_activity_effect_label(&label, agent, Rect::new(x, y, width, 1), frame);
        }
        self.apply_preview_selection_highlight_cells(
            frame,
            inner,
            &visible_plain_lines,
            visible_start,
        );
    }

    fn render_status_line(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let theme = ui_theme();
        let hints = self.keybind_hints_line();
        let base_style = Style::new().bg(theme.mantle).fg(theme.text);
        let chip_style = Style::new().bg(theme.surface0).fg(theme.blue).bold();
        let key_style = Style::new().bg(theme.mantle).fg(theme.lavender).bold();
        let text_style = Style::new().bg(theme.mantle).fg(theme.subtext0);
        let sep_style = Style::new().bg(theme.mantle).fg(theme.overlay0);

        let mut left: Vec<FtSpan> = vec![
            FtSpan::styled(" ".to_string(), base_style),
            FtSpan::styled(" Keys ".to_string(), chip_style),
            FtSpan::styled(" ".to_string(), base_style),
        ];
        left.extend(keybind_hint_spans(
            hints.as_str(),
            text_style,
            key_style,
            sep_style,
        ));

        let line = chrome_bar_line(
            usize::from(area.width),
            base_style,
            left,
            Vec::new(),
            Vec::new(),
        );
        Paragraph::new(FtText::from_line(line)).render(area, frame);
        let _ = frame.register_hit_region(area, HitId::new(HIT_ID_STATUS));
    }

    #[cfg(test)]
    pub(super) fn shell_lines(&self, preview_height: usize) -> Vec<String> {
        let mut lines = vec![
            format!("Grove Shell | Repo: {}", self.repo_name),
            format!(
                "Mode: {} | Focus: {}",
                self.mode_label(),
                self.focus_label()
            ),
            "Workspaces (j/k, arrows, Tab/h/l focus, Enter preview, n create, e edit, m merge, u update, s/x start-stop, D delete, S settings, ? help, ! unsafe, Esc list, mouse)"
                .to_string(),
        ];

        match &self.discovery_state {
            DiscoveryState::Error(message) => {
                lines.push(format!("! discovery failed: {message}"));
            }
            DiscoveryState::Empty => {
                lines.push("No workspaces discovered".to_string());
            }
            DiscoveryState::Ready => {
                for (idx, workspace) in self.state.workspaces.iter().enumerate() {
                    let selected = if idx == self.state.selected_index {
                        "▸"
                    } else {
                        " "
                    };
                    let workspace_name = Self::workspace_display_name(workspace);
                    lines.push(format!(
                        "{} {} | {} | {} | {}{}",
                        selected,
                        workspace_name,
                        workspace.branch,
                        workspace.agent.label(),
                        workspace.path.display(),
                        if workspace.is_orphaned {
                            " | session ended"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }

        if let Some(dialog) = &self.launch_dialog {
            lines.push(String::new());
            lines.push("Start Agent Dialog".to_string());
            lines.push(format!("Field: {}", dialog.focused_field.label()));
            lines.push(format!(
                "Prompt: {}",
                if dialog.prompt.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.prompt.clone()
                }
            ));
            lines.push(format!(
                "Pre-launch command: {}",
                if dialog.pre_launch_command.is_empty() {
                    "(empty)".to_string()
                } else {
                    dialog.pre_launch_command.clone()
                }
            ));
            lines.push(format!(
                "Unsafe launch: {}",
                if dialog.skip_permissions { "on" } else { "off" }
            ));
        }
        if let Some(dialog) = &self.delete_dialog {
            lines.push(String::new());
            lines.push("Delete Workspace Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.branch));
            lines.push(format!(
                "Delete local branch: {}",
                if dialog.delete_local_branch {
                    "on"
                } else {
                    "off"
                }
            ));
        }
        if let Some(dialog) = &self.merge_dialog {
            lines.push(String::new());
            lines.push("Merge Workspace Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.workspace_branch));
            lines.push(format!("Base branch: {}", dialog.base_branch));
            lines.push(format!(
                "Cleanup worktree: {}",
                if dialog.cleanup_workspace {
                    "on"
                } else {
                    "off"
                }
            ));
            lines.push(format!(
                "Cleanup local branch: {}",
                if dialog.cleanup_local_branch {
                    "on"
                } else {
                    "off"
                }
            ));
        }
        if let Some(dialog) = &self.update_from_base_dialog {
            lines.push(String::new());
            lines.push("Update From Base Dialog".to_string());
            lines.push(format!("Workspace: {}", dialog.workspace_name));
            lines.push(format!("Branch: {}", dialog.workspace_branch));
            lines.push(format!("Base branch: {}", dialog.base_branch));
        }

        let selected_workspace = self
            .state
            .selected_workspace()
            .map(|workspace| {
                format!(
                    "{} ({}, {})",
                    Self::workspace_display_name(workspace),
                    workspace.branch,
                    workspace.path.display()
                )
            })
            .unwrap_or_else(|| "none".to_string());

        lines.push(String::new());
        lines.push("Preview Pane".to_string());
        lines.push(format!("Selected workspace: {}", selected_workspace));
        let mut visible_lines = self.preview.visible_lines(preview_height);
        self.apply_interactive_cursor_overlay(&mut visible_lines, preview_height);
        if visible_lines.is_empty() {
            lines.push("(no preview output)".to_string());
        } else {
            lines.extend(visible_lines);
        }
        lines.push(self.status_bar_line());

        lines
    }

    pub(super) fn view_layout_for_size(
        width: u16,
        height: u16,
        sidebar_width_pct: u16,
    ) -> ViewLayout {
        Self::view_layout_for_size_with_sidebar(width, height, sidebar_width_pct, false)
    }

    pub(super) fn view_layout_for_size_with_sidebar(
        width: u16,
        height: u16,
        sidebar_width_pct: u16,
        sidebar_hidden: bool,
    ) -> ViewLayout {
        let area = Rect::from_size(width, height);
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(HEADER_HEIGHT),
                Constraint::Fill,
                Constraint::Fixed(STATUS_HEIGHT),
            ])
            .split(area);

        let sidebar_width = if sidebar_hidden {
            0
        } else {
            ((u32::from(rows[1].width) * u32::from(sidebar_width_pct)) / 100)
                .try_into()
                .unwrap_or(rows[1].width)
        };
        let divider_width = if sidebar_hidden { 0 } else { DIVIDER_WIDTH };
        let cols = Flex::horizontal()
            .constraints([
                Constraint::Fixed(sidebar_width),
                Constraint::Fixed(divider_width),
                Constraint::Fill,
            ])
            .split(rows[1]);

        ViewLayout {
            header: rows[0],
            sidebar: cols[0],
            divider: cols[1],
            preview: cols[2],
            status: rows[2],
        }
    }

    fn effective_viewport_size(&self) -> (u16, u16) {
        let from_hit_grid = self
            .last_hit_grid
            .borrow()
            .as_ref()
            .map(|grid| (grid.width(), grid.height()));
        let (width, height) = from_hit_grid.unwrap_or((self.viewport_width, self.viewport_height));
        (width.max(1), height.max(1))
    }

    pub(super) fn view_layout(&self) -> ViewLayout {
        let (width, height) = self.effective_viewport_size();
        Self::view_layout_for_size_with_sidebar(
            width,
            height,
            self.sidebar_width_pct,
            self.sidebar_hidden,
        )
    }

    fn divider_hit_area(divider: Rect, viewport_width: u16) -> Rect {
        let left = divider.x.saturating_sub(1);
        let right = divider.right().saturating_add(1).min(viewport_width);
        Rect::new(left, divider.y, right.saturating_sub(left), divider.height)
    }

    pub(super) fn hit_region_for_point(&self, x: u16, y: u16) -> (HitRegion, Option<u64>) {
        if let Some((id, _region, data)) = self
            .last_hit_grid
            .borrow()
            .as_ref()
            .and_then(|grid| grid.hit_test(x, y))
        {
            let mapped = match id.id() {
                HIT_ID_HEADER => HitRegion::Header,
                HIT_ID_STATUS => HitRegion::StatusLine,
                HIT_ID_DIVIDER => HitRegion::Divider,
                HIT_ID_PREVIEW => HitRegion::Preview,
                HIT_ID_WORKSPACE_LIST | HIT_ID_WORKSPACE_ROW => HitRegion::WorkspaceList,
                HIT_ID_CREATE_DIALOG
                | HIT_ID_LAUNCH_DIALOG
                | HIT_ID_DELETE_DIALOG
                | HIT_ID_KEYBIND_HELP_DIALOG => HitRegion::Outside,
                _ => HitRegion::Outside,
            };
            let row_data = if id.id() == HIT_ID_WORKSPACE_ROW {
                Some(data)
            } else {
                None
            };
            return (mapped, row_data);
        }

        let (viewport_width, viewport_height) = self.effective_viewport_size();
        let layout = self.view_layout();

        if x >= viewport_width || y >= viewport_height {
            return (HitRegion::Outside, None);
        }
        if y < layout.header.bottom() {
            return (HitRegion::Header, None);
        }
        if y >= layout.status.y {
            return (HitRegion::StatusLine, None);
        }

        let divider_area = Self::divider_hit_area(layout.divider, viewport_width);
        if x >= divider_area.x && x < divider_area.right() {
            return (HitRegion::Divider, None);
        }
        if x >= layout.sidebar.x && x < layout.sidebar.right() {
            return (HitRegion::WorkspaceList, None);
        }
        if x >= layout.preview.x && x < layout.preview.right() {
            return (HitRegion::Preview, None);
        }

        (HitRegion::Outside, None)
    }

    pub(super) fn interactive_cursor_target(
        &self,
        preview_height: usize,
    ) -> Option<(usize, usize, bool)> {
        let interactive = self.interactive.as_ref()?;
        if self.preview.lines.is_empty() {
            return None;
        }

        let pane_height = usize::from(interactive.pane_height.max(1));
        let cursor_row = usize::from(interactive.cursor_row);
        if cursor_row >= pane_height {
            return None;
        }

        let preview_len = self.preview.lines.len();
        let pane_start = preview_len.saturating_sub(pane_height);
        let cursor_line = pane_start.saturating_add(cursor_row);
        if cursor_line >= preview_len {
            return None;
        }

        let end = preview_len.saturating_sub(self.preview.offset);
        let start = end.saturating_sub(preview_height);
        if cursor_line < start || cursor_line >= end {
            return None;
        }

        let visible_index = cursor_line - start;
        Some((
            visible_index,
            usize::from(interactive.cursor_col),
            interactive.cursor_visible,
        ))
    }

    #[cfg(test)]
    fn apply_interactive_cursor_overlay(
        &self,
        visible_lines: &mut [String],
        preview_height: usize,
    ) {
        let Some((visible_index, cursor_col, cursor_visible)) =
            self.interactive_cursor_target(preview_height)
        else {
            return;
        };

        let Some(line) = visible_lines.get_mut(visible_index) else {
            return;
        };

        *line = render_cursor_overlay(line, cursor_col, cursor_visible);
    }

    fn apply_interactive_cursor_overlay_render(
        &self,
        plain_visible_lines: &[String],
        render_visible_lines: &mut [String],
        preview_height: usize,
    ) {
        let Some((visible_index, cursor_col, cursor_visible)) =
            self.interactive_cursor_target(preview_height)
        else {
            return;
        };

        let Some(plain_line) = plain_visible_lines.get(visible_index) else {
            return;
        };
        let Some(render_line) = render_visible_lines.get_mut(visible_index) else {
            return;
        };

        *render_line =
            render_cursor_overlay_ansi(render_line, plain_line, cursor_col, cursor_visible);
    }
}

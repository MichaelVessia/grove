use super::*;

impl GroveApp {
    const PREVIEW_MOUSE_SCROLL_LINES: i32 = 3;

    fn preview_tab_at_pointer(&self, x: u16, y: u16) -> Option<PreviewTab> {
        let layout = self.view_layout();
        if layout.preview.is_empty() {
            return None;
        }

        let preview_inner = Block::new().borders(Borders::ALL).inner(layout.preview);
        if preview_inner.is_empty() || preview_inner.height < PREVIEW_METADATA_ROWS {
            return None;
        }

        let tab_row_y = preview_inner.y.saturating_add(1);
        if y != tab_row_y {
            return None;
        }

        let mut tab_x = preview_inner.x;
        for (index, tab) in [PreviewTab::Agent, PreviewTab::Shell, PreviewTab::Git]
            .iter()
            .copied()
            .enumerate()
        {
            if index > 0 {
                tab_x = tab_x.saturating_add(1);
            }
            let Some(tab_width) = u16::try_from(tab.label().len().saturating_add(2)).ok() else {
                continue;
            };
            let tab_end = tab_x.saturating_add(tab_width).min(preview_inner.right());
            if x >= tab_x && x < tab_end {
                return Some(tab);
            }
            tab_x = tab_x.saturating_add(tab_width);
        }

        None
    }

    fn sidebar_ratio_for_drag_pointer(&self, pointer_x: i32) -> u16 {
        let layout = self.view_layout();
        let total_width = layout.preview.right().saturating_sub(layout.sidebar.x);
        if total_width == 0 {
            return self.sidebar_width_pct;
        }

        let left = i32::from(layout.sidebar.x);
        let right = left.saturating_add(i32::from(total_width).saturating_sub(1));
        let clamped_x = pointer_x.clamp(left, right);
        let relative_x = clamped_x.saturating_sub(left);
        let Some(relative_x_u16) = u16::try_from(relative_x).ok() else {
            return self.sidebar_width_pct;
        };

        ratio_from_drag(total_width, relative_x_u16)
    }

    fn sidebar_workspace_index_at_y(&self, y: u16) -> Option<usize> {
        if self.projects.is_empty() {
            return None;
        }

        if matches!(self.discovery_state, DiscoveryState::Error(_))
            && self.state.workspaces.is_empty()
        {
            return None;
        }

        let layout = self.view_layout();
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
        if y < sidebar_inner.y || y >= sidebar_inner.bottom() {
            return None;
        }

        let target_row = usize::from(y.saturating_sub(sidebar_inner.y));
        let mut visual_row = 0usize;
        for (project_index, project) in self.projects.iter().enumerate() {
            if project_index > 0 {
                if visual_row == target_row {
                    return None;
                }
                visual_row = visual_row.saturating_add(1);
            }

            if visual_row == target_row {
                return None;
            }
            visual_row = visual_row.saturating_add(1);

            let workspace_indices: Vec<usize> = self
                .state
                .workspaces
                .iter()
                .enumerate()
                .filter(|(_, workspace)| {
                    workspace
                        .project_path
                        .as_ref()
                        .is_some_and(|path| refer_to_same_location(path, &project.path))
                })
                .map(|(index, _)| index)
                .collect();
            if workspace_indices.is_empty() {
                if visual_row == target_row {
                    return None;
                }
                visual_row = visual_row.saturating_add(1);
                continue;
            }

            for workspace_index in workspace_indices {
                if visual_row == target_row {
                    return Some(workspace_index);
                }
                visual_row = visual_row.saturating_add(usize::from(WORKSPACE_ITEM_HEIGHT));
            }
        }

        None
    }

    fn select_workspace_by_mouse(&mut self, y: u16) {
        let Some(row) = self.sidebar_workspace_index_at_y(y) else {
            return;
        };

        if row != self.state.selected_index {
            self.state.selected_index = row;
            self.handle_workspace_selection_changed();
        }
    }

    pub(super) fn select_workspace_by_index(&mut self, index: usize) {
        if index >= self.state.workspaces.len() {
            return;
        }
        if index == self.state.selected_index {
            return;
        }

        self.state.selected_index = index;
        self.handle_workspace_selection_changed();
    }

    pub(super) fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        if let Some(state) = self.interactive.as_mut() {
            state.note_mouse_event(Instant::now());
        }

        let (region, row_data) = self.hit_region_for_point(mouse_event.x, mouse_event.y);
        let mut event = LogEvent::new("mouse", "event")
            .with_data("x", Value::from(mouse_event.x))
            .with_data("y", Value::from(mouse_event.y))
            .with_data("kind", Value::from(format!("{:?}", mouse_event.kind)))
            .with_data("region", Value::from(Self::hit_region_name(region)))
            .with_data("modal_open", Value::from(self.modal_open()))
            .with_data("interactive", Value::from(self.interactive.is_some()))
            .with_data("divider_drag_active", Value::from(self.divider_drag_active))
            .with_data("focus", Value::from(Self::focus_name(self.state.focus)))
            .with_data("mode", Value::from(Self::mode_name(self.state.mode)));
        if let Some(row_data) = row_data {
            event = event.with_data("row_data", Value::from(row_data));
        }
        if matches!(region, HitRegion::Preview)
            && let Some(point) = self.preview_text_point_at(mouse_event.x, mouse_event.y)
        {
            event = event
                .with_data(
                    "mapped_line",
                    Value::from(u64::try_from(point.line).unwrap_or(u64::MAX)),
                )
                .with_data(
                    "mapped_col",
                    Value::from(u64::try_from(point.col).unwrap_or(u64::MAX)),
                );
            event = self.add_selection_point_snapshot_fields(event, "mapped_", point);
        }
        self.event_log.log(event);

        if self.modal_open() {
            return;
        }

        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => match region {
                HitRegion::Divider => {
                    self.divider_drag_active = true;
                    let layout = self.view_layout();
                    self.divider_drag_pointer_offset =
                        i32::from(mouse_event.x).saturating_sub(i32::from(layout.divider.x));
                }
                HitRegion::WorkspaceList => {
                    if self.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    if let Some(row_data) = row_data {
                        if let Ok(index) = usize::try_from(row_data) {
                            self.select_workspace_by_index(index);
                        }
                    } else {
                        self.select_workspace_by_mouse(mouse_event.y);
                    }
                }
                HitRegion::Preview => {
                    if let Some(next_tab) =
                        self.preview_tab_at_pointer(mouse_event.x, mouse_event.y)
                    {
                        if self.interactive.is_some() {
                            self.exit_interactive_to_list();
                        }
                        reduce(&mut self.state, Action::EnterPreviewMode);
                        self.select_preview_tab(next_tab);
                        self.clear_preview_selection();
                    } else {
                        let interactive_before_click = self.interactive.is_some();
                        self.enter_preview_or_interactive();
                        if self.interactive.is_some() {
                            if interactive_before_click {
                                self.prepare_preview_selection_drag(mouse_event.x, mouse_event.y);
                            } else {
                                self.clear_preview_selection();
                            }
                        } else {
                            self.clear_preview_selection();
                        }
                    }
                }
                HitRegion::StatusLine | HitRegion::Header | HitRegion::Outside => {}
            },
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.divider_drag_active {
                    let drag_pointer =
                        i32::from(mouse_event.x).saturating_sub(self.divider_drag_pointer_offset);
                    let ratio = self.sidebar_ratio_for_drag_pointer(drag_pointer);
                    self.set_sidebar_ratio(ratio);
                } else if self.interactive.is_some() {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Moved => {
                if self.interactive.is_some() && !self.divider_drag_active {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.divider_drag_active = false;
                self.divider_drag_pointer_offset = 0;
                self.finish_preview_selection_drag(mouse_event.x, mouse_event.y);
            }
            MouseEventKind::ScrollUp => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    if self.preview_scroll_tab_is_focused() {
                        self.scroll_preview(-Self::PREVIEW_MOUSE_SCROLL_LINES);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    if self.preview_scroll_tab_is_focused() {
                        self.scroll_preview(Self::PREVIEW_MOUSE_SCROLL_LINES);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn persist_sidebar_ratio(&mut self) {
        let config = GroveConfig {
            multiplexer: self.multiplexer,
            sidebar_width_pct: self.sidebar_width_pct,
            projects: self.projects.clone(),
        };
        if let Err(error) = crate::infrastructure::config::save_to_path(&self.config_path, &config)
        {
            self.last_tmux_error = Some(format!("sidebar ratio persist failed: {error}"));
        }
    }
}

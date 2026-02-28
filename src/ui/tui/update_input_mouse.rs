use super::*;

impl GroveApp {
    const PREVIEW_MOUSE_SCROLL_LINES: i32 = 3;
    const SIDEBAR_MOUSE_SCROLL_WORKSPACES: usize = 1;
    const SIDEBAR_MOUSE_SCROLL_DEBOUNCE_MS: u64 = 50;
    const CREATE_DIALOG_TAB_ROW_OFFSET: u16 = 2;

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

    fn create_dialog_tab_at_pointer(&self, x: u16, y: u16) -> Option<CreateDialogTab> {
        self.create_dialog()?;

        let width = self.viewport_width.max(1);
        let height = self.viewport_height.max(1);
        if width < 20 || height < 10 {
            return None;
        }

        let dialog_width = width.saturating_sub(8).min(90);
        let dialog_height = 25u16;
        let dialog_x = width.saturating_sub(dialog_width) / 2;
        let dialog_y = height.saturating_sub(dialog_height) / 2;
        let dialog_area = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);
        let inner = Block::new().borders(Borders::ALL).inner(dialog_area);
        if inner.is_empty() {
            return None;
        }

        let tab_row_y = inner.y.saturating_add(Self::CREATE_DIALOG_TAB_ROW_OFFSET);
        if y != tab_row_y {
            return None;
        }

        let mut tab_x = inner.x;
        for (index, tab) in [CreateDialogTab::Manual, CreateDialogTab::PullRequest]
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
            let tab_end = tab_x.saturating_add(tab_width).min(inner.right());
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

    fn sidebar_workspace_index_at_point(&self, x: u16, y: u16) -> Option<usize> {
        let layout = self.view_layout();
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(layout.sidebar);
        if y < sidebar_inner.y || y >= sidebar_inner.bottom() {
            return None;
        }

        let row_map = self.sidebar_workspace_row_map();
        if row_map.is_empty() {
            return None;
        }

        let viewport_rows = usize::from(sidebar_inner.height.max(1));
        let needs_scrollbar = row_map.len() > viewport_rows;
        if needs_scrollbar && x >= sidebar_inner.right().saturating_sub(1) {
            return None;
        }

        let max_offset = row_map.len().saturating_sub(viewport_rows);
        let scroll_offset = {
            let list_state = self.sidebar_list_state.borrow();
            list_state.scroll_offset().min(max_offset)
        };
        let target_row = usize::from(y.saturating_sub(sidebar_inner.y));
        let row_index = scroll_offset.saturating_add(target_row);
        row_map.get(row_index).copied().flatten()
    }

    fn select_workspace_by_mouse(&mut self, x: u16, y: u16) {
        let Some(row) = self.sidebar_workspace_index_at_point(x, y) else {
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

    fn open_url_in_browser(&self, url: &str) -> Result<(), String> {
        #[cfg(test)]
        {
            let _ = url;
            Ok(())
        }

        #[cfg(not(test))]
        {
            let mut command = if cfg!(target_os = "macos") {
                let mut command = Command::new("open");
                command.arg(url);
                command
            } else if cfg!(target_os = "windows") {
                let mut command = Command::new("cmd");
                command.args(["/C", "start", "", url]);
                command
            } else {
                let mut command = Command::new("xdg-open");
                command.arg(url);
                command
            };

            command
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("failed opening url: {error}"))
        }
    }

    fn open_workspace_pull_request_link(&mut self, data: Option<u64>) {
        let Some(data) = data else {
            return;
        };
        let Some((workspace_index, pull_request_index)) = decode_workspace_pr_hit_data(data) else {
            return;
        };

        self.select_workspace_by_index(workspace_index);
        let Some(workspace) = self.state.workspaces.get(workspace_index) else {
            return;
        };
        let Some(pull_request) = workspace.pull_requests.get(pull_request_index) else {
            return;
        };

        if let Err(error) = self.open_url_in_browser(pull_request.url.as_str()) {
            self.show_error_toast(error);
        }
    }

    fn should_handle_sidebar_mouse_scroll(&mut self, delta: i8, now: Instant) -> bool {
        if let Some(last_scroll_at) = self.last_sidebar_mouse_scroll_at
            && self.last_sidebar_mouse_scroll_delta == delta
            && now.saturating_duration_since(last_scroll_at)
                < Duration::from_millis(Self::SIDEBAR_MOUSE_SCROLL_DEBOUNCE_MS)
        {
            return false;
        }

        self.last_sidebar_mouse_scroll_at = Some(now);
        self.last_sidebar_mouse_scroll_delta = delta;
        true
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
            .with_data("focus", Value::from(self.state.focus.name()))
            .with_data("mode", Value::from(self.state.mode.name()));
        if let Some(row_data) = row_data {
            event = event.with_data("row_data", Value::from(row_data));
        }
        if matches!(region, HitRegion::Preview)
            && let Some(point) = self.preview_text_point_at(mouse_event.x, mouse_event.y)
        {
            event = event
                .with_data("mapped_line", Value::from(usize_to_u64(point.line)))
                .with_data("mapped_col", Value::from(usize_to_u64(point.col)));
            event = self.add_selection_point_snapshot_fields(event, "mapped_", point);
        }
        self.event_log.log(event);

        if self.modal_open() {
            if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left))
                && let Some(next_tab) =
                    self.create_dialog_tab_at_pointer(mouse_event.x, mouse_event.y)
                && let Some(dialog) = self.create_dialog_mut()
                && dialog.tab != next_tab
            {
                dialog.tab = next_tab;
                dialog.focused_field = CreateDialogField::first_for_tab(next_tab);
            }
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
                        self.select_workspace_by_mouse(mouse_event.x, mouse_event.y);
                    }
                }
                HitRegion::WorkspacePullRequest => {
                    if self.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    self.open_workspace_pull_request_link(row_data);
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
                if matches!(region, HitRegion::WorkspaceList) {
                    if self.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    if self.should_handle_sidebar_mouse_scroll(-1, Instant::now()) {
                        for _ in 0..Self::SIDEBAR_MOUSE_SCROLL_WORKSPACES {
                            self.move_selection(Action::MoveSelectionUp);
                        }
                    }
                } else if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    if self.preview_scroll_tab_is_focused() {
                        self.scroll_preview(-Self::PREVIEW_MOUSE_SCROLL_LINES);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(region, HitRegion::WorkspaceList) {
                    if self.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    if self.should_handle_sidebar_mouse_scroll(1, Instant::now()) {
                        for _ in 0..Self::SIDEBAR_MOUSE_SCROLL_WORKSPACES {
                            self.move_selection(Action::MoveSelectionDown);
                        }
                    }
                } else if matches!(region, HitRegion::Preview) {
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
        // Global settings are loaded from config.toml and intentionally not written at runtime.
    }
}

use super::update_prelude::*;

impl GroveApp {
    const PREVIEW_MOUSE_SCROLL_LINES: i32 = 3;
    const SIDEBAR_MOUSE_SCROLL_WORKSPACES: usize = 1;
    const SIDEBAR_MOUSE_SCROLL_DEBOUNCE_MS: u64 = 50;
    const CREATE_DIALOG_TAB_ROW_OFFSET: u16 = 2;
    const DIVIDER_POINTER_ID: u32 = 1;

    pub(super) fn preview_tab_id_at_pointer(&self, x: u16, y: u16) -> Option<u64> {
        let (_, _, preview_rect) = self.effective_workspace_rects();
        if preview_rect.is_empty() {
            return None;
        }

        let preview_inner = Block::new().borders(Borders::ALL).inner(preview_rect);
        if preview_inner.is_empty() || preview_inner.height < PREVIEW_METADATA_ROWS {
            return None;
        }

        let tab_row_y = preview_inner.y.saturating_add(1);
        if y != tab_row_y {
            return None;
        }

        let workspace = self.state.selected_workspace()?;
        let tabs = self.workspace_tabs.get(workspace.path.as_path())?;
        let mut tab_x = preview_inner.x;
        for (index, tab) in tabs.tabs.iter().enumerate() {
            if index > 0 {
                tab_x = tab_x.saturating_add(1);
            }
            let label = format!(" {} ", tab.title);
            let Some(tab_width) = u16::try_from(text_display_width(label.as_str())).ok() else {
                continue;
            };
            let tab_end = tab_x.saturating_add(tab_width).min(preview_inner.right());
            if x >= tab_x && x < tab_end {
                return Some(tab.id);
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
        let dialog_area =
            Self::centered_modal_rect(Rect::from_size(width, height), dialog_width, dialog_height);
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

    fn create_dialog_tab_from_hit_grid(&self, x: u16, y: u16) -> Option<CreateDialogTab> {
        self.last_hit_grid
            .borrow()
            .as_ref()
            .and_then(|grid| grid.hit_test(x, y))
            .and_then(|(id, _region, data)| {
                if id.id() == HIT_ID_CREATE_DIALOG_TAB {
                    decode_create_dialog_tab_hit_data(data)
                } else {
                    None
                }
            })
    }

    fn sidebar_ratio_for_drag_pointer(&self, pointer_x: i32) -> u16 {
        let (sidebar_rect, _, preview_rect) = self.effective_workspace_rects();
        let total_width = preview_rect.right().saturating_sub(sidebar_rect.x);
        if total_width == 0 {
            return self.sidebar_width_pct;
        }

        let left = i32::from(sidebar_rect.x);
        let right = left.saturating_add(i32::from(total_width).saturating_sub(1));
        let clamped_x = pointer_x.clamp(left, right);
        let relative_x = clamped_x.saturating_sub(left);
        let Some(relative_x_u16) = u16::try_from(relative_x).ok() else {
            return self.sidebar_width_pct;
        };

        ratio_from_drag(total_width, relative_x_u16)
    }

    fn next_divider_resize_event_sequence(&mut self) -> u64 {
        let sequence = self.divider_resize_event_seq;
        self.divider_resize_event_seq = self.divider_resize_event_seq.saturating_add(1);
        sequence
    }

    fn divider_resize_target(&self) -> Option<ftui::layout::pane::PaneResizeTarget> {
        if self.sidebar_hidden {
            return None;
        }
        self.panes.workspace_resize_target()
    }

    fn divider_resize_current_position(&self) -> Option<ftui::layout::pane::PanePointerPosition> {
        match self.divider_resize.state() {
            ftui::layout::pane::PaneDragResizeState::Idle => None,
            ftui::layout::pane::PaneDragResizeState::Armed { current, .. }
            | ftui::layout::pane::PaneDragResizeState::Dragging { current, .. } => Some(current),
        }
    }

    fn pane_pointer_position(x: u16, y: u16) -> ftui::layout::pane::PanePointerPosition {
        ftui::layout::pane::PanePointerPosition::new(i32::from(x), i32::from(y))
    }

    fn apply_divider_resize_transition(
        &mut self,
        transition: &ftui::layout::pane::PaneDragResizeTransition,
    ) {
        match transition.effect {
            ftui::layout::pane::PaneDragResizeEffect::DragStarted { total_delta_x, .. }
            | ftui::layout::pane::PaneDragResizeEffect::DragUpdated { total_delta_x, .. }
            | ftui::layout::pane::PaneDragResizeEffect::Committed { total_delta_x, .. } => {
                let drag_pointer = self.divider_resize_anchor_x.saturating_add(total_delta_x);
                let ratio = self.sidebar_ratio_for_drag_pointer(drag_pointer);
                self.set_sidebar_ratio(ratio);
            }
            ftui::layout::pane::PaneDragResizeEffect::Armed { .. }
            | ftui::layout::pane::PaneDragResizeEffect::Canceled { .. }
            | ftui::layout::pane::PaneDragResizeEffect::KeyboardApplied { .. }
            | ftui::layout::pane::PaneDragResizeEffect::WheelApplied { .. }
            | ftui::layout::pane::PaneDragResizeEffect::Noop { .. } => {}
        }

        if matches!(transition.to, ftui::layout::pane::PaneDragResizeState::Idle) {
            self.divider_resize_anchor_x = 0;
        }
    }

    fn apply_divider_resize_event(&mut self, kind: ftui::layout::pane::PaneSemanticInputEventKind) {
        let event = ftui::layout::pane::PaneSemanticInputEvent::new(
            self.next_divider_resize_event_sequence(),
            kind,
        );
        let Ok(transition) = self.divider_resize.apply_event(&event) else {
            return;
        };
        self.apply_divider_resize_transition(&transition);
    }

    pub(super) fn sidebar_selection_at_point(&self, x: u16, y: u16) -> Option<SidebarSelectable> {
        let (sidebar_rect, _, _) = self.effective_workspace_rects();
        let sidebar_inner = Block::new().borders(Borders::ALL).inner(sidebar_rect);
        if y < sidebar_inner.y || y >= sidebar_inner.bottom() {
            return None;
        }

        let row_map = self.sidebar_selectable_row_map();
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

    pub(super) fn select_sidebar_target(&mut self, target: SidebarSelectable) {
        match target {
            SidebarSelectable::Attention(item_index) => self.select_attention_item(item_index),
            SidebarSelectable::Workspace(index) => {
                self.selected_attention_item = None;
                self.select_workspace_by_index(index);
            }
        }
    }

    pub(super) fn select_workspace_by_mouse(&mut self, x: u16, y: u16) {
        let Some(target) = self.sidebar_selection_at_point(x, y) else {
            return;
        };
        self.select_sidebar_target(target);
    }

    pub(super) fn select_workspace_by_index(&mut self, index: usize) {
        if index >= self.state.workspaces.len() {
            return;
        }
        self.selected_attention_item = None;
        if index == self.state.selected_index {
            return;
        }

        self.state.select_index(index);
        self.handle_workspace_selection_changed();
    }

    pub(super) fn select_attention_item(&mut self, item_index: usize) {
        let Some(workspace_path) = self
            .attention_items
            .get(item_index)
            .map(|item| item.workspace_path.clone())
        else {
            return;
        };
        self.selected_attention_item = Some(item_index);
        if let Some(workspace_index) = self
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.path == workspace_path)
        {
            let changed = self.state.select_index(workspace_index);
            if changed {
                self.handle_workspace_selection_changed();
            }
        }
        self.focus_selected_workspace_attention_tab();
        self.poll_preview();
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

    fn project_dialog_list_row_at_pointer(&self, x: u16, y: u16) -> Option<usize> {
        self.last_hit_grid
            .borrow()
            .as_ref()
            .and_then(|grid| grid.hit_test(x, y))
            .and_then(|(id, _region, data)| {
                if id.id() == HIT_ID_PROJECT_DIALOG_LIST {
                    usize::try_from(data).ok()
                } else {
                    None
                }
            })
    }

    fn project_add_dialog_result_row_at_pointer(&self, x: u16, y: u16) -> Option<usize> {
        self.last_hit_grid
            .borrow()
            .as_ref()
            .and_then(|grid| grid.hit_test(x, y))
            .and_then(|(id, _region, data)| {
                if id.id() == HIT_ID_PROJECT_ADD_RESULTS_LIST {
                    usize::try_from(data).ok()
                } else {
                    None
                }
            })
    }

    fn rect_contains_point(area: Rect, x: u16, y: u16) -> bool {
        x >= area.x && x < area.right() && y >= area.y && y < area.bottom()
    }

    pub(super) fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        if let Some(state) = self.session.interactive.as_mut() {
            state.note_mouse_event(Instant::now());
        }

        let (region, row_data) = self.hit_region_for_point(mouse_event.x, mouse_event.y);
        let mut event = LogEvent::new("mouse", "event")
            .with_data("x", Value::from(mouse_event.x))
            .with_data("y", Value::from(mouse_event.y))
            .with_data("kind", Value::from(format!("{:?}", mouse_event.kind)))
            .with_data("region", Value::from(Self::hit_region_name(region)))
            .with_data("modal_open", Value::from(self.modal_open()))
            .with_data(
                "interactive",
                Value::from(self.session.interactive.is_some()),
            )
            .with_data(
                "divider_drag_active",
                Value::from(self.divider_resize.is_active()),
            )
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
        self.telemetry.event_log.log(event);

        if self.modal_open() {
            if self.project_dialog().is_some()
                && matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left))
            {
                if self
                    .project_dialog()
                    .and_then(|dialog| dialog.add_dialog.as_ref())
                    .is_some()
                {
                    if let Some(index) =
                        self.project_add_dialog_result_row_at_pointer(mouse_event.x, mouse_event.y)
                    {
                        if let Some(project_dialog) = self.project_dialog_mut()
                            && let Some(add_dialog) = project_dialog.add_dialog.as_mut()
                        {
                            add_dialog.set_selected_path_match_index(index);
                        }
                        self.accept_selected_project_add_path_match();
                        return;
                    }
                    if let Some(layout) = self.project_add_dialog_layout() {
                        if Self::rect_contains_point(
                            layout.path_input,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut()
                                && let Some(add_dialog) = project_dialog.add_dialog.as_mut()
                            {
                                add_dialog.focused_field = ProjectAddDialogField::Path;
                                add_dialog.sync_focus();
                            }
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.name_input,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut()
                                && let Some(add_dialog) = project_dialog.add_dialog.as_mut()
                            {
                                add_dialog.focused_field = ProjectAddDialogField::Name;
                                add_dialog.sync_focus();
                            }
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.add_button,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            self.add_project_from_dialog();
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.cancel_button,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut() {
                                project_dialog.add_dialog = None;
                            }
                            return;
                        }
                    }
                    return;
                }

                if self
                    .project_dialog()
                    .and_then(|dialog| dialog.defaults_dialog.as_ref())
                    .is_some()
                {
                    if let Some(layout) = self.project_defaults_dialog_layout() {
                        if Self::rect_contains_point(
                            layout.base_branch_input,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut()
                                && let Some(defaults_dialog) =
                                    project_dialog.defaults_dialog.as_mut()
                            {
                                defaults_dialog.focused_field =
                                    ProjectDefaultsDialogField::BaseBranch;
                                defaults_dialog.sync_focus();
                            }
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.init_command_input,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut()
                                && let Some(defaults_dialog) =
                                    project_dialog.defaults_dialog.as_mut()
                            {
                                defaults_dialog.focused_field =
                                    ProjectDefaultsDialogField::WorkspaceInitCommand;
                                defaults_dialog.sync_focus();
                            }
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.claude_env_input,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut()
                                && let Some(defaults_dialog) =
                                    project_dialog.defaults_dialog.as_mut()
                            {
                                defaults_dialog.focused_field =
                                    ProjectDefaultsDialogField::ClaudeEnv;
                                defaults_dialog.sync_focus();
                            }
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.codex_env_input,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut()
                                && let Some(defaults_dialog) =
                                    project_dialog.defaults_dialog.as_mut()
                            {
                                defaults_dialog.focused_field =
                                    ProjectDefaultsDialogField::CodexEnv;
                                defaults_dialog.sync_focus();
                            }
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.opencode_env_input,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut()
                                && let Some(defaults_dialog) =
                                    project_dialog.defaults_dialog.as_mut()
                            {
                                defaults_dialog.focused_field =
                                    ProjectDefaultsDialogField::OpenCodeEnv;
                                defaults_dialog.sync_focus();
                            }
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.save_button,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            self.save_project_defaults_from_dialog();
                            return;
                        }
                        if Self::rect_contains_point(
                            layout.cancel_button,
                            mouse_event.x,
                            mouse_event.y,
                        ) {
                            if let Some(project_dialog) = self.project_dialog_mut() {
                                project_dialog.defaults_dialog = None;
                            }
                            return;
                        }
                    }
                    return;
                }

                if let Some(index) =
                    self.project_dialog_list_row_at_pointer(mouse_event.x, mouse_event.y)
                    && let Some(dialog) = self.project_dialog_mut()
                {
                    dialog.set_selected_filtered_index(index);
                    return;
                }
            }
            if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left))
                && let Some(next_tab) = self
                    .create_dialog_tab_from_hit_grid(mouse_event.x, mouse_event.y)
                    .or_else(|| self.create_dialog_tab_at_pointer(mouse_event.x, mouse_event.y))
                && let Some(dialog) = self.create_dialog_mut()
                && !dialog.is_add_worktree_mode()
                && dialog.tab != next_tab
            {
                dialog.tab = next_tab;
                dialog.focused_field = dialog.first_field();
            }
            return;
        }

        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => match region {
                HitRegion::Divider => {
                    let (_, divider_rect, _) = self.effective_workspace_rects();
                    self.divider_resize_anchor_x = i32::from(divider_rect.x);
                    if let Some(target) = self.divider_resize_target() {
                        self.apply_divider_resize_event(
                            ftui::layout::pane::PaneSemanticInputEventKind::PointerDown {
                                target,
                                pointer_id: Self::DIVIDER_POINTER_ID,
                                button: ftui::layout::pane::PanePointerButton::Primary,
                                position: Self::pane_pointer_position(mouse_event.x, mouse_event.y),
                            },
                        );
                    }
                }
                HitRegion::WorkspaceList => {
                    if self.session.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    let _ = self.focus_manager.focus(FOCUS_ID_WORKSPACE_LIST);
                    if let Some(row_data) = row_data {
                        if let Ok(index) = usize::try_from(row_data) {
                            self.select_workspace_by_index(index);
                        }
                    } else {
                        self.select_workspace_by_mouse(mouse_event.x, mouse_event.y);
                    }
                }
                HitRegion::WorkspacePullRequest => {
                    if self.session.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    let _ = self.focus_manager.focus(FOCUS_ID_WORKSPACE_LIST);
                    self.open_workspace_pull_request_link(row_data);
                }
                HitRegion::Preview => {
                    if let Some(tab_id) =
                        self.preview_tab_id_at_pointer(mouse_event.x, mouse_event.y)
                    {
                        if self.session.interactive.is_some() {
                            self.exit_interactive_to_list();
                        }
                        reduce(&mut self.state, Action::EnterPreviewMode);
                        let _ = self.focus_manager.focus(FOCUS_ID_PREVIEW);
                        self.acknowledge_selected_workspace_attention_for_preview_focus();
                        let _ = self.select_tab_by_id_for_selected_workspace(tab_id);
                        self.clear_preview_selection();
                    } else {
                        let interactive_before_click = self.session.interactive.is_some();
                        self.enter_preview_or_interactive();
                        if self.session.interactive.is_some() {
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
                if let (Some(target), Some(previous)) = (
                    self.divider_resize_target(),
                    self.divider_resize_current_position(),
                ) {
                    let position = Self::pane_pointer_position(mouse_event.x, mouse_event.y);
                    self.apply_divider_resize_event(
                        ftui::layout::pane::PaneSemanticInputEventKind::PointerMove {
                            target,
                            pointer_id: Self::DIVIDER_POINTER_ID,
                            position,
                            delta_x: position.x.saturating_sub(previous.x),
                            delta_y: position.y.saturating_sub(previous.y),
                        },
                    );
                } else if self.session.interactive.is_some() {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Moved => {
                if self.session.interactive.is_some() && !self.divider_resize.is_active() {
                    self.update_preview_selection_drag(mouse_event.x, mouse_event.y);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(target) = self.divider_resize_target() {
                    self.apply_divider_resize_event(
                        ftui::layout::pane::PaneSemanticInputEventKind::PointerUp {
                            target,
                            pointer_id: Self::DIVIDER_POINTER_ID,
                            button: ftui::layout::pane::PanePointerButton::Primary,
                            position: Self::pane_pointer_position(mouse_event.x, mouse_event.y),
                        },
                    );
                }
                self.finish_preview_selection_drag(mouse_event.x, mouse_event.y);
            }
            MouseEventKind::ScrollUp => {
                if matches!(region, HitRegion::WorkspaceList) {
                    if self.session.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    let _ = self.focus_manager.focus(FOCUS_ID_WORKSPACE_LIST);
                    if self.should_handle_sidebar_mouse_scroll(-1, Instant::now()) {
                        for _ in 0..Self::SIDEBAR_MOUSE_SCROLL_WORKSPACES {
                            self.move_selection(Action::MoveSelectionUp);
                        }
                    }
                } else if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    let _ = self.focus_manager.focus(FOCUS_ID_PREVIEW);
                    self.acknowledge_selected_workspace_attention_for_preview_focus();
                    if self.preview_scroll_tab_is_focused() {
                        self.scroll_preview(-Self::PREVIEW_MOUSE_SCROLL_LINES);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(region, HitRegion::WorkspaceList) {
                    if self.session.interactive.is_some() {
                        self.exit_interactive_to_list();
                    } else {
                        reduce(&mut self.state, Action::EnterListMode);
                    }
                    let _ = self.focus_manager.focus(FOCUS_ID_WORKSPACE_LIST);
                    if self.should_handle_sidebar_mouse_scroll(1, Instant::now()) {
                        for _ in 0..Self::SIDEBAR_MOUSE_SCROLL_WORKSPACES {
                            self.move_selection(Action::MoveSelectionDown);
                        }
                    }
                } else if matches!(region, HitRegion::Preview) {
                    self.state.mode = UiMode::Preview;
                    self.state.focus = PaneFocus::Preview;
                    let _ = self.focus_manager.focus(FOCUS_ID_PREVIEW);
                    self.acknowledge_selected_workspace_attention_for_preview_focus();
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

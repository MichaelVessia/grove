use super::*;

impl GroveApp {
    pub(super) fn view_layout_for_size(
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
        Self::view_layout_for_size(width, height, self.sidebar_width_pct, self.sidebar_hidden)
    }

    pub(super) fn divider_hit_area(divider: Rect, viewport_width: u16) -> Rect {
        if divider.is_empty() {
            return divider;
        }
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
                HIT_ID_WORKSPACE_PR_LINK => HitRegion::WorkspacePullRequest,
                HIT_ID_CREATE_DIALOG
                | HIT_ID_LAUNCH_DIALOG
                | HIT_ID_DELETE_DIALOG
                | HIT_ID_STOP_DIALOG
                | HIT_ID_CONFIRM_DIALOG
                | HIT_ID_KEYBIND_HELP_DIALOG => HitRegion::Outside,
                _ => HitRegion::Outside,
            };
            let row_data = if id.id() == HIT_ID_WORKSPACE_ROW || id.id() == HIT_ID_WORKSPACE_PR_LINK
            {
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
    pub(super) fn apply_interactive_cursor_overlay(
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

    pub(super) fn apply_interactive_cursor_overlay_render(
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

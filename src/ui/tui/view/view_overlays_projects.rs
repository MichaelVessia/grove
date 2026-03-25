use super::view_prelude::*;
use crate::ui::tui::dialogs_projects_search::project_path_search_input;

const PROJECT_DIALOG_MIN_WIDTH: u16 = 44;
const PROJECT_DIALOG_WIDTH: u16 = 96;
const PROJECT_LIST_DIALOG_HEIGHT: u16 = 20;
const PROJECT_ADD_DIALOG_HEIGHT: u16 = 15;
const PROJECT_DEFAULTS_DIALOG_HEIGHT: u16 = 20;
const MODAL_BUTTON_WIDTH: u16 = 12;
const MODAL_BUTTON_GAP: u16 = 2;

#[derive(Debug, Clone, Copy)]
pub(super) struct ProjectAddDialogLayout {
    pub(super) dialog_area: Rect,
    path_label: Rect,
    pub(super) path_input: Rect,
    results_label: Rect,
    pub(super) results: Rect,
    name_label: Rect,
    pub(super) name_input: Rect,
    actions: Rect,
    pub(super) add_button: Rect,
    pub(super) cancel_button: Rect,
    hints: Rect,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProjectDefaultsDialogLayout {
    pub(super) dialog_area: Rect,
    project_label: Rect,
    project_path: Rect,
    base_branch_label: Rect,
    pub(super) base_branch_input: Rect,
    init_command_label: Rect,
    pub(super) init_command_input: Rect,
    claude_env_label: Rect,
    pub(super) claude_env_input: Rect,
    codex_env_label: Rect,
    pub(super) codex_env_input: Rect,
    opencode_env_label: Rect,
    pub(super) opencode_env_input: Rect,
    note: Rect,
    actions: Rect,
    pub(super) save_button: Rect,
    pub(super) cancel_button: Rect,
    hints: Rect,
}

#[derive(Clone)]
struct ProjectListModalContent<'a> {
    dialog: &'a ProjectDialogState,
    projects: &'a [ProjectConfig],
    theme: ftui::ResolvedTheme,
}

#[derive(Clone)]
struct ProjectAddModalContent<'a> {
    dialog: &'a ProjectAddDialogState,
    theme: ftui::ResolvedTheme,
}

#[derive(Clone)]
struct ProjectDefaultsModalContent<'a> {
    dialog: &'a ProjectDefaultsDialogState,
    project_label: &'a str,
    project_path: &'a str,
    theme: ftui::ResolvedTheme,
}

fn modal_button_rects(actions: Rect) -> (Rect, Rect) {
    let total_width = MODAL_BUTTON_WIDTH
        .saturating_mul(2)
        .saturating_add(MODAL_BUTTON_GAP);
    let buttons_x = actions
        .x
        .saturating_add(actions.width.saturating_sub(total_width) / 2);
    let add_button = Rect::new(
        buttons_x,
        actions.y,
        MODAL_BUTTON_WIDTH,
        actions.height.max(1),
    );
    let cancel_button = Rect::new(
        add_button.right().saturating_add(MODAL_BUTTON_GAP),
        actions.y,
        MODAL_BUTTON_WIDTH,
        actions.height.max(1),
    );
    (add_button, cancel_button)
}

fn project_add_dialog_layout(area: Rect) -> Option<ProjectAddDialogLayout> {
    if area.width < PROJECT_DIALOG_MIN_WIDTH || area.height < PROJECT_ADD_DIALOG_HEIGHT {
        return None;
    }

    let dialog_width = area.width.saturating_sub(8).min(PROJECT_DIALOG_WIDTH);
    let dialog_area = GroveApp::centered_modal_rect(area, dialog_width, PROJECT_ADD_DIALOG_HEIGHT);
    let inner = Block::new().borders(Borders::ALL).inner(dialog_area);
    if inner.is_empty() {
        return None;
    }

    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Min(3),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(3),
        ])
        .split(inner);
    let (add_button, cancel_button) = modal_button_rects(rows[7]);

    Some(ProjectAddDialogLayout {
        dialog_area,
        path_label: rows[0],
        path_input: rows[1],
        results_label: rows[2],
        results: rows[3],
        name_label: rows[4],
        name_input: rows[5],
        actions: rows[7],
        add_button,
        cancel_button,
        hints: rows[8],
    })
}

fn project_defaults_dialog_layout(area: Rect) -> Option<ProjectDefaultsDialogLayout> {
    if area.width < PROJECT_DIALOG_MIN_WIDTH || area.height < PROJECT_DEFAULTS_DIALOG_HEIGHT {
        return None;
    }

    let dialog_width = area.width.saturating_sub(8).min(PROJECT_DIALOG_WIDTH);
    let dialog_area =
        GroveApp::centered_modal_rect(area, dialog_width, PROJECT_DEFAULTS_DIALOG_HEIGHT);
    let inner = Block::new().borders(Borders::ALL).inner(dialog_area);
    if inner.is_empty() {
        return None;
    }

    let rows = Flex::vertical()
        .constraints([
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(1),
            Constraint::Fixed(2),
        ])
        .split(inner);
    let (save_button, cancel_button) = modal_button_rects(rows[13]);

    Some(ProjectDefaultsDialogLayout {
        dialog_area,
        project_label: rows[0],
        project_path: rows[1],
        base_branch_label: rows[2],
        base_branch_input: rows[3],
        init_command_label: rows[4],
        init_command_input: rows[5],
        claude_env_label: rows[6],
        claude_env_input: rows[7],
        codex_env_label: rows[8],
        codex_env_input: rows[9],
        opencode_env_label: rows[10],
        opencode_env_input: rows[11],
        note: rows[12],
        actions: rows[13],
        save_button,
        cancel_button,
        hints: rows[14],
    })
}

fn modal_input_style(theme: ftui::ResolvedTheme) -> Style {
    Style::new()
        .fg(packed(theme.text))
        .bg(packed(theme.surface))
}

fn modal_input_selection_style(theme: ftui::ResolvedTheme) -> Style {
    Style::new()
        .fg(packed(theme.text))
        .bg(packed(theme.selection_bg))
}

fn modal_input_placeholder_style(theme: ftui::ResolvedTheme) -> Style {
    Style::new().fg(packed(theme.border))
}

fn modal_input_cursor_style(theme: ftui::ResolvedTheme) -> Style {
    Style::new()
        .fg(packed(theme.background))
        .bg(packed(theme.secondary))
}

fn modal_content_style(theme: ftui::ResolvedTheme) -> Style {
    Style::new()
        .bg(packed(theme.background))
        .fg(packed(theme.text))
}

fn render_modal_button(
    frame: &mut Frame,
    area: Rect,
    theme: ftui::ResolvedTheme,
    label: &str,
    focused: bool,
    accent: PackedRgba,
) {
    let style = if focused {
        Style::new().fg(packed(theme.background)).bg(accent).bold()
    } else {
        Style::new()
            .fg(packed(theme.text))
            .bg(packed(theme.surface))
    };
    Paragraph::new(format!(
        "{:^width$}",
        label,
        width = usize::from(area.width)
    ))
    .style(style)
    .render(area, frame);
}

impl Widget for ProjectListModalContent<'_> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        if area.is_empty() {
            return;
        }

        let content_style = modal_content_style(self.theme);
        Paragraph::new("").style(content_style).render(area, frame);

        let block = Block::new()
            .title("Projects")
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(packed(self.theme.info)).bold());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Fixed(1),
                Constraint::Min(1),
                Constraint::Fixed(3),
            ])
            .split(inner);

        let filter_input = self
            .dialog
            .filter_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("Type project name or path")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));
        Widget::render(&filter_input, rows[0], frame);
        if filter_input.focused() {
            frame.set_cursor(Some(filter_input.cursor_position(rows[0])));
            frame.set_cursor_visible(true);
        }

        Paragraph::new(format!("{} projects", self.projects.len()))
            .style(Style::new().fg(packed(self.theme.border)))
            .render(rows[1], frame);

        if self.dialog.filtered_project_indices.is_empty() {
            Paragraph::new("No matches")
                .style(Style::new().fg(packed(self.theme.text_subtle)))
                .render(rows[2], frame);
        } else {
            let items = self
                .dialog
                .filtered_project_indices
                .iter()
                .filter_map(|project_index| {
                    self.projects
                        .get(*project_index)
                        .map(|project| (project_index, project))
                })
                .map(|(project_index, project)| {
                    let label = format!(
                        "{:>2}. {}  {}",
                        project_index.saturating_add(1),
                        project.name,
                        project.path.display()
                    );
                    ListItem::new(label).style(Style::new().fg(packed(self.theme.text_muted)))
                })
                .collect::<Vec<_>>();
            let list = List::new(items)
                .highlight_symbol("> ")
                .highlight_style(
                    Style::new()
                        .fg(packed(self.theme.text))
                        .bg(packed(self.theme.selection_bg))
                        .bold(),
                )
                .hit_id(HitId::new(HIT_ID_PROJECT_DIALOG_LIST));
            let mut list_state = self.dialog.project_list.clone();
            StatefulWidget::render(&list, rows[2], frame, &mut list_state);
        }

        Paragraph::new("Enter open, Up/Down or Tab/S-Tab/C-n/C-p navigate, Ctrl+A add, Ctrl+E defaults, Ctrl+X/Del remove, Esc close")
            .wrap(ftui::text::WrapMode::Word)
            .style(Style::new().fg(packed(self.theme.border)))
            .render(rows[3], frame);
    }
}

impl Widget for ProjectAddModalContent<'_> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        let Some(layout) = project_add_dialog_layout(area) else {
            return;
        };

        let content_style = modal_content_style(self.theme);
        Paragraph::new("")
            .style(content_style)
            .render(layout.dialog_area, frame);

        let block = Block::new()
            .title("Add Project")
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(packed(self.theme.secondary)).bold());
        block.render(layout.dialog_area, frame);

        Paragraph::new("Path")
            .style(Style::new().fg(packed(self.theme.text_subtle)))
            .render(layout.path_label, frame);
        let path_input = self
            .dialog
            .path_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("Paste repo path or type 2+ chars")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));
        Widget::render(&path_input, layout.path_input, frame);
        if path_input.focused() {
            frame.set_cursor(Some(path_input.cursor_position(layout.path_input)));
            frame.set_cursor_visible(true);
        }

        let query = self.dialog.path_input.value().trim();
        let search_input = project_path_search_input(query);
        let already_added_count = self
            .dialog
            .path_matches
            .iter()
            .filter(|path_match| path_match.already_added)
            .count();
        let results_label = match search_input.as_ref() {
            Some(search_input) if !query.is_empty() => format!(
                "{} repo matches under {}{}",
                self.dialog.path_matches.len(),
                search_input.search_root.display(),
                if already_added_count == 0 {
                    String::new()
                } else {
                    format!(", {} already added", already_added_count)
                }
            ),
            _ => "Repo matches".to_string(),
        };
        Paragraph::new(results_label)
            .style(Style::new().fg(packed(self.theme.text_subtle)))
            .render(layout.results_label, frame);

        if self.dialog.path_matches.is_empty() {
            let empty_label = if query.is_empty() {
                "Paste a repo path or type 2+ chars to search".to_string()
            } else if search_input
                .as_ref()
                .is_some_and(|search| !search.path_like)
                && query.chars().count() < 2
            {
                "Type at least 2 chars to search your repo roots".to_string()
            } else if let Some(search_input) = search_input {
                format!(
                    "No repo matches under {}",
                    search_input.search_root.display()
                )
            } else {
                "No repo matches".to_string()
            };
            Paragraph::new(empty_label)
                .wrap(ftui::text::WrapMode::Word)
                .style(Style::new().fg(packed(self.theme.text_subtle)))
                .render(layout.results, frame);
        } else {
            let max_row_width = usize::from(layout.results.width.saturating_sub(4));
            let items = self
                .dialog
                .path_matches
                .iter()
                .map(|path_match| {
                    let project_name = project_display_name(&path_match.path);
                    let badge_label = "Already added";
                    let separator = "  ";
                    let badge_reserved = if path_match.already_added {
                        ftui::text::display_width(separator)
                            .saturating_add(ftui::text::display_width(badge_label))
                    } else {
                        0
                    };
                    let reserved_width = ftui::text::display_width(project_name.as_str())
                        .saturating_add(ftui::text::display_width(separator))
                        .saturating_add(badge_reserved);
                    let path_width = max_row_width.saturating_sub(reserved_width).max(8);
                    let rendered_path = ftui::text::truncate_with_ellipsis(
                        path_match.path.display().to_string().as_str(),
                        path_width,
                        "…",
                    );
                    let mut spans = vec![
                        FtSpan::styled(
                            project_name,
                            Style::new().fg(if path_match.already_added {
                                packed(self.theme.text_subtle)
                            } else {
                                packed(self.theme.text)
                            }),
                        ),
                        FtSpan::styled("  ", Style::new().fg(packed(self.theme.text_subtle))),
                        FtSpan::styled(
                            rendered_path,
                            Style::new().fg(if path_match.already_added {
                                packed(self.theme.border)
                            } else {
                                packed(self.theme.text_muted)
                            }),
                        ),
                    ];
                    if path_match.already_added {
                        spans.push(FtSpan::styled(
                            "  ",
                            Style::new().fg(packed(self.theme.border)),
                        ));
                        spans.push(FtSpan::styled(
                            badge_label,
                            Style::new()
                                .fg(packed(self.theme.background))
                                .bg(packed(self.theme.overlay))
                                .bold(),
                        ));
                    }
                    ListItem::new(FtText::from_lines(vec![FtLine::from_spans(spans)]))
                })
                .collect::<Vec<_>>();
            let list = List::new(items)
                .highlight_symbol("> ")
                .highlight_style(
                    Style::new()
                        .fg(packed(self.theme.text))
                        .bg(packed(self.theme.selection_bg))
                        .bold(),
                )
                .hit_id(HitId::new(HIT_ID_PROJECT_ADD_RESULTS_LIST));
            let mut list_state = self.dialog.path_match_list.clone();
            StatefulWidget::render(&list, layout.results, frame, &mut list_state);
        }

        Paragraph::new("Name")
            .style(Style::new().fg(packed(self.theme.text_subtle)))
            .render(layout.name_label, frame);
        let name_input = self
            .dialog
            .name_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("Optional, defaults to repo directory name")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));
        Widget::render(&name_input, layout.name_input, frame);
        if name_input.focused() {
            frame.set_cursor(Some(name_input.cursor_position(layout.name_input)));
            frame.set_cursor_visible(true);
        }

        Paragraph::new("")
            .style(Style::new().bg(packed(self.theme.background)))
            .render(layout.actions, frame);
        render_modal_button(
            frame,
            layout.add_button,
            self.theme,
            "Add",
            self.dialog.focused_field == ProjectAddDialogField::AddButton,
            packed(self.theme.success),
        );
        render_modal_button(
            frame,
            layout.cancel_button,
            self.theme,
            "Cancel",
            self.dialog.focused_field == ProjectAddDialogField::CancelButton,
            packed(self.theme.selection_bg),
        );

        Paragraph::new(
            "Enter accept match or confirm, Up/Down browse matches, already added repos are informational, Tab/S-Tab move, Esc back",
        )
        .wrap(ftui::text::WrapMode::Word)
        .style(Style::new().fg(packed(self.theme.border)))
        .render(layout.hints, frame);
    }
}

impl Widget for ProjectDefaultsModalContent<'_> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        let Some(layout) = project_defaults_dialog_layout(area) else {
            return;
        };

        let content_style = modal_content_style(self.theme);
        Paragraph::new("")
            .style(content_style)
            .render(layout.dialog_area, frame);

        let block = Block::new()
            .title("Project Defaults")
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(packed(self.theme.accent)).bold());
        block.render(layout.dialog_area, frame);

        Paragraph::new(format!("Project  {}", self.project_label))
            .style(Style::new().fg(packed(self.theme.text)))
            .render(layout.project_label, frame);
        Paragraph::new(format!("Path  {}", self.project_path))
            .style(Style::new().fg(packed(self.theme.border)))
            .render(layout.project_path, frame);

        let label_style = Style::new().fg(packed(self.theme.text_subtle));
        Paragraph::new("Base branch")
            .style(label_style)
            .render(layout.base_branch_label, frame);
        Paragraph::new("Init command")
            .style(label_style)
            .render(layout.init_command_label, frame);
        Paragraph::new("Claude env")
            .style(label_style)
            .render(layout.claude_env_label, frame);
        Paragraph::new("Codex env")
            .style(label_style)
            .render(layout.codex_env_label, frame);
        Paragraph::new("OpenCode env")
            .style(label_style)
            .render(layout.opencode_env_label, frame);

        let base_branch_input = self
            .dialog
            .base_branch_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("Optional override, empty uses selected branch")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));
        let init_command_input = self
            .dialog
            .workspace_init_command_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("Runs once per workspace start")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));
        let claude_env_input = self
            .dialog
            .claude_env_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("KEY=VALUE; KEY2=VALUE")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));
        let codex_env_input = self
            .dialog
            .codex_env_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("KEY=VALUE; KEY2=VALUE")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));
        let opencode_env_input = self
            .dialog
            .opencode_env_input
            .clone()
            .with_style(modal_input_style(self.theme))
            .with_placeholder("KEY=VALUE; KEY2=VALUE")
            .with_placeholder_style(modal_input_placeholder_style(self.theme))
            .with_cursor_style(modal_input_cursor_style(self.theme))
            .with_selection_style(modal_input_selection_style(self.theme));

        Widget::render(&base_branch_input, layout.base_branch_input, frame);
        Widget::render(&init_command_input, layout.init_command_input, frame);
        Widget::render(&claude_env_input, layout.claude_env_input, frame);
        Widget::render(&codex_env_input, layout.codex_env_input, frame);
        Widget::render(&opencode_env_input, layout.opencode_env_input, frame);

        match self.dialog.focused_field {
            ProjectDefaultsDialogField::BaseBranch => {
                frame.set_cursor(Some(
                    base_branch_input.cursor_position(layout.base_branch_input),
                ));
                frame.set_cursor_visible(true);
            }
            ProjectDefaultsDialogField::WorkspaceInitCommand => {
                frame.set_cursor(Some(
                    init_command_input.cursor_position(layout.init_command_input),
                ));
                frame.set_cursor_visible(true);
            }
            ProjectDefaultsDialogField::ClaudeEnv => {
                frame.set_cursor(Some(
                    claude_env_input.cursor_position(layout.claude_env_input),
                ));
                frame.set_cursor_visible(true);
            }
            ProjectDefaultsDialogField::CodexEnv => {
                frame.set_cursor(Some(
                    codex_env_input.cursor_position(layout.codex_env_input),
                ));
                frame.set_cursor_visible(true);
            }
            ProjectDefaultsDialogField::OpenCodeEnv => {
                frame.set_cursor(Some(
                    opencode_env_input.cursor_position(layout.opencode_env_input),
                ));
                frame.set_cursor_visible(true);
            }
            ProjectDefaultsDialogField::SaveButton | ProjectDefaultsDialogField::CancelButton => {}
        }

        Paragraph::new("Env changes apply on next agent start or restart")
            .style(Style::new().fg(packed(self.theme.border)))
            .render(layout.note, frame);
        Paragraph::new("")
            .style(Style::new().bg(packed(self.theme.background)))
            .render(layout.actions, frame);
        render_modal_button(
            frame,
            layout.save_button,
            self.theme,
            "Save",
            self.dialog.focused_field == ProjectDefaultsDialogField::SaveButton,
            packed(self.theme.success),
        );
        render_modal_button(
            frame,
            layout.cancel_button,
            self.theme,
            "Cancel",
            self.dialog.focused_field == ProjectDefaultsDialogField::CancelButton,
            packed(self.theme.selection_bg),
        );
        Paragraph::new("Tab/S-Tab or C-n/C-p move, Enter confirm, Esc back")
            .style(Style::new().fg(packed(self.theme.border)))
            .render(layout.hints, frame);
    }
}

impl GroveApp {
    pub(super) fn project_add_dialog_layout(&self) -> Option<ProjectAddDialogLayout> {
        project_add_dialog_layout(Rect::from_size(
            self.viewport_width.max(1),
            self.viewport_height.max(1),
        ))
    }

    pub(super) fn project_defaults_dialog_layout(&self) -> Option<ProjectDefaultsDialogLayout> {
        project_defaults_dialog_layout(Rect::from_size(
            self.viewport_width.max(1),
            self.viewport_height.max(1),
        ))
    }

    pub(super) fn render_project_dialog_overlay(&self, frame: &mut Frame, area: Rect) {
        let Some(dialog) = self.project_dialog() else {
            return;
        };
        if area.width < PROJECT_DIALOG_MIN_WIDTH || area.height < 14 {
            return;
        }

        let theme = self.active_ui_theme();
        let dialog_width = area.width.saturating_sub(8).min(PROJECT_DIALOG_WIDTH);

        if let Some(add_dialog) = dialog.add_dialog.as_ref() {
            let content = ProjectAddModalContent {
                dialog: add_dialog,
                theme,
            };
            Modal::new(content)
                .size(
                    ModalSizeConstraints::new()
                        .min_width(dialog_width)
                        .max_width(dialog_width)
                        .min_height(PROJECT_ADD_DIALOG_HEIGHT)
                        .max_height(PROJECT_ADD_DIALOG_HEIGHT),
                )
                .backdrop(BackdropConfig::new(packed(theme.background), 0.55))
                .hit_id(HitId::new(HIT_ID_PROJECT_ADD_DIALOG))
                .render(area, frame);
            return;
        }

        if let Some(defaults_dialog) = dialog.defaults_dialog.as_ref() {
            let project_label = self
                .projects
                .get(defaults_dialog.project_index)
                .map(|project| project.name.clone())
                .unwrap_or_else(|| "(missing project)".to_string());
            let project_path = self
                .projects
                .get(defaults_dialog.project_index)
                .map(|project| project.path.display().to_string())
                .unwrap_or_else(|| "(missing path)".to_string());
            let content = ProjectDefaultsModalContent {
                dialog: defaults_dialog,
                project_label: project_label.as_str(),
                project_path: project_path.as_str(),
                theme,
            };

            Modal::new(content)
                .size(
                    ModalSizeConstraints::new()
                        .min_width(dialog_width)
                        .max_width(dialog_width)
                        .min_height(PROJECT_DEFAULTS_DIALOG_HEIGHT)
                        .max_height(PROJECT_DEFAULTS_DIALOG_HEIGHT),
                )
                .backdrop(BackdropConfig::new(packed(theme.background), 0.55))
                .hit_id(HitId::new(HIT_ID_PROJECT_DEFAULTS_DIALOG))
                .render(area, frame);
            return;
        }

        let dialog_height = area.height.min(PROJECT_LIST_DIALOG_HEIGHT);
        let content = ProjectListModalContent {
            dialog,
            projects: self.projects.as_slice(),
            theme,
        };

        Modal::new(content)
            .size(
                ModalSizeConstraints::new()
                    .min_width(dialog_width)
                    .max_width(dialog_width)
                    .min_height(dialog_height)
                    .max_height(dialog_height),
            )
            .backdrop(BackdropConfig::new(packed(theme.background), 0.55))
            .hit_id(HitId::new(HIT_ID_PROJECT_DIALOG))
            .render(area, frame);
    }
}

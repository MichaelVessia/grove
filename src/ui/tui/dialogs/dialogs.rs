use super::*;
use crate::domain::PermissionMode;

pub(super) fn modal_labeled_input_row(
    content_width: usize,
    theme: ftui::ResolvedTheme,
    label: &str,
    value: &str,
    placeholder: &str,
    focused: bool,
) -> FtLine<'static> {
    let row_bg = if focused {
        packed(theme.selection_bg)
    } else {
        packed(theme.background)
    };
    let marker = if focused { ">" } else { " " };
    let badge = format!("[{label}] ");
    let prefix = format!("{marker} {badge}");
    let prefix_width = text_display_width(prefix.as_str());
    let value_raw = if value.is_empty() { placeholder } else { value };
    let rendered = ftui::text::truncate_with_ellipsis(
        value_raw,
        content_width.saturating_sub(prefix_width),
        "…",
    );
    let used = prefix_width.saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled(
            marker,
            Style::new()
                .fg(if focused {
                    packed(theme.warning)
                } else {
                    packed(theme.border)
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(" ", Style::new().bg(row_bg)),
        FtSpan::styled(
            badge,
            Style::new().fg(packed(theme.primary)).bg(row_bg).bold(),
        ),
        FtSpan::styled(
            rendered,
            Style::new()
                .fg(if value.is_empty() {
                    packed(theme.border)
                } else {
                    packed(theme.text)
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(pad, Style::new().bg(row_bg)),
    ])
}

pub(super) fn modal_static_badged_row(
    content_width: usize,
    theme: ftui::ResolvedTheme,
    label: &str,
    value: &str,
    badge_fg: PackedRgba,
    value_fg: PackedRgba,
) -> FtLine<'static> {
    let badge = format!("[{label}] ");
    let prefix = format!("  {badge}");
    let available = content_width.saturating_sub(text_display_width(prefix.as_str()));
    let rendered = ftui::text::truncate_with_ellipsis(value, available, "…");
    let used =
        text_display_width(prefix.as_str()).saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled("  ", Style::new().bg(packed(theme.background))),
        FtSpan::styled(
            badge,
            Style::new()
                .fg(badge_fg)
                .bg(packed(theme.background))
                .bold(),
        ),
        FtSpan::styled(
            rendered,
            Style::new().fg(value_fg).bg(packed(theme.background)),
        ),
        FtSpan::styled(pad, Style::new().bg(packed(theme.background))),
    ])
}

pub(super) fn modal_focus_badged_row(
    content_width: usize,
    theme: ftui::ResolvedTheme,
    label: &str,
    value: &str,
    focused: bool,
    badge_fg: PackedRgba,
    value_fg: PackedRgba,
) -> FtLine<'static> {
    let row_bg = if focused {
        packed(theme.selection_bg)
    } else {
        packed(theme.background)
    };
    let marker = if focused { ">" } else { " " };
    let badge = format!("[{label}] ");
    let prefix = format!("{marker} {badge}");
    let prefix_width = text_display_width(prefix.as_str());
    let rendered =
        ftui::text::truncate_with_ellipsis(value, content_width.saturating_sub(prefix_width), "…");
    let used = prefix_width.saturating_add(text_display_width(rendered.as_str()));
    let pad = " ".repeat(content_width.saturating_sub(used));

    FtLine::from_spans(vec![
        FtSpan::styled(
            marker,
            Style::new()
                .fg(if focused {
                    packed(theme.warning)
                } else {
                    packed(theme.border)
                })
                .bg(row_bg)
                .bold(),
        ),
        FtSpan::styled(" ", Style::new().bg(row_bg)),
        FtSpan::styled(badge, Style::new().fg(badge_fg).bg(row_bg).bold()),
        FtSpan::styled(rendered, Style::new().fg(value_fg).bg(row_bg).bold()),
        FtSpan::styled(pad, Style::new().bg(row_bg)),
    ])
}

pub(super) fn modal_actions_row(
    content_width: usize,
    theme: ftui::ResolvedTheme,
    primary_label: &str,
    secondary_label: &str,
    primary_focused: bool,
    secondary_focused: bool,
) -> FtLine<'static> {
    let actions_bg = if primary_focused || secondary_focused {
        packed(theme.selection_bg)
    } else {
        packed(theme.background)
    };
    let actions_prefix = if primary_focused || secondary_focused {
        "> "
    } else {
        "  "
    };
    let primary = if primary_focused {
        format!("[{primary_label}]")
    } else {
        format!(" {primary_label} ")
    };
    let secondary = if secondary_focused {
        format!("[{secondary_label}]")
    } else {
        format!(" {secondary_label} ")
    };
    let row = ftui::text::truncate_to_width(
        format!("{actions_prefix}{primary}   {secondary}").as_str(),
        content_width,
    );
    let used = text_display_width(row.as_str());
    let padded = format!("{row}{}", " ".repeat(content_width.saturating_sub(used)));

    FtLine::from_spans(vec![FtSpan::styled(
        padded,
        Style::new().fg(packed(theme.text)).bg(actions_bg).bold(),
    )])
}

pub(super) fn unsafe_state_label(permission_mode: PermissionMode) -> &'static str {
    match permission_mode {
        PermissionMode::Default => "off, standard safety checks",
        PermissionMode::Auto => "auto, classifier-guarded",
        PermissionMode::Unsafe => "on, bypass approvals and sandbox",
    }
}

pub(super) fn modal_start_agent_config_rows<F>(
    content_width: usize,
    theme: ftui::ResolvedTheme,
    start_config: &StartAgentConfigState,
    is_focused: F,
) -> [FtLine<'static>; 4]
where
    F: Fn(StartAgentConfigField) -> bool,
{
    [
        modal_labeled_input_row(
            content_width,
            theme,
            "Name",
            start_config.name.as_str(),
            "Optional tab title (defaults to agent + number)",
            is_focused(StartAgentConfigField::Name),
        ),
        modal_labeled_input_row(
            content_width,
            theme,
            "Prompt",
            start_config.prompt.as_str(),
            "Describe initial task for the agent",
            is_focused(StartAgentConfigField::Prompt),
        ),
        modal_labeled_input_row(
            content_width,
            theme,
            "InitCmd",
            start_config.init_command.as_str(),
            "Runs once per workspace start (shared across panes)",
            is_focused(StartAgentConfigField::InitCommand),
        ),
        modal_focus_badged_row(
            content_width,
            theme,
            "Unsafe",
            unsafe_state_label(start_config.permission_mode),
            is_focused(StartAgentConfigField::Unsafe),
            packed(theme.accent),
            if start_config.permission_mode != PermissionMode::Default {
                packed(theme.error)
            } else {
                packed(theme.text)
            },
        ),
    ]
}

pub(super) fn modal_wrapped_rows(
    content_width: usize,
    text: &str,
    style: Style,
) -> Vec<FtLine<'static>> {
    ftui::text::wrap_text(text, content_width, ftui::text::WrapMode::Word)
        .into_iter()
        .map(|line| FtLine::from_spans(vec![FtSpan::styled(line, style)]))
        .collect()
}

pub(super) fn modal_wrapped_hint_rows(
    content_width: usize,
    theme: ftui::ResolvedTheme,
    text: &str,
) -> Vec<FtLine<'static>> {
    modal_wrapped_rows(
        content_width,
        text,
        Style::new()
            .fg(packed(theme.border))
            .bg(packed(theme.background)),
    )
}

fn confirm_dialog_focus_ids() -> [u64; 2] {
    [
        FOCUS_ID_CONFIRM_CONFIRM_BUTTON,
        FOCUS_ID_CONFIRM_CANCEL_BUTTON,
    ]
}

fn confirm_dialog_focus_id(field: ConfirmDialogField) -> u64 {
    match field {
        ConfirmDialogField::ConfirmButton => FOCUS_ID_CONFIRM_CONFIRM_BUTTON,
        ConfirmDialogField::CancelButton => FOCUS_ID_CONFIRM_CANCEL_BUTTON,
    }
}

fn confirm_dialog_focus_field(focus_id: Option<u64>) -> Option<ConfirmDialogField> {
    match focus_id {
        Some(FOCUS_ID_CONFIRM_CONFIRM_BUTTON) => Some(ConfirmDialogField::ConfirmButton),
        Some(FOCUS_ID_CONFIRM_CANCEL_BUTTON) => Some(ConfirmDialogField::CancelButton),
        _ => None,
    }
}

fn stop_dialog_focus_ids() -> [u64; 2] {
    [FOCUS_ID_STOP_CONFIRM_BUTTON, FOCUS_ID_STOP_CANCEL_BUTTON]
}

fn stop_dialog_focus_id(field: StopDialogField) -> u64 {
    match field {
        StopDialogField::StopButton => FOCUS_ID_STOP_CONFIRM_BUTTON,
        StopDialogField::CancelButton => FOCUS_ID_STOP_CANCEL_BUTTON,
    }
}

fn stop_dialog_focus_field(focus_id: Option<u64>) -> Option<StopDialogField> {
    match focus_id {
        Some(FOCUS_ID_STOP_CONFIRM_BUTTON) => Some(StopDialogField::StopButton),
        Some(FOCUS_ID_STOP_CANCEL_BUTTON) => Some(StopDialogField::CancelButton),
        _ => None,
    }
}

fn project_dialog_focus_ids() -> [u64; 2] {
    [
        FOCUS_ID_PROJECT_DIALOG_FILTER_INPUT,
        FOCUS_ID_PROJECT_DIALOG_LIST,
    ]
}

fn session_cleanup_dialog_focus_ids() -> [u64; 4] {
    [
        FOCUS_ID_SESSION_CLEANUP_INCLUDE_STALE,
        FOCUS_ID_SESSION_CLEANUP_INCLUDE_ATTACHED,
        FOCUS_ID_SESSION_CLEANUP_APPLY_BUTTON,
        FOCUS_ID_SESSION_CLEANUP_CANCEL_BUTTON,
    ]
}

fn session_cleanup_dialog_focus_id(field: SessionCleanupDialogField) -> u64 {
    match field {
        SessionCleanupDialogField::IncludeStale => FOCUS_ID_SESSION_CLEANUP_INCLUDE_STALE,
        SessionCleanupDialogField::IncludeAttached => FOCUS_ID_SESSION_CLEANUP_INCLUDE_ATTACHED,
        SessionCleanupDialogField::ApplyButton => FOCUS_ID_SESSION_CLEANUP_APPLY_BUTTON,
        SessionCleanupDialogField::CancelButton => FOCUS_ID_SESSION_CLEANUP_CANCEL_BUTTON,
    }
}

fn session_cleanup_dialog_focus_field(focus_id: Option<u64>) -> Option<SessionCleanupDialogField> {
    match focus_id {
        Some(FOCUS_ID_SESSION_CLEANUP_INCLUDE_STALE) => {
            Some(SessionCleanupDialogField::IncludeStale)
        }
        Some(FOCUS_ID_SESSION_CLEANUP_INCLUDE_ATTACHED) => {
            Some(SessionCleanupDialogField::IncludeAttached)
        }
        Some(FOCUS_ID_SESSION_CLEANUP_APPLY_BUTTON) => Some(SessionCleanupDialogField::ApplyButton),
        Some(FOCUS_ID_SESSION_CLEANUP_CANCEL_BUTTON) => {
            Some(SessionCleanupDialogField::CancelButton)
        }
        _ => None,
    }
}

fn delete_dialog_focus_ids() -> [u64; 4] {
    [
        FOCUS_ID_DELETE_LOCAL_BRANCH,
        FOCUS_ID_DELETE_KILL_TMUX_SESSIONS,
        FOCUS_ID_DELETE_CONFIRM_BUTTON,
        FOCUS_ID_DELETE_CANCEL_BUTTON,
    ]
}

fn delete_dialog_focus_id(field: DeleteDialogField) -> u64 {
    match field {
        DeleteDialogField::DeleteLocalBranch => FOCUS_ID_DELETE_LOCAL_BRANCH,
        DeleteDialogField::KillTmuxSessions => FOCUS_ID_DELETE_KILL_TMUX_SESSIONS,
        DeleteDialogField::DeleteButton => FOCUS_ID_DELETE_CONFIRM_BUTTON,
        DeleteDialogField::CancelButton => FOCUS_ID_DELETE_CANCEL_BUTTON,
    }
}

fn delete_dialog_focus_field(focus_id: Option<u64>) -> Option<DeleteDialogField> {
    match focus_id {
        Some(FOCUS_ID_DELETE_LOCAL_BRANCH) => Some(DeleteDialogField::DeleteLocalBranch),
        Some(FOCUS_ID_DELETE_KILL_TMUX_SESSIONS) => Some(DeleteDialogField::KillTmuxSessions),
        Some(FOCUS_ID_DELETE_CONFIRM_BUTTON) => Some(DeleteDialogField::DeleteButton),
        Some(FOCUS_ID_DELETE_CANCEL_BUTTON) => Some(DeleteDialogField::CancelButton),
        _ => None,
    }
}

fn merge_dialog_focus_ids() -> [u64; 4] {
    [
        FOCUS_ID_MERGE_CLEANUP_WORKSPACE,
        FOCUS_ID_MERGE_CLEANUP_LOCAL_BRANCH,
        FOCUS_ID_MERGE_CONFIRM_BUTTON,
        FOCUS_ID_MERGE_CANCEL_BUTTON,
    ]
}

fn merge_dialog_focus_id(field: MergeDialogField) -> u64 {
    match field {
        MergeDialogField::CleanupWorkspace => FOCUS_ID_MERGE_CLEANUP_WORKSPACE,
        MergeDialogField::CleanupLocalBranch => FOCUS_ID_MERGE_CLEANUP_LOCAL_BRANCH,
        MergeDialogField::MergeButton => FOCUS_ID_MERGE_CONFIRM_BUTTON,
        MergeDialogField::CancelButton => FOCUS_ID_MERGE_CANCEL_BUTTON,
    }
}

fn merge_dialog_focus_field(focus_id: Option<u64>) -> Option<MergeDialogField> {
    match focus_id {
        Some(FOCUS_ID_MERGE_CLEANUP_WORKSPACE) => Some(MergeDialogField::CleanupWorkspace),
        Some(FOCUS_ID_MERGE_CLEANUP_LOCAL_BRANCH) => Some(MergeDialogField::CleanupLocalBranch),
        Some(FOCUS_ID_MERGE_CONFIRM_BUTTON) => Some(MergeDialogField::MergeButton),
        Some(FOCUS_ID_MERGE_CANCEL_BUTTON) => Some(MergeDialogField::CancelButton),
        _ => None,
    }
}

fn update_from_base_dialog_focus_ids() -> [u64; 2] {
    [
        FOCUS_ID_UPDATE_FROM_BASE_CONFIRM_BUTTON,
        FOCUS_ID_UPDATE_FROM_BASE_CANCEL_BUTTON,
    ]
}

fn update_from_base_dialog_focus_id(field: UpdateFromBaseDialogField) -> u64 {
    match field {
        UpdateFromBaseDialogField::UpdateButton => FOCUS_ID_UPDATE_FROM_BASE_CONFIRM_BUTTON,
        UpdateFromBaseDialogField::CancelButton => FOCUS_ID_UPDATE_FROM_BASE_CANCEL_BUTTON,
    }
}

fn update_from_base_dialog_focus_field(focus_id: Option<u64>) -> Option<UpdateFromBaseDialogField> {
    match focus_id {
        Some(FOCUS_ID_UPDATE_FROM_BASE_CONFIRM_BUTTON) => {
            Some(UpdateFromBaseDialogField::UpdateButton)
        }
        Some(FOCUS_ID_UPDATE_FROM_BASE_CANCEL_BUTTON) => {
            Some(UpdateFromBaseDialogField::CancelButton)
        }
        _ => None,
    }
}

fn pull_upstream_dialog_focus_ids() -> [u64; 2] {
    [
        FOCUS_ID_PULL_UPSTREAM_CONFIRM_BUTTON,
        FOCUS_ID_PULL_UPSTREAM_CANCEL_BUTTON,
    ]
}

fn pull_upstream_dialog_focus_id(field: PullUpstreamDialogField) -> u64 {
    match field {
        PullUpstreamDialogField::PullButton => FOCUS_ID_PULL_UPSTREAM_CONFIRM_BUTTON,
        PullUpstreamDialogField::CancelButton => FOCUS_ID_PULL_UPSTREAM_CANCEL_BUTTON,
    }
}

fn pull_upstream_dialog_focus_field(focus_id: Option<u64>) -> Option<PullUpstreamDialogField> {
    match focus_id {
        Some(FOCUS_ID_PULL_UPSTREAM_CONFIRM_BUTTON) => Some(PullUpstreamDialogField::PullButton),
        Some(FOCUS_ID_PULL_UPSTREAM_CANCEL_BUTTON) => Some(PullUpstreamDialogField::CancelButton),
        _ => None,
    }
}

fn settings_dialog_focus_ids() -> [u64; 3] {
    [
        FOCUS_ID_SETTINGS_THEME,
        FOCUS_ID_SETTINGS_SAVE_BUTTON,
        FOCUS_ID_SETTINGS_CANCEL_BUTTON,
    ]
}

fn settings_dialog_focus_id(field: SettingsDialogField) -> u64 {
    match field {
        SettingsDialogField::Theme => FOCUS_ID_SETTINGS_THEME,
        SettingsDialogField::SaveButton => FOCUS_ID_SETTINGS_SAVE_BUTTON,
        SettingsDialogField::CancelButton => FOCUS_ID_SETTINGS_CANCEL_BUTTON,
    }
}

fn settings_dialog_focus_field(focus_id: Option<u64>) -> Option<SettingsDialogField> {
    match focus_id {
        Some(FOCUS_ID_SETTINGS_THEME) => Some(SettingsDialogField::Theme),
        Some(FOCUS_ID_SETTINGS_SAVE_BUTTON) => Some(SettingsDialogField::SaveButton),
        Some(FOCUS_ID_SETTINGS_CANCEL_BUTTON) => Some(SettingsDialogField::CancelButton),
        _ => None,
    }
}

fn rename_tab_dialog_focus_ids() -> [u64; 3] {
    [
        FOCUS_ID_RENAME_TAB_TITLE,
        FOCUS_ID_RENAME_TAB_RENAME_BUTTON,
        FOCUS_ID_RENAME_TAB_CANCEL_BUTTON,
    ]
}

fn rename_tab_dialog_focus_id(field: RenameTabDialogField) -> u64 {
    match field {
        RenameTabDialogField::Title => FOCUS_ID_RENAME_TAB_TITLE,
        RenameTabDialogField::RenameButton => FOCUS_ID_RENAME_TAB_RENAME_BUTTON,
        RenameTabDialogField::CancelButton => FOCUS_ID_RENAME_TAB_CANCEL_BUTTON,
    }
}

fn rename_tab_dialog_focus_field(focus_id: Option<u64>) -> Option<RenameTabDialogField> {
    match focus_id {
        Some(FOCUS_ID_RENAME_TAB_TITLE) => Some(RenameTabDialogField::Title),
        Some(FOCUS_ID_RENAME_TAB_RENAME_BUTTON) => Some(RenameTabDialogField::RenameButton),
        Some(FOCUS_ID_RENAME_TAB_CANCEL_BUTTON) => Some(RenameTabDialogField::CancelButton),
        _ => None,
    }
}

fn edit_dialog_focus_ids() -> [u64; 3] {
    [
        FOCUS_ID_EDIT_BASE_BRANCH,
        FOCUS_ID_EDIT_SAVE_BUTTON,
        FOCUS_ID_EDIT_CANCEL_BUTTON,
    ]
}

fn edit_dialog_focus_id(field: EditDialogField) -> u64 {
    match field {
        EditDialogField::BaseBranch => FOCUS_ID_EDIT_BASE_BRANCH,
        EditDialogField::SaveButton => FOCUS_ID_EDIT_SAVE_BUTTON,
        EditDialogField::CancelButton => FOCUS_ID_EDIT_CANCEL_BUTTON,
    }
}

fn edit_dialog_focus_field(focus_id: Option<u64>) -> Option<EditDialogField> {
    match focus_id {
        Some(FOCUS_ID_EDIT_BASE_BRANCH) => Some(EditDialogField::BaseBranch),
        Some(FOCUS_ID_EDIT_SAVE_BUTTON) => Some(EditDialogField::SaveButton),
        Some(FOCUS_ID_EDIT_CANCEL_BUTTON) => Some(EditDialogField::CancelButton),
        _ => None,
    }
}

fn launch_dialog_focus_ids() -> [u64; 7] {
    [
        FOCUS_ID_LAUNCH_AGENT,
        FOCUS_ID_LAUNCH_NAME,
        FOCUS_ID_LAUNCH_PROMPT,
        FOCUS_ID_LAUNCH_INIT_COMMAND,
        FOCUS_ID_LAUNCH_UNSAFE,
        FOCUS_ID_LAUNCH_START_BUTTON,
        FOCUS_ID_LAUNCH_CANCEL_BUTTON,
    ]
}

pub(super) fn launch_dialog_focus_id(field: LaunchDialogField) -> u64 {
    match field {
        LaunchDialogField::Agent => FOCUS_ID_LAUNCH_AGENT,
        LaunchDialogField::StartConfig(StartAgentConfigField::Name) => FOCUS_ID_LAUNCH_NAME,
        LaunchDialogField::StartConfig(StartAgentConfigField::Prompt) => FOCUS_ID_LAUNCH_PROMPT,
        LaunchDialogField::StartConfig(StartAgentConfigField::InitCommand) => {
            FOCUS_ID_LAUNCH_INIT_COMMAND
        }
        LaunchDialogField::StartConfig(StartAgentConfigField::Unsafe) => FOCUS_ID_LAUNCH_UNSAFE,
        LaunchDialogField::StartButton => FOCUS_ID_LAUNCH_START_BUTTON,
        LaunchDialogField::CancelButton => FOCUS_ID_LAUNCH_CANCEL_BUTTON,
    }
}

fn launch_dialog_focus_field(focus_id: Option<u64>) -> Option<LaunchDialogField> {
    match focus_id {
        Some(FOCUS_ID_LAUNCH_AGENT) => Some(LaunchDialogField::Agent),
        Some(FOCUS_ID_LAUNCH_NAME) => {
            Some(LaunchDialogField::StartConfig(StartAgentConfigField::Name))
        }
        Some(FOCUS_ID_LAUNCH_PROMPT) => Some(LaunchDialogField::StartConfig(
            StartAgentConfigField::Prompt,
        )),
        Some(FOCUS_ID_LAUNCH_INIT_COMMAND) => Some(LaunchDialogField::StartConfig(
            StartAgentConfigField::InitCommand,
        )),
        Some(FOCUS_ID_LAUNCH_UNSAFE) => Some(LaunchDialogField::StartConfig(
            StartAgentConfigField::Unsafe,
        )),
        Some(FOCUS_ID_LAUNCH_START_BUTTON) => Some(LaunchDialogField::StartButton),
        Some(FOCUS_ID_LAUNCH_CANCEL_BUTTON) => Some(LaunchDialogField::CancelButton),
        _ => None,
    }
}

fn create_dialog_focus_ids(dialog: &CreateDialogState) -> Vec<u64> {
    if dialog.is_add_worktree_mode() {
        return vec![
            FOCUS_ID_CREATE_PROJECT,
            FOCUS_ID_CREATE_CREATE_BUTTON,
            FOCUS_ID_CREATE_CANCEL_BUTTON,
        ];
    }

    match dialog.tab {
        CreateDialogTab::Manual => {
            let mut members = Vec::new();
            if !dialog.register_as_base {
                members.push(FOCUS_ID_CREATE_WORKSPACE_NAME);
            }
            members.push(FOCUS_ID_CREATE_REGISTER_AS_BASE);
            members.push(FOCUS_ID_CREATE_PROJECT);
            members.push(FOCUS_ID_CREATE_CREATE_BUTTON);
            members.push(FOCUS_ID_CREATE_CANCEL_BUTTON);
            members
        }
        CreateDialogTab::PullRequest => vec![
            FOCUS_ID_CREATE_PROJECT,
            FOCUS_ID_CREATE_PULL_REQUEST_URL,
            FOCUS_ID_CREATE_CREATE_BUTTON,
            FOCUS_ID_CREATE_CANCEL_BUTTON,
        ],
    }
}

pub(super) fn create_dialog_focus_id(field: CreateDialogField) -> u64 {
    match field {
        CreateDialogField::WorkspaceName => FOCUS_ID_CREATE_WORKSPACE_NAME,
        CreateDialogField::RegisterAsBase => FOCUS_ID_CREATE_REGISTER_AS_BASE,
        CreateDialogField::PullRequestUrl => FOCUS_ID_CREATE_PULL_REQUEST_URL,
        CreateDialogField::Project => FOCUS_ID_CREATE_PROJECT,
        CreateDialogField::CreateButton => FOCUS_ID_CREATE_CREATE_BUTTON,
        CreateDialogField::CancelButton => FOCUS_ID_CREATE_CANCEL_BUTTON,
    }
}

fn create_dialog_focus_field(focus_id: Option<u64>) -> Option<CreateDialogField> {
    match focus_id {
        Some(FOCUS_ID_CREATE_WORKSPACE_NAME) => Some(CreateDialogField::WorkspaceName),
        Some(FOCUS_ID_CREATE_REGISTER_AS_BASE) => Some(CreateDialogField::RegisterAsBase),
        Some(FOCUS_ID_CREATE_PULL_REQUEST_URL) => Some(CreateDialogField::PullRequestUrl),
        Some(FOCUS_ID_CREATE_PROJECT) => Some(CreateDialogField::Project),
        Some(FOCUS_ID_CREATE_CREATE_BUTTON) => Some(CreateDialogField::CreateButton),
        Some(FOCUS_ID_CREATE_CANCEL_BUTTON) => Some(CreateDialogField::CancelButton),
        _ => None,
    }
}

fn project_add_dialog_focus_ids() -> [u64; 4] {
    [
        FOCUS_ID_PROJECT_ADD_PATH_INPUT,
        FOCUS_ID_PROJECT_ADD_NAME_INPUT,
        FOCUS_ID_PROJECT_ADD_ADD_BUTTON,
        FOCUS_ID_PROJECT_ADD_CANCEL_BUTTON,
    ]
}

fn project_add_dialog_focus_id(field: ProjectAddDialogField) -> u64 {
    match field {
        ProjectAddDialogField::Path => FOCUS_ID_PROJECT_ADD_PATH_INPUT,
        ProjectAddDialogField::Name => FOCUS_ID_PROJECT_ADD_NAME_INPUT,
        ProjectAddDialogField::AddButton => FOCUS_ID_PROJECT_ADD_ADD_BUTTON,
        ProjectAddDialogField::CancelButton => FOCUS_ID_PROJECT_ADD_CANCEL_BUTTON,
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ModalDialogSpec<'a> {
    pub(super) dialog_width: u16,
    pub(super) dialog_height: u16,
    pub(super) title: &'a str,
    pub(super) theme: ftui::ResolvedTheme,
    pub(super) border_color: PackedRgba,
    pub(super) hit_id: u32,
}

pub(super) fn render_modal_dialog(
    frame: &mut Frame,
    area: Rect,
    body: FtText<'static>,
    spec: ModalDialogSpec<'_>,
) {
    let content = OverlayModalContent {
        title: spec.title,
        body,
        theme: spec.theme,
        border_color: spec.border_color,
    };

    Modal::new(content)
        .size(
            ModalSizeConstraints::new()
                .min_width(spec.dialog_width)
                .max_width(spec.dialog_width)
                .min_height(spec.dialog_height)
                .max_height(spec.dialog_height),
        )
        .backdrop(BackdropConfig::new(packed(spec.theme.background), 0.55))
        .hit_id(HitId::new(spec.hit_id))
        .render(area, frame);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_labeled_input_row_uses_native_single_cell_truncation() {
        let theme = ui_theme();
        let row = modal_labeled_input_row(7, theme, "X", "abcdef", "", false);

        assert_eq!(row.width(), 7);
        assert_eq!(row.to_plain_text(), "  [X] a");
    }

    #[test]
    fn modal_actions_row_pads_to_exact_width_after_truncation() {
        let theme = ui_theme();
        let row = modal_actions_row(12, theme, "Primary", "Secondary", false, false);

        assert_eq!(row.width(), 12);
        assert_eq!(row.to_plain_text(), "   Primary  ");
    }
}

#[derive(Debug, Clone)]
pub(super) struct OverlayModalContent<'a> {
    pub(super) title: &'a str,
    pub(super) body: FtText<'static>,
    pub(super) theme: ftui::ResolvedTheme,
    pub(super) border_color: PackedRgba,
}

impl Widget for OverlayModalContent<'_> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        if area.is_empty() {
            return;
        }

        let content_style = Style::new()
            .bg(packed(self.theme.background))
            .fg(packed(self.theme.text));

        Paragraph::new("").style(content_style).render(area, frame);

        let block = Block::new()
            .title(self.title)
            .title_alignment(BlockAlignment::Center)
            .borders(Borders::ALL)
            .style(content_style)
            .border_style(Style::new().fg(self.border_color).bold());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        Paragraph::new(self.body.clone())
            .style(content_style)
            .render(inner, frame);
    }
}

macro_rules! active_dialog_accessors {
    ($get:ident, $get_mut:ident, $set:ident, $variant:ident, $ty:ty) => {
        pub(super) fn $get(&self) -> Option<&$ty> {
            match self.dialogs.active_dialog.as_ref() {
                Some(ActiveDialog::$variant(dialog)) => Some(dialog),
                _ => None,
            }
        }

        pub(super) fn $get_mut(&mut self) -> Option<&mut $ty> {
            match self.dialogs.active_dialog.as_mut() {
                Some(ActiveDialog::$variant(dialog)) => Some(dialog),
                _ => None,
            }
        }

        pub(super) fn $set(&mut self, dialog: $ty) {
            self.clear_active_dialog_focus_trap();
            self.dialogs.active_dialog = Some(ActiveDialog::$variant(dialog));
            self.sync_active_dialog_focus_trap();
        }
    };
}

macro_rules! active_dialog_take_accessor {
    ($take:ident, $variant:ident, $ty:ty) => {
        pub(super) fn $take(&mut self) -> Option<$ty> {
            let active_dialog = self.dialogs.active_dialog.take()?;
            match active_dialog {
                ActiveDialog::$variant(dialog) => {
                    self.close_focus_trap_for_active_dialog(&ActiveDialog::$variant(
                        dialog.clone(),
                    ));
                    Some(dialog)
                }
                other => {
                    self.dialogs.active_dialog = Some(other);
                    None
                }
            }
        }
    };
}

macro_rules! active_dialog_read_accessors {
    ($get:ident, $set:ident, $variant:ident, $ty:ty) => {
        pub(super) fn $get(&self) -> Option<&$ty> {
            match self.dialogs.active_dialog.as_ref() {
                Some(ActiveDialog::$variant(dialog)) => Some(dialog),
                _ => None,
            }
        }

        pub(super) fn $set(&mut self, dialog: $ty) {
            self.clear_active_dialog_focus_trap();
            self.dialogs.active_dialog = Some(ActiveDialog::$variant(dialog));
            self.sync_active_dialog_focus_trap();
        }
    };
}

impl GroveApp {
    pub(super) fn dialog_focus_is(&self, focus_id: u64) -> bool {
        self.focus_manager.current() == Some(focus_id)
    }

    pub(super) fn focus_dialog_field(&mut self, focus_id: u64) {
        let _ = self.focus_manager.focus(focus_id);
        self.sync_active_dialog_focus_field();
    }

    pub(super) fn focus_next_dialog_field(&mut self) {
        let _ = self.focus_manager.focus_next();
        self.sync_active_dialog_focus_field();
    }

    pub(super) fn focus_prev_dialog_field(&mut self) {
        let _ = self.focus_manager.focus_prev();
        self.sync_active_dialog_focus_field();
    }

    pub(super) fn sync_active_dialog_focus_field(&mut self) {
        let focus_id = self.focus_manager.current();
        match self.dialogs.active_dialog.as_mut() {
            Some(ActiveDialog::Create(dialog)) => {
                if let Some(field) = create_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::Launch(dialog)) => {
                if let Some(field) = launch_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::Stop(dialog)) => {
                if let Some(field) = stop_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::Confirm(dialog)) => {
                if let Some(field) = confirm_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::SessionCleanup(dialog)) => {
                if let Some(field) = session_cleanup_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::Delete(dialog)) => {
                if let Some(field) = delete_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::Merge(dialog)) => {
                if let Some(field) = merge_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::UpdateFromBase(dialog)) => {
                if let Some(field) = update_from_base_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::PullUpstream(dialog)) => {
                if let Some(field) = pull_upstream_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::Settings(dialog)) => {
                if let Some(field) = settings_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::Edit(dialog)) => {
                if let Some(field) = edit_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            Some(ActiveDialog::RenameTab(dialog)) => {
                if let Some(field) = rename_tab_dialog_focus_field(focus_id) {
                    dialog.focused_field = field;
                }
            }
            _ => {}
        }
    }

    fn open_focus_trap_for_active_dialog(&mut self, dialog: &ActiveDialog) {
        match dialog {
            ActiveDialog::Create(dialog) => {
                let members = create_dialog_focus_ids(dialog);
                let initial = create_dialog_focus_id(dialog.focused_field);
                self.activate_focus_trap(FOCUS_GROUP_CREATE_DIALOG, &members, initial);
            }
            ActiveDialog::Launch(dialog) => {
                let members = launch_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_LAUNCH_DIALOG,
                    &members,
                    launch_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::Stop(dialog) => {
                let members = stop_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_STOP_DIALOG,
                    &members,
                    stop_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::Confirm(dialog) => {
                let members = confirm_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_CONFIRM_DIALOG,
                    &members,
                    confirm_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::Project(dialog) => {
                let members = project_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_PROJECT_DIALOG,
                    &members,
                    FOCUS_ID_PROJECT_DIALOG_FILTER_INPUT,
                );
                if let Some(add_dialog) = dialog.add_dialog.as_ref() {
                    let add_members = project_add_dialog_focus_ids();
                    self.activate_focus_trap(
                        FOCUS_GROUP_PROJECT_ADD_DIALOG,
                        &add_members,
                        project_add_dialog_focus_id(add_dialog.focused_field),
                    );
                }
            }
            ActiveDialog::SessionCleanup(dialog) => {
                let members = session_cleanup_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_SESSION_CLEANUP_DIALOG,
                    &members,
                    session_cleanup_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::Delete(dialog) => {
                let members = delete_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_DELETE_DIALOG,
                    &members,
                    delete_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::Merge(dialog) => {
                let members = merge_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_MERGE_DIALOG,
                    &members,
                    merge_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::UpdateFromBase(dialog) => {
                let members = update_from_base_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_UPDATE_FROM_BASE_DIALOG,
                    &members,
                    update_from_base_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::PullUpstream(dialog) => {
                let members = pull_upstream_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_PULL_UPSTREAM_DIALOG,
                    &members,
                    pull_upstream_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::Settings(dialog) => {
                let members = settings_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_SETTINGS_DIALOG,
                    &members,
                    settings_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::Edit(dialog) => {
                let members = edit_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_EDIT_DIALOG,
                    &members,
                    edit_dialog_focus_id(dialog.focused_field),
                );
            }
            ActiveDialog::RenameTab(dialog) => {
                let members = rename_tab_dialog_focus_ids();
                self.activate_focus_trap(
                    FOCUS_GROUP_RENAME_TAB_DIALOG,
                    &members,
                    rename_tab_dialog_focus_id(dialog.focused_field),
                );
            }
            _ => {}
        }
    }

    fn close_focus_trap_for_active_dialog(&mut self, dialog: &ActiveDialog) {
        match dialog {
            ActiveDialog::Create(dialog) => {
                let members = create_dialog_focus_ids(dialog);
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Launch(_) => {
                let members = launch_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Stop(_) => {
                let members = stop_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Confirm(_) => {
                let members = confirm_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Project(dialog) => {
                if dialog.add_dialog.is_some() {
                    let add_members = project_add_dialog_focus_ids();
                    self.deactivate_focus_trap(&add_members);
                }
                let members = project_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::SessionCleanup(_) => {
                let members = session_cleanup_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Delete(_) => {
                let members = delete_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Merge(_) => {
                let members = merge_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::UpdateFromBase(_) => {
                let members = update_from_base_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::PullUpstream(_) => {
                let members = pull_upstream_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Settings(_) => {
                let members = settings_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::Edit(_) => {
                let members = edit_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            ActiveDialog::RenameTab(_) => {
                let members = rename_tab_dialog_focus_ids();
                self.deactivate_focus_trap(&members);
            }
            _ => {}
        }
    }

    fn sync_active_dialog_focus_trap(&mut self) {
        if let Some(dialog) = self.dialogs.active_dialog.clone() {
            self.open_focus_trap_for_active_dialog(&dialog);
            self.sync_active_dialog_focus_field();
        }
    }

    fn clear_active_dialog_focus_trap(&mut self) {
        if let Some(dialog) = self.dialogs.active_dialog.clone() {
            self.close_focus_trap_for_active_dialog(&dialog);
        }
    }

    pub(super) fn refresh_active_dialog_focus_trap(&mut self) {
        self.clear_active_dialog_focus_trap();
        self.sync_active_dialog_focus_trap();
    }

    pub(super) fn open_project_add_dialog_focus_trap(&mut self, field: ProjectAddDialogField) {
        let members = project_add_dialog_focus_ids();
        self.activate_focus_trap(
            FOCUS_GROUP_PROJECT_ADD_DIALOG,
            &members,
            project_add_dialog_focus_id(field),
        );
    }

    pub(super) fn close_project_add_dialog_focus_trap(&mut self) {
        let members = project_add_dialog_focus_ids();
        self.deactivate_focus_trap(&members);
    }

    pub(super) fn close_active_dialog(&mut self) {
        self.clear_active_dialog_focus_trap();
        self.dialogs.active_dialog = None;
    }

    pub(super) fn active_dialog_kind(&self) -> Option<&'static str> {
        match self.dialogs.active_dialog.as_ref() {
            Some(ActiveDialog::Launch(_)) => Some("launch"),
            Some(ActiveDialog::Stop(_)) => Some("stop"),
            Some(ActiveDialog::Confirm(_)) => Some("confirm"),
            Some(ActiveDialog::SessionCleanup(_)) => Some("session_cleanup"),
            Some(ActiveDialog::Delete(_)) => Some("delete"),
            Some(ActiveDialog::Merge(_)) => Some("merge"),
            Some(ActiveDialog::UpdateFromBase(_)) => Some("update_from_base"),
            Some(ActiveDialog::PullUpstream(_)) => Some("pull_upstream"),
            Some(ActiveDialog::Create(_)) => Some("create"),
            Some(ActiveDialog::Edit(_)) => Some("edit"),
            Some(ActiveDialog::RenameTab(_)) => Some("rename_tab"),
            Some(ActiveDialog::Project(_)) => Some("project"),
            Some(ActiveDialog::Settings(_)) => Some("settings"),
            Some(ActiveDialog::Performance(_)) => Some("performance"),
            None => None,
        }
    }

    active_dialog_accessors!(
        launch_dialog,
        launch_dialog_mut,
        set_launch_dialog,
        Launch,
        LaunchDialogState
    );
    active_dialog_take_accessor!(take_launch_dialog, Launch, LaunchDialogState);
    active_dialog_read_accessors!(stop_dialog, set_stop_dialog, Stop, StopDialogState);
    active_dialog_take_accessor!(take_stop_dialog, Stop, StopDialogState);
    active_dialog_read_accessors!(
        confirm_dialog,
        set_confirm_dialog,
        Confirm,
        ConfirmDialogState
    );
    active_dialog_take_accessor!(take_confirm_dialog, Confirm, ConfirmDialogState);
    active_dialog_accessors!(
        session_cleanup_dialog,
        session_cleanup_dialog_mut,
        set_session_cleanup_dialog,
        SessionCleanup,
        SessionCleanupDialogState
    );
    active_dialog_accessors!(
        delete_dialog,
        delete_dialog_mut,
        set_delete_dialog,
        Delete,
        DeleteDialogState
    );
    active_dialog_take_accessor!(take_delete_dialog, Delete, DeleteDialogState);
    active_dialog_accessors!(
        merge_dialog,
        merge_dialog_mut,
        set_merge_dialog,
        Merge,
        MergeDialogState
    );
    active_dialog_take_accessor!(take_merge_dialog, Merge, MergeDialogState);
    active_dialog_read_accessors!(
        update_from_base_dialog,
        set_update_from_base_dialog,
        UpdateFromBase,
        UpdateFromBaseDialogState
    );
    active_dialog_take_accessor!(
        take_update_from_base_dialog,
        UpdateFromBase,
        UpdateFromBaseDialogState
    );
    active_dialog_read_accessors!(
        pull_upstream_dialog,
        set_pull_upstream_dialog,
        PullUpstream,
        PullUpstreamDialogState
    );
    active_dialog_take_accessor!(
        take_pull_upstream_dialog,
        PullUpstream,
        PullUpstreamDialogState
    );
    active_dialog_accessors!(
        create_dialog,
        create_dialog_mut,
        set_create_dialog,
        Create,
        CreateDialogState
    );
    active_dialog_accessors!(
        edit_dialog,
        edit_dialog_mut,
        set_edit_dialog,
        Edit,
        EditDialogState
    );
    active_dialog_accessors!(
        rename_tab_dialog,
        rename_tab_dialog_mut,
        set_rename_tab_dialog,
        RenameTab,
        RenameTabDialogState
    );
    active_dialog_accessors!(
        settings_dialog,
        settings_dialog_mut,
        set_settings_dialog,
        Settings,
        SettingsDialogState
    );
    pub(super) fn performance_dialog(&self) -> Option<&PerformanceDialogState> {
        match self.dialogs.active_dialog.as_ref() {
            Some(ActiveDialog::Performance(dialog)) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn set_performance_dialog(&mut self, dialog: PerformanceDialogState) {
        self.clear_active_dialog_focus_trap();
        self.dialogs.active_dialog = Some(ActiveDialog::Performance(dialog));
    }

    pub(super) fn allows_text_input_modifiers(modifiers: Modifiers) -> bool {
        modifiers.is_empty() || modifiers == Modifiers::SHIFT
    }

    pub(super) fn project_dialog(&self) -> Option<&ProjectDialogState> {
        match self.dialogs.active_dialog.as_ref() {
            Some(ActiveDialog::Project(dialog)) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn project_dialog_mut(&mut self) -> Option<&mut ProjectDialogState> {
        match self.dialogs.active_dialog.as_mut() {
            Some(ActiveDialog::Project(dialog)) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn set_project_dialog(&mut self, dialog: ProjectDialogState) {
        self.clear_active_dialog_focus_trap();
        self.dialogs.active_dialog = Some(ActiveDialog::Project(Box::new(dialog)));
        self.sync_active_dialog_focus_trap();
    }

    pub(super) fn handle_keybind_help_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Escape | KeyCode::Enter | KeyCode::Char('?') => {
                self.dialogs.keybind_help_open = false;
            }
            _ => {}
        }
    }
}

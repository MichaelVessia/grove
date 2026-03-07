#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplayStateSnapshot {
    selected_index: usize,
    task_count: usize,
    worktree_count: usize,
    selected_task: Option<String>,
    selected_worktree: Option<String>,
    focus: ReplayFocus,
    mode: ReplayMode,
    preview_tab: ReplayPreviewTab,
    interactive_session: Option<String>,
    poll_generation: u64,
    preview_offset: usize,
    preview_auto_scroll: bool,
    preview_line_count: usize,
    preview_line_hash: u64,
    output_changing: bool,
    pending_input_depth: u64,
    active_modal: Option<String>,
    keybind_help_open: bool,
    command_palette_open: bool,
    #[serde(default)]
    theme_name: ThemeName,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplaySnapshotFile {
    schema_version: u64,
    trace_path: String,
    steps: Vec<ReplaySnapshotStep>,
    final_state: ReplayStateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ReplaySnapshotStep {
    seq: u64,
    msg_kind: String,
    state: ReplayStateSnapshot,
    frame_hash: u64,
}

impl ReplayStateSnapshot {
    fn from_app(app: &GroveApp) -> Self {
        let preview_line_hash = {
            let mut hasher = DefaultHasher::new();
            app.preview.render_lines.hash(&mut hasher);
            hasher.finish()
        };

        Self {
            selected_index: app.state.selected_index,
            task_count: app.state.tasks.len(),
            worktree_count: app.state.workspaces.len(),
            selected_task: app.state.selected_task().map(|task| task.slug.clone()),
            selected_worktree: app
                .state
                .selected_worktree()
                .map(|worktree| worktree.repository_name.clone()),
            focus: ReplayFocus::from_focus(app.state.focus),
            mode: ReplayMode::from_mode(app.state.mode),
            preview_tab: ReplayPreviewTab::from_preview_tab(app.preview_tab),
            interactive_session: app.interactive_target_session(),
            poll_generation: app.polling.poll_generation,
            preview_offset: app.preview.offset,
            preview_auto_scroll: app.preview.auto_scroll,
            preview_line_count: app.preview.render_lines.len(),
            preview_line_hash,
            output_changing: app.polling.output_changing,
            pending_input_depth: app.pending_input_depth(),
            active_modal: app.active_dialog_kind().map(str::to_string),
            keybind_help_open: app.dialogs.keybind_help_open,
            command_palette_open: app.dialogs.command_palette.is_visible(),
            theme_name: app.theme_name,
        }
    }
}

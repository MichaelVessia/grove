use super::view_prelude::*;

impl GroveApp {
    fn workspace_attention_color(&self, attention: WorkspaceAttention) -> PackedRgba {
        let theme = self.active_ui_theme();
        match attention {
            WorkspaceAttention::NeedsAttention => theme.yellow,
        }
    }

    pub(super) fn workspace_attention_indicator(
        &self,
        workspace_path: &Path,
    ) -> Option<(&'static str, PackedRgba)> {
        let attention = self.workspace_attention(workspace_path)?;
        let symbol = match attention {
            WorkspaceAttention::NeedsAttention => "!",
        };
        Some((symbol, self.workspace_attention_color(attention)))
    }

    pub(super) fn pane_border_style(&self, focused: bool) -> Style {
        let theme = self.active_ui_theme();
        if focused {
            return Style::new().fg(theme.blue).bold();
        }

        Style::new().fg(theme.overlay0)
    }

    pub(super) fn workspace_agent_color(&self, agent: AgentType) -> PackedRgba {
        let theme = self.active_ui_theme();
        match agent {
            AgentType::Claude => theme.peach,
            AgentType::Codex => theme.text,
            AgentType::OpenCode => theme.text,
        }
    }

    fn preview_activity_effect_gradient(&self) -> ColorGradient {
        let theme = self.active_ui_theme();
        ColorGradient::new(vec![
            (0.0, theme.blue),
            (0.5, theme.overlay0),
            (1.0, theme.blue),
        ])
    }

    pub(super) fn render_preview_activity_effect_label(
        &self,
        label: &str,
        area: Rect,
        frame: &mut Frame,
    ) {
        self.render_accent_activity_effect_label(label, area, frame);
    }

    fn render_accent_activity_effect_label(&self, label: &str, area: Rect, frame: &mut Frame) {
        if area.is_empty() || label.is_empty() {
            return;
        }

        let theme = self.active_ui_theme();
        StyledText::new(label)
            .bold()
            .base_color(theme.blue)
            .effect(TextEffect::AnimatedGradient {
                gradient: self.preview_activity_effect_gradient(),
                speed: 1.8,
            })
            .time(self.activity_animation_time())
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

    pub(super) fn workspace_display_name(workspace: &Workspace) -> String {
        workspace.name.clone()
    }
}

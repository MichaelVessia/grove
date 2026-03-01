use super::view_prelude::*;

impl GroveApp {
    fn workspace_attention_color(&self, attention: WorkspaceAttention) -> PackedRgba {
        let theme = ui_theme();
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
        let theme = ui_theme();
        if focused {
            return Style::new().fg(theme.blue).bold();
        }

        Style::new().fg(theme.overlay0)
    }

    pub(super) fn workspace_agent_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.peach,
            AgentType::Codex => theme.text,
            AgentType::OpenCode => theme.text,
        }
    }

    fn activity_effect_secondary_color(&self, agent: AgentType) -> PackedRgba {
        let theme = ui_theme();
        match agent {
            AgentType::Claude => theme.text,
            AgentType::Codex => theme.overlay0,
            AgentType::OpenCode => theme.crust,
        }
    }

    fn activity_effect_gradient(&self, agent: AgentType) -> ColorGradient {
        let primary = self.workspace_agent_color(agent);
        let secondary = self.activity_effect_secondary_color(agent);
        ColorGradient::new(vec![(0.0, primary), (0.5, secondary), (1.0, primary)])
    }

    fn activity_effect_time(&self) -> f64 {
        self.polling.fast_animation_frame as f64 * (FAST_ANIMATION_INTERVAL_MS as f64 / 1000.0)
    }

    pub(super) fn render_activity_effect_label(
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

    pub(super) fn workspace_display_name(workspace: &Workspace) -> String {
        workspace.name.clone()
    }
}

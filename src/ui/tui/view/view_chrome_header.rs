use super::*;

impl GroveApp {
    pub(super) fn render_header(&self, frame: &mut Frame, area: Rect) {
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
}

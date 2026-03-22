use super::update_prelude::*;

const DIFF_POLL_INTERVAL_FOCUSED_MS: u64 = 2_000;
const DIFF_POLL_INTERVAL_UNFOCUSED_MS: u64 = 10_000;
const DIFF_STAT_POLL_INTERVAL_MS: u64 = 5_000;

fn format_diff_summary(files_changed: usize, insertions: usize, deletions: usize) -> String {
    let file_word = if files_changed == 1 { "file" } else { "files" };
    if insertions == 0 && deletions == 0 {
        return format!("{files_changed} {file_word} changed");
    }
    format!("{files_changed} {file_word} changed, +{insertions} -{deletions}")
}

fn parse_diff_stat_summary(stat_output: &str) -> (usize, usize, usize) {
    let last_line = stat_output.lines().last().unwrap_or("");
    let mut files = 0usize;
    let mut insertions = 0usize;
    let mut deletions = 0usize;

    for segment in last_line.split(',') {
        let trimmed = segment.trim();
        if let Some(rest) = trimmed
            .strip_suffix("file changed")
            .or_else(|| trimmed.strip_suffix("files changed"))
        {
            files = rest.trim().parse().unwrap_or(0);
        } else if trimmed.contains("insertion")
            && let Some(num) = trimmed.split_whitespace().next()
        {
            insertions = num.parse().unwrap_or(0);
        } else if trimmed.contains("deletion")
            && let Some(num) = trimmed.split_whitespace().next()
        {
            deletions = num.parse().unwrap_or(0);
        }
    }
    (files, insertions, deletions)
}

fn run_diff_capture(workspace_path: PathBuf) -> DiffCaptureCompletion {
    let started_at = std::time::Instant::now();

    let (stat_result, diff_result, staged_result) = std::thread::scope(|s| {
        let wp = &workspace_path;
        let stat = s.spawn(move || {
            std::process::Command::new("git")
                .args(["diff", "HEAD", "--stat"])
                .current_dir(wp)
                .output()
        });
        let diff = s.spawn(move || {
            std::process::Command::new("git")
                .args(["diff", "--color=always"])
                .current_dir(wp)
                .output()
        });
        let staged = s.spawn(move || {
            std::process::Command::new("git")
                .args(["diff", "--cached", "--color=always"])
                .current_dir(wp)
                .output()
        });
        (
            stat.join().unwrap(),
            diff.join().unwrap(),
            staged.join().unwrap(),
        )
    });

    let elapsed = std::time::Instant::now().saturating_duration_since(started_at);
    let capture_ms = elapsed.as_millis() as u64;

    let build_output = || -> Result<String, String> {
        let stat_output = stat_result.map_err(|e| format!("git diff --stat failed: {e}"))?;
        let diff_output = diff_result.map_err(|e| format!("git diff failed: {e}"))?;
        let staged_output = staged_result.map_err(|e| format!("git diff --cached failed: {e}"))?;

        let stat_str = String::from_utf8_lossy(&stat_output.stdout);
        let diff_str = String::from_utf8_lossy(&diff_output.stdout);
        let staged_str = String::from_utf8_lossy(&staged_output.stdout);

        let (files, ins, del) = parse_diff_stat_summary(&stat_str);
        let summary = format_diff_summary(files, ins, del);

        let mut output = String::new();
        output.push_str(&summary);
        output.push('\n');

        if !staged_str.is_empty() {
            output.push_str("\n── staged ──\n\n");
            output.push_str(&staged_str);
        }
        if !diff_str.is_empty() {
            if !staged_str.is_empty() {
                output.push_str("\n── unstaged ──\n\n");
            }
            output.push_str(&diff_str);
        }
        if staged_str.is_empty() && diff_str.is_empty() {
            output.push_str("\n(no changes)");
        }
        Ok(output)
    };

    DiffCaptureCompletion {
        workspace_path,
        capture_ms,
        result: build_output(),
    }
}

impl GroveApp {
    pub(super) fn poll_diff_for_selected_workspace(&mut self) {
        if self.preview_tab != PreviewTab::Diff {
            return;
        }
        if self.polling.diff_capture_in_flight {
            return;
        }
        let Some(workspace) = self.state.selected_workspace() else {
            return;
        };
        self.polling.diff_capture_in_flight = true;
        let workspace_path = workspace.path.clone();
        self.queue_cmd(Cmd::task(move || {
            Msg::DiffCaptureCompleted(run_diff_capture(workspace_path))
        }));
    }

    pub(super) fn handle_diff_capture_completed(&mut self, completion: DiffCaptureCompletion) {
        self.polling.diff_capture_in_flight = false;
        if self.preview_tab != PreviewTab::Diff {
            return;
        }
        let Some(workspace) = self.state.selected_workspace() else {
            return;
        };
        if workspace.path != completion.workspace_path {
            return;
        }
        match completion.result {
            Ok(ref output) => {
                self.preview.apply_capture(output);
                let (_, ins, del) = parse_diff_stat_summary(output);
                if ins > 0 || del > 0 {
                    self.workspace_diff_stats.insert(
                        completion.workspace_path.clone(),
                        DiffStatBadge {
                            insertions: ins,
                            deletions: del,
                        },
                    );
                } else {
                    self.workspace_diff_stats.remove(&completion.workspace_path);
                }
                self.telemetry.event_log.log(
                    LogEvent::new("diff_poll", "capture_completed")
                        .with_data(
                            "workspace_path",
                            Value::from(completion.workspace_path.to_string_lossy().to_string()),
                        )
                        .with_data("capture_ms", Value::from(completion.capture_ms))
                        .with_data("output_bytes", Value::from(usize_to_u64(output.len()))),
                );
            }
            Err(error) => {
                self.preview
                    .apply_capture(&format!("(diff capture failed: {error})"));
                self.telemetry.event_log.log(
                    LogEvent::new("diff_poll", "capture_failed")
                        .with_data("error", Value::from(error))
                        .with_data("capture_ms", Value::from(completion.capture_ms)),
                );
            }
        }
    }

    pub(super) fn handle_diff_stat_completed(&mut self, completion: DiffStatCompletion) {
        self.polling.diff_stat_in_flight = false;
        if completion.insertions > 0 || completion.deletions > 0 {
            self.workspace_diff_stats.insert(
                completion.workspace_path,
                DiffStatBadge {
                    insertions: completion.insertions,
                    deletions: completion.deletions,
                },
            );
        } else {
            self.workspace_diff_stats.remove(&completion.workspace_path);
        }
    }

    pub(super) fn maybe_poll_diff(&mut self) {
        self.maybe_poll_diff_stat();
        if self.preview_tab != PreviewTab::Diff {
            return;
        }
        let focused = self.state.mode == UiMode::Preview && self.state.focus == PaneFocus::Preview;
        let interval_ms = if focused {
            DIFF_POLL_INTERVAL_FOCUSED_MS
        } else {
            DIFF_POLL_INTERVAL_UNFOCUSED_MS
        };
        let now = Instant::now();
        if let Some(last) = self.polling.last_diff_poll_at {
            let since = now.saturating_duration_since(last);
            if since < Duration::from_millis(interval_ms) {
                return;
            }
        }
        self.polling.last_diff_poll_at = Some(now);
        self.poll_diff_for_selected_workspace();
    }

    fn maybe_poll_diff_stat(&mut self) {
        if self.polling.diff_stat_in_flight || self.polling.diff_capture_in_flight {
            return;
        }
        let Some(workspace) = self.state.selected_workspace() else {
            return;
        };
        let now = Instant::now();
        if let Some(last) = self.polling.last_diff_stat_poll_at
            && now.saturating_duration_since(last)
                < Duration::from_millis(DIFF_STAT_POLL_INTERVAL_MS)
        {
            return;
        }
        self.polling.last_diff_stat_poll_at = Some(now);
        self.polling.diff_stat_in_flight = true;
        let workspace_path = workspace.path.clone();
        self.queue_cmd(Cmd::task(move || {
            let result = std::process::Command::new("git")
                .args(["diff", "HEAD", "--shortstat"])
                .current_dir(&workspace_path)
                .output();
            let (insertions, deletions) = match result {
                Ok(output) => {
                    let stat_str = String::from_utf8_lossy(&output.stdout);
                    let (_, ins, del) = parse_diff_stat_summary(&stat_str);
                    (ins, del)
                }
                Err(_) => (0, 0),
            };
            Msg::DiffStatCompleted(DiffStatCompletion {
                workspace_path,
                insertions,
                deletions,
            })
        }));
    }

    pub(super) fn diff_stat_for_workspace(
        &self,
        workspace_path: &std::path::Path,
    ) -> Option<&DiffStatBadge> {
        self.workspace_diff_stats.get(workspace_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_diff_summary_header_with_changes() {
        assert_eq!(format_diff_summary(3, 2, 1), "3 files changed, +2 -1");
    }

    #[test]
    fn format_diff_summary_header_no_changes() {
        let header = format_diff_summary(0, 0, 0);
        assert_eq!(header, "0 files changed");
    }

    #[test]
    fn format_diff_summary_header_single_file() {
        let header = format_diff_summary(1, 10, 3);
        assert_eq!(header, "1 file changed, +10 -3");
    }

    #[test]
    fn parse_diff_stat_summary_typical_output() {
        let stat = " src/main.rs | 5 ++---\n 1 file changed, 2 insertions(+), 3 deletions(-)";
        let (files, ins, del) = parse_diff_stat_summary(stat);
        assert_eq!((files, ins, del), (1, 2, 3));
    }

    #[test]
    fn parse_diff_stat_summary_multiple_files() {
        let stat = " 3 files changed, 47 insertions(+), 12 deletions(-)";
        let (files, ins, del) = parse_diff_stat_summary(stat);
        assert_eq!((files, ins, del), (3, 47, 12));
    }

    #[test]
    fn parse_diff_stat_summary_empty() {
        let (files, ins, del) = parse_diff_stat_summary("");
        assert_eq!((files, ins, del), (0, 0, 0));
    }

    #[test]
    fn parse_diff_stat_summary_shortstat_output() {
        let stat = " 3 files changed, 47 insertions(+), 12 deletions(-)";
        let (files, ins, del) = parse_diff_stat_summary(stat);
        assert_eq!((files, ins, del), (3, 47, 12));
    }

    #[test]
    fn parse_diff_stat_summary_insertions_only() {
        let stat = " 1 file changed, 5 insertions(+)";
        let (files, ins, del) = parse_diff_stat_summary(stat);
        assert_eq!((files, ins, del), (1, 5, 0));
    }

    #[test]
    fn parse_diff_stat_summary_deletions_only() {
        let stat = " 2 files changed, 8 deletions(-)";
        let (files, ins, del) = parse_diff_stat_summary(stat);
        assert_eq!((files, ins, del), (2, 0, 8));
    }
}

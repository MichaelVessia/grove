use super::{CaptureChange, OutputDigest};

pub(crate) fn evaluate_capture_change(
    previous: Option<&OutputDigest>,
    raw_output: &str,
) -> CaptureChange {
    super::evaluate_capture_change(previous, raw_output)
}

pub fn tmux_capture_error_indicates_missing_session(error: &str) -> bool {
    super::tmux_capture_error_indicates_missing_session(error)
}

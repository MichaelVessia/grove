use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CliErrorCode {
    InvalidArgument,
    WorkspaceInvalidName,
    RepoNotFound,
    WorkspaceNotFound,
    WorkspaceAlreadyExists,
    Conflict,
    TmuxCommandFailed,
    GitCommandFailed,
    IoError,
    Internal,
}

impl CliErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "INVALID_ARGUMENT",
            Self::WorkspaceInvalidName => "WORKSPACE_INVALID_NAME",
            Self::RepoNotFound => "REPO_NOT_FOUND",
            Self::WorkspaceNotFound => "WORKSPACE_NOT_FOUND",
            Self::WorkspaceAlreadyExists => "WORKSPACE_ALREADY_EXISTS",
            Self::Conflict => "CONFLICT",
            Self::TmuxCommandFailed => "TMUX_COMMAND_FAILED",
            Self::GitCommandFailed => "GIT_COMMAND_FAILED",
            Self::IoError => "IO_ERROR",
            Self::Internal => "INTERNAL",
        }
    }
}

impl std::fmt::Display for CliErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn classify_error_message(message: &str) -> CliErrorCode {
    let normalized = message.to_ascii_lowercase();

    if contains_any(&normalized, &["workspace name must be [a-za-z0-9_-]"]) {
        return CliErrorCode::WorkspaceInvalidName;
    }

    if contains_any(
        &normalized,
        &[
            "workspace name is required",
            "workspace branch is required",
            "base branch is required",
            "existing branch is required",
            "workspace branch matches base branch",
            "invalid argument",
            "selector mismatch",
            "missing selector",
        ],
    ) {
        return CliErrorCode::InvalidArgument;
    }

    if contains_any(
        &normalized,
        &[
            "repo name unavailable",
            "workspace project root unavailable",
            "not a git repository",
            "rev-parse --show-toplevel",
        ],
    ) {
        return CliErrorCode::RepoNotFound;
    }

    if contains_any(
        &normalized,
        &[
            "workspace path does not exist on disk",
            "workspace not found",
        ],
    ) {
        return CliErrorCode::WorkspaceNotFound;
    }

    if contains_any(
        &normalized,
        &[
            "already checked out",
            "already exists",
            "already registered",
        ],
    ) {
        return CliErrorCode::WorkspaceAlreadyExists;
    }

    if contains_any(
        &normalized,
        &[
            "conflict (content):",
            "automatic merge failed",
            "base worktree has uncommitted changes",
            "workspace worktree has uncommitted changes",
        ],
    ) {
        return CliErrorCode::Conflict;
    }

    if contains_any(
        &normalized,
        &["command failed: tmux ", "tmux command failed"],
    ) {
        return CliErrorCode::TmuxCommandFailed;
    }

    if contains_any(
        &normalized,
        &[
            "git command failed:",
            "git worktree remove failed:",
            "git worktree prune failed:",
            "git pull failed:",
            "git merge failed:",
        ],
    ) {
        return CliErrorCode::GitCommandFailed;
    }

    if contains_any(
        &normalized,
        &[
            "io error:",
            "home directory unavailable",
            "launcher script write failed:",
        ],
    ) {
        return CliErrorCode::IoError;
    }

    CliErrorCode::Internal
}

fn contains_any(message: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| message.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::{CliErrorCode, classify_error_message};

    #[test]
    fn error_code_wire_values_are_stable() {
        assert_eq!(
            CliErrorCode::InvalidArgument.to_string(),
            "INVALID_ARGUMENT"
        );
        assert_eq!(
            CliErrorCode::WorkspaceInvalidName.to_string(),
            "WORKSPACE_INVALID_NAME"
        );
        assert_eq!(CliErrorCode::RepoNotFound.to_string(), "REPO_NOT_FOUND");
        assert_eq!(
            CliErrorCode::WorkspaceNotFound.to_string(),
            "WORKSPACE_NOT_FOUND"
        );
        assert_eq!(
            CliErrorCode::WorkspaceAlreadyExists.to_string(),
            "WORKSPACE_ALREADY_EXISTS"
        );
        assert_eq!(CliErrorCode::Conflict.to_string(), "CONFLICT");
        assert_eq!(
            CliErrorCode::TmuxCommandFailed.to_string(),
            "TMUX_COMMAND_FAILED"
        );
        assert_eq!(
            CliErrorCode::GitCommandFailed.to_string(),
            "GIT_COMMAND_FAILED"
        );
        assert_eq!(CliErrorCode::IoError.to_string(), "IO_ERROR");
        assert_eq!(CliErrorCode::Internal.to_string(), "INTERNAL");
    }

    #[test]
    fn classifier_maps_workspace_validation_errors() {
        assert_eq!(
            classify_error_message("workspace name is required"),
            CliErrorCode::InvalidArgument
        );
        assert_eq!(
            classify_error_message("workspace name must be [A-Za-z0-9_-]"),
            CliErrorCode::WorkspaceInvalidName
        );
    }

    #[test]
    fn classifier_maps_target_resolution_errors() {
        assert_eq!(
            classify_error_message("workspace project root unavailable"),
            CliErrorCode::RepoNotFound
        );
        assert_eq!(
            classify_error_message("workspace path does not exist on disk"),
            CliErrorCode::WorkspaceNotFound
        );
        assert_eq!(
            classify_error_message(
                "fatal: 'feature-auth' is already checked out at '/repos/grove-feature-auth'"
            ),
            CliErrorCode::WorkspaceAlreadyExists
        );
    }

    #[test]
    fn classifier_maps_runtime_and_adapter_errors() {
        assert_eq!(
            classify_error_message(
                "git merge --no-ff feature-a: CONFLICT (content): Merge conflict in src/a.rs"
            ),
            CliErrorCode::Conflict
        );
        assert_eq!(
            classify_error_message(
                "command failed: tmux new-session -d -s foo; duplicate session: foo"
            ),
            CliErrorCode::TmuxCommandFailed
        );
        assert_eq!(
            classify_error_message("git worktree remove failed: fatal: not a working tree"),
            CliErrorCode::GitCommandFailed
        );
        assert_eq!(
            classify_error_message("launcher script write failed: permission denied"),
            CliErrorCode::IoError
        );
    }

    #[test]
    fn classifier_precedence_prefers_repo_resolution_before_git_command_failure() {
        assert_eq!(
            classify_error_message("git command failed: fatal: not a git repository"),
            CliErrorCode::RepoNotFound
        );
    }

    #[test]
    fn classifier_precedence_prefers_conflict_before_generic_git_failure() {
        assert_eq!(
            classify_error_message(
                "git merge failed: Automatic merge failed; fix conflicts and then commit the result."
            ),
            CliErrorCode::Conflict
        );
    }

    #[test]
    fn classifier_falls_back_to_internal_for_unknown_errors() {
        assert_eq!(
            classify_error_message("unexpected launcher state without known signature"),
            CliErrorCode::Internal
        );
    }
}

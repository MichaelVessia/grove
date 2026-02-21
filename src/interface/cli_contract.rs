use serde::Serialize;

use crate::interface::cli_errors::CliErrorCode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextAction {
    pub command: String,
    pub description: String,
}

impl NextAction {
    pub fn new(command: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            description: description.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl ErrorDetail {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn from_code(code: CliErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuccessEnvelope<T: Serialize> {
    pub ok: bool,
    pub command: String,
    pub result: T,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub next_actions: Vec<NextAction>,
}

impl<T: Serialize> SuccessEnvelope<T> {
    pub fn new(
        command: impl Into<String>,
        result: T,
        warnings: Vec<String>,
        next_actions: Vec<NextAction>,
    ) -> Self {
        Self {
            ok: true,
            command: command.into(),
            result,
            warnings,
            next_actions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorEnvelope {
    pub ok: bool,
    pub command: String,
    pub error: ErrorDetail,
    pub fix: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub next_actions: Vec<NextAction>,
}

impl ErrorEnvelope {
    pub fn new(
        command: impl Into<String>,
        error: ErrorDetail,
        fix: impl Into<String>,
        warnings: Vec<String>,
        next_actions: Vec<NextAction>,
    ) -> Self {
        Self {
            ok: false,
            command: command.into(),
            error,
            fix: fix.into(),
            warnings,
            next_actions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum CommandEnvelope<T: Serialize> {
    Success(SuccessEnvelope<T>),
    Error(ErrorEnvelope),
}

impl<T: Serialize> CommandEnvelope<T> {
    pub fn success(
        command: impl Into<String>,
        result: T,
        warnings: Vec<String>,
        next_actions: Vec<NextAction>,
    ) -> Self {
        Self::Success(SuccessEnvelope::new(
            command,
            result,
            warnings,
            next_actions,
        ))
    }

    pub fn error(
        command: impl Into<String>,
        error: ErrorDetail,
        fix: impl Into<String>,
        warnings: Vec<String>,
        next_actions: Vec<NextAction>,
    ) -> Self {
        Self::Error(ErrorEnvelope::new(
            command,
            error,
            fix,
            warnings,
            next_actions,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandEnvelope, ErrorDetail, NextAction};
    use crate::interface::cli_errors::CliErrorCode;
    use serde::Serialize;
    use serde_json::json;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    struct WorkspaceResult {
        workspace: String,
    }

    #[test]
    fn success_envelope_serializes_without_warnings_when_empty() {
        let envelope = CommandEnvelope::success(
            "workspace list",
            WorkspaceResult {
                workspace: "feature/auth".to_string(),
            },
            Vec::new(),
            vec![NextAction::new(
                "workspace create --name fix",
                "Create another workspace",
            )],
        );

        let value = serde_json::to_value(envelope).expect("success envelope should serialize");

        assert_eq!(
            value,
            json!({
                "ok": true,
                "command": "workspace list",
                "result": {
                    "workspace": "feature/auth"
                },
                "next_actions": [{
                    "command": "workspace create --name fix",
                    "description": "Create another workspace"
                }]
            })
        );
    }

    #[test]
    fn success_envelope_serializes_warnings_when_present() {
        let envelope = CommandEnvelope::success(
            "workspace merge",
            WorkspaceResult {
                workspace: "feature/auth".to_string(),
            },
            vec!["cleanup skipped".to_string()],
            vec![NextAction::new(
                "workspace delete --workspace feature/auth",
                "Clean up workspace",
            )],
        );

        let value = serde_json::to_value(envelope).expect("success envelope should serialize");
        assert_eq!(
            value,
            json!({
                "ok": true,
                "command": "workspace merge",
                "result": {
                    "workspace": "feature/auth"
                },
                "warnings": ["cleanup skipped"],
                "next_actions": [{
                    "command": "workspace delete --workspace feature/auth",
                    "description": "Clean up workspace"
                }]
            })
        );
    }

    #[test]
    fn error_envelope_serializes_without_warnings_when_empty() {
        let envelope = CommandEnvelope::<WorkspaceResult>::error(
            "workspace delete",
            ErrorDetail::new("WORKSPACE_NOT_FOUND", "workspace was not found"),
            "Run workspace list and pick an existing workspace",
            Vec::new(),
            vec![NextAction::new(
                "workspace list",
                "Inspect current workspaces",
            )],
        );

        let value = serde_json::to_value(envelope).expect("error envelope should serialize");
        assert_eq!(
            value,
            json!({
                "ok": false,
                "command": "workspace delete",
                "error": {
                    "code": "WORKSPACE_NOT_FOUND",
                    "message": "workspace was not found"
                },
                "fix": "Run workspace list and pick an existing workspace",
                "next_actions": [{
                    "command": "workspace list",
                    "description": "Inspect current workspaces"
                }]
            })
        );
    }

    #[test]
    fn error_envelope_serializes_warnings_when_present() {
        let envelope = CommandEnvelope::<WorkspaceResult>::error(
            "workspace create --start",
            ErrorDetail::new("TMUX_COMMAND_FAILED", "tmux session start failed"),
            "Retry agent start after checking tmux server health",
            vec!["workspace created but agent start failed".to_string()],
            vec![
                NextAction::new("agent start --workspace feature/auth", "Retry launch"),
                NextAction::new(
                    "workspace delete --workspace feature/auth",
                    "Delete workspace if launch is no longer needed",
                ),
            ],
        );

        let value = serde_json::to_value(envelope).expect("error envelope should serialize");
        assert_eq!(
            value,
            json!({
                "ok": false,
                "command": "workspace create --start",
                "error": {
                    "code": "TMUX_COMMAND_FAILED",
                    "message": "tmux session start failed"
                },
                "fix": "Retry agent start after checking tmux server health",
                "warnings": ["workspace created but agent start failed"],
                "next_actions": [
                    {
                        "command": "agent start --workspace feature/auth",
                        "description": "Retry launch"
                    },
                    {
                        "command": "workspace delete --workspace feature/auth",
                        "description": "Delete workspace if launch is no longer needed"
                    }
                ]
            })
        );
    }

    #[test]
    fn error_detail_from_code_uses_stable_error_code_value() {
        let detail = ErrorDetail::from_code(CliErrorCode::InvalidArgument, "bad selector");
        assert_eq!(detail.code, "INVALID_ARGUMENT");
    }
}

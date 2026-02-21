use serde::Serialize;

use crate::interface::cli_contract::{CommandEnvelope, ErrorDetail, NextAction};
use crate::interface::cli_errors::CliErrorCode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DryRunStepTemplate {
    pub step_id: String,
    pub summary: String,
}

impl DryRunStepTemplate {
    pub fn new(step_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            summary: summary.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DryRunPlanStep {
    pub step_id: String,
    pub index: usize,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DryRunPlan {
    pub steps: Vec<DryRunPlanStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DryRunResult {
    pub dry_run: bool,
    pub plan: DryRunPlan,
    pub predicted_effects: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn indexed_steps(step_templates: &[DryRunStepTemplate]) -> Vec<DryRunPlanStep> {
    step_templates
        .iter()
        .enumerate()
        .map(|(index, step)| DryRunPlanStep {
            step_id: step.step_id.clone(),
            index,
            summary: step.summary.clone(),
        })
        .collect()
}

pub fn dry_run_success_envelope(
    command: impl Into<String>,
    step_templates: &[DryRunStepTemplate],
    predicted_effects: Vec<String>,
    warnings: Vec<String>,
    next_actions: Vec<NextAction>,
) -> CommandEnvelope<DryRunResult> {
    let steps = indexed_steps(step_templates);
    let result = DryRunResult {
        dry_run: true,
        plan: DryRunPlan { steps },
        predicted_effects,
        warnings,
    };
    CommandEnvelope::success(command, result, Vec::new(), next_actions)
}

pub fn dry_run_validation_failure_envelope(
    command: impl Into<String>,
    message: impl Into<String>,
    next_actions: Vec<NextAction>,
) -> CommandEnvelope<DryRunResult> {
    CommandEnvelope::error(
        command,
        ErrorDetail::from_code(CliErrorCode::InvalidArgument, message),
        "Correct invalid arguments and rerun with --dry-run",
        Vec::new(),
        next_actions,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DryRunStepTemplate, dry_run_success_envelope, dry_run_validation_failure_envelope,
        indexed_steps,
    };
    use crate::interface::cli_contract::NextAction;
    use serde_json::json;

    #[test]
    fn indexed_steps_assign_stable_step_ids_and_ordered_indices() {
        let steps = indexed_steps(&[
            DryRunStepTemplate::new("validate-inputs", "Validate command arguments"),
            DryRunStepTemplate::new("resolve-target", "Resolve workspace selector"),
            DryRunStepTemplate::new("merge-base", "Merge base branch into workspace"),
        ]);

        assert_eq!(steps[0].step_id, "validate-inputs");
        assert_eq!(steps[0].index, 0);
        assert_eq!(steps[1].step_id, "resolve-target");
        assert_eq!(steps[1].index, 1);
        assert_eq!(steps[2].step_id, "merge-base");
        assert_eq!(steps[2].index, 2);
    }

    #[test]
    fn dry_run_success_envelope_serializes_step_ids_indices_and_effects() {
        let envelope = dry_run_success_envelope(
            "workspace merge --dry-run",
            &[
                DryRunStepTemplate::new("validate-inputs", "Validate command arguments"),
                DryRunStepTemplate::new("resolve-target", "Resolve workspace selector"),
            ],
            vec![
                "would merge feature/auth into main".to_string(),
                "would leave workspace intact".to_string(),
            ],
            vec!["cleanup flags are disabled".to_string()],
            vec![NextAction::new(
                "workspace merge --workspace feature-auth",
                "Run merge without dry-run",
            )],
        );

        let value =
            serde_json::to_value(envelope).expect("dry-run success envelope should serialize");
        assert_eq!(
            value,
            json!({
                "ok": true,
                "command": "workspace merge --dry-run",
                "result": {
                    "dry_run": true,
                    "plan": {
                        "steps": [
                            {
                                "step_id": "validate-inputs",
                                "index": 0,
                                "summary": "Validate command arguments"
                            },
                            {
                                "step_id": "resolve-target",
                                "index": 1,
                                "summary": "Resolve workspace selector"
                            }
                        ]
                    },
                    "predicted_effects": [
                        "would merge feature/auth into main",
                        "would leave workspace intact"
                    ],
                    "warnings": ["cleanup flags are disabled"]
                },
                "next_actions": [{
                    "command": "workspace merge --workspace feature-auth",
                    "description": "Run merge without dry-run"
                }]
            })
        );
    }

    #[test]
    fn dry_run_validation_failure_uses_invalid_argument_code() {
        let envelope = dry_run_validation_failure_envelope(
            "workspace merge --dry-run",
            "workspace branch is required",
            vec![NextAction::new(
                "workspace list",
                "Inspect workspace names before retrying dry-run",
            )],
        );

        let value =
            serde_json::to_value(envelope).expect("dry-run validation failure should serialize");
        assert_eq!(
            value,
            json!({
                "ok": false,
                "command": "workspace merge --dry-run",
                "error": {
                    "code": "INVALID_ARGUMENT",
                    "message": "workspace branch is required"
                },
                "fix": "Correct invalid arguments and rerun with --dry-run",
                "next_actions": [{
                    "command": "workspace list",
                    "description": "Inspect workspace names before retrying dry-run"
                }]
            })
        );
    }
}

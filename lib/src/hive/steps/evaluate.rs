use async_trait::async_trait;
use tracing::{info_span, Instrument};

use crate::{
    hive::node::{Context, Derivation, ExecuteStep, Goal, StepOutput},
    nix::{get_eval_command, EvalGoal, StreamTracing},
    HiveLibError,
};

pub struct Output(pub Derivation);
pub struct Step;

#[async_trait]
impl ExecuteStep for Step {
    fn should_execute(&self, ctx: &Context) -> bool {
        !matches!(ctx.goal, Goal::Keys)
    }

    fn name(&self) -> &'static str {
        "Evaluate the node"
    }

    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let mut command = get_eval_command(
            &ctx.hivepath,
            &EvalGoal::GetTopLevel(ctx.name),
            ctx.modifiers,
        );

        let (status, stdout_vec, stderr) = command
            .execute(true)
            .instrument(info_span!("evaluate"))
            .await?;

        if status.success() {
            let stdout: Vec<String> = stdout_vec
                .into_iter()
                .map(|l| l.to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let derivation: Derivation =
                serde_json::from_str(&stdout.join("\n")).expect("failed to parse derivation");

            ctx.state.insert(StepOutput::Evaluation(Output(derivation)));

            return Ok(());
        }

        Err(HiveLibError::NixEvalInteralError(ctx.name.clone(), stderr))
    }
}

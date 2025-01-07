use std::fmt::Display;

use async_trait::async_trait;
use tracing::{instrument, warn};

use crate::{
    hive::node::{push, should_apply_locally, Context, ExecuteStep, Goal},
    HiveLibError,
};

pub struct EvaluatedOutputStep;
pub struct BuildOutputStep;

impl Display for EvaluatedOutputStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Push the evaluated output")
    }
}

impl Display for BuildOutputStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Push the build output")
    }
}

#[async_trait]
impl ExecuteStep for EvaluatedOutputStep {
    fn should_execute(&self, ctx: &Context) -> bool {
        !matches!(ctx.goal, Goal::Keys) && ctx.node.build_remotely
    }

    #[instrument(skip_all, name = "push_eval")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let top_level = ctx.state.get_evaluation().unwrap();

        push(
            ctx.node,
            ctx.name,
            crate::hive::node::Push::Derivation(&top_level.0),
        ).await.inspect_err(|_| {
                if should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string()) {
                    warn!("Remote push failed, but this node matches our local hostname ({0}). Perhaps you want to apply this node locally? Use `--always-build-local {0}` to override deployment.buildOnTarget", ctx.name.to_string());
                } else {
                    warn!("Use `--always-build-local {0}` to override deployment.buildOnTarget and force {0} to build locally", ctx.name.to_string());
                }
        })
    }
}

#[async_trait]
impl ExecuteStep for BuildOutputStep {
    fn should_execute(&self, ctx: &Context) -> bool {
        // skip if we are not building
        !matches!(ctx.goal, Goal::Keys | Goal::Push)
            && (
                // skip step if we are building remotely, or we are applying locally
                !ctx.node.build_remotely
                    || !should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.0)
            )
    }

    #[instrument(skip_all, name = "push_build")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let built_path = ctx.state.get_build().unwrap();

        push(
            ctx.node,
            ctx.name,
            crate::hive::node::Push::Path(&built_path.0),
        )
        .await
    }
}

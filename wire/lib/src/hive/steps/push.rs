// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::fmt::Display;

use tracing::{instrument, warn};

use crate::{
    HiveLibError,
    commands::common::push,
    hive::node::{Context, ExecuteStep, Goal, should_apply_locally},
};

#[derive(Debug, PartialEq)]
pub struct PushEvaluatedOutput;
#[derive(Debug, PartialEq)]
pub struct PushBuildOutput;

impl Display for PushEvaluatedOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Push the evaluated output")
    }
}

impl Display for PushBuildOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Push the build output")
    }
}

impl ExecuteStep for PushEvaluatedOutput {
    fn should_execute(&self, ctx: &Context) -> bool {
        !matches!(ctx.goal, Goal::Keys)
            && (ctx.node.build_remotely | matches!(ctx.goal, Goal::Push))
    }

    #[instrument(skip_all, name = "push_eval")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let top_level = ctx.state.evaluation.as_ref().unwrap();

        push(
            ctx.node,
            ctx.name,
            crate::hive::node::Push::Derivation(top_level),
            ctx.modifiers,
            ctx.clobber_lock.clone()
        ).await.inspect_err(|_| {
                if should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string()) {
                    warn!("Remote push failed, but this node matches our local hostname ({0}). Perhaps you want to apply this node locally? Use `--always-build-local {0}` to override deployment.buildOnTarget", ctx.name.to_string());
                } else {
                    warn!("Use `--always-build-local {0}` to override deployment.buildOnTarget and force {0} to build locally", ctx.name.to_string());
                }
        })
    }
}

impl ExecuteStep for PushBuildOutput {
    fn should_execute(&self, ctx: &Context) -> bool {
        if matches!(ctx.goal, Goal::Keys | Goal::Push) {
            // skip if we are not building
            return false;
        }

        if ctx.node.build_remotely {
            // skip if we are building remotely
            return false;
        }

        if should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.0) {
            // skip step if we are applying locally
            return false;
        }

        true
    }

    #[instrument(skip_all, name = "push_build")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let built_path = ctx.state.build.as_ref().unwrap();

        push(
            ctx.node,
            ctx.name,
            crate::hive::node::Push::Path(built_path),
            ctx.modifiers,
            ctx.clobber_lock.clone(),
        )
        .await
    }
}

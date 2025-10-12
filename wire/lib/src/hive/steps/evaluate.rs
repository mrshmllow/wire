// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::fmt::Display;

use tracing::instrument;

use crate::{
    EvalGoal, HiveLibError,
    commands::common::evaluate_hive_attribute,
    hive::node::{Context, ExecuteStep, Goal},
};

#[derive(Debug, PartialEq)]
pub struct Evaluate;

impl Display for Evaluate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Evaluate the node")
    }
}

impl ExecuteStep for Evaluate {
    fn should_execute(&self, ctx: &Context) -> bool {
        !matches!(ctx.goal, Goal::Keys)
    }

    #[instrument(skip_all, name = "eval")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let output = evaluate_hive_attribute(
            &ctx.hivepath,
            &EvalGoal::GetTopLevel(ctx.name),
            ctx.modifiers,
            ctx.clobber_lock.clone(),
        )
        .await?;

        ctx.state.evaluation = serde_json::from_str(&output).expect("failed to parse derivation");

        Ok(())
    }
}

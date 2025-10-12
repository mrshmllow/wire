// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::fmt::Display;

use tracing::{info, instrument, warn};

use crate::{
    HiveLibError,
    hive::node::{Context, ExecuteStep, should_apply_locally},
};

#[derive(Debug, PartialEq)]
pub struct Ping;

impl Display for Ping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ping node")
    }
}

impl ExecuteStep for Ping {
    fn should_execute(&self, ctx: &Context) -> bool {
        !should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string())
    }

    #[instrument(skip_all, name = "ping")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        loop {
            info!("Attempting host {}", ctx.node.target.get_preferred_host()?);

            if ctx.node.ping(ctx.clobber_lock.clone()).await.is_ok() {
                return Ok(());
            }

            warn!(
                "Failed to ping host {}",
                // ? will take us out if we ran out of hosts
                ctx.node.target.get_preferred_host()?
            );
            ctx.node.target.host_failed();
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::fmt::Display;

use tokio::process::Command;
use tracing::error;

use crate::{
    errors::HiveLibError,
    hive::node::{Context, ExecuteStep},
};

#[derive(PartialEq, Debug)]
pub(crate) struct CleanUp;

impl Display for CleanUp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Clean up")
    }
}

impl ExecuteStep for CleanUp {
    fn should_execute(&self, ctx: &Context) -> bool {
        !ctx.should_apply_locally
    }

    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let output = Command::new("ssh")
            .args(
                ctx.node
                    .target
                    .create_ssh_args(ctx.modifiers, true, false)?,
            )
            .args(["-O", "stop", ctx.node.target.get_preferred_host()?])
            .output()
            .await;

        // non failing error handling because the apply was successful until
        // this point.
        match output {
            Err(err) => {
                error!("failed to wind-down ControlMaster with `ssh -O stop`: {err}");
            }
            Ok(std::process::Output { status, stderr, .. }) if !status.success() => {
                error!(
                    "failed to wind-down ControlMaster with `ssh -O stop`: {}",
                    String::from_utf8_lossy(&stderr)
                );
            }
            Ok(_) => {}
        }

        Ok(())
    }
}

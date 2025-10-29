// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::fmt::Display;

use tokio::process::Command;
use tracing::debug;

use crate::{
    SubCommandModifiers,
    errors::HiveLibError,
    hive::node::{Context, ExecuteStep, Node},
};

#[derive(PartialEq, Debug)]
pub(crate) struct CleanUp;

impl Display for CleanUp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Clean up")
    }
}

pub(crate) async fn clean_up_control_master(
    node: &Node,
    modifiers: SubCommandModifiers,
) -> Result<(), HiveLibError> {
    let output = Command::new("ssh")
        .args(node.target.create_ssh_args(modifiers, true, false))
        .args(["-O", "stop", node.target.get_preferred_host()?])
        .output()
        .await;

    match output {
        Err(err) => {
            debug!("failed to wind-down ControlMaster with `ssh -O stop`: {err}");
        }
        Ok(std::process::Output { status, stderr, .. }) if !status.success() => {
            debug!(
                "failed to wind-down ControlMaster with `ssh -O stop`: {}",
                String::from_utf8_lossy(&stderr)
            );
        }
        Ok(_) => {}
    }

    Ok(())
}

impl ExecuteStep for CleanUp {
    fn should_execute(&self, ctx: &Context) -> bool {
        !ctx.should_apply_locally
    }

    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let _ = clean_up_control_master(ctx.node, ctx.modifiers).await;

        Ok(())
    }
}

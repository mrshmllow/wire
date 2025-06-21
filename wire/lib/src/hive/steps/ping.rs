use std::fmt::Display;

use async_trait::async_trait;
use tokio::process::Command;
use tracing::{Instrument, info, instrument, warn};

use crate::{
    HiveLibError,
    hive::node::{Context, ExecuteStep, should_apply_locally},
};

pub struct PingStep;

impl Display for PingStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ping node")
    }
}

#[async_trait]
impl ExecuteStep for PingStep {
    fn should_execute(&self, ctx: &Context) -> bool {
        !should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string())
    }

    #[instrument(skip_all, name = "ping")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        loop {
            info!("Attempting host {}", ctx.node.target.get_preffered_host()?);

            let mut command = Command::new("nix");

            command
                .args(["--extra-experimental-features", "nix-command"])
                .arg("store")
                .arg("ping")
                .arg("--store")
                .arg(format!(
                    "ssh://{}@{}",
                    ctx.node.target.user,
                    ctx.node.target.get_preffered_host()?
                ))
                .env("NIX_SSHOPTS", format!("-p {}", ctx.node.target.port));

            let (status, _stdout, _) = crate::nix::StreamTracing::execute(&mut command, true)
                .in_current_span()
                .await?;

            if status.success() {
                return Ok(());
            }

            warn!(
                "Failed to ping host {}",
                ctx.node.target.get_preffered_host()?
            );
            ctx.node.target.host_failed();
        }
    }
}

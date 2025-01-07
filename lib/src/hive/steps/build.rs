use std::fmt::Display;

use async_trait::async_trait;
use tokio::process::Command;
use tracing::{info, instrument, Instrument};

use crate::{
    create_ssh_command,
    hive::node::{Context, ExecuteStep, Goal},
    nix::StreamTracing,
    HiveLibError,
};

pub struct Step;

impl Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Build the node")
    }
}

#[async_trait]
impl ExecuteStep for Step {
    fn should_execute(&self, ctx: &Context) -> bool {
        !matches!(ctx.goal, Goal::Keys | Goal::Push)
    }

    #[instrument(skip_all, name = "build")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let top_level = ctx.state.evaluation.as_ref().unwrap();

        let mut command = if ctx.node.build_remotely {
            let mut command = create_ssh_command(&ctx.node.target, false);
            command.arg("nix");
            command
        } else {
            Command::new("nix")
        };

        command
            .args(["--extra-experimental-features", "nix-command"])
            .arg("build")
            .arg("--print-build-logs")
            .arg("--print-out-paths")
            .arg(&top_level.0);

        let (status, stdout, stderr_vec) = command.execute(true).in_current_span().await?;

        if status.success() {
            info!("Built output: {stdout:?}", stdout = stdout);

            let stdout = stdout
                .into_iter()
                .map(|l| l.to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
                .join("\n");

            ctx.state.build = Some(stdout);

            return Ok(());
        }

        let stderr: Vec<String> = stderr_vec
            .into_iter()
            .map(|l| l.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Err(HiveLibError::NixBuildError(ctx.name.clone(), stderr))
    }
}

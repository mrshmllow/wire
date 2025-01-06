use std::fmt::Display;

use async_trait::async_trait;
use tokio::process::Command;
use tracing::{info, instrument, warn, Instrument};
use tracing_indicatif::suspend_tracing_indicatif;

use crate::{
    create_ssh_command,
    hive::node::{should_apply_locally, Context, ExecuteStep, Goal, SwitchToConfigurationGoal},
    nix::StreamTracing,
    HiveLibError,
};

pub struct SwitchToConfigurationStep;

impl Display for SwitchToConfigurationStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Switch to configuration")
    }
}

#[async_trait]
impl ExecuteStep for SwitchToConfigurationStep {
    fn should_execute(&self, ctx: &Context) -> bool {
        matches!(ctx.goal, Goal::SwitchToConfiguration(..))
    }

    #[instrument(skip_all, name = "switch")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let built_path = ctx.state.get_build().unwrap();

        let Goal::SwitchToConfiguration(goal) = &ctx.goal else {
            unreachable!("Cannot reach as guarded by should_execute")
        };

        info!("Running switch-to-configuration {goal}");

        let cmd = format!("{}/bin/switch-to-configuration", built_path.0);

        let mut command =
            if should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string()) {
                // Refresh sudo timeout
                warn!(
                    "Running switch-to-configuration {goal:?} ON THIS MACHINE for node {0}",
                    ctx.name
                );
                info!("Attempting to elevate for local deployment.");
                suspend_tracing_indicatif(|| {
                    let mut command = std::process::Command::new("sudo");
                    command.arg("-v").output()
                })
                .map_err(HiveLibError::FailedToElevate)?;
                let mut command = Command::new("sudo");
                command.arg(cmd);
                command
            } else {
                let mut command = create_ssh_command(&ctx.node.target, true);
                command.arg(cmd);
                command
            };

        command.arg(match goal {
            SwitchToConfigurationGoal::Switch => "switch",
            SwitchToConfigurationGoal::Boot => "boot",
            SwitchToConfigurationGoal::Test => "test",
            SwitchToConfigurationGoal::DryActivate => "dry-activate",
        });

        let (status, _, stderr_vec) = command.execute(true).in_current_span().await?;

        if status.success() {
            info!("Done");

            return Ok(());
        }

        let stderr: Vec<String> = stderr_vec
            .into_iter()
            .map(|l| l.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Err(HiveLibError::SwitchToConfigurationError(
            *goal,
            ctx.name.clone(),
            stderr,
        ))
    }
}

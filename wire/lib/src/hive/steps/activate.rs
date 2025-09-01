use std::{fmt::Display, process::Output};

use async_trait::async_trait;
use tracing::{error, info, instrument, warn};
use tracing_indicatif::suspend_tracing_indicatif;

use crate::{
    HiveLibError,
    commands::{ChildOutputMode, WireCommand, WireCommandChip, elevated::ElevatedCommand},
    create_ssh_command,
    errors::{ActivationError, NetworkError},
    hive::node::{Context, ExecuteStep, Goal, SwitchToConfigurationGoal, should_apply_locally},
};

pub struct SwitchToConfigurationStep;

impl Display for SwitchToConfigurationStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Switch to configuration")
    }
}

pub(crate) fn get_elevation(reason: &str) -> Result<Output, ActivationError> {
    info!("Attempting to elevate for {reason}.");
    suspend_tracing_indicatif(|| {
        let mut command = std::process::Command::new("sudo");
        command.arg("-v").output()
    })
    .map_err(ActivationError::FailedToElevate)
}

pub async fn wait_for_ping(ctx: &Context<'_>) -> Result<(), HiveLibError> {
    for num in 0..3 {
        warn!(
            "Trying to ping {host} (attempt {num}/3)",
            host = ctx.node.target.get_preffered_host()?
        );

        if ctx.node.ping().await.is_ok() {
            info!(
                "Regained connection to {name} via {host}",
                name = ctx.name,
                host = ctx.node.target.get_preffered_host()?
            );

            return Ok(());
        }
    }

    Err(HiveLibError::NetworkError(NetworkError::HostUnreachable(
        ctx.node.target.get_preffered_host()?.to_string(),
    )))
}

#[async_trait]
impl ExecuteStep for SwitchToConfigurationStep {
    fn should_execute(&self, ctx: &Context) -> bool {
        matches!(ctx.goal, Goal::SwitchToConfiguration(..))
    }

    #[instrument(skip_all, name = "switch")]
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError> {
        let built_path = ctx.state.build.as_ref().unwrap();

        let Goal::SwitchToConfiguration(goal) = &ctx.goal else {
            unreachable!("Cannot reach as guarded by should_execute")
        };

        if !matches!(
            goal,
            SwitchToConfigurationGoal::DryActivate | SwitchToConfigurationGoal::Boot
        ) {
            info!("Setting profiles in anticipation for switch-to-configuration {goal}");

            let mut command =
                ElevatedCommand::spawn_new(&ctx.node.target, ChildOutputMode::Nix).await?;
            let command_string =
                format!("nix-env -p /nix/var/nix/profiles/system/ --set {built_path}");

            let child = suspend_tracing_indicatif(|| {
                command.run_command(
                    command_string,
                    false,
                    should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string()),
                )
            })?;

            let _ = child
                .wait_till_success()
                .await
                .map_err(HiveLibError::DetachedError)?;

            info!("Set system profile");
        }

        info!("Running switch-to-configuration {goal}");

        let mut command =
            ElevatedCommand::spawn_new(&ctx.node.target, ChildOutputMode::Nix).await?;

        let command_string = format!(
            "{built_path}/bin/switch-to-configuration {}",
            match goal {
                SwitchToConfigurationGoal::Switch => "switch",
                SwitchToConfigurationGoal::Boot => "boot",
                SwitchToConfigurationGoal::Test => "test",
                SwitchToConfigurationGoal::DryActivate => "dry-activate",
            }
        );

        let child = suspend_tracing_indicatif(|| {
            command.run_command(
                command_string,
                false,
                should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string()),
            )
        })?;

        let result = child.wait_till_success().await;

        match result {
            Ok(_) => {
                if !ctx.reboot {
                    return Ok(());
                }

                if should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string()) {
                    error!("Refusing to reboot local machine!");

                    return Ok(());
                }

                let mut command =
                    ElevatedCommand::spawn_new(&ctx.node.target, ChildOutputMode::Nix).await?;

                warn!("Rebooting {name}!", name = ctx.name);

                let command_string = format!("reboot now");

                let reboot = suspend_tracing_indicatif(|| {
                    command.run_command(command_string, false, false)
                })?;

                // consume result, impossible to know if the machine failed to reboot or we
                // simply disconnected
                let _ = reboot
                    .wait_till_success()
                    .await
                    .map_err(HiveLibError::DetachedError)?;

                info!("Rebooted {name}, waiting to reconnect...", name = ctx.name);

                if wait_for_ping(ctx).await.is_ok() {
                    return Ok(());
                }

                error!(
                    "Failed to get regain connection to {name} via {host} after reboot.",
                    name = ctx.name,
                    host = ctx.node.target.get_preffered_host()?
                );

                return Err(HiveLibError::NetworkError(
                    NetworkError::HostUnreachableAfterReboot(
                        ctx.node.target.get_preffered_host()?.to_string(),
                    ),
                ));
            }
            Err(error) => {
                warn!(
                    "Activation command for {name} exited unsuccessfully.",
                    name = ctx.name
                );

                // Bail if the command couldn't of broken the system
                // and don't try to regain connection to localhost
                if matches!(goal, SwitchToConfigurationGoal::DryActivate)
                    || should_apply_locally(ctx.node.allow_local_deployment, &ctx.name.to_string())
                {
                    return Err(HiveLibError::ActivationError(
                        ActivationError::SwitchToConfigurationError(*goal, ctx.name.clone(), error),
                    ));
                }

                if wait_for_ping(ctx).await.is_ok() {
                    return Ok(());
                }

                error!(
                    "Failed to get regain connection to {name} via {host} after {goal} activation.",
                    name = ctx.name,
                    host = ctx.node.target.get_preffered_host()?
                );

                return Err(HiveLibError::NetworkError(
                    NetworkError::HostUnreachableAfterReboot(
                        ctx.node.target.get_preffered_host()?.to_string(),
                    ),
                ));
            }
        }
    }
}

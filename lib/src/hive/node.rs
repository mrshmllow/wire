use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tracing::instrument;
use tracing::{info, info_span, Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::create_ssh_command;
use crate::nix::{get_eval_command, EvalGoal, StreamTracing};
use crate::SubCommandModifiers;

use super::key::{Key, PushKeys, UploadKeyAt};
use super::HiveLibError;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, derive_more::Display)]
pub struct Name(pub Arc<str>);

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Target {
    #[serde(rename = "host")]
    pub host: Arc<str>,

    #[serde(rename = "user")]
    pub user: Arc<str>,

    #[serde(rename = "port")]
    pub port: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Node {
    #[serde(rename = "target")]
    pub target: Target,

    #[serde(rename = "buildOnTarget")]
    pub build_remotely: bool,

    #[serde(default)]
    pub tags: im::HashSet<String>,

    #[serde(rename(deserialize = "_keys", serialize = "keys"))]
    pub keys: im::Vector<Key>,
}

#[derive(derive_more::Display)]
pub enum Push<'a> {
    Derivation(&'a Derivation),
    Path(&'a String),
}

pub trait Evaluatable {
    fn evaluate(
        self,
        hivepath: PathBuf,
        modifiers: SubCommandModifiers,
    ) -> impl std::future::Future<Output = Result<Derivation, HiveLibError>> + Send;

    fn build(
        self,
        hivepath: PathBuf,
        span: &Span,
        modifiers: SubCommandModifiers,
    ) -> impl std::future::Future<Output = Result<String, HiveLibError>> + Send;

    fn achieve_goal(
        self,
        hivepath: PathBuf,
        span: Span,
        goal: &Goal,
        no_keys: bool,
        modifiers: SubCommandModifiers,
    ) -> impl std::future::Future<Output = Result<(), HiveLibError>> + Send;

    fn switch_to_configuration(
        self,
        hivepath: PathBuf,
        span: &Span,
        goal: &SwitchToConfigurationGoal,
        modifiers: SubCommandModifiers,
    ) -> impl std::future::Future<Output = Result<(), HiveLibError>> + Send;

    fn eval_and_push(
        self,
        hivepath: PathBuf,
        span: &Span,
        modifiers: SubCommandModifiers,
    ) -> impl std::future::Future<Output = Result<(), HiveLibError>> + Send;

    fn push(
        self,
        span: &Span,
        push: Push<'_>,
    ) -> impl std::future::Future<Output = Result<(), HiveLibError>> + Send;
}

#[derive(Deserialize, Debug)]
pub struct Derivation(pub String);

impl Display for Derivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f).and_then(|()| write!(f, "^*"))
    }
}

#[derive(derive_more::Display, Debug)]
pub enum SwitchToConfigurationGoal {
    Switch,
    Boot,
    Test,
    DryActivate,
}

#[derive(derive_more::Display)]
pub enum Goal {
    SwitchToConfiguration(SwitchToConfigurationGoal),
    Build,
    Push,
    Keys,
}

impl Evaluatable for (&Name, &Node) {
    /// Evaluate the node and returns the top level Deriviation
    async fn evaluate(
        self,
        hivepath: PathBuf,
        modifiers: SubCommandModifiers,
    ) -> Result<Derivation, HiveLibError> {
        let mut command = get_eval_command(&hivepath, &EvalGoal::GetTopLevel(self.0), modifiers);

        let (status, stdout_vec, stderr) = command
            .execute(true)
            .instrument(info_span!("evaluate"))
            .await?;

        if status.success() {
            let stdout: Vec<String> = stdout_vec
                .into_iter()
                .map(|l| l.to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let derivation: Derivation =
                serde_json::from_str(&stdout.join("\n")).expect("failed to parse derivation");

            return Ok(derivation);
        }

        Err(HiveLibError::NixEvalInteralError(self.0.clone(), stderr))
    }

    /// Pushes a path or derivation to the node
    #[instrument(skip_all)]
    async fn push(self, span: &Span, push: Push<'_>) -> Result<(), HiveLibError> {
        span.pb_inc_length(1);
        let mut command = Command::new("nix");

        command
            .args(["--extra-experimental-features", "nix-command"])
            .arg("copy")
            .arg("--substitute-on-destination")
            .arg("--to")
            .arg(format!(
                "ssh://{}@{}",
                self.1.target.user, self.1.target.host
            ))
            .env("NIX_SSHOPTS", format!("-p {}", self.1.target.port));

        match push {
            Push::Derivation(drv) => command.args([drv.to_string(), "--derivation".to_string()]),
            Push::Path(path) => command.arg(path),
        };

        let (status, _stdout, stderr_vec) =
            command.execute(true).instrument(info_span!("copy")).await?;

        span.pb_inc(1);

        if !status.success() {
            return Err(HiveLibError::NixCopyError(self.0.clone(), stderr_vec));
        }

        Ok(())
    }

    /// Builds the evaluated node remotely or locally. Pushes the derivation / the build output as required.
    #[instrument(skip_all)]
    async fn build(
        self,
        hivepath: PathBuf,
        span: &Span,
        modifiers: SubCommandModifiers,
    ) -> Result<String, HiveLibError> {
        span.pb_inc_length(2);
        let top_level = self.evaluate(hivepath, modifiers).await?;
        span.pb_inc(1);

        info!("Top level: {top_level}");

        let mut command = if self.1.build_remotely {
            self.push(span, Push::Derivation(&top_level)).await?;

            let mut command = create_ssh_command(&self.1.target, false);
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
            .arg(top_level.to_string());

        let (status, stdout, stderr_vec) = command.execute(true).in_current_span().await?;

        span.pb_inc(1);

        if status.success() {
            info!("Built output: {stdout:?}", stdout = stdout);

            let stdout = stdout
                .into_iter()
                .map(|l| l.to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
                .join("\n");

            if !self.1.build_remotely {
                self.push(span, Push::Path(&stdout)).await?;
            };

            return Ok(stdout);
        }

        let stderr: Vec<String> = stderr_vec
            .into_iter()
            .map(|l| l.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Err(HiveLibError::NixBuildError(self.0.clone(), stderr))
    }

    #[instrument(skip_all)]
    async fn eval_and_push(
        self,
        hivepath: PathBuf,
        span: &Span,
        modifiers: SubCommandModifiers,
    ) -> Result<(), HiveLibError> {
        span.pb_inc_length(1);
        let top_level = self.evaluate(hivepath, modifiers).await?;
        span.pb_inc(1);

        self.push(span, Push::Derivation(&top_level)).await?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn switch_to_configuration(
        self,
        hivepath: PathBuf,
        span: &Span,
        goal: &SwitchToConfigurationGoal,
        modifiers: SubCommandModifiers,
    ) -> Result<(), HiveLibError> {
        let built_path = self.build(hivepath, span, modifiers).await?;

        span.pb_inc_length(1);

        info!("Running switch-to-configuration {goal:?}");

        let cmd = format!("{built_path}/bin/switch-to-configuration");
        let mut command = create_ssh_command(&self.1.target, true);

        command.arg(cmd).arg(match goal {
            SwitchToConfigurationGoal::Switch => "switch",
            SwitchToConfigurationGoal::Boot => "boot",
            SwitchToConfigurationGoal::Test => "test",
            SwitchToConfigurationGoal::DryActivate => "dry-activate",
        });

        let (status, _, stderr_vec) = command.execute(true).in_current_span().await?;

        span.pb_inc(1);

        if status.success() {
            info!("Done");

            return Ok(());
        }

        let stderr: Vec<String> = stderr_vec
            .into_iter()
            .map(|l| l.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Err(HiveLibError::NixBuildError(self.0.clone(), stderr))
    }

    #[instrument(skip_all)]
    async fn achieve_goal(
        self,
        hivepath: PathBuf,
        span: Span,
        goal: &Goal,
        no_keys: bool,
        modifiers: SubCommandModifiers,
    ) -> Result<(), HiveLibError> {
        match goal {
            Goal::SwitchToConfiguration(goal) => {
                if let SwitchToConfigurationGoal::Switch = goal
                    && !no_keys
                {
                    self.push_keys(UploadKeyAt::PreActivation, &span).await?;
                }

                self.switch_to_configuration(hivepath, &span, goal, modifiers)
                    .await?;

                if let SwitchToConfigurationGoal::Switch = goal
                    && !no_keys
                {
                    self.push_keys(UploadKeyAt::PostActivation, &span).await?;
                }

                Ok(())
            }
            Goal::Build => {
                self.build(hivepath, &span, modifiers).await?;

                Ok(())
            }
            Goal::Push => self.eval_and_push(hivepath, &span, modifiers).await,
            Goal::Keys => self.push_keys(UploadKeyAt::All, &span).await,
        }
    }
}

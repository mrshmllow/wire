use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tracing::instrument;
use tracing::{info, info_span, Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::nix::{get_eval_command, EvalGoal, StreamTracing};

use super::HiveLibError;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, derive_more::Display)]
pub struct NodeName(pub Arc<str>);

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
}

pub trait Evaluatable {
    fn evaluate(
        self,
        hivepath: PathBuf,
    ) -> impl std::future::Future<Output = Result<Derivation, HiveLibError>> + Send;

    fn build(
        self,
        hivepath: PathBuf,
        span: &Span,
    ) -> impl std::future::Future<Output = Result<String, HiveLibError>> + Send;

    fn switch_to_configuration(
        self,
        hivepath: PathBuf,
        span: Span,
    ) -> impl std::future::Future<Output = Result<(), HiveLibError>> + Send;
}

#[derive(Deserialize, Debug)]
pub struct Derivation(pub String);

impl Display for Derivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f).and_then(|_| write!(f, "^*"))
    }
}

impl Evaluatable for (&NodeName, &Node) {
    async fn evaluate(self, hivepath: PathBuf) -> Result<Derivation, HiveLibError> {
        let mut command = get_eval_command(hivepath, EvalGoal::GetTopLevel(self.0));

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

    #[instrument(skip_all)]
    async fn build(self, hivepath: PathBuf, span: &Span) -> Result<String, HiveLibError> {
        span.pb_inc_length(1);
        let top_level = self.evaluate(hivepath).await?;
        span.pb_inc(1);

        info!("Top level: {top_level}");

        if self.1.build_remotely {
            span.pb_inc_length(1);
            let mut command = Command::new("nix");

            command
                .arg("copy")
                .arg("--substitute-on-destination")
                .arg("--derivation")
                .arg("--to")
                .arg(format!(
                    "ssh://{}@{}",
                    self.1.target.user, self.1.target.host
                ))
                .arg(top_level.to_string());

            let (status, _stdout, stderr_vec) =
                command.execute(true).instrument(info_span!("copy")).await?;

            span.pb_inc(1);

            if !status.success() {
                return Err(HiveLibError::NixCopyError(self.0.clone(), stderr_vec));
            }
        }

        let mut command = match self.1.build_remotely {
            true => {
                let mut command = Command::new("ssh");

                command
                    .arg("-l")
                    .arg(self.1.target.user.as_ref())
                    .arg(self.1.target.host.as_ref())
                    .args(["sudo", "-H", "--"])
                    .arg("nix")
                    .arg("--extra-experimental-features")
                    .arg("nix-command");

                command
            }
            false => Command::new("nix"),
        };

        command
            .arg("build")
            .arg("--verbose")
            .arg("--print-build-logs")
            .arg("--print-out-paths")
            .arg(top_level.to_string());

        let (status, stdout, stderr_vec) = command.execute(true).in_current_span().await?;

        span.pb_inc(1);

        if status.success() {
            info!("Built output: {stdout:?}", stdout = stdout);

            let stdout: Vec<String> = stdout
                .into_iter()
                .map(|l| l.to_string())
                .filter(|s| !s.is_empty())
                .collect();

            return Ok(stdout.join("\n"));
        }

        let stderr: Vec<String> = stderr_vec
            .into_iter()
            .map(|l| l.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Err(HiveLibError::NixBuildError(self.0.clone(), stderr))
    }

    #[instrument(skip_all)]
    async fn switch_to_configuration(
        self,
        hivepath: PathBuf,
        span: Span,
    ) -> Result<(), HiveLibError> {
        span.pb_inc_length(2);
        let built_path = self.build(hivepath, &span).await?;
        span.pb_inc(1);

        let cmd = format!("{built_path}/bin/switch-to-configuration");
        let mut command = Command::new("ssh");

        command
            .arg("-l")
            .arg(self.1.target.user.as_ref())
            .arg(self.1.target.host.as_ref())
            .args(["sudo", "-H", "--"])
            .arg(cmd);

        command.arg("switch");

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
}

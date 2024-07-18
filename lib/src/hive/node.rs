use serde::{Deserialize, Serialize};
use std::borrow::BorrowMut;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info, info_span, instrument, Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::nix::{get_eval_command, CommandTracer, EvalGoal, StreamTracing};

use super::HiveLibError;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, derive_more::Display)]
pub struct NodeName(pub Arc<str>);

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Node {
    #[serde(rename = "targetHosts")]
    pub target_hosts: im::HashSet<String>,

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
        let mut command = get_eval_command(hivepath, EvalGoal::GetTopLevel(&self.0));
        let mut stream: CommandTracer = command.borrow_mut().into();

        stream.log_stderr(true);

        let (status, stdout_vec, stderr) =
            stream.execute().instrument(info_span!("evaluate")).await?;

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

    #[instrument(skip(self, span, hivepath), fields(node = %self.0))]
    async fn build(self, hivepath: PathBuf, span: Span) -> Result<(), HiveLibError> {
        let top_level = self.evaluate(hivepath).await?;
        span.pb_inc(1);

        let mut command = Command::new("nix");

        command
            .arg("build")
            .arg("--verbose")
            .arg("--print-build-logs")
            .arg("--print-out-paths")
            .arg(top_level.to_string());

        let mut stream: CommandTracer = command.borrow_mut().into();

        stream.log_stderr(true);

        let (status, stdout, stderr_vec) = stream.execute().in_current_span().await?;

        info!("Built output: {stdout:?}", stdout = stdout);

        span.pb_inc(1);

        if status.success() {
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

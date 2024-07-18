use serde::{Deserialize, Serialize};
use std::borrow::BorrowMut;
use std::path::PathBuf;
use std::{collections::HashSet, fmt::Display};
use tokio::process::Command;
use tracing::{info, info_span, instrument, Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::nix::{get_eval_command, CommandTracer, EvalGoal, StreamTracing};

use super::HiveLibError;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    #[serde(rename = "targetHosts")]
    pub target_hosts: HashSet<String>,

    #[serde(default)]
    pub tags: HashSet<String>,
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
    ) -> impl std::future::Future<Output = Result<(String, Node), HiveLibError>> + Send;
}

#[derive(Deserialize, Debug)]
pub struct Derivation(pub String);

impl Display for Derivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f).and_then(|_| write!(f, "^*"))
    }
}

impl Evaluatable for (String, Node) {
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

        Err(HiveLibError::NixEvalInteralError(stderr))
    }

    #[instrument(skip(self, span, hivepath), fields(node = %self.0))]
    async fn build(self, hivepath: PathBuf, span: Span) -> Result<(String, Node), HiveLibError> {
        let top_level = self.clone().evaluate(hivepath).await?;
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

        if status.success() {
            return Ok(self);
        }

        let stderr: Vec<String> = stderr_vec
            .into_iter()
            .map(|l| l.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Err(HiveLibError::NixBuildError(top_level, stderr))
    }
}

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::{collections::HashSet, fmt::Display};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{info, info_span, instrument, Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::nix::{get_eval_command, EvalGoal};

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
        let mut command = get_eval_command(hivepath, EvalGoal::GetTopLevel(&self.0))
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(HiveLibError::NixExecError)?;

        let stderr_handle = command
            .stderr
            .take()
            .expect("child did not have a handle to stderr");

        let stdout_handle = command
            .stdout
            .take()
            .expect("child did not have a handle to stdout");

        let stdout_task = tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(stdout_handle).lines();
            let mut collect = String::new();

            while let Some(line) = stdout_reader
                .next_line()
                .await
                .expect("failed to read line")
            {
                collect.push_str(&line);
                collect.push('\n');
            }

            collect
        });

        let stderr_task = tokio::spawn(
            async move {
                let mut stderr_reader = BufReader::new(stderr_handle).lines();
                let mut collect = String::new();

                while let Some(line) = stderr_reader
                    .next_line()
                    .await
                    .expect("failed to read stderr line")
                {
                    info!("{line}");

                    collect.push_str(&line);
                    collect.push('\n');
                }

                collect
            }
            .instrument(info_span!("evaluate", node = %self.0)),
        );

        let handle = tokio::spawn(async move { command.wait().await.expect("failed to wait") });
        let stdout = stdout_task.await.expect("failed to wait for stdout handle");
        let stderr = stderr_task.await.expect("failed to wait for stderr handle");

        if handle.await.unwrap().success() {
            let derivation: Derivation =
                serde_json::from_str(&stdout).expect("failed to parse derivation");

            return Ok(derivation);
        }

        Err(HiveLibError::NixEvalError(stderr))
    }

    #[instrument(skip(self, span, hivepath), fields(node = %self.0))]
    async fn build(self, hivepath: PathBuf, span: Span) -> Result<(String, Node), HiveLibError> {
        let top_level = self.clone().evaluate(hivepath).await?;
        span.pb_inc(1);

        let mut command = Command::new("nix")
            .arg("build")
            .arg("--verbose")
            .arg("--print-build-logs")
            .arg(top_level.to_string())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(HiveLibError::NixExecError)?;

        let stderr_handle = command
            .stderr
            .take()
            .expect("child did not have a handle to stderr");

        let stderr_task = tokio::spawn(
            async move {
                let mut stderr_reader = BufReader::new(stderr_handle).lines();
                let mut collect = String::new();

                while let Some(line) = stderr_reader
                    .next_line()
                    .await
                    .expect("failed to read stderr line")
                {
                    info!("{line}");

                    collect.push_str(&line);
                    collect.push('\n');
                }

                collect
            }
            .instrument(info_span!("build")),
        );

        let handle = tokio::spawn(async move { command.wait().await.expect("failed to wait") });
        let stderr = stderr_task.await.expect("failed to wait for stderr handle");

        if handle.await.unwrap().success() {
            return Ok(self);
        }

        Err(HiveLibError::NixBuildError(top_level, stderr))
    }
}

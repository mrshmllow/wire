#![feature(async_closure)]
use hive::node::NodeName;
use nix_log::{NixLog, Trace};
use std::path::PathBuf;
use thiserror::Error;
use tokio::task::JoinError;

pub mod hive;
mod nix;
mod nix_log;

#[derive(Debug, Error)]
pub enum HiveLibError {
    #[error("no hive could be found in {}", .0.display())]
    NoHiveFound(PathBuf),

    #[error("failed to execute nix command")]
    NixExecError(#[source] tokio::io::Error),

    #[error("failed to evaluate nix expression (last 20 lines):\n{}", .0[.0.len() - 20..].join("\n"))]
    NixEvalError(Vec<String>),

    #[error("failed to evaluate node {0} (filtered logs, run with -vvv to see all):\n{}", .1.iter().filter(|l| l.is_error()).map(|l| l.to_string()).collect::<Vec<String>>().join("\n"))]
    NixEvalInteralError(NodeName, Vec<NixLog>),

    #[error("failed to copy drv to node {0} (filtered logs, run with -vvv to see all):\n{}", .1.iter().filter(|l| l.is_error()).map(|l| l.to_string()).collect::<Vec<String>>().join("\n"))]
    NixCopyError(NodeName, Vec<NixLog>),

    #[error("failed to build node {0} (last 20 lines):\n{}", .1[.1.len() - 20..].join("\n"))]
    NixBuildError(NodeName, Vec<String>),

    #[error("node {0} not exist in hive")]
    NodeDoesNotExist(String),

    #[error("failed to execute command")]
    SpawnFailed(#[source] tokio::io::Error),

    #[error("failed to join task")]
    JoinError(#[source] JoinError),

    #[error("there was no handle to io on the child process")]
    NoHandle,

    #[error("failed to parse nix log \"{0}\"")]
    ParseLogError(String, #[source] serde_json::Error),
}

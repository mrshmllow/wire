#![feature(let_chains)]
#![deny(clippy::pedantic)]
use hive::{key::Error, node::Name};
use nix_log::{NixLog, Trace};
use std::path::PathBuf;
use thiserror::Error;
use tokio::task::JoinError;

pub mod hive;
mod nix;
mod nix_log;

fn format_error_lines(lines: &[String]) -> String {
    lines
        .iter()
        .rev()
        .take(20)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Error)]
pub enum HiveLibError {
    #[error("no hive could be found in {}", .0.display())]
    NoHiveFound(PathBuf),

    #[error("failed to execute nix command")]
    NixExecError(#[source] tokio::io::Error),

    #[error("failed to evaluate nix expression (last 20 lines):\n{}", format_error_lines(.0))]
    NixEvalError(Vec<String>),

    #[error("failed to evaluate node {0} (filtered logs, run with -vvv to see all):\n{}", .1.iter().filter(|l| l.is_error()).map(|l| l.to_string()).collect::<Vec<String>>().join("\n"))]
    NixEvalInteralError(Name, Vec<NixLog>),

    #[error("failed to copy drv to node {0} (filtered logs, run with -vvv to see all):\n{}", .1.iter().filter(|l| l.is_error()).map(|l| l.to_string()).collect::<Vec<String>>().join("\n"))]
    NixCopyError(Name, Vec<NixLog>),

    #[error("failed to build node {0} (last 20 lines):\n{}", format_error_lines(.1))]
    NixBuildError(Name, Vec<String>),

    #[error("failed to push keys to {0} (last 20 lines):\n{}", format_error_lines(.1))]
    KeyCommandError(Name, Vec<String>),

    #[error("failed to push a key")]
    KeyError(#[source] Error),

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

    #[error("an operation failed in regards to buffers")]
    BufferOperationError(#[source] tokio::io::Error),
}

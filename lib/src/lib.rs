#![feature(async_closure)]
use hive::node::Derivation;
use std::path::PathBuf;
use thiserror::Error;
use tokio::task::JoinError;

pub mod hive;
mod nix;

#[derive(Debug, Error)]
pub enum HiveLibError {
    #[error("no hive could be found in {}", .0.display())]
    NoHiveFound(PathBuf),

    #[error("failed to execute nix command")]
    NixExecError(#[source] tokio::io::Error),

    #[error("failed to evaluate nix expression (last 20 lines):\n{}", .0[.0.len() - 20..].join("\n"))]
    NixEvalError(Vec<String>),

    #[error("failed to build deriviation {0} (last 20 lines):\n{}", .1[.1.len() - 20..].join("\n"))]
    NixBuildError(Derivation, Vec<String>),

    #[error("node {0} not exist in hive")]
    NodeDoesNotExist(String),

    #[error("failed to execute command")]
    SpawnFailed(#[source] tokio::io::Error),

    #[error("failed to join task")]
    JoinError(#[source] JoinError),

    #[error("there was no handle to io on the child process")]
    NoHandle,
}

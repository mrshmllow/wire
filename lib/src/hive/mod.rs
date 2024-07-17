use node::{Derivation, Node};
use std::collections::hash_map::OccupiedEntry;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::process::Command;
use tracing::{debug, error, info, instrument, trace};

use serde::{Deserialize, Serialize};

use crate::nix::{get_eval_command, EvalGoal};
pub mod node;

#[derive(Serialize, Deserialize, Debug)]
pub struct Hive {
    pub nodes: HashMap<String, Node>,
    pub path: PathBuf,
}

#[derive(Debug, Error)]
pub enum HiveLibError {
    #[error("no hive could be found in {}", .0.display())]
    NoHiveFound(PathBuf),

    #[error("failed to execute nix command")]
    NixExecError(#[source] tokio::io::Error),

    #[error("failed to evaluate nix expression: {0}")]
    NixEvalError(String),

    #[error("failed to evaluate nix build deriviation {0}: {1}")]
    NixBuildError(Derivation, String),
}

pub enum HiveAction<'a> {
    Inspect,
    EvaluateNode(OccupiedEntry<'a, String, Node>),
}

pub trait HiveBuilder {
    fn new_from_path(
        path: &Path,
    ) -> impl std::future::Future<Output = Result<Hive, HiveLibError>> + Send;
}

impl HiveBuilder for Hive {
    #[instrument]
    async fn new_from_path(path: &Path) -> Result<Hive, HiveLibError> {
        info!("Searching upwards for hive in {}", path.display());
        let filepath = find_hive(path).ok_or(HiveLibError::NoHiveFound(path.to_path_buf()))?;
        info!("Using hive {}", filepath.display());

        let command = get_eval_command(filepath, EvalGoal::Inspect)
            .output()
            .await
            .map_err(HiveLibError::NixExecError)?;

        let stdout = String::from_utf8_lossy(&command.stdout);
        let stderr = String::from_utf8_lossy(&command.stderr);

        debug!("Output of nix eval: {stdout}");

        if command.status.success() {
            let hive: Hive = serde_json::from_str(&stdout).unwrap();

            return Ok(hive);
        }

        Err(HiveLibError::NixEvalError(stderr.to_string()))
    }
}

fn find_hive(path: &Path) -> Option<PathBuf> {
    trace!("Searching for hive in {}", path.display());
    let filepath = path.join("hive.nix");

    if filepath.is_file() {
        return Some(filepath);
    }

    if let Some(parent) = path.parent() {
        return find_hive(parent);
    }

    error!("No hive found");
    None
}

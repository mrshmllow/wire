use node::{Node, NodeName};
use std::collections::hash_map::OccupiedEntry;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, instrument, trace};

use serde::{Deserialize, Serialize};

use crate::nix::{get_eval_command, EvalGoal};
use crate::HiveLibError;
pub mod node;

#[derive(Serialize, Deserialize, Debug)]
pub struct Hive {
    pub nodes: im::HashMap<NodeName, Node>,
    pub path: PathBuf,
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

        Err(HiveLibError::NixEvalError(
            stderr.split("\n").map(|s| s.to_string()).collect(),
        ))
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

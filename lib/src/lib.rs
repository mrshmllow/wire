#![feature(error_generic_member_access)]

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
};
use thiserror::Error;
use tokio::{fs::read_to_string, io::BufReader};
use tokio::{io::AsyncBufReadExt, process::Command};
use tracing::{debug, error, info, instrument, trace};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub target_hosts: HashSet<String>,

    #[serde(default)]
    pub tags: HashSet<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Hive {
    #[serde(flatten)]
    pub hosts: HashMap<String, Node>,
}

#[derive(Debug, Error)]
pub enum HiveLibError {
    #[error("no hive could be found in {}", .0.display())]
    NoHiveFound(PathBuf),

    #[error("failed to execute nix command")]
    NixExecError(#[source] tokio::io::Error),
}

pub trait HiveBuilder {
    fn new_from_path(
        path: &Path,
    ) -> impl std::future::Future<Output = Result<Hive, HiveLibError>> + Send;

    // fn build_node_locally(
    //     &self,
    // ) -> impl std::future::Future<Output = Result<(), HiveLibError>> + Send;
}

impl HiveBuilder for Hive {
    #[instrument]
    async fn new_from_path(path: &Path) -> Result<Hive, HiveLibError> {
        info!("Searching upwards for hive in {}", path.display());
        let filepath = find_hive(path).ok_or(HiveLibError::NoHiveFound(path.to_path_buf()))?;
        info!("Using hive {}", filepath.display());

        let hive_string = read_to_string(filepath).await.unwrap();

        let command = Command::new("nix")
            .arg("eval")
            .arg("--json")
            .arg("--impure")
            .arg("-E")
            .arg(hive_string)
            .output()
            .await
            .map_err(HiveLibError::NixExecError)?;

        let output = String::from_utf8_lossy(&command.stdout);
        let hive: Hive = serde_json::from_str(&output).unwrap();

        Ok(hive)
    }
}

#[instrument]
pub async fn eval_node(path: &Path) -> Result<(), HiveLibError> {
    info!("Searching upwards for hive in {}", path.display());
    let filepath = find_hive(path).ok_or(HiveLibError::NoHiveFound(path.to_path_buf()))?;
    info!("Using hive {}", filepath.display());

    let mut command = Command::new("nix")
        .arg("eval")
        .arg("--impure")
        .arg("-E")
        .arg(
            format!(
                "let evaluate = import ./lib/src/evaluate.nix; hive = evaluate {{hive = import {path};}}; in hive.getTopLevel \"{node}\"",
                path = filepath.to_str().unwrap(),
                node = "node-a"
            )
        )
        .stdout(Stdio::piped())
        .spawn()
        .map_err(HiveLibError::NixExecError)?;

    let stdout = command
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let mut stdout_reader = BufReader::new(stdout).lines();

    tokio::spawn(async move {
        let status = command
            .wait()
            .await
            .expect("child process encountered an error");

        debug!("command status was: {}", status);
    });

    while let Some(line) = stdout_reader.next_line().await.unwrap() {
        info!("Line: {}", line);
    }

    Ok(())
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

use node::{Name, Node};
use std::collections::hash_map::OccupiedEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, trace};

use serde::{Deserialize, Serialize};

use crate::nix::{get_eval_command, EvalGoal};
use crate::{HiveLibError, SubCommandModifiers};
pub mod key;
pub mod node;
mod steps;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Hive {
    pub nodes: HashMap<Name, Node>,
    pub path: PathBuf,
}

pub enum Action<'a> {
    Inspect,
    EvaluateNode(OccupiedEntry<'a, String, Node>),
}

impl Hive {
    #[instrument]
    pub async fn new_from_path(
        path: &Path,
        modifiers: SubCommandModifiers,
    ) -> Result<Hive, HiveLibError> {
        info!("Searching upwards for hive in {}", path.display());
        let filepath = find_hive(path).ok_or(HiveLibError::NoHiveFound(path.to_path_buf()))?;
        info!("Using hive {}", filepath.display());

        let command = get_eval_command(&filepath, &EvalGoal::Inspect, modifiers)
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
            stderr
                .split('\n')
                .map(std::string::ToString::to_string)
                .collect(),
        ))
    }

    /// # Errors
    ///
    /// Returns an error if a node in nodes does not exist in the hive.
    pub fn force_always_local(&mut self, nodes: Vec<String>) -> Result<(), HiveLibError> {
        for node in nodes {
            info!("Forcing a local build for {node}");

            self.nodes
                .get_mut(&Name(Arc::from(node.clone())))
                .ok_or(HiveLibError::NodeDoesNotExist(node.to_string()))?
                .build_remotely = false;
        }

        Ok(())
    }
}

fn find_hive(path: &Path) -> Option<PathBuf> {
    trace!("Searching for hive in {}", path.display());
    let filepath_hive = path.join("hive.nix");

    if filepath_hive.is_file() {
        return Some(filepath_hive);
    }

    let filepath_flake = path.join("flake.nix");

    if filepath_flake.is_file() {
        return Some(filepath_flake);
    }

    if let Some(parent) = path.parent() {
        return find_hive(parent);
    }

    error!("No hive found");
    None
}

#[cfg(test)]
mod tests {
    use im::vector;

    use super::*;
    use std::env;

    #[test]
    fn test_hive_dot_nix_priority() {
        let mut path: PathBuf = env::var("WIRE_TEST_DIR").unwrap().into();
        path.push("test_hive_dot_nix_priority");

        let hive = find_hive(&path).unwrap();

        assert!(hive.ends_with("hive.nix"));
    }

    #[tokio::test]
    #[cfg_attr(feature = "no_web_tests", ignore)]
    async fn test_hive_file() {
        let mut path: PathBuf = env::var("WIRE_TEST_DIR").unwrap().into();
        path.push("test_hive_file");

        let hive = Hive::new_from_path(&path, SubCommandModifiers::default())
            .await
            .unwrap();

        let node = Node {
            tags: im::HashSet::new(),
            target: node::Target {
                host: "192.168.122.96".into(),
                user: "root".into(),
                port: 22,
            },
            build_remotely: true,
            keys: im::Vector::new(),
            allow_local_deployment: false,
        };

        let mut nodes = HashMap::new();
        nodes.insert(Name("node-a".into()), node);

        path.push("hive.nix");

        assert_eq!(hive, Hive { nodes, path });
    }

    #[tokio::test]
    #[cfg_attr(feature = "no_web_tests", ignore)]
    async fn non_trivial_hive() {
        let mut path: PathBuf = env::var("WIRE_TEST_DIR").unwrap().into();
        path.push("non_trivial_hive");

        let hive = Hive::new_from_path(&path, SubCommandModifiers::default())
            .await
            .unwrap();

        let node = Node {
            tags: im::HashSet::new(),
            target: node::Target {
                host: "name".into(),
                user: "root".into(),
                port: 22,
            },
            build_remotely: true,
            keys: vector![key::Key {
                name: "different-than-a".into(),
                dest_dir: "/run/keys/".into(),
                path: "/run/keys/different-than-a".into(),
                group: "root".into(),
                user: "root".into(),
                permissions: "0600".into(),
                source: key::Source::String("hi".into()),
                upload_at: key::UploadKeyAt::PreActivation,
            }],
            allow_local_deployment: false,
        };

        let mut nodes = HashMap::new();
        nodes.insert(Name("node-a".into()), node);

        path.push("hive.nix");

        assert_eq!(hive, Hive { nodes, path });
    }

    #[tokio::test]
    #[cfg_attr(feature = "no_web_tests", ignore)]
    async fn no_nixpkgs() {
        let mut path: PathBuf = env::var("WIRE_TEST_DIR").unwrap().into();
        path.push("no_nixpkgs");

        assert!(matches!(
            Hive::new_from_path(&path, SubCommandModifiers::default()).await,
            Err(HiveLibError::NixEvalError(..))
        ));
    }

    #[tokio::test]
    #[cfg_attr(feature = "no_web_tests", ignore)]
    async fn _keys_should_fail() {
        let mut path: PathBuf = env::var("WIRE_TEST_DIR").unwrap().into();
        path.push("_keys_should_fail");

        assert!(matches!(
            Hive::new_from_path(&path, SubCommandModifiers::default()).await,
            Err(HiveLibError::NixEvalError(..))
        ));
    }
}

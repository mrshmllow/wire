// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use nix_compat::flakeref::FlakeRef;
use node::{Name, Node};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::OccupiedEntry;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tracing::{info, instrument};

use crate::commands::common::evaluate_hive_attribute;
use crate::errors::{HiveInitializationError, HiveLocationError};
use crate::{EvalGoal, HiveLibError, SubCommandModifiers};
pub mod node;
pub mod steps;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hive {
    pub nodes: HashMap<Name, Node>,

    #[serde(deserialize_with = "check_schema_version", rename = "_schema")]
    pub schema: u32,
}

pub enum Action<'a> {
    Inspect,
    EvaluateNode(OccupiedEntry<'a, String, Node>),
}

fn check_schema_version<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let version = u32::deserialize(d)?;
    if version != Hive::SCHEMA_VERSION {
        return Err(D::Error::custom(
            "Version mismatch for Hive. Please ensure the binary and your wire input match!",
        ));
    }
    Ok(version)
}

impl Hive {
    pub const SCHEMA_VERSION: u32 = 0;

    #[instrument(skip_all, name = "eval_hive")]
    pub async fn new_from_path(
        location: &HiveLocation,
        modifiers: SubCommandModifiers,
        clobber_lock: Arc<Mutex<()>>,
    ) -> Result<Hive, HiveLibError> {
        let output =
            evaluate_hive_attribute(location, &EvalGoal::Inspect, modifiers, clobber_lock).await?;

        info!("evaluate_hive_attribute ouputted {output}");

        let hive: Hive = serde_json::from_str(&output).map_err(|err| {
            HiveLibError::HiveInitializationError(HiveInitializationError::ParseEvaluateError(err))
        })?;

        Ok(hive)
    }

    /// # Errors
    ///
    /// Returns an error if a node in nodes does not exist in the hive.
    pub fn force_always_local(&mut self, nodes: Vec<String>) -> Result<(), HiveLibError> {
        for node in nodes {
            info!("Forcing a local build for {node}");

            self.nodes
                .get_mut(&Name(Arc::from(node.clone())))
                .ok_or(HiveLibError::HiveInitializationError(
                    HiveInitializationError::NodeDoesNotExist(node.clone()),
                ))?
                .build_remotely = false;
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum HiveLocation {
    HiveNix(PathBuf),
    Flake(String),
}

pub fn find_hive(path: String) -> Result<HiveLocation, HiveLocationError> {
    let flakeref = FlakeRef::from_str(&path);

    let path_to_location = |path: PathBuf| {
        Ok(match path.file_name().and_then(OsStr::to_str) {
            Some("hive.nix") => HiveLocation::HiveNix(path.clone()),
            Some(_) => {
                if fs::metadata(path.join("flake.nix")).is_ok() {
                    HiveLocation::Flake(path.join("flake.nix").display().to_string())
                } else {
                    HiveLocation::HiveNix(path.join("hive.nix"))
                }
            },
            None => return Err(HiveLocationError::MalformedPath(path.clone())),
        })
    };

    match flakeref {
        Err(nix_compat::flakeref::FlakeRefError::UrlParseError(_err)) => {
            let path = PathBuf::from(path);
            Ok(path_to_location(path)?)
        }
        Err(err) => Err(HiveLocationError::Malformed(err)),
        Ok(FlakeRef::Path { path, .. }) => Ok(path_to_location(path)?),
        Ok(
            FlakeRef::Git { .. }
            | FlakeRef::GitHub { .. }
            | FlakeRef::GitLab { .. }
            | FlakeRef::Tarball { .. }
            | FlakeRef::Mercurial { .. }
            | FlakeRef::SourceHut { .. },
        ) => Ok(HiveLocation::Flake(path)),
        Ok(flakeref) => Err(HiveLocationError::TypeUnsupported(flakeref)),
    }
}

#[cfg(test)]
mod tests {
    use im::vector;

    use crate::{
        errors::CommandError, get_test_path, hive::steps::keys::{Key, Source, UploadKeyAt}, location, test_support::{get_clobber_lock, make_flake_sandbox}
    };

    use super::*;
    use std::{assert_matches::assert_matches, env};

    // flake should always come before hive.nix
    #[test]
    fn test_hive_dot_nix_priority() {
        let location = location!(get_test_path!());

        assert_matches!(location, HiveLocation::Flake(..));
    }

    #[tokio::test]
    #[cfg_attr(feature = "no_web_tests", ignore)]
    async fn test_hive_file() {
        let location = location!(get_test_path!());

        let hive = Hive::new_from_path(&location, SubCommandModifiers::default(), get_clobber_lock())
            .await
            .unwrap();

        let node = Node {
            target: node::Target::from_host("192.168.122.96"),
            ..Default::default()
        };

        let mut nodes = HashMap::new();
        nodes.insert(Name("node-a".into()), node);

        assert_eq!(
            hive,
            Hive {
                nodes,
                schema: Hive::SCHEMA_VERSION
            }
        );
    }

    #[tokio::test]
    #[cfg_attr(feature = "no_web_tests", ignore)]
    async fn non_trivial_hive() {
        let location = location!(get_test_path!());

        let hive = Hive::new_from_path(&location, SubCommandModifiers::default(), get_clobber_lock())
            .await
            .unwrap();

        let node = Node {
            target: node::Target::from_host("name"),
            keys: vector![Key {
                name: "different-than-a".into(),
                dest_dir: "/run/keys/".into(),
                path: "/run/keys/different-than-a".into(),
                group: "root".into(),
                user: "root".into(),
                permissions: "0600".into(),
                source: Source::String("hi".into()),
                upload_at: UploadKeyAt::PreActivation,
                environment: im::HashMap::new()
            }],
            ..Default::default()
        };

        let mut nodes = HashMap::new();
        nodes.insert(Name("node-a".into()), node);

        assert_eq!(
            hive,
            Hive {
                nodes,
                schema: Hive::SCHEMA_VERSION
            }
        );
    }

    #[tokio::test]
    #[cfg_attr(feature = "no_web_tests", ignore)]
    async fn flake_hive() {
        let tmp_dir = make_flake_sandbox(&get_test_path!()).unwrap();

        let location = find_hive(tmp_dir.path().display().to_string()).unwrap();
        let hive = Hive::new_from_path(
            &location,
            SubCommandModifiers::default(),
            get_clobber_lock(),
        )
        .await
        .unwrap();

        let mut nodes = HashMap::new();

        // a merged node
        nodes.insert(Name("node-a".into()), Node::from_host("node-a"));
        // a non-merged node
        nodes.insert(Name("node-b".into()), Node::from_host("node-b"));

        assert_eq!(
            hive,
            Hive {
                nodes,
                schema: Hive::SCHEMA_VERSION
            }
        );

        tmp_dir.close().unwrap();
    }

    #[tokio::test]
    async fn no_nixpkgs() {
        let location = location!(get_test_path!());

        assert_matches!(
            Hive::new_from_path(&location, SubCommandModifiers::default(), get_clobber_lock()).await,
            Err(HiveLibError::NixEvalError {
                source: CommandError::CommandFailed {
                    logs,
                    ..
                },
                ..
            })
            if logs.contains("makeHive called without meta.nixpkgs specified")
        );
    }

    #[tokio::test]
    async fn _keys_should_fail() {
        let location = location!(get_test_path!());

        assert_matches!(
            Hive::new_from_path(&location, SubCommandModifiers::default(), get_clobber_lock()).await,
            Err(HiveLibError::NixEvalError {
                source: CommandError::CommandFailed {
                    logs,
                    ..
                },
                ..
            })
            if logs.contains("The option `deployment._keys' is read-only, but it's set multiple times.")
        );
    }
}

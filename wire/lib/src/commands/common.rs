use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    SubCommandModifiers,
    commands::{ChildOutputMode, WireCommand, WireCommandChip, nonelevated::NonElevatedCommand},
    errors::{HiveInitializationError, HiveLibError},
    hive::{
        find_hive,
        node::{Name, Node, Push},
    },
    nix::EvalGoal,
};

pub async fn push(
    node: &Node,
    name: &Name,
    push: Push<'_>,
    clobber_lock: Arc<Mutex<()>>,
) -> Result<(), HiveLibError> {
    let mut command = NonElevatedCommand::spawn_new(None, ChildOutputMode::Nix).await?;

    let command_string = format!(
        "nix --extra-experimental-features nix-command \
        copy --substitute-on-destination --to ssh://{user}@{host} {path}",
        user = node.target.user,
        host = node.target.get_preffered_host()?,
        path = match push {
            Push::Derivation(drv) => format!("{drv} --derivation"),
            Push::Path(path) => path.to_string(),
        }
    );

    let child = command.run_command_with_env(
        command_string,
        false,
        HashMap::from([("NIX_SSHOPTS".into(), format!("-p {}", node.target.port))]),
        clobber_lock,
    )?;

    child
        .wait_till_success()
        .await
        .map_err(|error| HiveLibError::NixCopyError {
            name: name.clone(),
            path: push.to_string(),
            error,
        })?;

    Ok(())
}

/// Evaluates the hive in path with regards to the given goal,
/// and returns stdout.
pub async fn evaluate_hive_attribute(
    path: &Path,
    goal: &EvalGoal<'_>,
    modifiers: SubCommandModifiers,
    clobber_lock: Arc<Mutex<()>>,
) -> Result<String, HiveLibError> {
    // assert!(check_nix_available(), "nix is not available on this system");

    let canon_path =
        find_hive(&path.canonicalize().unwrap()).ok_or(HiveLibError::HiveInitializationError(
            HiveInitializationError::NoHiveFound(path.to_path_buf()),
        ))?;

    let mut command = NonElevatedCommand::spawn_new(None, ChildOutputMode::Nix).await?;
    let attribute = if canon_path.ends_with("flake.nix") {
        format!(
            "{}#wire --apply \"hive: {}\"",
            canon_path.to_str().unwrap(),
            match goal {
                EvalGoal::Inspect => "hive.inspect".to_string(),
                EvalGoal::GetTopLevel(node) => format!("hive.topLevels.{node}"),
            }
        )
    } else {
        format!(
            "--file {} {}",
            &canon_path.to_string_lossy(),
            match goal {
                EvalGoal::Inspect => "inspect".to_string(),
                EvalGoal::GetTopLevel(node) => format!("topLevels.{node}"),
            }
        )
    };

    let command_string = format!(
        "nix --extra-experimental-features nix-command \
        --extra-experimental-features flakes \
        eval --json {mods} {attribute}",
        mods = if modifiers.show_trace {
            "--show-trace"
        } else {
            ""
        },
    );

    let child = command.run_command(command_string, false, clobber_lock)?;

    child
        .wait_till_success()
        .await
        .map_err(|source| HiveLibError::NixEvalError { attribute, source })
        .map(|(_, stdout)| stdout)
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tracing::instrument;

use crate::{
    EvalGoal, SubCommandModifiers,
    commands::{CommandArguments, Either, WireCommandChip, run_command, run_command_with_env},
    errors::HiveLibError,
    hive::{
        HiveLocation,
        node::{Context, Push},
    },
};

pub async fn push(context: &Context<'_>, push: Push<'_>) -> Result<(), HiveLibError> {
    let command_string = format!(
        "nix --extra-experimental-features nix-command \
        copy --substitute-on-destination --to ssh://{user}@{host} {path}",
        user = context.node.target.user,
        host = context.node.target.get_preferred_host()?,
        path = match push {
            Push::Derivation(drv) => format!("{drv} --derivation"),
            Push::Path(path) => path.clone(),
        }
    );

    let child = run_command_with_env(
        &CommandArguments::new(
            command_string,
            context.modifiers,
            context.clobber_lock.clone(),
        )
        .nix(),
        HashMap::from([(
            "NIX_SSHOPTS".into(),
            context
                .node
                .target
                .create_ssh_opts(context.modifiers, false)?,
        )]),
    )?;

    child
        .wait_till_success()
        .await
        .map_err(|error| HiveLibError::NixCopyError {
            name: context.name.clone(),
            path: push.to_string(),
            error: Box::new(error),
        })?;

    Ok(())
}

/// Evaluates the hive in flakeref with regards to the given goal,
/// and returns stdout.
#[instrument(ret(level = tracing::Level::TRACE), skip_all)]
pub async fn evaluate_hive_attribute(
    location: &HiveLocation,
    goal: &EvalGoal<'_>,
    modifiers: SubCommandModifiers,
    clobber_lock: Arc<Mutex<()>>,
) -> Result<String, HiveLibError> {
    let attribute = match location {
        HiveLocation::Flake(uri) => {
            format!(
                "{uri}#wire --apply \"hive: {}\"",
                match goal {
                    EvalGoal::Inspect => "hive.inspect".to_string(),
                    EvalGoal::GetTopLevel(node) => format!("hive.topLevels.{node}"),
                }
            )
        }
        HiveLocation::HiveNix(path) => {
            format!(
                "--file {} {}",
                &path.to_string_lossy(),
                match goal {
                    EvalGoal::Inspect => "inspect".to_string(),
                    EvalGoal::GetTopLevel(node) => format!("topLevels.{node}"),
                }
            )
        }
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

    let child = run_command(&CommandArguments::new(command_string, modifiers, clobber_lock).nix())?;

    child
        .wait_till_success()
        .await
        .map_err(|source| HiveLibError::NixEvalError { attribute, source })
        .map(|x| match x {
            Either::Left((_, stdout)) | Either::Right((_, stdout)) => stdout,
        })
}

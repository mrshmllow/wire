use std::collections::HashMap;

use crate::{
    commands::{ChildOutputMode, WireCommand, WireCommandChip, nonelevated::NonElevatedCommand},
    errors::HiveLibError,
    hive::node::{Name, Node, Push},
};

pub async fn push(node: &Node, name: &Name, push: Push<'_>) -> Result<(), HiveLibError> {
    let mut command = NonElevatedCommand::spawn_new(&node.target, ChildOutputMode::Nix).await?;

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
        false,
        HashMap::from([("NIX_SSHOPTS".into(), format!("-p {}", node.target.port))]),
    )?;

    child
        .wait_till_success()
        .await
        .map_err(|x| HiveLibError::NixCopyError {
            name: name.clone(),
            path: push.to_string(),
            error: x,
        })?;

    Ok(())
}

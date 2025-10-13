// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

#![deny(clippy::pedantic)]
use nix::sys::stat;
use nix::unistd::{self, Group, User};
use prost::Message;
use std::env;
use std::io::BufReader;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::{
    io::{Cursor, Read},
    os::unix::fs::chown,
};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use key_agent::keys::Keys;

fn create_path(key_path: &Path) -> Result<(), anyhow::Error> {
    let prefix = key_path.parent().unwrap();
    std::fs::create_dir_all(prefix)?;

    Ok(())
}

/// Returns path of FIFO created.
fn create_fifo(user: &str) -> Result<String, anyhow::Error> {
    let path_string = "/run/wire_keyagent_fifo";
    let fifo_path = Path::new(path_string);

    if std::fs::exists(fifo_path)? {
        std::fs::remove_file(fifo_path)?;
    }

    unistd::mkfifo(fifo_path, stat::Mode::S_IRUSR | stat::Mode::S_IWUSR)?;

    let user = User::from_name(user)?;

    chown(
        fifo_path,
        Some(user.as_ref().map_or(0, |user| user.uid.into())),
        Some(user.map_or(0, |user| user.gid.into())),
    )?;

    Ok(path_string.to_string())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let length: usize = env::args().nth(1).expect("failed to grab arg").parse()?;
    let fifo_owner = env::args().nth(2).expect("failed to grab user");
    let mut msg_buf = vec![0u8; length];

    let fifo_path = create_fifo(&fifo_owner)?;
    let mut reader = BufReader::new(std::fs::File::open(fifo_path)?);

    reader.read_exact(&mut msg_buf)?;

    let msg = Keys::decode(&mut Cursor::new(&msg_buf))?;

    println!("{msg:?}");

    for key in msg.keys {
        let path = PathBuf::from(&key.destination);
        create_path(&path)?;

        let mut file = File::create(path).await?;
        let mut permissions = file.metadata().await?.permissions();

        permissions.set_mode(key.permissions);
        file.set_permissions(permissions).await?;

        let user = User::from_name(&key.user)?;
        let group = Group::from_name(&key.group)?;

        chown(
            key.destination,
            // Default uid/gid to 0. This is then wrapped around an Option again for
            // the function.
            Some(user.map_or(0, |user| user.uid.into())),
            Some(group.map_or(0, |group| group.gid.into())),
        )?;

        let mut file_buf = vec![
            0u8;
            key.length
                .try_into()
                .expect("failed to convert size to usize")
        ];

        reader.read_exact(&mut file_buf)?;
        file.write_all(&file_buf).await?;

        println!("Wrote to {file:?}");
    }

    Ok(())
}

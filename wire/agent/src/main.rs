#![deny(clippy::pedantic)]
use agent::keys::Keys;
use anyhow::bail;
use clap::{Parser, Subcommand};
use nix::unistd::{Group, User};
use prost::Message;
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::{
    io::{Cursor, Read},
    os::unix::fs::chown,
};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

fn create_path(key_path: &Path) -> Result<(), anyhow::Error> {
    let prefix = key_path.parent().unwrap();
    std::fs::create_dir_all(prefix)?;

    Ok(())
}

#[derive(Subcommand, Debug)]
enum Operations {
    #[command()]
    PushKeys {
        #[arg(short, long)]
        length: usize,
    },
}

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    operation: Operations,
}

async fn push_keys(length: usize) -> Result<(), anyhow::Error> {
    let mut stdin = std::io::stdin();
    let mut msg_buf = vec![0u8; length];
    stdin.read_exact(&mut msg_buf)?;

    let msg = Keys::decode(&mut Cursor::new(&msg_buf))?;
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
            user.map(|user| user.uid.into()),
            group.map(|group| group.gid.into()),
        )?;

        let mut file_buf = vec![
            0u8;
            key.length
                .try_into()
                .expect("failed to convert size to usize")
        ];

        stdin.read_exact(&mut file_buf)?;
        file.write_all(&file_buf).await?;

        println!("Wrote to {file:?}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    match args.operation {
        Operations::PushKeys { length } => push_keys(length).await,
    }
}

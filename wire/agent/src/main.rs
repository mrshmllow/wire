#![deny(clippy::pedantic)]
use agent::keys::Keys;
use clap::Parser;
use nix::unistd::{Group, User};
use prost::Message;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::{
    io::{Cursor, Read},
    os::unix::fs::chown,
};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixListener;

mod cli;

fn create_path(key_path: &Path) -> Result<(), anyhow::Error> {
    let prefix = key_path.parent().unwrap();
    std::fs::create_dir_all(prefix)?;

    Ok(())
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

async fn magic_rollback(
    grace_period: cli::Time,
    timeout: cli::Time,
    known_working_closure: Box<Path>,
) -> Result<(), anyhow::Error> {
    // NOTE: The default permissions of the socket do seem to be good enough
    // (755, rwxr-xr-x), but we may wish to revisit this to set it to 700.
    let socket = UnixListener::bind(".wire-agent").unwrap();

    loop {
        match socket.accept().await {
            Ok((stream, _)) => {}
            Err(e) => {
                println!("accept() encountered an issue: {e}");
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = cli::Args::parse();

    match args.operation {
        cli::Operations::PushKeys { length } => push_keys(length).await,
        cli::Operations::Rollback {
            grace_period,
            timeout,
            known_working_closure,
        } => magic_rollback(grace_period, timeout, known_working_closure).await,
    }
}

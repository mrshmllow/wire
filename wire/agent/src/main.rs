#![deny(clippy::pedantic)]
use agent::keys::{Keys, OpCode, Rollback};
use anyhow::bail;
use clap::Parser;
use cli::Time;
use nix::unistd::{geteuid, Group, User};
use prost::bytes::BytesMut;
use prost::Message;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::{
    io::{Cursor, Read},
    os::unix::fs::chown,
};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::select;
use tokio::sync::mpsc;
use tracing::{debug, error, info, info_span, instrument};
use tracing_subscriber::fmt;

mod cli;

fn create_path(key_path: &Path) -> Result<(), anyhow::Error> {
    let prefix = key_path.parent().unwrap();
    std::fs::create_dir_all(prefix)?;

    Ok(())
}

#[instrument]
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

#[instrument]
async fn rollback(
    grace_period: Time,
    timeout: Time,
    known_working_closure: PathBuf,
) -> Result<(), anyhow::Error> {
    let (tx, mut rx) = mpsc::channel(1);
    let sleeper = tokio::time::sleep(timeout.0);
    tokio::pin!(sleeper);

    tokio::spawn(async move {
        let server = UnixListener::bind(".wire-agent").unwrap();
        loop {
            info!("accepting new connection");
            match server.accept().await {
                Ok((mut stream, _)) => {
                    info!("accepted connection");
                    let mut buf = BytesMut::new();
                    stream.read_buf(&mut buf).await.unwrap();
                    let rollback = Rollback::decode(buf).unwrap();
                    if let OpCode::Query = rollback.opcode() {
                        let mut resp = rollback.clone();
                        resp.message = String::from("acknowledged");
                        let n = stream.write(resp.encode_to_vec().as_slice()).await.unwrap();
                        info!({ written = n }, "response sent");
                    }
                    tx.send(rollback.clone()).await.unwrap();
                }
                Err(err) => error!({ error = %err }),
            }
        }
    });

    loop {
        select! {
            msg = rx.recv() => {
                let d = msg.unwrap();
                debug!("received socket message: {:?}", d);
                match d.opcode() {
                    OpCode::Unspecified | OpCode::Query => (),
                    OpCode::Finish => {
                        return Ok(());
                    },

                }
            }

            () =  &mut sleeper => {
                let path  = known_working_closure.join("bin/switch-to-configuration");
                info!("timer elapsed, rolling back to {}", path.to_str().unwrap());

                let span = info_span!("activation");
                let _enter = span.enter();

                let mut child = Command::new(path)
                   .arg("switch")
                   .stdout(Stdio::piped())
                   .stderr(Stdio::piped())
                   .spawn()
                   .unwrap();

                // Approach adopted from the handle_io logic
                let stderr = BufReader::new(child.stderr.take().unwrap());
                let mut lines = stderr.lines();

                while let Some(line) = lines.next_line().await.unwrap() {
                    info!("{line}");
                }

                return Ok(());

            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = cli::Args::parse();
    if !geteuid().is_root() {
        bail!("agent must be ran as root");
    }
    let format = fmt::format()
        .with_level(false)
        .without_time()
        .with_target(true)
        .compact();
    tracing_subscriber::fmt().event_format(format).init();

    match args.operation {
        cli::Operations::PushKeys { length } => push_keys(length).await,
        cli::Operations::Rollback {
            grace_period,
            timeout,
            known_working_closure,
        } => rollback(grace_period, timeout, known_working_closure).await,
        cli::Operations::Dummy => {
            let b = Rollback {
                opcode: OpCode::Finish.into(),
                timeout: 10,
                grace_period: 10,
                closure: "/run/current-system".to_string(),
                message: String::new(),
            };

            let mut a = UnixStream::connect(".wire-agent").await?;
            _ = a.write(&b.encode_to_vec()).await?;

            let mut resp = BytesMut::new();
            a.read_buf(&mut resp).await?;
            let v = Rollback::decode(&mut resp).unwrap();
            println!("response: {v:?}");

            Ok(())
        }
    }
}

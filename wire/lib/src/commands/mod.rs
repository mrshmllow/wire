use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use itertools::Either;

use crate::{
    SubCommandModifiers,
    commands::{
        elevated::{ElevatedChildChip, ElevatedCommand},
        nonelevated::{NonElevatedChildChip, NonElevatedCommand},
    },
    errors::{DetachedError, HiveLibError},
    hive::node::Target,
    nix_log::{Action, Internal, NixLog, Trace},
};

pub(crate) mod common;
pub(crate) mod elevated;
pub(crate) mod nonelevated;

#[derive(Copy, Clone)]
pub(crate) enum ChildOutputMode {
    Raw,
    Nix,
}

pub(crate) async fn get_elevated_command(
    target: Option<&'_ Target>,
    output_mode: ChildOutputMode,
    modifiers: SubCommandModifiers,
) -> Result<Either<ElevatedCommand<'_>, NonElevatedCommand<'_>>, HiveLibError> {
    if modifiers.non_interactive {
        return Ok(Either::Left(
            ElevatedCommand::spawn_new(target, output_mode).await?,
        ));
    }

    return Ok(Either::Right(
        NonElevatedCommand::spawn_new(target, output_mode).await?,
    ));
}

pub(crate) trait WireCommand<'target>: Sized {
    type ChildChip;

    async fn spawn_new(
        target: Option<&'target Target>,
        output_mode: ChildOutputMode,
    ) -> Result<Self, HiveLibError>;

    fn run_command<S: AsRef<str>>(
        &mut self,
        command_string: S,
        keep_stdin_open: bool,
        clobber_lock: Arc<Mutex<()>>,
    ) -> Result<Self::ChildChip, HiveLibError> {
        self.run_command_with_env(
            command_string,
            keep_stdin_open,
            std::collections::HashMap::new(),
            clobber_lock,
        )
    }

    fn run_command_with_env<S: AsRef<str>>(
        &mut self,
        command_string: S,
        keep_stdin_open: bool,
        args: HashMap<String, String>,
        clobber_lock: Arc<Mutex<()>>,
    ) -> Result<Self::ChildChip, HiveLibError>;
}

pub(crate) trait WireCommandChip {
    type ExitStatus;

    async fn wait_till_success(self) -> Result<Self::ExitStatus, DetachedError>;
    async fn write_stdin(&mut self, data: Vec<u8>) -> Result<(), HiveLibError>;
}

impl WireCommand<'_> for Either<ElevatedCommand<'_>, NonElevatedCommand<'_>> {
    type ChildChip = Either<ElevatedChildChip, NonElevatedChildChip>;

    /// How'd you get here?
    async fn spawn_new(
        _target: Option<&'_ Target>,
        _output_mode: ChildOutputMode,
    ) -> Result<Self, HiveLibError> {
        unimplemented!()
    }

    fn run_command_with_env<S: AsRef<str>>(
        &mut self,
        command_string: S,
        keep_stdin_open: bool,
        args: HashMap<String, String>,
        clobber_lock: Arc<Mutex<()>>,
    ) -> Result<Self::ChildChip, HiveLibError> {
        match self {
            Self::Left(left) => left
                .run_command_with_env(command_string, keep_stdin_open, args, clobber_lock)
                .map(Either::Left),
            Self::Right(right) => right
                .run_command_with_env(command_string, keep_stdin_open, args, clobber_lock)
                .map(Either::Right),
        }
    }
}

impl WireCommandChip for Either<ElevatedChildChip, NonElevatedChildChip> {
    type ExitStatus = Either<portable_pty::ExitStatus, (std::process::ExitStatus, String)>;

    async fn write_stdin(&mut self, data: Vec<u8>) -> Result<(), HiveLibError> {
        match self {
            Self::Left(left) => left.write_stdin(data).await,
            Self::Right(right) => right.write_stdin(data).await,
        }
    }

    async fn wait_till_success(self) -> Result<Self::ExitStatus, DetachedError> {
        match self {
            Self::Left(left) => left.wait_till_success().await.map(Either::Left),
            Self::Right(right) => right.wait_till_success().await.map(Either::Right),
        }
    }
}

impl ChildOutputMode {
    fn trace(self, line: String) -> Option<NixLog> {
        let log = match self {
            ChildOutputMode::Nix => {
                let log =
                    serde_json::from_str::<Internal>(line.strip_prefix("@nix ").unwrap_or(&line))
                        .map(NixLog::Internal)
                        .unwrap_or(NixLog::Raw(line));

                // Throw out stop logs
                if let NixLog::Internal(Internal {
                    action: Action::Stop,
                }) = log
                {
                    return None;
                }

                log
            }
            Self::Raw => NixLog::Raw(line),
        };

        log.trace();

        Some(log)
    }
}

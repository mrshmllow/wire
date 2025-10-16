// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    SubCommandModifiers,
    commands::{
        interactive::{InteractiveChildChip, InteractiveCommand},
        noninteractive::{NonInteractiveChildChip, NonInteractiveCommand},
    },
    errors::{CommandError, HiveLibError},
    hive::node::Target,
    nix_log::{Action, Internal, NixLog, Trace},
};

pub(crate) mod common;
pub(crate) mod interactive;
pub(crate) mod interactive_logbuffer;
pub(crate) mod noninteractive;

#[derive(Copy, Clone)]
pub(crate) enum ChildOutputMode {
    Raw,
    Nix,
}

#[derive(Debug)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

pub(crate) struct CommandArguments<S: AsRef<str>> {
    pub(crate) command_string: S,
    pub(crate) keep_stdin_open: bool,
    pub(crate) elevated: bool,
    pub(crate) clobber_lock: Arc<Mutex<()>>,
}

pub(crate) async fn get_command(
    target: Option<&'_ Target>,
    output_mode: ChildOutputMode,
    modifiers: SubCommandModifiers,
) -> Result<Either<InteractiveCommand<'_>, NonInteractiveCommand<'_>>, HiveLibError> {
    if modifiers.non_interactive {
        return Ok(Either::Right(
            NonInteractiveCommand::spawn_new(target, output_mode, modifiers).await?,
        ));
    }

    return Ok(Either::Left(
        InteractiveCommand::spawn_new(target, output_mode, modifiers).await?,
    ));
}

pub(crate) trait WireCommand<'target>: Sized {
    type ChildChip;

    async fn spawn_new(
        target: Option<&'target Target>,
        output_mode: ChildOutputMode,
        modifiers: SubCommandModifiers,
    ) -> Result<Self, HiveLibError>;

    fn run_command<S: AsRef<str>>(
        &mut self,
        command_arugments: CommandArguments<S>,
    ) -> Result<Self::ChildChip, HiveLibError> {
        self.run_command_with_env(command_arugments, std::collections::HashMap::new())
    }

    fn run_command_with_env<S: AsRef<str>>(
        &mut self,
        command_arugments: CommandArguments<S>,
        args: HashMap<String, String>,
    ) -> Result<Self::ChildChip, HiveLibError>;
}

pub(crate) trait WireCommandChip {
    type ExitStatus;

    async fn wait_till_success(self) -> Result<Self::ExitStatus, CommandError>;
    async fn write_stdin(&mut self, data: Vec<u8>) -> Result<(), HiveLibError>;
}

impl WireCommand<'_> for Either<InteractiveCommand<'_>, NonInteractiveCommand<'_>> {
    type ChildChip = Either<InteractiveChildChip, NonInteractiveChildChip>;

    /// How'd you get here?
    async fn spawn_new(
        _target: Option<&'_ Target>,
        _output_mode: ChildOutputMode,
        _modifiers: SubCommandModifiers,
    ) -> Result<Self, HiveLibError> {
        unimplemented!()
    }

    fn run_command_with_env<S: AsRef<str>>(
        &mut self,
        command_arugments: CommandArguments<S>,
        envs: HashMap<String, String>,
    ) -> Result<Self::ChildChip, HiveLibError> {
        match self {
            Self::Left(left) => left
                .run_command_with_env(command_arugments, envs)
                .map(Either::Left),
            Self::Right(right) => right
                .run_command_with_env(command_arugments, envs)
                .map(Either::Right),
        }
    }
}

type ExitStatus = Either<(portable_pty::ExitStatus, String), (std::process::ExitStatus, String)>;

impl WireCommandChip for Either<InteractiveChildChip, NonInteractiveChildChip> {
    type ExitStatus = ExitStatus;

    async fn write_stdin(&mut self, data: Vec<u8>) -> Result<(), HiveLibError> {
        match self {
            Self::Left(left) => left.write_stdin(data).await,
            Self::Right(right) => right.write_stdin(data).await,
        }
    }

    async fn wait_till_success(self) -> Result<Self::ExitStatus, CommandError> {
        match self {
            Self::Left(left) => left.wait_till_success().await.map(Either::Left),
            Self::Right(right) => right.wait_till_success().await.map(Either::Right),
        }
    }
}

impl ExitStatus {
    fn get_stdout(&self) -> &String {
        match self {
            Self::Left((_, stdout)) | Self::Right((_, stdout)) => stdout,
        }
    }
}

impl ChildOutputMode {
    fn trace(self, line: String, hint_error: bool) -> Option<NixLog> {
        let log = match self {
            ChildOutputMode::Nix => {
                let log =
                    serde_json::from_str::<Internal>(line.strip_prefix("@nix ").unwrap_or(&line))
                        .map(NixLog::Internal)
                        .unwrap_or(if hint_error {
                            NixLog::RawError(line)
                        } else {
                            NixLog::Raw(line)
                        });

                // Throw out stop logs
                if let NixLog::Internal(Internal {
                    action: Action::Stop,
                }) = log
                {
                    return None;
                }

                log
            }
            Self::Raw if hint_error => NixLog::RawError(line),
            Self::Raw => NixLog::Raw(line),
        };

        log.trace();

        Some(log)
    }
}

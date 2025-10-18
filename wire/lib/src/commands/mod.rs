// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use nix_compat::log::{AT_NIX_PREFIX, LogMessage};

use crate::{
    SubCommandModifiers,
    commands::{
        interactive::{InteractiveChildChip, interactive_command_with_env},
        noninteractive::{NonInteractiveChildChip, non_interactive_command_with_env},
    },
    errors::{CommandError, HiveLibError},
    hive::node::Target,
    nix_log::{self, SubcommandLog, Trace},
};

pub(crate) mod common;
pub(crate) mod interactive;
pub(crate) mod interactive_logbuffer;
pub(crate) mod noninteractive;

#[derive(Copy, Clone, Debug)]
pub(crate) enum ChildOutputMode {
    Raw,
    Nix,
}

#[derive(Debug)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

#[derive(Debug)]
pub(crate) struct CommandArguments<'t, S: AsRef<str>> {
    pub(crate) modifiers: SubCommandModifiers,
    pub(crate) target: Option<&'t Target>,
    pub(crate) output_mode: ChildOutputMode,
    pub(crate) command_string: S,
    pub(crate) keep_stdin_open: bool,
    pub(crate) elevated: bool,
    pub(crate) clobber_lock: Arc<Mutex<()>>,
}

pub(crate) fn run_command<S: AsRef<str>>(
    arguments: &CommandArguments<'_, S>,
) -> Result<Either<InteractiveChildChip, NonInteractiveChildChip>, HiveLibError> {
    run_command_with_env(arguments, HashMap::new())
}

pub(crate) fn run_command_with_env<S: AsRef<str>>(
    arguments: &CommandArguments<'_, S>,
    envs: HashMap<String, String>,
) -> Result<Either<InteractiveChildChip, NonInteractiveChildChip>, HiveLibError> {
    // use the non interactive command runner when forced or when there is simply no reason
    // for user input to be taken (local, and not elevated)
    if arguments.modifiers.non_interactive || (!arguments.elevated && arguments.target.is_none()) {
        return Ok(Either::Right(non_interactive_command_with_env(
            arguments, envs,
        )?));
    }

    Ok(Either::Left(interactive_command_with_env(arguments, envs)?))
}

pub(crate) trait WireCommandChip {
    type ExitStatus;

    async fn wait_till_success(self) -> Result<Self::ExitStatus, CommandError>;
    async fn write_stdin(&mut self, data: Vec<u8>) -> Result<(), HiveLibError>;
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

impl ChildOutputMode {
    fn trace(self, line: &String) -> Option<nix_log::SubcommandLog<'_>> {
        let log = match self {
            ChildOutputMode::Nix => {
                let log = serde_json::from_str::<LogMessage>(
                    line.strip_prefix(AT_NIX_PREFIX).unwrap_or(line),
                )
                .map(SubcommandLog::Internal)
                .unwrap_or(SubcommandLog::Raw(line.into()));

                if !matches!(log, SubcommandLog::Internal(LogMessage::Msg { .. })) {
                    return None;
                }

                log
            }
            Self::Raw => SubcommandLog::Raw(line.into()),
        };

        log.trace();

        Some(log)
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::{
    collections::HashMap,
    str::from_utf8,
    sync::{Arc, LazyLock, Mutex},
};

use aho_corasick::{AhoCorasick, PatternID};
use gjson::Value;
use nix_compat::log::{AT_NIX_PREFIX, VerbosityLevel};
use num_enum::TryFromPrimitive;
use tracing::{debug, error, info, trace, warn};

use crate::{
    SubCommandModifiers,
    commands::{
        interactive::{InteractiveChildChip, interactive_command_with_env},
        noninteractive::{NonInteractiveChildChip, non_interactive_command_with_env},
    },
    errors::{CommandError, HiveLibError},
    hive::node::Target,
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
    modifiers: SubCommandModifiers,
    target: Option<&'t Target>,
    output_mode: ChildOutputMode,
    command_string: S,
    keep_stdin_open: bool,
    elevated: bool,
    clobber_lock: Arc<Mutex<()>>,
    log_stdout: bool,
}

static AHO_CORASICK: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasick::builder()
        .ascii_case_insensitive(false)
        .match_kind(aho_corasick::MatchKind::LeftmostFirst)
        .build([AT_NIX_PREFIX])
        .unwrap()
});

impl<'a, S: AsRef<str>> CommandArguments<'a, S> {
    pub(crate) fn new(
        command_string: S,
        modifiers: SubCommandModifiers,
        clobber_lock: Arc<Mutex<()>>,
    ) -> Self {
        Self {
            command_string,
            keep_stdin_open: false,
            elevated: false,
            log_stdout: false,
            target: None,
            output_mode: ChildOutputMode::Raw,
            modifiers,
            clobber_lock,
        }
    }

    pub(crate) fn on_target(mut self, target: Option<&'a Target>) -> Self {
        self.target = target;
        self
    }

    pub(crate) fn nix(mut self) -> Self {
        self.output_mode = ChildOutputMode::Nix;
        self
    }

    pub(crate) fn keep_stdin_open(mut self) -> Self {
        self.keep_stdin_open = true;
        self
    }

    pub(crate) fn elevated(mut self) -> Self {
        self.elevated = true;
        self
    }

    pub(crate) fn log_stdout(mut self) -> Self {
        self.log_stdout = true;
        self
    }
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

fn trace_gjson_str<'a>(log: &'a Value<'a>, msg: &'a str) -> Option<String> {
    if msg.is_empty() {
        return None;
    }

    let level = log.get("level");

    if !level.exists() {
        return None;
    }

    let level = match VerbosityLevel::try_from_primitive(level.u64()) {
        Ok(level) => level,
        Err(err) => {
            error!("nix log `level` did not match to a VerbosityLevel: {err:?}");
            return None;
        }
    };

    let msg = strip_ansi_escapes::strip_str(msg);

    match level {
        VerbosityLevel::Info => info!("{msg}"),
        VerbosityLevel::Warn | VerbosityLevel::Notice => warn!("{msg}"),
        VerbosityLevel::Error => error!("{msg}"),
        VerbosityLevel::Debug => debug!("{msg}"),
        VerbosityLevel::Vomit | VerbosityLevel::Talkative | VerbosityLevel::Chatty => {
            trace!("{msg}");
        }
    }

    if matches!(
        level,
        VerbosityLevel::Error | VerbosityLevel::Warn | VerbosityLevel::Notice
    ) {
        return Some(msg);
    }

    None
}

impl ChildOutputMode {
    /// this function is by far the biggest hotspot in the whole tree
    /// Returns a string if this log is notable to be stored as an error message
    fn trace_slice(self, line: &mut [u8]) -> Option<String> {
        let slice = match self {
            Self::Raw => {
                warn!("{}", String::from_utf8_lossy(line));
                return None;
            }
            Self::Nix => {
                let position = AHO_CORASICK.find(&line).map(|x| &mut line[x.end()..]);

                if let Some(json_buf) = position {
                    json_buf
                } else {
                    // usually happens when ssh is outputting something
                    warn!("{}", String::from_utf8_lossy(line));
                    return None;
                }
            }
        };

        let Ok(str) = from_utf8(slice) else {
            error!("nix log was not valid utf8!");
            return None;
        };

        let log = gjson::parse(str);

        let text = log.get("text");

        if text.exists() {
            return trace_gjson_str(&log, text.str());
        }

        let text = log.get("msg");

        if text.exists() {
            return trace_gjson_str(&log, text.str());
        }

        None
    }
}

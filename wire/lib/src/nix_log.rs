// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use nix_compat::log::{LogMessage, VerbosityLevel};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};
use tracing::{Level as tracing_level, event, warn};

#[derive(Debug)]
pub enum SubcommandLog<'a> {
    Internal(LogMessage<'a>),
    Raw(String),
}

pub(crate) trait Trace {
    fn trace(&self);
}

pub fn get_errorish_message<'a>(message: &'a LogMessage<'a>) -> Option<&'a Cow<'a, str>> {
    if let LogMessage::Msg {
        level: VerbosityLevel::Error | VerbosityLevel::Warn | VerbosityLevel::Notice,
        msg,
    } = message
    {
        return Some(msg);
    }

    None
}

fn nix_level_to_tracing(level: &VerbosityLevel) -> tracing_level {
    match level {
        VerbosityLevel::Info => tracing_level::INFO,
        VerbosityLevel::Warn | VerbosityLevel::Notice => tracing_level::WARN,
        VerbosityLevel::Error => tracing_level::ERROR,
        VerbosityLevel::Debug => tracing_level::DEBUG,
        VerbosityLevel::Vomit | VerbosityLevel::Talkative | VerbosityLevel::Chatty => {
            tracing_level::TRACE
        }
    }
}

impl Trace for LogMessage<'_> {
    fn trace(&self) {
        match self {
            LogMessage::Msg { level, msg } => {
                if msg.is_empty() {
                    return;
                }

                let stripped = strip_ansi_escapes::strip(msg.as_bytes());
                let msg = String::from_utf8_lossy(&stripped);

                match nix_level_to_tracing(level) {
                    tracing_level::INFO => event!(tracing_level::INFO, "{msg}"),
                    tracing_level::WARN => event!(tracing_level::WARN, "{msg}"),
                    tracing_level::ERROR => event!(tracing_level::ERROR, "{msg}"),
                    tracing_level::DEBUG => event!(tracing_level::DEBUG, "{msg}"),
                    tracing_level::TRACE => event!(tracing_level::TRACE, "{msg}"),
                }
            }
            LogMessage::Start { text, level, .. } => {
                if text.is_empty() {
                    return;
                }

                match nix_level_to_tracing(level) {
                    tracing_level::INFO => event!(tracing_level::INFO, "{text}"),
                    tracing_level::WARN => event!(tracing_level::WARN, "{text}"),
                    tracing_level::ERROR => event!(tracing_level::ERROR, "{text}"),
                    tracing_level::DEBUG => event!(tracing_level::DEBUG, "{text}"),
                    tracing_level::TRACE => event!(tracing_level::TRACE, "{text}"),
                }
            }
            LogMessage::SetPhase { phase } => {
                if phase.is_empty() {
                    return;
                }

                event!(tracing_level::INFO, set_phase = phase);
            }
            _ => {}
        }
    }
}

impl Trace for SubcommandLog<'_> {
    fn trace(&self) {
        match self {
            SubcommandLog::Internal(line) => {
                line.trace();
            }
            SubcommandLog::Raw(line) => {
                if line.is_empty() {
                    return;
                }

                warn!("{line}");
            }
        }
    }
}

impl Display for SubcommandLog<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SubcommandLog::Internal(line) => match line {
                LogMessage::Msg { level, msg } => write!(f, "{level:?}: {msg}"),
                _ => Ok(()),
            },
            SubcommandLog::Raw(line) => Display::fmt(&line, f),
        }
    }
}

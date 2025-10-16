// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use nix_compat::log::{LogMessage, VerbosityLevel};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
};
use tracing::{Level as tracing_level, event, info};

// static DIGEST_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9a-z]{32}").unwrap());

#[derive(Debug)]
pub enum SubcommandLog<'a> {
    Internal(LogMessage<'a>),
    Raw(Cow<'a, str>),
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

impl Trace for LogMessage<'_> {
    fn trace(&self) {
        if let LogMessage::Msg { level, msg } = &self {
            if msg.is_empty() {
                return;
            }

            match level {
                VerbosityLevel::Info => event!(tracing_level::INFO, "{msg}"),
                VerbosityLevel::Warn | VerbosityLevel::Notice => {
                    event!(tracing_level::WARN, "{msg}");
                }
                VerbosityLevel::Error => event!(tracing_level::ERROR, "{msg}"),
                VerbosityLevel::Debug => event!(tracing_level::DEBUG, "{msg}"),
                VerbosityLevel::Vomit | VerbosityLevel::Talkative | VerbosityLevel::Chatty => {
                    event!(tracing_level::TRACE, "{msg}");
                }
            }
        }
    }
}

impl Trace for SubcommandLog<'_> {
    fn trace(&self) {
        match self {
            SubcommandLog::Internal(line) => {
                line.trace();

                // tracing_indicatif::span_ext::IndicatifSpanExt::pb_set_message(
                //     &Span::current(),
                //     &DIGEST_RE.replace_all(&line.to_string(), "…"),
                // );
            }
            SubcommandLog::Raw(line) => info!("{line}"),
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

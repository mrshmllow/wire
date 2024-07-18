use serde::{Deserialize, Serialize};
use serde_repr::*;
use std::fmt::{Debug, Display};
use tracing::{event, info, Level};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action")]
pub enum NixLogAction {
    #[serde(rename = "msg", alias = "start")]
    Message {
        level: NixLogLevel,
        #[serde(rename = "msg", alias = "text")]
        message: Option<String>,
    },
    #[serde(rename = "stop", alias = "result")]
    Stop,
}

// TODO: Sometimes non-errors have a level 0
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum NixLogLevel {
    Error = 0,
    Info = 1,
    Talkative = 2,
    Chatty = 3,
    Debug = 4,
    Vomit = 5,
    UndocumentedLevel = 6,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InternalNixLog {
    #[serde(flatten)]
    pub action: NixLogAction,
}

#[derive(Debug)]
pub enum NixLog {
    Internal(InternalNixLog),
    Raw(String),
}

pub(crate) trait Trace {
    fn trace(&self);
    fn is_error(&self) -> bool;
}

impl Trace for InternalNixLog {
    fn trace(&self) {
        match &self.action {
            NixLogAction::Message { level, message } => {
                let text = match message {
                    Some(text) if text.is_empty() => return,
                    None => return,
                    Some(text) => text,
                };

                match level {
                    NixLogLevel::Info | NixLogLevel::Talkative | NixLogLevel::Chatty => {
                        event!(Level::INFO, "{text}")
                    }
                    NixLogLevel::Debug | NixLogLevel::Vomit => {
                        event!(Level::DEBUG, "{text}")
                    }
                    NixLogLevel::UndocumentedLevel => event!(Level::TRACE, "{text}"),
                    NixLogLevel::Error => event!(Level::ERROR, "{text}"),
                }
            }
            NixLogAction::Stop => {}
        }
    }

    fn is_error(&self) -> bool {
        matches!(&self.action, NixLogAction::Message { level, message: _ } if matches!(level, NixLogLevel::Error))
    }
}

impl Trace for NixLog {
    fn trace(&self) {
        match self {
            NixLog::Internal(line) => line.trace(),
            NixLog::Raw(line) => info!("{line}"),
        }
    }

    fn is_error(&self) -> bool {
        match self {
            NixLog::Internal(line) => line.is_error(),
            NixLog::Raw(..) => false,
        }
    }
}

impl Display for InternalNixLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.action {
            NixLogAction::Message { level, message } => {
                write!(
                    f,
                    "{level:?}: {}",
                    match message {
                        Some(message) => message,
                        None => "Nix log without text",
                    }
                )
            }
            _ => write!(f, ""),
        }
    }
}

impl Display for NixLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            NixLog::Internal(line) => Display::fmt(&line, f),
            NixLog::Raw(line) => Display::fmt(&line, f),
        }
    }
}

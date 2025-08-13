use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::{Debug, Display};
use tracing::{Level as tracing_level, event, info};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action")]
pub enum Action {
    #[serde(rename = "msg", alias = "start")]
    Message {
        level: Level,
        #[serde(rename = "msg", alias = "text")]
        message: Option<String>,
    },
    #[serde(rename = "stop", alias = "result")]
    Stop,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum Level {
    Error = 0,
    Warn = 1,
    Notice = 2,
    Info = 3,
    Talkative = 4,
    Chatty = 5,
    Debug = 6,
    Vomit = 7,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Internal {
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Debug)]
pub enum NixLog {
    Internal(Internal),
    Raw(String),
}

pub(crate) trait Trace {
    fn trace(&self);
    fn is_error(&self) -> bool;
}

impl Trace for Internal {
    fn trace(&self) {
        match &self.action {
            Action::Message { level, message } => {
                let text = match message {
                    Some(text) if text.is_empty() => return,
                    None => return,
                    Some(text) => text,
                };

                match level {
                    Level::Info => event!(tracing_level::INFO, "{text}"),
                    Level::Warn | Level::Notice => event!(tracing_level::WARN, "{text}"),
                    Level::Error => event!(tracing_level::ERROR, "{text}"),
                    Level::Debug => event!(tracing_level::DEBUG, "{text}"),
                    Level::Vomit | Level::Talkative | Level::Chatty => {
                        event!(tracing_level::TRACE, "{text}");
                    }
                }
            }
            Action::Stop => {}
        }
    }

    fn is_error(&self) -> bool {
        matches!(&self.action, Action::Message { level, message: _ } if matches!(level, Level::Error))
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

impl Display for Internal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.action {
            Action::Message { level, message } => {
                write!(
                    f,
                    "{level:?}: {}",
                    match message {
                        Some(message) => message,
                        None => "Nix log without text",
                    }
                )
            }
            Action::Stop => write!(f, ""),
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

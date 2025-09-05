use std::{num::ParseIntError, path::PathBuf, process::ExitStatus, sync::mpsc::RecvError};

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use tokio::task::JoinError;

use crate::{
    format_error_lines,
    hive::node::{Name, SwitchToConfigurationGoal},
    nix_log::{NixLog, Trace},
};

#[cfg(debug_assertions)]
const DOCS_URL: &str = "http://localhost:5173/reference/errors.html";
#[cfg(not(debug_assertions))]
const DOCS_URL: &str = "https://wire.althaea.zone/reference/errors.html";

#[derive(Debug, Diagnostic, Error)]
pub enum KeyError {
    #[diagnostic(
        code(wire::Key::File),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("error reading file")]
    File(#[source] std::io::Error),

    #[diagnostic(
        code(wire::Key::SpawningCommand),
        help("Ensure wire has the correct $PATH for this command"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("error spawning key command")]
    CommandSpawnError {
        #[source]
        error: std::io::Error,

        #[source_code]
        command: String,

        #[label(primary, "Program ran")]
        command_span: Option<SourceSpan>,
    },

    #[diagnostic(
        code(wire::Key::Resolving),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Error resolving key command child process")]
    CommandResolveError {
        #[source]
        error: std::io::Error,

        #[source_code]
        command: String,
    },

    #[diagnostic(
        code(wire::Key::CommandExit),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("key command failed with status {}: {}", .0,.1)]
    CommandError(ExitStatus, String),

    #[diagnostic(
        code(wire::Key::Empty),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Command list empty")]
    Empty,

    #[diagnostic(
        code(wire::Key::ParseKeyPermissions),
        help("Refer to the documentation for the format of key file permissions."),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to parse key permissions")]
    ParseKeyPermissions(#[source] ParseIntError),
}

#[derive(Debug, Diagnostic, Error)]
pub enum KeyAgentError {
    #[diagnostic(
        code(wire::KeyAgent::SpawningAgent),
        help("Please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Error spawning key agent")]
    SpawningAgent(#[source] std::io::Error),

    #[diagnostic(
        code(wire::KeyAgent::Resolving),
        help("Please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Error resolving key agent child process")]
    ResolvingError(#[source] std::io::Error),

    #[diagnostic(
        code(wire::KeyAgent::Fail),
        help("If you suspect the reason is wire's fault, please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to push keys (last 20 lines):\n{lines}", lines = format_error_lines(.1))]
    AgentFailed(Name, Vec<String>),
}

#[derive(Debug, Diagnostic, Error)]
pub enum ActivationError {
    #[diagnostic(
        code(wire::Activation::SwitchToConfiguration),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to run switch-to-configuration {0} on node {1}")]
    SwitchToConfigurationError(SwitchToConfigurationGoal, Name, #[source] DetachedError),

    #[diagnostic(
        code(wire::Activation::Elevate),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to elevate")]
    FailedToElevate(#[source] std::io::Error),

    #[diagnostic(
        code(wire::Activation::NixEnv),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to run nix-env on node {0} (last 20 lines):\n{lines}", lines = format_error_lines(.1))]
    NixEnvError(Name, Vec<String>),
}

#[derive(Debug, Diagnostic, Error)]
pub enum NetworkError {
    #[diagnostic(
        code(wire::Network::HostUnreachable),
        help(
            "If you failed due to a fault in DNS, note that a node can have multiple targets defined."
        ),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Cannot reach host {host}")]
    HostUnreachable {
        host: String,
        #[source]
        source: DetachedError,
    },

    #[diagnostic(
        code(wire::Network::HostUnreachableAfterReboot),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to get regain connection to {0} after activation.")]
    HostUnreachableAfterReboot(String),

    #[diagnostic(
        code(wire::Network::HostUnreachableAfterActivation),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Cannot reach host {0} after activation")]
    HostUnreachableAfterActivation(String),

    #[diagnostic(
        code(wire::Network::HostsExhausted),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Ran out of contactable hosts")]
    HostsExhausted,
}

#[derive(Debug, Diagnostic, Error)]
pub enum HiveInitializationError {
    #[diagnostic(
        code(wire::HiveInit::NoHiveFound),
        help(
            "Double check the path is correct. You can adjust the hive path with `--path` when the hive lies outside of the CWD."
        ),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("No hive could be found in {}", .0.display())]
    NoHiveFound(PathBuf),

    #[diagnostic(
        code(wire::HiveInit::NixEval),
        help("Check your hive is syntactically valid."),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to evaluate your hive!")]
    NixEvalError(#[source] DetachedError),

    #[diagnostic(
        code(wire::HiveInit::Parse),
        help("Please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to parse internal wire json.")]
    ParseEvaluateError(#[source] serde_json::Error),

    #[diagnostic(
        code(wire::HiveInit::NodeDoesNotExist),
        help("Please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("node {0} not exist in hive")]
    NodeDoesNotExist(String),
}

#[derive(Debug, Diagnostic, Error)]
pub enum NixChildError {
    #[diagnostic(
        code(wire::NixChild::JoiningTasks),
        help("This should never happen, please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())

    )]
    #[error("Could not join nix logging task")]
    JoinError(#[source] tokio::task::JoinError),

    #[diagnostic(
        code(wire::NixChild::NoHandle),
        help("This should never happen, please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("There was no handle to io on the child process")]
    NoHandle,

    #[diagnostic(
        code(wire::NixChild::SpawnFailed),
        help("Please run wire under a host with nix installed."),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to execute nix")]
    SpawnFailed(#[source] tokio::io::Error),

    #[diagnostic(
        code(wire::NixChild::Resolving),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Error resolving nix child process")]
    ResolveError(#[source] std::io::Error),
}

#[derive(Debug, Diagnostic, Error)]
pub enum DetachedError {
    #[diagnostic(
        code(wire::Detached::TermAttrs),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to set PTY attrs")]
    TermAttrs(#[source] nix::errno::Errno),

    #[diagnostic(
        code(wire::Detached::PosixPipe),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("There was an error in regards to a pipe")]
    PosixPipe(#[source] nix::errno::Errno),

    /// Error wrapped around `portable_pty`'s anyhow
    /// errors
    #[diagnostic(
        code(wire::Detached::PortablePty),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("There was an error from the portable_pty crate")]
    PortablePty(#[source] anyhow::Error),

    #[diagnostic(
        code(wire::Detached::Joining),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to join on some tokio task")]
    JoinError(#[source] JoinError),

    #[diagnostic(
        code(wire::Detached::WaitForStatus),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to wait for the child's status")]
    WaitForStatus(#[source] std::io::Error),

    #[diagnostic(
        code(wire::Detatched::NoHandle),
        help("This should never happen, please create an issue!"),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("There was no handle to child io")]
    NoHandle,

    #[diagnostic(
        code(wire::Detached::WritingClientStdout),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to write to client stdout.")]
    WritingClientStdout(#[source] std::io::Error),

    #[diagnostic(
        code(wire::Detached::WritingMasterStdin),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("Failed to write to PTY master stdout.")]
    WritingMasterStdout(#[source] std::io::Error),

    #[diagnostic(
        code(wire::Detached::Recv),
        url("{DOCS_URL}#{}", self.code().unwrap()),
        help("please create an issue!"),
    )]
    #[error("Failed to receive a message from the begin channel")]
    RecvError(#[source] RecvError),

    #[diagnostic(
        code(wire::Detached::CommandFailed),
        url("{DOCS_URL}#{}", self.code().unwrap()),
        help("`nix` commands are filtered, run with -vvv to view all"),
    )]
    #[error("{} failed with {} (last 20 lines):\n{}", .command_ran, .code, .logs)]
    CommandFailed {
        command_ran: String,
        logs: String,
        code: String,
    },
}

#[derive(Debug, Diagnostic, Error)]
pub enum HiveLibError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    HiveInitializationError(HiveInitializationError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    NetworkError(NetworkError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    ActivationError(ActivationError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    DetachedError(DetachedError),

    #[error("Failed to apply key {}", .0)]
    KeyError(
        String,
        #[source]
        #[diagnostic_source]
        KeyError,
    ),

    #[error("Wire key-agent failed")]
    KeyAgentError(
        #[source]
        #[diagnostic_source]
        KeyAgentError,
    ),

    #[error(transparent)]
    #[diagnostic(transparent)]
    NixChildError(NixChildError),

    #[diagnostic(
        code(wire::EvaluateNode),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error(
        "failed to evaluate node {0} (filtered logs, run with -vvv to see all):\n{log}",
        log = .1.iter().filter(|l| l.is_error()).map(std::string::ToString::to_string).collect::<Vec<String>>().join("\n"))
    ]
    NixEvalInternalError(Name, Vec<NixLog>),

    #[diagnostic(
        code(wire::BuildNode),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to build node {name}")]
    NixBuildError {
        name: Name,
        #[source]
        source: DetachedError,
    },

    #[diagnostic(
        code(wire::CopyPath),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("failed to copy path {path} to node {name}")]
    NixCopyError {
        name: Name,
        path: String,
        #[source]
        error: DetachedError,
    },

    #[diagnostic(
        code(wire::BufferOperation),
        url("{DOCS_URL}#{}", self.code().unwrap())
    )]
    #[error("an operation failed in regards to buffers")]
    BufferOperationError(#[source] tokio::io::Error),

    #[diagnostic(code(wire::Evaluate))]
    #[error("failed to evaluate `{attribute}` from the context of a hive.")]
    NixEvalError {
        attribute: String,

        #[source]
        source: DetachedError,
    },
}

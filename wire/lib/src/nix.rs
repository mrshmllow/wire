use std::path::Path;
use std::process::{Command, ExitStatus};
use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tracing::{Instrument, error, info, trace};

use crate::errors::{HiveInitializationError, NixChildError};
use crate::hive::find_hive;
use crate::hive::node::Name;
use crate::nix_log::{Action, Internal, NixLog, Trace};
use crate::{HiveLibError, SubCommandModifiers};

pub enum EvalGoal<'a> {
    Inspect,
    GetTopLevel(&'a Name),
}

fn check_nix_available() -> bool {
    match Command::new("nix")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => true,
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                false
            } else {
                error!(
                    "Something weird happened checking for nix availability, {}",
                    e
                );
                false
            }
        }
    }
}

pub async fn handle_io<R>(reader: R, should_trace: bool) -> Result<Vec<NixLog>, HiveLibError>
where
    R: AsyncRead + Unpin,
{
    let mut io_reader = BufReader::new(reader).lines();
    let mut collect = Vec::new();

    while let Some(line) = io_reader
        .next_line()
        .await
        .map_err(|err| HiveLibError::NixChildError(NixChildError::SpawnFailed(err)))?
    {
        let log = serde_json::from_str::<Internal>(line.strip_prefix("@nix ").unwrap_or(&line))
            .map(NixLog::Internal)
            .unwrap_or(NixLog::Raw(line.clone()));

        // Throw out stop logs
        if let NixLog::Internal(Internal {
            action: Action::Stop,
        }) = log
        {
            continue;
        }

        if cfg!(debug_assertions) {
            trace!(line);
        }

        if should_trace {
            match log {
                NixLog::Raw(ref string) => info!("{string}"),
                NixLog::Internal(ref internal) => internal.trace(),
            }

            // Span::current().pb_set_message(&DIGEST_RE.replace_all(&log.to_string(), "…"));
        }

        collect.push(log);
    }

    Ok(collect)
}

pub trait StreamTracing {
    async fn execute(
        &mut self,
        log_stderr: bool,
    ) -> Result<(ExitStatus, Vec<NixLog>, Vec<NixLog>), HiveLibError>;
}

impl StreamTracing for tokio::process::Command {
    async fn execute(
        &mut self,
        log_stderr: bool,
    ) -> Result<(ExitStatus, Vec<NixLog>, Vec<NixLog>), HiveLibError> {
        let mut child = self
            .args(["--log-format", "internal-json"])
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|err| HiveLibError::NixChildError(NixChildError::SpawnFailed(err)))?;

        let stdout_handle = child
            .stdout
            .take()
            .ok_or(HiveLibError::NixChildError(NixChildError::NoHandle))?;
        let stderr_handle = child
            .stderr
            .take()
            .ok_or(HiveLibError::NixChildError(NixChildError::NoHandle))?;

        let stderr_task = tokio::spawn(handle_io(stderr_handle, log_stderr).in_current_span());
        let stdout_task = tokio::spawn(handle_io(stdout_handle, false));

        let handle = tokio::spawn(async move {
            child
                .wait()
                .await
                .map_err(|err| HiveLibError::NixChildError(NixChildError::SpawnFailed(err)))
        });

        let (result, stdout, stderr) = tokio::try_join!(handle, stdout_task, stderr_task)
            .map_err(|err| HiveLibError::NixChildError(NixChildError::JoinError(err)))?;

        Ok((result?, stdout?, stderr?))
    }
}

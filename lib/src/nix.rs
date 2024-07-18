use lazy_static::lazy_static;
use regex::Regex;
use std::process::ExitStatus;
use std::{path::PathBuf, process::Stdio};
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tokio::{io::BufReader, process::Command};
use tracing::{info, trace, Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::nix_log::{InternalNixLog, NixLog, NixLogAction, Trace};
use crate::HiveLibError;

lazy_static! {
    static ref DIGEST_RE: Regex = Regex::new(r"[0-9a-z]{32}").unwrap();
}

pub enum EvalGoal<'a> {
    Inspect,
    GetTopLevel(&'a String),
}

pub fn get_eval_command(path: PathBuf, goal: EvalGoal) -> Command {
    let mut command = Command::new("nix");
    command.args(["eval", "--json", "--impure", "--verbose", "--expr"]);

    command.arg(format!(
        "let evaluate = import ./lib/src/evaluate.nix; hive = evaluate {{hivePath = {path};}}; in {goal}",
        path = path.to_str().unwrap(),
        goal = match goal {
            EvalGoal::Inspect => "hive.inspect".to_string(),
            EvalGoal::GetTopLevel(node) => format!("hive.getTopLevel \"{node}\"", node = node),
        }
    ));

    command
}

async fn handle_io<R>(reader: R, should_trace: bool) -> Result<Vec<NixLog>, HiveLibError>
where
    R: AsyncRead + Unpin,
{
    let mut io_reader = BufReader::new(reader).lines();
    let mut collect = Vec::new();

    while let Some(line) = io_reader
        .next_line()
        .await
        .map_err(HiveLibError::SpawnFailed)?
    {
        let log =
            serde_json::from_str::<InternalNixLog>(line.strip_prefix("@nix ").unwrap_or(&line))
                .map(NixLog::Internal)
                .unwrap_or(NixLog::Raw(line.to_string()));

        trace!(line);

        if should_trace {
            match log {
                NixLog::Raw(ref string) => info!("{string}"),
                NixLog::Internal(ref internal) => internal.trace(),
            }

            // We do this to ignore any "stop" logs, preventing flashing
            if let NixLog::Internal(ref log) = log {
                if let NixLogAction::Message {
                    level: _,
                    message: _,
                } = &log.action
                {
                    Span::current().pb_set_message(&DIGEST_RE.replace_all(&log.to_string(), "…"))
                }
            }
        }

        collect.push(log);
    }

    Ok(collect)
}

pub struct CommandTracer<'a> {
    command: &'a mut Command,
    log_stderr: bool,
}

impl<'a> From<&'a mut Command> for CommandTracer<'a> {
    fn from(command: &'a mut Command) -> Self {
        CommandTracer {
            command,
            log_stderr: false,
        }
    }
}

pub trait StreamTracing {
    async fn execute(self) -> Result<(ExitStatus, Vec<NixLog>, Vec<NixLog>), HiveLibError>;
    fn log_stderr(&mut self, log: bool) -> &mut Self;
}

impl<'a> StreamTracing for CommandTracer<'a> {
    async fn execute(self) -> Result<(ExitStatus, Vec<NixLog>, Vec<NixLog>), HiveLibError> {
        let mut child = self
            .command
            .args(["--log-format", "internal-json"])
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(HiveLibError::SpawnFailed)?;

        let stdout_handle = child.stdout.take().ok_or(HiveLibError::NoHandle)?;
        let stderr_handle = child.stderr.take().ok_or(HiveLibError::NoHandle)?;

        let stderr_task = tokio::spawn(handle_io(stderr_handle, self.log_stderr).in_current_span());
        let stdout_task = tokio::spawn(handle_io(stdout_handle, false));

        let handle =
            tokio::spawn(async move { child.wait().await.map_err(HiveLibError::SpawnFailed) });

        let (result, stdout, stderr) =
            tokio::try_join!(handle, stdout_task, stderr_task).map_err(HiveLibError::JoinError)?;

        Ok((result?, stdout?, stderr?))
    }

    fn log_stderr(&mut self, log: bool) -> &mut Self {
        self.log_stderr = log;
        self
    }
}

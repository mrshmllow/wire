use lazy_static::lazy_static;
use regex::Regex;
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncRead};
use tracing::{error, info, trace, Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::hive::node::NodeName;
use crate::nix_log::{InternalNixLog, NixLog, NixLogAction, Trace};
use crate::HiveLibError;

lazy_static! {
    static ref DIGEST_RE: Regex = Regex::new(r"[0-9a-z]{32}").unwrap();
}

pub enum EvalGoal<'a> {
    Inspect,
    GetTopLevel(&'a NodeName),
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

pub fn get_eval_command(path: PathBuf, goal: EvalGoal) -> tokio::process::Command {
    let runtime = match env::var_os("WIRE_RUNTIME") {
        Some(runtime) => runtime.into_string().unwrap(),
        None => panic!("WIRE_RUNTIME environment variable not set"),
    };

    if !check_nix_available() {
        panic!("nix is not available on this system");
    }

    let canon_path = path.canonicalize().unwrap();

    let mut command = tokio::process::Command::new("nix");
    command.args(["--extra-experimental-features", "nix-command"]);
    command.args(["--extra-experimental-features", "flakes"]);
    command.args(["eval", "--json", "--impure", "--show-trace", "--expr"]);

    command.arg(format!(
        "let evaluate = import {runtime}/evaluate.nix; hive = evaluate {{hive = {hive}; path = {path}; nixosConfigurations = {nixosConfigurations}; nixpkgs = {nixpkgs};}}; in {goal}",
        hive = match canon_path.ends_with("flake.nix") {
            true => format!("(builtins.getFlake \"git+file://{path}\").colmena",
                path = canon_path.parent().unwrap().to_str().unwrap(),
            ),
            false => format!("import {path}", path = canon_path.to_str().unwrap()),
        },
        nixosConfigurations = match canon_path.ends_with("flake.nix") { 
            true => format!("(builtins.getFlake \"git+file://{path}\").nixosConfigurations or {{}}",
                path = canon_path.parent().unwrap().to_str().unwrap()
            ),
            false => "{}".to_string(),
        },
        nixpkgs = match canon_path.ends_with("flake.nix") { 
            true => format!("(builtins.getFlake \"git+file://{path}\").inputs.nixpkgs.outPath or null",
                path = canon_path.parent().unwrap().to_str().unwrap()
            ),
            false => "null".to_string(),
        },
        path = canon_path.to_str().unwrap(),
        goal = match goal {
            EvalGoal::Inspect => "hive.inspect".to_string(),
            EvalGoal::GetTopLevel(node) => format!("hive.getTopLevel \"{node}\"", node = node),
        }
    ));

    command
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
        .map_err(HiveLibError::SpawnFailed)?
    {
        let log =
            serde_json::from_str::<InternalNixLog>(line.strip_prefix("@nix ").unwrap_or(&line))
                .map(NixLog::Internal)
                .unwrap_or(NixLog::Raw(line.to_string()));

        // Throw out stop logs
        if let NixLog::Internal(InternalNixLog {
            action: NixLogAction::Stop,
        }) = log
        {
            continue;
        }

        trace!(line);

        if should_trace {
            match log {
                NixLog::Raw(ref string) => info!("{string}"),
                NixLog::Internal(ref internal) => internal.trace(),
            }

            Span::current().pb_set_message(&DIGEST_RE.replace_all(&log.to_string(), "…"));
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
            .map_err(HiveLibError::SpawnFailed)?;

        let stdout_handle = child.stdout.take().ok_or(HiveLibError::NoHandle)?;
        let stderr_handle = child.stderr.take().ok_or(HiveLibError::NoHandle)?;

        let stderr_task = tokio::spawn(handle_io(stderr_handle, log_stderr).in_current_span());
        let stdout_task = tokio::spawn(handle_io(stdout_handle, false));

        let handle =
            tokio::spawn(async move { child.wait().await.map_err(HiveLibError::SpawnFailed) });

        let (result, stdout, stderr) =
            tokio::try_join!(handle, stdout_task, stderr_task).map_err(HiveLibError::JoinError)?;

        Ok((result?, stdout?, stderr?))
    }
}

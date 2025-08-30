use std::{
    collections::VecDeque,
    process::{ExitStatus, Stdio},
    sync::Arc,
};

use tokio::{
    io::BufReader,
    process::{Child, Command},
    sync::Mutex,
    task::JoinSet,
};
use tracing::debug;

use crate::{
    Target,
    commands::{ChildOutputMode, WireCommand, WireCommandChip},
    errors::{DetachedError, HiveLibError},
    nix_log::NixLog,
};

pub(crate) struct LocalCommand<'t> {
    target: &'t Target,
    output_mode: Arc<ChildOutputMode>,
}

pub(crate) struct LocalChildChip {
    error_collection: Arc<Mutex<VecDeque<String>>>,
    child: Child,
    joinset: JoinSet<()>,
    command_string: String
}

impl<'t> WireCommand<'t> for LocalCommand<'t> {
    type ChildChip = LocalChildChip;

    async fn spawn_new(
        target: &'t Target,
        output_mode: ChildOutputMode,
    ) -> Result<Self, crate::errors::HiveLibError> {
        let output_mode = Arc::new(output_mode);

        Ok(Self {
            target,
            output_mode,
        })
    }

    /// `_keep_stdin_open` has no effect, unimplemented
    fn run_command<S: AsRef<str>>(
        &mut self,
        command_string: S,
        _keep_stdin_open: bool,
        local: bool,
    ) -> Result<Self::ChildChip, crate::errors::HiveLibError> {
        let mut command = if local {
            let mut command = Command::new("sh");

            command.arg("-c");

            command
        } else {
            create_sync_ssh_command(self.target)?
        };

        command.arg(command_string.as_ref());

        if matches!(*self.output_mode, ChildOutputMode::Nix) {
            command.args(["--log-format", "internal-json"]);
        }

        command.stdin(Stdio::null());
        command.stderr(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.kill_on_drop(true);

        let mut child = command.spawn().unwrap();
        let error_collection = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(10)));

        let stdout_handle = child
            .stdout
            .take()
            .ok_or(HiveLibError::DetachedError(DetachedError::NoHandle))?;
        let stderr_handle = child
            .stderr
            .take()
            .ok_or(HiveLibError::DetachedError(DetachedError::NoHandle))?;

        let mut joinset = JoinSet::new();

        joinset.spawn(handle_io(
            stderr_handle,
            self.output_mode.clone(),
            error_collection.clone(),
        ));
        joinset.spawn(handle_io(
            stdout_handle,
            self.output_mode.clone(),
            error_collection.clone(),
        ));

        Ok(LocalChildChip {
            error_collection,
            child,
            joinset,
            command_string: command_string.as_ref().to_string()
        })
    }
}

impl WireCommandChip for LocalChildChip {
    type ExitStatus = ExitStatus;

    async fn wait_till_success(mut self) -> Result<Self::ExitStatus, DetachedError> {
        let status = self.child.wait().await.unwrap();
        let _ = self
            .joinset
            .join_all()
            .await;

        let mut collection = self.error_collection.lock().await;

        if !status.success() {
            let logs = collection.make_contiguous().join("\n");

            return Err(DetachedError::CommandFailed {
                command_ran: self.command_string,
                logs,
                code: match status.code() {
                    Some(code) => format!("code {code}"),
                    None => "no exit code".to_string()
                }
            });
        }

        Ok(status)
    }

    /// Unimplemented until needed.
    async fn write_stdin(&self, _data: Vec<u8>) -> Result<(), HiveLibError> {
        Ok(())
    }
}

pub async fn handle_io<R>(
    reader: R,
    output_mode: Arc<ChildOutputMode>,
    collection: Arc<Mutex<VecDeque<String>>>,
)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut io_reader = tokio::io::AsyncBufReadExt::lines(BufReader::new(reader));

    while let Some(line) = io_reader.next_line().await.unwrap() {
        let log = output_mode.trace(line.to_string());

        if let Some(NixLog::Internal(log)) = log {
            if let Some(message) = log.is_error_ish() {
                let mut queue = collection.lock().await;
                // add at most 10 message to the front, drop the rest.
                queue.push_front(message);
                queue.truncate(10);
            }
        }
    }

    debug!("io_handler: goodbye!");
}

fn create_sync_ssh_command(target: &Target) -> Result<Command, HiveLibError> {
    let mut command = Command::new("ssh");

    command.args(["-l", target.user.as_ref()]);
    command.arg(target.get_preffered_host()?.as_ref());
    command.args(["-p", &target.port.to_string()]);

    Ok(command)
}

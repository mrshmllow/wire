use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};
use nix::{
    poll::{PollFd, PollFlags, PollTimeout, poll},
    unistd::{pipe as posix_pipe, read as posix_read, write as posix_write},
};
use portable_pty::{NativePtySystem, PtySize};
use rand::distr::Alphabetic;
use std::sync::mpsc::{self, Sender};
use std::{
    io::{Read, Write},
    os::fd::{AsFd, OwnedFd},
    sync::Arc,
};
use tracing::{debug, error, info, trace, warn};

use crate::errors::DetachedError;
use crate::{
    commands::{ChildOutputMode, WireCommand, WireCommandChip},
    errors::HiveLibError,
    hive::node::Target,
};

type MasterWriter = Box<dyn Write + Send>;
type MasterReader = Box<dyn Read + Send>;
type Child = Box<dyn portable_pty::Child + Send + Sync>;

pub(crate) struct RemoteNewCommand<'t> {
    target: &'t Target,
    output_mode: Arc<ChildOutputMode>,
    quit_needle: Arc<String>,
    start_needle: Arc<String>,
}

pub(crate) struct RemoteChildChip {
    child: Child,

    cancel_stdin_pipe_w: OwnedFd,
    write_stdin_pipe_w: OwnedFd,
}

/// the underlying command began
const THREAD_BEGAN_SIGNAL: &[u8; 1] = b"b";
const THREAD_QUIT_SIGNAL: &[u8; 1] = b"q";

impl<'t> WireCommand<'t> for RemoteNewCommand<'t> {
    type ChildChip = RemoteChildChip;

    async fn spawn_new(
        target: &'t Target,
        output_mode: ChildOutputMode,
    ) -> Result<RemoteNewCommand<'t>, HiveLibError> {
        let output_mode = Arc::new(output_mode);
        let tmp_prefix = rand::distr::SampleString::sample_string(&Alphabetic, &mut rand::rng(), 5);
        let quit_needle = Arc::new(format!("{tmp_prefix}_WIRE_QUIT"));
        let start_needle = Arc::new(format!("{tmp_prefix}_WIRE_START"));

        Ok(Self {
            target,
            output_mode,
            quit_needle,
            start_needle,
        })
    }

    fn run_command<S: AsRef<str>>(
        &mut self,
        command_string: S,
        keep_stdin_open: bool,
    ) -> Result<Self::ChildChip, crate::errors::HiveLibError> {
        warn!(
            "Please authenticate for \"sudo {}\"",
            command_string.as_ref()
        );

        let pty_system = NativePtySystem::default();
        let pty_pair = portable_pty::PtySystem::openpty(&pty_system, PtySize::default()).unwrap();

        if let Some(fd) = pty_pair.master.as_raw_fd() {
            // convert raw fd to a BorrowedFd
            // safe as `fd` is dropped well before `pty_pair.master`
            let fd = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(fd) };
            let mut termios = tcgetattr(fd)
                .map_err(|x| HiveLibError::DetachedError(DetachedError::TermAttrs(x)))?;

            termios.local_flags &= !LocalFlags::ECHO;
            // Key agent does not work well without canonical mode
            termios.local_flags &= !LocalFlags::ICANON;
            // Actually quit
            termios.local_flags &= !LocalFlags::ISIG;

            tcsetattr(fd, SetArg::TCSANOW, &termios)
                .map_err(|x| HiveLibError::DetachedError(DetachedError::TermAttrs(x)))?;
        }

        let command_string = &format!(
            "echo '{}' && {command} && echo '{}'",
            self.start_needle,
            self.quit_needle,
            command = command_string.as_ref()
        );

        debug!("{command_string}");

        let mut command = create_sync_ssh_command(self.target)?;

        command.args([
            // force ssh to use our pesudo terminal
            "-tt",
            &format!("sudo -u root sh -c \"{command_string}\""),
        ]);

        let child = pty_pair
            .slave
            .spawn_command(command)
            .map_err(|x| HiveLibError::DetachedError(DetachedError::PortablePty(x)))?;

        // Release any handles owned by the slave: we don't need it now
        // that we've spawned the child.
        drop(pty_pair.slave);

        let reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|x| HiveLibError::DetachedError(DetachedError::PortablePty(x)))?;
        let master_writer = pty_pair
            .master
            .take_writer()
            .map_err(|x| HiveLibError::DetachedError(DetachedError::PortablePty(x)))?;

        let stdout_quit_needle = self.quit_needle.clone();
        let stdout_start_needle = self.start_needle.clone();
        let stdout_output_mode = self.output_mode.clone();

        let (began_tx, began_rx) = mpsc::channel::<()>();

        let (write_pipe_r, write_pipe_w) =
            posix_pipe().map_err(|x| HiveLibError::DetachedError(DetachedError::PosixPipe(x)))?;
        let (cancel_pipe_r, cancel_pipe_w) =
            posix_pipe().map_err(|x| HiveLibError::DetachedError(DetachedError::PosixPipe(x)))?;

        std::thread::spawn(move || {
            dynamic_watch_sudo_stdout(
                &began_tx,
                reader,
                &stdout_quit_needle,
                &stdout_start_needle,
                &stdout_output_mode,
            )
        });
        std::thread::spawn(move || {
            watch_stdin_from_user(&cancel_pipe_r, master_writer, &write_pipe_r)
        });

        info!("Setup threads");

        let () = began_rx
            .recv()
            .map_err(|x| HiveLibError::DetachedError(DetachedError::RecvError(x)))?;

        if keep_stdin_open {
            trace!("Sending THREAD_BEGAN_SIGNAL");

            posix_write(&cancel_pipe_w, THREAD_BEGAN_SIGNAL)
                .map_err(|x| HiveLibError::DetachedError(DetachedError::PosixPipe(x)))?;
        } else {
            trace!("Sending THREAD_QUIT_SIGNAL");

            posix_write(&cancel_pipe_w, THREAD_QUIT_SIGNAL)
                .map_err(|x| HiveLibError::DetachedError(DetachedError::PosixPipe(x)))?;
        }

        Ok(RemoteChildChip::new(child, cancel_pipe_w, write_pipe_w))
    }
}

impl RemoteChildChip {
    fn new(child: Child, cancel_stdin_pipe_w: OwnedFd, write_stdin_pipe_w: OwnedFd) -> Self {
        Self {
            child,
            cancel_stdin_pipe_w,
            write_stdin_pipe_w,
        }
    }
}

impl WireCommandChip for RemoteChildChip {
    type ExitStatus = portable_pty::ExitStatus;

    async fn get_status(mut self) -> Result<Self::ExitStatus, HiveLibError> {
        info!("trying to grab status...");

        drop(self.write_stdin_pipe_w);

        let exit_status = tokio::task::spawn_blocking(move || self.child.wait())
            .await
            .map_err(|x| HiveLibError::DetachedError(DetachedError::JoinError(x)))?
            .map_err(|x| HiveLibError::DetachedError(DetachedError::WaitForStatus(x)))?;

        posix_write(&self.cancel_stdin_pipe_w, THREAD_QUIT_SIGNAL)
            .map_err(|x| HiveLibError::DetachedError(DetachedError::PosixPipe(x)))?;

        Ok(exit_status)
    }

    async fn write_stdin(&self, data: Vec<u8>) -> Result<(), HiveLibError> {
        trace!("Writing {} bytes to stdin", data.len());

        posix_write(&self.write_stdin_pipe_w, &data)
            .map_err(|x| HiveLibError::DetachedError(DetachedError::PosixPipe(x)))?;

        Ok(())
    }
}

fn create_sync_ssh_command(target: &Target) -> Result<portable_pty::CommandBuilder, HiveLibError> {
    let mut command = portable_pty::CommandBuilder::new("ssh");

    command.args(["-l", target.user.as_ref()]);
    command.arg(target.get_preffered_host()?.as_ref());
    command.args(["-p", &target.port.to_string()]);

    Ok(command)
}

/// Cancels on `"WIRE_BEGIN"` written to `reader`
fn dynamic_watch_sudo_stdout(
    began_tx: &Sender<()>,
    mut reader: MasterReader,
    quit_needle: &Arc<String>,
    start_needle: &Arc<String>,
    output_mode: &Arc<ChildOutputMode>,
) -> Result<(), DetachedError> {
    let mut buffer = [0u8; 1024];
    let mut stdout = std::io::stdout();
    let mut began = false;

    'outer: loop {
        match reader.read(&mut buffer) {
            Ok(0) => break 'outer,
            Ok(n) => {
                let new_data = String::from_utf8_lossy(&buffer[..n]);

                for line in new_data.split_inclusive('\n') {
                    trace!("line: {line}");

                    if line.contains(start_needle.as_ref()) {
                        debug!("{start_needle} was found, switching mode...");
                        let _ = began_tx.send(());
                        began = true;
                    }

                    if line.contains(quit_needle.as_ref()) {
                        info!("{quit_needle} was found, breaking...");
                        break 'outer;
                    }

                    if began {
                        output_mode.trace(line.to_string());
                    } else {
                        stdout
                            .write_all(new_data.as_bytes())
                            .map_err(DetachedError::WritingClientStdout)?;
                        stdout.flush().map_err(DetachedError::WritingClientStdout)?;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from PTY: {e}");
                break;
            }
        }
    }

    let _ = began_tx.send(());
    info!("stdout: goodbye");

    Ok(())
}

/// Exits on any data written to `cancel_pipe_r`
fn watch_stdin_from_user(
    cancel_pipe_r: &OwnedFd,
    mut master_writer: MasterWriter,
    write_pipe_r: &OwnedFd,
) -> Result<(), DetachedError> {
    const WRITER_POSITION: usize = 0;
    const SIGNAL_POSITION: usize = 1;
    const USER_POSITION: usize = 2;

    let mut buffer = [0u8; 1024];
    let stdin = std::io::stdin();
    let mut cancel_pipe_buf = [0u8; 1];

    let user_stdin_fd = std::os::fd::AsFd::as_fd(&stdin);
    let cancel_pipe_r_fd = cancel_pipe_r.as_fd();

    let mut all_fds = vec![
        PollFd::new(write_pipe_r.as_fd(), PollFlags::POLLIN),
        PollFd::new(cancel_pipe_r.as_fd(), PollFlags::POLLIN),
        PollFd::new(user_stdin_fd, PollFlags::POLLIN),
    ];

    loop {
        match poll(&mut all_fds, PollTimeout::NONE) {
            Ok(0) => {} // timeout, impossible
            Ok(_) => {
                // The user stdin pipe can be removed
                if all_fds.get(USER_POSITION).is_some() {
                    if let Some(events) = all_fds[USER_POSITION].revents() {
                        if events.contains(PollFlags::POLLIN) {
                            trace!("Got stdin from user...");
                            let n = posix_read(user_stdin_fd, &mut buffer)
                                .map_err(DetachedError::PosixPipe)?;
                            master_writer
                                .write_all(&buffer[..n])
                                .map_err(DetachedError::WritingMasterStdout)?;
                            master_writer
                                .flush()
                                .map_err(DetachedError::WritingMasterStdout)?;
                        }
                    }
                }

                if let Some(events) = all_fds[WRITER_POSITION].revents() {
                    if events.contains(PollFlags::POLLIN) {
                        trace!("Got stdin from writer...");
                        let n = posix_read(write_pipe_r, &mut buffer)
                            .map_err(DetachedError::PosixPipe)?;
                        master_writer
                            .write_all(&buffer[..n])
                            .map_err(DetachedError::WritingMasterStdout)?;
                        master_writer
                            .flush()
                            .map_err(DetachedError::WritingMasterStdout)?;
                    }
                }
                if let Some(events) = all_fds[SIGNAL_POSITION].revents() {
                    if events.contains(PollFlags::POLLIN) {
                        let n = posix_read(cancel_pipe_r_fd, &mut cancel_pipe_buf)
                            .map_err(DetachedError::PosixPipe)?;
                        let message = &cancel_pipe_buf[..n];

                        trace!("Got byte from signal pipe: {message:?}");

                        if message == THREAD_QUIT_SIGNAL {
                            return Ok(());
                        }

                        if message == THREAD_BEGAN_SIGNAL {
                            all_fds.remove(USER_POSITION);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Poll error: {e}");
                break;
            }
        }
    }

    debug!("stdin_thread: goodbye");
    Ok(())
}

use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};
use nix::{
    poll::{PollFd, PollFlags, PollTimeout, poll},
    unistd::{pipe as posix_pipe, read as posix_read, write as posix_write},
};
use portable_pty::{NativePtySystem, PtySize};
use rand::distr::Alphabetic;
use std::{
    io::{Read, Write},
    os::fd::{AsFd, OwnedFd},
    sync::Arc,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};

use crate::{
    commands::{ChildOutputMode, WireCommand, WireCommandChip},
    errors::HiveLibError,
    hive::node::Target,
};

pub(crate) struct RemoteNewCommand<'t> {
    target: &'t Target,
    output_mode: Arc<ChildOutputMode>,
    quit_needle: Arc<String>,

    cancel_stdin_pipe_r: Option<OwnedFd>,
    cancel_stdin_pipe_w: Option<OwnedFd>,
    write_stdin_pipe_r: Option<OwnedFd>,
    write_stdin_pipe_w: Option<OwnedFd>,
}

pub(crate) struct RemoteChildChip {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    cancel_token: CancellationToken,

    cancel_stdin_pipe_w: OwnedFd,
    write_stdin_pipe_w: OwnedFd,
}

impl<'t> WireCommand<'t> for RemoteNewCommand<'t> {
    type ChildChip = RemoteChildChip;

    async fn spawn_new(
        target: &'t Target,
        output_mode: ChildOutputMode,
    ) -> Result<RemoteNewCommand<'t>, HiveLibError> {
        let output_mode = Arc::new(output_mode);
        let tmp_prefix = rand::distr::SampleString::sample_string(&Alphabetic, &mut rand::rng(), 5);
        let quit_needle = Arc::new(format!("{tmp_prefix}_WIRE_QUIT"));
        let (pipe_r, pipe_w) = posix_pipe().unwrap();
        let (write_pipe_r, write_pipe_w) = posix_pipe().unwrap();

        Ok(Self {
            target,
            output_mode,
            quit_needle,
            cancel_stdin_pipe_r: Some(pipe_r),
            cancel_stdin_pipe_w: Some(pipe_w),
            write_stdin_pipe_r: Some(write_pipe_r),
            write_stdin_pipe_w: Some(write_pipe_w),
        })
    }

    fn run_command<S: AsRef<str>>(
        &mut self,
        command_string: S,
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
            let mut termios = tcgetattr(fd).unwrap();

            termios.local_flags &= !LocalFlags::ECHO;
            // Key agent does not work well without canonical mode
            termios.local_flags &= !LocalFlags::ICANON;
            // Actually quit
            termios.local_flags &= !LocalFlags::ISIG;

            tcsetattr(fd, SetArg::TCSANOW, &termios).unwrap();
        }

        let command_string = &format!(
            "echo 'WIRE_BEGIN' && {command} && echo '{}'",
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

        let mut child = pty_pair.slave.spawn_command(command).unwrap();

        // Release any handles owned by the slave: we don't need it now
        // that we've spawned the child.
        drop(pty_pair.slave);

        let reader = pty_pair.master.try_clone_reader().unwrap();
        let master_writer = pty_pair.master.take_writer().unwrap();
        let cancel_token = CancellationToken::new();

        let stdout_token = cancel_token.clone();
        let stdout_quit_needle = self.quit_needle.clone();
        let stdout_output_mode = self.output_mode.clone();
        let stdout_thread = std::thread::spawn(move || {
            dynamic_watch_sudo_stdout(
                &stdout_token,
                reader,
                &stdout_quit_needle,
                &stdout_output_mode,
            );
        });

        let pipe_r = self.cancel_stdin_pipe_r.take().unwrap();
        let pipe_w = self.cancel_stdin_pipe_w.take().unwrap();
        let write_pipe_r = self.write_stdin_pipe_r.take().unwrap();
        let write_pipe_w = self.write_stdin_pipe_w.take().unwrap();
        let stdin_thread = std::thread::spawn(move || {
            watch_stdin_from_user(&pipe_r, master_writer, &write_pipe_r);
        });

        info!("Setup threads");

        loop {
            if cancel_token.is_cancelled() {
                break;
            }
        }

        info!("Cancelled...");

        // posix_write(&self.cancel_stdin_pipe_w.unwrap(), b"x").unwrap();

        // stdin_thread.join().unwrap();
        // let a = stdout_thread.join().unwrap();
        // info!("Joined stdin");

        // let child = child.wait().unwrap();
        // info!("child!? {child}");

        Ok(RemoteChildChip::new(child, pipe_w, write_pipe_w))
    }

    async fn get_status(
        self,
        command_child: Self::ChildChip,
    ) -> Result<portable_pty::ExitStatus, crate::errors::HiveLibError> {
        todo!();
    }
}

impl RemoteChildChip {
    fn new(
        child: Box<dyn portable_pty::Child + Send + Sync>,
        cancel_stdin_pipe_w: OwnedFd,
        write_stdin_pipe_w: OwnedFd,
    ) -> Self {
        let cancel_token = CancellationToken::new();

        Self {
            child,
            cancel_token,
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
            .unwrap()
            .unwrap();

        posix_write(&self.cancel_stdin_pipe_w, b"x").unwrap();

        Ok(exit_status)
    }

    async fn write_stdin(&self, data: Vec<u8>) -> Result<(), HiveLibError> {
        trace!("Writing {} bytes to stdin", data.len());

        posix_write(&self.write_stdin_pipe_w, &data).unwrap();

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
    token: &CancellationToken,
    mut reader: Box<dyn Read + Send>,
    quit_needle: &Arc<String>,
    output_mode: &Arc<ChildOutputMode>,
) {
    let mut buffer = [0u8; 1024];
    let mut stdout = std::io::stdout();
    let mut began = false;

    'outer: loop {
        match reader.read(&mut buffer) {
            Ok(0) => break 'outer,
            Ok(n) => {
                let new_data = String::from_utf8_lossy(&buffer[..n]);
                // debug!("got {n}: {new_data}");

                for line in new_data.split_inclusive('\n') {
                    trace!("line: {line}");

                    if line.contains("WIRE_BEGIN") {
                        debug!("WIRE_BEGIN was found, switching mode...");
                        // sender.send(()).unwrap();
                        token.cancel();
                        began = true;
                    }

                    if line.contains(quit_needle.as_ref()) {
                        info!("{quit_needle} was found, breaking...");
                        break 'outer;
                    }

                    if began {
                        output_mode.trace(line.to_string());
                    } else {
                        stdout.write_all(new_data.as_bytes()).unwrap();
                        stdout.flush().unwrap();
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from PTY: {e}");
                break;
            }
        }
    }

    token.cancel();
    info!("stdout: goodbye");
}

/// Exits on any data written to `cancel_pipe_r`
fn watch_stdin_from_user(
    cancel_pipe_r: &OwnedFd,
    mut master_writer: Box<dyn Write + Send>,
    write_pipe_r: &OwnedFd,
) {
    let mut buffer = [0u8; 1024];
    let stdin = std::io::stdin();
    let mut pipe_buf = [0u8; 1];

    let stdin_fd = std::os::fd::AsFd::as_fd(&stdin);
    let pipe_r_fd = cancel_pipe_r.as_fd();

    let mut poll_fds = [
        PollFd::new(stdin_fd, PollFlags::POLLIN),
        PollFd::new(cancel_pipe_r.as_fd(), PollFlags::POLLIN),
        PollFd::new(write_pipe_r.as_fd(), PollFlags::POLLIN),
    ];

    loop {
        match poll(&mut poll_fds, PollTimeout::NONE) {
            Ok(0) => {} // timeout
            Ok(_) => {
                if let Some(events) = poll_fds[0].revents() {
                    if events.contains(PollFlags::POLLIN) {
                        debug!("Got stdin...");
                        let n = posix_read(stdin_fd, &mut buffer).unwrap();
                        // info!("Going to write: {}", String::from_utf8_lossy(&buffer[..n]));
                        master_writer.write_all(&buffer[..n]).unwrap();
                        master_writer.flush().unwrap();
                    }
                }
                if let Some(events) = poll_fds[1].revents() {
                    if events.contains(PollFlags::POLLIN) {
                        debug!("Stdin reader: Got cancel from cancel_pipe_r");
                        let _ = posix_read(pipe_r_fd, &mut pipe_buf);
                        break;
                    }
                }
                if let Some(events) = poll_fds[2].revents() {
                    if events.contains(PollFlags::POLLIN) {
                        debug!("Got stdin from writer...");
                        let n = posix_read(write_pipe_r, &mut buffer).unwrap();
                        // info!("Going to write: {}", String::from_utf8_lossy(&buffer[..n]));
                        master_writer.write_all(&buffer[..n]).unwrap();
                        master_writer.flush().unwrap();
                    }
                }
            }
            Err(e) => {
                error!("Poll error: {e}");
                break;
            }
        }
    }

    info!("stdin_thread: goodbye");
}

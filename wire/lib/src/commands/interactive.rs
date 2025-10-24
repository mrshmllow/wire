// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use aho_corasick::{AhoCorasick, PatternID};
use itertools::Itertools;
use nix::sys::termios::{LocalFlags, SetArg, Termios, tcgetattr, tcsetattr};
use nix::{
    poll::{PollFd, PollFlags, PollTimeout, poll},
    unistd::{pipe as posix_pipe, read as posix_read, write as posix_write},
};
use portable_pty::{CommandBuilder, NativePtySystem, PtyPair, PtySize};
use rand::distr::Alphabetic;
use std::collections::VecDeque;
use std::sync::mpsc::{self, Sender};
use std::sync::{Condvar, Mutex};
use std::thread::JoinHandle;
use std::{
    io::{Read, Write},
    os::fd::{AsFd, OwnedFd},
    sync::Arc,
};
use tracing::instrument;
use tracing::{Span, debug, error, info, trace, warn};

use crate::commands::CommandArguments;
use crate::commands::interactive_logbuffer::LogBuffer;
use crate::errors::CommandError;
use crate::{STDIN_CLOBBER_LOCK, SubCommandModifiers};
use crate::{
    commands::{ChildOutputMode, WireCommandChip},
    errors::HiveLibError,
    hive::node::Target,
};

type MasterWriter = Box<dyn Write + Send>;
type MasterReader = Box<dyn Read + Send>;
type Child = Box<dyn portable_pty::Child + Send + Sync>;

pub(crate) struct InteractiveChildChip {
    child: Child,

    cancel_stdin_pipe_w: OwnedFd,
    write_stdin_pipe_w: OwnedFd,

    stderr_collection: Arc<Mutex<VecDeque<String>>>,
    stdout_collection: Arc<Mutex<VecDeque<String>>>,

    original_command: String,

    completion_status: Arc<CompletionStatus>,
    stdout_handle: JoinHandle<Result<(), CommandError>>,
}

struct StdinTermiosAttrGuard(Termios);

struct CompletionStatus {
    completed: Mutex<bool>,
    success: Mutex<Option<bool>>,
    condvar: Condvar,
}

struct WatchStdoutArguments {
    began_tx: Sender<()>,
    reader: MasterReader,
    succeed_needle: Arc<Vec<u8>>,
    failed_needle: Arc<Vec<u8>>,
    start_needle: Arc<Vec<u8>>,
    output_mode: ChildOutputMode,
    stderr_collection: Arc<Mutex<VecDeque<String>>>,
    stdout_collection: Arc<Mutex<VecDeque<String>>>,
    completion_status: Arc<CompletionStatus>,
    span: Span,
    log_stdout: bool,
}

/// the underlying command began
const THREAD_BEGAN_SIGNAL: &[u8; 1] = b"b";
const THREAD_QUIT_SIGNAL: &[u8; 1] = b"q";

/// substitutes STDOUT with #$line. stdout is far less common than stderr.
const IO_SUBS: &str = "1> >(while IFS= read -r line; do echo \"#$line\"; done)";

#[instrument(skip_all, name = "run-int", fields(elevated = %arguments.elevated))]
pub(crate) fn interactive_command_with_env<S: AsRef<str>>(
    arguments: &CommandArguments<S>,
    envs: std::collections::HashMap<String, String>,
) -> Result<InteractiveChildChip, HiveLibError> {
    print_authenticate_warning(arguments)?;

    let (succeed_needle, failed_needle, start_needle) = create_needles();

    let pty_system = NativePtySystem::default();
    let pty_pair = portable_pty::PtySystem::openpty(&pty_system, PtySize::default()).unwrap();
    setup_master(&pty_pair)?;

    let command_string = &format!(
        "echo '{start}' && {command} {flags} {IO_SUBS} && echo '{succeed}' || echo '{failed}'",
        start = String::from_utf8_lossy(&start_needle),
        succeed = String::from_utf8_lossy(&succeed_needle),
        failed = String::from_utf8_lossy(&failed_needle),
        command = arguments.command_string.as_ref(),
        flags = match arguments.output_mode {
            ChildOutputMode::Nix => "--log-format internal-json",
            ChildOutputMode::Raw => "",
        }
    );

    debug!("{command_string}");

    let mut command = build_command(arguments, command_string)?;

    // give command all env vars
    for (key, value) in envs {
        command.env(key, value);
    }

    let clobber_guard = STDIN_CLOBBER_LOCK.lock().unwrap();
    let _guard = StdinTermiosAttrGuard::new().map_err(HiveLibError::CommandError)?;
    let child = pty_pair
        .slave
        .spawn_command(command)
        .map_err(|x| HiveLibError::CommandError(CommandError::PortablePty(x)))?;

    // Release any handles owned by the slave: we don't need it now
    // that we've spawned the child.
    drop(pty_pair.slave);

    let reader = pty_pair
        .master
        .try_clone_reader()
        .map_err(|x| HiveLibError::CommandError(CommandError::PortablePty(x)))?;
    let master_writer = pty_pair
        .master
        .take_writer()
        .map_err(|x| HiveLibError::CommandError(CommandError::PortablePty(x)))?;

    let stderr_collection = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(10)));
    let stdout_collection = Arc::new(Mutex::new(VecDeque::<String>::with_capacity(10)));
    let (began_tx, began_rx) = mpsc::channel::<()>();
    let completion_status = Arc::new(CompletionStatus::new());

    let stdout_handle = {
        let arguments = WatchStdoutArguments {
            began_tx,
            reader,
            succeed_needle: succeed_needle.clone(),
            failed_needle: failed_needle.clone(),
            start_needle: start_needle.clone(),
            output_mode: arguments.output_mode,
            stderr_collection: stderr_collection.clone(),
            stdout_collection: stdout_collection.clone(),
            completion_status: completion_status.clone(),
            span: Span::current(),
            log_stdout: arguments.log_stdout,
        };

        std::thread::spawn(move || dynamic_watch_sudo_stdout(arguments))
    };

    let (write_stdin_pipe_r, write_stdin_pipe_w) =
        posix_pipe().map_err(|x| HiveLibError::CommandError(CommandError::PosixPipe(x)))?;
    let (cancel_stdin_pipe_r, cancel_stdin_pipe_w) =
        posix_pipe().map_err(|x| HiveLibError::CommandError(CommandError::PosixPipe(x)))?;

    std::thread::spawn(move || {
        watch_stdin_from_user(
            &cancel_stdin_pipe_r,
            master_writer,
            &write_stdin_pipe_r,
            Span::current(),
        )
    });

    info!("Setup threads");

    let () = began_rx
        .recv()
        .map_err(|x| HiveLibError::CommandError(CommandError::RecvError(x)))?;

    drop(clobber_guard);

    if arguments.keep_stdin_open {
        trace!("Sending THREAD_BEGAN_SIGNAL");

        posix_write(&cancel_stdin_pipe_w, THREAD_BEGAN_SIGNAL)
            .map_err(|x| HiveLibError::CommandError(CommandError::PosixPipe(x)))?;
    } else {
        trace!("Sending THREAD_QUIT_SIGNAL");

        posix_write(&cancel_stdin_pipe_w, THREAD_QUIT_SIGNAL)
            .map_err(|x| HiveLibError::CommandError(CommandError::PosixPipe(x)))?;
    }

    Ok(InteractiveChildChip {
        child,
        cancel_stdin_pipe_w,
        write_stdin_pipe_w,
        stderr_collection,
        stdout_collection,
        original_command: arguments.command_string.as_ref().to_string(),
        completion_status,
        stdout_handle,
    })
}

fn print_authenticate_warning<S: AsRef<str>>(
    arguments: &CommandArguments<S>,
) -> Result<(), HiveLibError> {
    if !arguments.elevated {
        return Ok(());
    }

    eprintln!(
        "{} | Authenticate for \"sudo {}\":",
        arguments
            .target
            .map_or(Ok("localhost (!)".to_string()), |target| Ok(format!(
                "{}@{}:{}",
                target.user,
                target.get_preferred_host()?,
                target.port
            )))?,
        arguments.command_string.as_ref()
    );

    Ok(())
}

type Needles = (Arc<Vec<u8>>, Arc<Vec<u8>>, Arc<Vec<u8>>);

fn create_needles() -> Needles {
    let tmp_prefix = rand::distr::SampleString::sample_string(&Alphabetic, &mut rand::rng(), 5);

    (
        Arc::new(format!("{tmp_prefix}_W_Q").as_bytes().to_vec()),
        Arc::new(format!("{tmp_prefix}_W_F").as_bytes().to_vec()),
        Arc::new(format!("{tmp_prefix}_W_S").as_bytes().to_vec()),
    )
}

fn setup_master(pty_pair: &PtyPair) -> Result<(), HiveLibError> {
    if let Some(fd) = pty_pair.master.as_raw_fd() {
        // convert raw fd to a BorrowedFd
        // safe as `fd` is dropped well before `pty_pair.master`
        let fd = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(fd) };
        let mut termios =
            tcgetattr(fd).map_err(|x| HiveLibError::CommandError(CommandError::TermAttrs(x)))?;

        termios.local_flags &= !LocalFlags::ECHO;
        // Key agent does not work well without canonical mode
        termios.local_flags &= !LocalFlags::ICANON;
        // Actually quit
        termios.local_flags &= !LocalFlags::ISIG;

        tcsetattr(fd, SetArg::TCSANOW, &termios)
            .map_err(|x| HiveLibError::CommandError(CommandError::TermAttrs(x)))?;
    }

    Ok(())
}

fn build_command<S: AsRef<str>>(
    arguments: &CommandArguments<'_, S>,
    command_string: &String,
) -> Result<CommandBuilder, HiveLibError> {
    let mut command = if let Some(target) = arguments.target {
        let mut command = create_sync_ssh_command(target, arguments.modifiers)?;

        // force ssh to use our pesudo terminal
        command.arg("-tt");

        command
    } else {
        let mut command = portable_pty::CommandBuilder::new("sh");

        command.arg("-c");

        command
    };

    if arguments.elevated {
        command.arg(format!("sudo -u root -- sh -c '{command_string}'"));
    } else {
        command.arg(command_string);
    }

    Ok(command)
}

impl CompletionStatus {
    fn new() -> Self {
        CompletionStatus {
            completed: Mutex::new(false),
            success: Mutex::new(None),
            condvar: Condvar::new(),
        }
    }

    fn mark_completed(&self, was_successful: bool) {
        let mut completed = self.completed.lock().unwrap();
        let mut success = self.success.lock().unwrap();

        *completed = true;
        *success = Some(was_successful);

        self.condvar.notify_all();
    }

    fn wait(&self) -> Option<bool> {
        let mut completed = self.completed.lock().unwrap();

        while !*completed {
            completed = self.condvar.wait(completed).unwrap();
        }

        *self.success.lock().unwrap()
    }
}

impl WireCommandChip for InteractiveChildChip {
    type ExitStatus = (portable_pty::ExitStatus, String);

    #[instrument(skip_all)]
    async fn wait_till_success(mut self) -> Result<Self::ExitStatus, CommandError> {
        drop(self.write_stdin_pipe_w);

        let exit_status = tokio::task::spawn_blocking(move || self.child.wait())
            .await
            .map_err(CommandError::JoinError)?
            .map_err(CommandError::WaitForStatus)?;

        debug!("exit_status: {exit_status:?}");

        self.stdout_handle
            .join()
            .map_err(|_| CommandError::ThreadPanic)??;
        let success = self.completion_status.wait();
        let _ = posix_write(&self.cancel_stdin_pipe_w, THREAD_QUIT_SIGNAL);

        if let Some(true) = success {
            let logs = self
                .stdout_collection
                .lock()
                .unwrap()
                .iter()
                .rev()
                .map(|x| x.trim())
                .join("\n");

            return Ok((exit_status, logs));
        }

        debug!("child did not succeed");

        let logs = self
            .stderr_collection
            .lock()
            .unwrap()
            .iter()
            .rev()
            .join("\n");

        Err(CommandError::CommandFailed {
            command_ran: self.original_command,
            logs,
            code: format!("code {}", exit_status.exit_code()),
            reason: match success {
                Some(_) => "marked-unsuccessful",
                None => "child-crashed-before-succeeding",
            },
        })
    }

    async fn write_stdin(&mut self, data: Vec<u8>) -> Result<(), HiveLibError> {
        trace!("Writing {} bytes to stdin", data.len());

        posix_write(&self.write_stdin_pipe_w, &data)
            .map_err(|x| HiveLibError::CommandError(CommandError::PosixPipe(x)))?;

        Ok(())
    }
}

impl StdinTermiosAttrGuard {
    fn new() -> Result<Self, CommandError> {
        let stdin = std::io::stdin();
        let stdin_fd = stdin.as_fd();

        let mut termios = tcgetattr(stdin_fd).map_err(CommandError::TermAttrs)?;
        let original_termios = termios.clone();

        termios.local_flags &= !(LocalFlags::ECHO | LocalFlags::ICANON);
        tcsetattr(stdin_fd, SetArg::TCSANOW, &termios).map_err(CommandError::TermAttrs)?;

        Ok(StdinTermiosAttrGuard(original_termios))
    }
}

impl Drop for StdinTermiosAttrGuard {
    fn drop(&mut self) {
        let stdin = std::io::stdin();
        let stdin_fd = stdin.as_fd();

        let _ = tcsetattr(stdin_fd, SetArg::TCSANOW, &self.0);
    }
}

fn create_sync_ssh_command(
    target: &Target,
    modifiers: SubCommandModifiers,
) -> Result<portable_pty::CommandBuilder, HiveLibError> {
    let mut command = portable_pty::CommandBuilder::new("ssh");
    command.args(target.create_ssh_args(modifiers, false, false));
    command.arg(target.get_preferred_host()?.to_string());
    Ok(command)
}

#[instrument(skip_all, name = "log", parent = arguments.span)]
fn dynamic_watch_sudo_stdout(arguments: WatchStdoutArguments) -> Result<(), CommandError> {
    let WatchStdoutArguments {
        began_tx,
        mut reader,
        succeed_needle,
        failed_needle,
        start_needle,
        output_mode,
        stdout_collection,
        stderr_collection,
        completion_status,
        log_stdout,
        ..
    } = arguments;

    let aho_corasick = AhoCorasick::builder()
        .ascii_case_insensitive(false)
        .match_kind(aho_corasick::MatchKind::LeftmostFirst)
        .build([
            start_needle.as_ref(),
            succeed_needle.as_ref(),
            failed_needle.as_ref(),
        ])
        .unwrap();

    let mut buffer = [0u8; 1024];
    let mut stderr = std::io::stderr();
    let mut began = false;
    let mut log_buffer = LogBuffer::new();
    let mut raw_mode_buffer = Vec::new();
    let mut belled = false;

    'outer: loop {
        match reader.read(&mut buffer) {
            Ok(0) => break 'outer,
            Ok(n) => {
                if !began {
                    if handle_rawmode_data(
                        &mut stderr,
                        &buffer,
                        n,
                        &mut raw_mode_buffer,
                        &aho_corasick,
                        &began_tx,
                    )? {
                        began = true;
                        continue;
                    }

                    if belled {
                        continue;
                    }

                    stderr
                        .write(b"\x07")
                        .map_err(CommandError::WritingClientStderr)?;
                    stderr.flush().map_err(CommandError::WritingClientStderr)?;

                    belled = true;

                    continue;
                }

                log_buffer.process_slice(&buffer[..n]);

                while let Some(mut line) = log_buffer.next_line() {
                    let searched = aho_corasick
                        .find_iter(&line)
                        .map(|x| x.pattern())
                        .collect::<Vec<_>>();

                    if searched.iter().any(|x| x == &PatternID::must(1)) {
                        debug!("succeed needle was found, marking child as succeeding.");
                        completion_status.mark_completed(true);
                        break 'outer;
                    }

                    if searched.iter().any(|x| x == &PatternID::must(2)) {
                        debug!("failed needle was found, elevated child did not succeed.");
                        completion_status.mark_completed(false);
                        break 'outer;
                    }

                    if line.starts_with(b"#") {
                        let stripped = &mut line[1..];

                        if log_stdout {
                            output_mode.trace_slice(stripped);
                        }

                        let mut queue = stdout_collection.lock().unwrap();
                        queue.push_front(String::from_utf8_lossy(stripped).to_string());
                        continue;
                    }

                    let log = output_mode.trace_slice(&mut line);

                    if let Some(error_msg) = log {
                        let mut queue = stderr_collection.lock().unwrap();

                        // add at most 20 message to the front, drop the rest.
                        queue.push_front(error_msg);
                        queue.truncate(20);
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

    // failsafe if there were errors or the reader stopped
    if !*completion_status.completed.lock().unwrap() {
        completion_status.mark_completed(false);
    }

    debug!("stdout: goodbye");

    Ok(())
}

/// Returns Ok(true) if the data indicates the command was started
fn handle_rawmode_data<W: std::io::Write>(
    stderr: &mut W,
    buffer: &[u8],
    n: usize,
    raw_mode_buffer: &mut Vec<u8>,
    aho_corasick: &AhoCorasick,
    began_tx: &Sender<()>,
) -> Result<bool, CommandError> {
    raw_mode_buffer.extend_from_slice(&buffer[..n]);

    if aho_corasick
        .find_iter(&raw_mode_buffer)
        .any(|x| x.pattern() == PatternID::must(0))
    {
        debug!("start needle was found, switching mode...");
        let _ = began_tx.send(());
        return Ok(true);
    }

    stderr
        .write_all(&buffer[..n])
        .map_err(CommandError::WritingClientStderr)?;

    stderr.flush().map_err(CommandError::WritingClientStderr)?;

    Ok(false)
}

/// Exits on any data written to `cancel_pipe_r`
#[instrument(skip_all, level = "trace", parent = span)]
fn watch_stdin_from_user(
    cancel_pipe_r: &OwnedFd,
    mut master_writer: MasterWriter,
    write_pipe_r: &OwnedFd,
    span: Span,
) -> Result<(), CommandError> {
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
                if all_fds.get(USER_POSITION).is_some()
                    && let Some(events) = all_fds[USER_POSITION].revents()
                    && events.contains(PollFlags::POLLIN)
                {
                    trace!("Got stdin from user...");
                    let n =
                        posix_read(user_stdin_fd, &mut buffer).map_err(CommandError::PosixPipe)?;
                    master_writer
                        .write_all(&buffer[..n])
                        .map_err(CommandError::WritingMasterStdout)?;
                    master_writer
                        .flush()
                        .map_err(CommandError::WritingMasterStdout)?;
                }

                if let Some(events) = all_fds[WRITER_POSITION].revents()
                    && events.contains(PollFlags::POLLIN)
                {
                    trace!("Got stdin from writer...");
                    let n =
                        posix_read(write_pipe_r, &mut buffer).map_err(CommandError::PosixPipe)?;
                    master_writer
                        .write_all(&buffer[..n])
                        .map_err(CommandError::WritingMasterStdout)?;
                    master_writer
                        .flush()
                        .map_err(CommandError::WritingMasterStdout)?;
                }

                if let Some(events) = all_fds[SIGNAL_POSITION].revents()
                    && events.contains(PollFlags::POLLIN)
                {
                    let n = posix_read(cancel_pipe_r_fd, &mut cancel_pipe_buf)
                        .map_err(CommandError::PosixPipe)?;
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
            Err(e) => {
                error!("Poll error: {e}");
                break;
            }
        }
    }

    debug!("stdin_thread: goodbye");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{assert_matches::assert_matches, sync::mpsc::TryRecvError};

    #[test]
    fn test_rawmode_data() {
        let aho_corasick = AhoCorasick::builder()
            .ascii_case_insensitive(false)
            .match_kind(aho_corasick::MatchKind::LeftmostFirst)
            .build(["START_NEEDLE"])
            .unwrap();
        let mut stderr = vec![];
        let (began_tx, began_rx) = mpsc::channel::<()>();

        // each "Bla" is 4 bytes.
        let buffer = "bla bla bla START_NEEDLE bla bla bla".as_bytes();
        let mut raw_mode_buffer = vec![];

        // handle 1 "bla"
        assert_matches!(
            handle_rawmode_data(
                &mut stderr,
                buffer,
                4,
                &mut raw_mode_buffer,
                &aho_corasick,
                &began_tx
            ),
            Ok(false)
        );
        assert_eq!(raw_mode_buffer, b"bla ");
        assert_matches!(began_rx.try_recv(), Err(TryRecvError::Empty));

        let buffer = &buffer[4..];

        // handle 2 "bla"'s and half a "START_NEEDLE"
        let n = 4 + 4 + 6;
        assert_matches!(
            handle_rawmode_data(
                &mut stderr,
                buffer,
                n,
                &mut raw_mode_buffer,
                &aho_corasick,
                &began_tx
            ),
            Ok(false)
        );
        assert_matches!(began_rx.try_recv(), Err(TryRecvError::Empty));
        assert_eq!(raw_mode_buffer, b"bla bla bla START_");

        let buffer = &buffer[n..];

        // handle rest of the data
        let n = buffer.len();
        assert_matches!(
            handle_rawmode_data(
                &mut stderr,
                buffer,
                n,
                &mut raw_mode_buffer,
                &aho_corasick,
                &began_tx
            ),
            Ok(true)
        );
        assert_matches!(began_rx.try_recv(), Ok(()));
        assert_eq!(raw_mode_buffer, b"bla bla bla START_NEEDLE bla bla bla");
    }
}

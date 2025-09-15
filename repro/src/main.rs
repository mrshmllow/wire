use std::sync::mpsc::channel;

use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

fn main() {
    println!("Hello from repro");

    let pty_system = NativePtySystem::default();

    let pair = pty_system.openpty(PtySize::default()).unwrap();

    if let Some(fd) = pair.master.as_raw_fd() {
        // convert raw fd to a BorrowedFd
        // safe as `fd` is dropped well before `pty_pair.master`
        let fd = unsafe { std::os::unix::io::BorrowedFd::borrow_raw(fd) };
        let mut termios = tcgetattr(fd).unwrap();

        termios.local_flags &= !LocalFlags::ECHO;
        // // Key agent does not work well without canonical mode
        // termios.local_flags &= !LocalFlags::ICANON;
        // // Actually quit
        // termios.local_flags &= !LocalFlags::ISIG;

        tcsetattr(fd, SetArg::TCSANOW, &termios).unwrap();
    }

    let mut cmd = CommandBuilder::new("ssh");
    cmd.args(["-l", "root"]);
    cmd.arg("two");
    cmd.args(["-p", "22"]);
    // cmd.arg("owner@two");
    cmd.arg("-tt");
    cmd.arg("sudo -u root -- sh -c 'key_agent 1000'");
    // cmd.arg("key_agent 1");

    // let mut cmd = CommandBuilder::new("cat");
    let mut child = pair.slave.spawn_command(cmd).unwrap();

    drop(pair.slave);

    let (tx, rx) = channel();
    let mut reader = pair.master.try_clone_reader().unwrap();

    std::thread::spawn(move || {
        let mut s = String::new();
        reader.read_to_string(&mut s).unwrap();
        tx.send(s).unwrap();
    });

    {
        let mut writer = pair.master.take_writer().unwrap();

        println!("sending data...");
        let to_write = b"a".repeat(1000);
        // to_write.push(b'\n');

        std::thread::spawn(move || {
            writer.write_all(&to_write).unwrap();
        });
    }

    println!("child status: {:?}", child.wait().unwrap());

    // Take care to drop the master after our processes are
    // done, as some platforms get unhappy if it is dropped
    // sooner than that.
    drop(pair.master);

    // Now wait for the output to be read by our reader thread
    let output = rx.recv().unwrap();

    // We print with escapes escaped because the windows conpty
    // implementation synthesizes title change escape sequences
    // in the output stream and it can be confusing to see those
    // printed out raw in another terminal.
    print!("output: ");
    for c in output.escape_debug() {
        print!("{}", c);
    }
}

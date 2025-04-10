use std::{fs, io, path::Path, process::Command};

use tempdir::TempDir;

pub fn make_flake_sandbox(path: &Path) -> Result<TempDir, io::Error> {
    let tmp_dir = TempDir::new("wire-test")?;

    let a = Command::new("ls")
        .arg("-a")
        .current_dir(tmp_dir.path())
        .output()?;

    println!("{a:?}");

    Command::new("git")
        .arg("init")
        .current_dir(tmp_dir.path())
        .spawn()?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let to = tmp_dir.as_ref().join(entry.file_name());

        fs::copy(entry.path(), &to)?;

        Command::new("git")
            .arg("add")
            .arg(to)
            .current_dir(tmp_dir.path())
            .spawn()?;
    }

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(tmp_dir.path())
        .spawn()?;

    Command::new("nix")
        .args(["flake", "lock"])
        .current_dir(tmp_dir.path())
        .spawn()?;

    Ok(tmp_dir)
}

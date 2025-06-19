use std::{fs, io, path::Path, process::Command};

use tempdir::TempDir;

pub fn make_flake_sandbox(path: &Path) -> Result<TempDir, io::Error> {
    let tmp_dir = TempDir::new("wire-test")?;

    Command::new("git")
        .args(["init", "-b", "tmp"])
        .current_dir(tmp_dir.path())
        .status()?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;

        fs::copy(entry.path(), tmp_dir.as_ref().join(entry.file_name()))?;
    }

    Command::new("git")
        .args(["add", "-A"])
        .current_dir(tmp_dir.path())
        .status()?;

    Command::new("nix")
        .args(["flake", "lock"])
        .current_dir(tmp_dir.path())
        .status()?;

    Ok(tmp_dir)
}

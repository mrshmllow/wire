use std::{path::PathBuf, str::FromStr, time::Duration};

use anyhow::bail;
use clap::{Parser, Subcommand};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Time(pub Duration);

#[derive(Subcommand, Debug)]
pub enum Operations {
    #[command()]
    /// Push keys
    PushKeys {
        #[arg(short, long)]
        /// Total length for the protobuf message
        length: usize,
    },
    #[command()]
    /// Initiate magic rollback session
    Rollback {
        // Waiting period before transitioning to the timeout phase, where the
        // agent awaits a response from wire.
        //
        /// Grace period
        grace_period: Time,
        // The waiting period to receive a response from wire, if the time
        // exceeds the specified value, the agent will initiate a rollback.
        /// Waiting period before rolling back
        timeout: Time,
        /// Store path to known-working system closure, typically the
        /// current/previous one.
        known_working_closure: PathBuf,
    },
    #[command()]
    Dummy,
}

impl FromStr for Time {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let num: u64 = match s.parse() {
            Ok(v) => v,
            Err(e) => bail!(e),
        };

        Ok(Time(Duration::from_secs(num)))
    }
}

#[derive(Parser, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub operation: Operations,
}

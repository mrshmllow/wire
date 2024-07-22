use anyhow::anyhow;
use clap::{Parser, Subcommand, ValueEnum};
use clap_num::number_range;
use lib::hive::node::{NodeGoal, NodeName, SwitchToConfigurationGoal};

use std::{fmt::Display, sync::Arc};

#[derive(Parser)]
#[command(
    name = "wire",
    bin_name = "wire",
    about = "a tool to deploy nixos systems"
)]
pub struct WireCli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Path to directory containing hive
    #[arg(long, global = true, default_value = std::env::current_dir().unwrap().into_os_string())]
    pub path: std::path::PathBuf,
}

#[derive(Clone, Debug)]
pub enum ApplyTarget {
    Node(NodeName),
    Tag(String),
}

impl From<String> for ApplyTarget {
    fn from(value: String) -> Self {
        match value.starts_with("@") {
            true => ApplyTarget::Tag(value[1..].to_string()),
            false => ApplyTarget::Node(NodeName(Arc::from(value.as_str()))),
        }
    }
}

fn more_than_zero(s: &str) -> Result<usize, String> {
    number_range(s, 1, usize::MAX)
}

#[derive(Subcommand)]
pub enum Commands {
    Apply {
        #[arg(value_enum, default_value_t)]
        goal: Goal,

        /// List of literal node names or `@` prefixed tags.
        #[arg(short, long)]
        on: Vec<ApplyTarget>,

        #[arg(short, long, default_value_t = 10, value_parser=more_than_zero)]
        parallel: usize,
    },
    /// Inspect hive
    Inspect {
        /// Include liveliness
        #[arg(short, long, default_value_t = false)]
        online: bool,

        /// Return in JSON format
        #[arg(short, long, default_value_t = false)]
        json: bool,
    },
    /// Inspect log of builds
    Log {
        /// Host identifier
        #[arg()]
        host: String,
        /// Reverse-index of log. 0 is the latest
        #[arg(default_value_t = 0)]
        index: i32,
    },
}

#[derive(Clone, Debug, Default, ValueEnum, Display)]
pub enum Goal {
    /// Make the configuration the boot default and activate now
    #[default]
    Switch,
    /// Make the configuration the boot default
    Build,
    /// Copy closures to remote hosts
    Push,
    /// Push deployment keys to remote hosts
    Keys,
    /// Activate system profile on next boot
    Boot,
    /// Activate the configuration, but don't make it the boot default
    Test,
    /// Show what would be done if this configuration were activated.
    DryActivate,
}

impl TryFrom<Goal> for NodeGoal {
    type Error = anyhow::Error;

    fn try_from(value: Goal) -> Result<Self, Self::Error> {
        match value {
            Goal::Build => Ok(NodeGoal::Build),
            Goal::Push => Ok(NodeGoal::Push),
            Goal::Boot => Ok(NodeGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::Boot,
            )),
            Goal::Switch => Ok(NodeGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::Switch,
            )),
            Goal::Test => Ok(NodeGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::Test,
            )),
            Goal::DryActivate => Ok(NodeGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::DryActivate,
            )),
            Goal::Keys => Err(anyhow!("Keys is not a node goal")),
        }
    }
}

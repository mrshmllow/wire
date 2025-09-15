use clap::crate_version;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use clap_num::number_range;
use clap_verbosity_flag::WarnLevel;
use lib::SubCommandModifiers;
use lib::hive::Hive;
use lib::hive::node::{Goal as HiveGoal, Name, SwitchToConfigurationGoal};

use std::io::IsTerminal;
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

#[allow(clippy::struct_excessive_bools)]
#[derive(Parser)]
#[command(
    name = "wire",
    bin_name = "wire",
    about = "a tool to deploy nixos systems",
    version = format!("{}\nDebug: Hive::SCHEMA_VERSION {}", crate_version!(), Hive::SCHEMA_VERSION)
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<WarnLevel>,

    /// Path to directory containing hive
    #[arg(long, global = true, default_value = std::env::current_dir().unwrap().into_os_string())]
    pub path: std::path::PathBuf,

    // Unused until a solution to tracing-indicatif log deadlocking is found...
    /// Hide progress bars. Defaults to true if stdin does not refer to a tty (unix pipelines, in CI).
    #[arg(long, global = true, default_value_t = !std::io::stdin().is_terminal())]
    pub no_progress: bool,

    /// Never accept user input.
    /// Defaults to true if stdin does not refer to a tty (unix pipelines, in CI).
    #[arg(long, global = true, default_value_t = !std::io::stdin().is_terminal())]
    pub non_interactive: bool,

    /// Show trace logs
    #[arg(long, global = true, default_value_t = false)]
    pub show_trace: bool,

    #[arg(long, hide = true, global = true)]
    pub markdown_help: bool,
}

#[derive(Clone, Debug)]
pub enum ApplyTarget {
    Node(Name),
    Tag(String),
}

impl From<String> for ApplyTarget {
    fn from(value: String) -> Self {
        if let Some(stripped) = value.strip_prefix("@") {
            ApplyTarget::Tag(stripped.to_string())
        } else {
            ApplyTarget::Node(Name(Arc::from(value.as_str())))
        }
    }
}

impl Display for ApplyTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ApplyTarget::Node(name) => name.fmt(f),
            ApplyTarget::Tag(tag) => write!(f, "@{tag}"),
        }
    }
}

fn more_than_zero(s: &str) -> Result<usize, String> {
    number_range(s, 1, usize::MAX)
}

#[derive(Args)]
pub struct ApplyArgs {
    #[arg(value_enum, default_value_t)]
    pub goal: Goal,

    /// List of literal node names or `@` prefixed tags.
    #[arg(short, long, value_name = "NODE | @TAG", num_args = 1..)]
    pub on: Vec<ApplyTarget>,

    #[arg(short, long, default_value_t = 10, value_parser=more_than_zero)]
    pub parallel: usize,

    /// Skip key uploads. noop when [GOAL] = Keys
    #[arg(short, long, default_value_t = false)]
    pub no_keys: bool,

    /// Overrides deployment.buildOnTarget.
    #[arg(short, long, value_name = "NODE")]
    pub always_build_local: Vec<String>,

    /// Reboot the nodes after activation
    #[arg(short, long, default_value_t = false)]
    pub reboot: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Deploy nodes
    Apply(ApplyArgs),
    /// Inspect hive
    #[clap(visible_alias = "show")]
    Inspect {
        /// Include liveliness
        #[arg(short, long, default_value_t = false)]
        online: bool,

        /// Return in JSON format
        #[arg(short, long, default_value_t = false)]
        json: bool,
    },
    /// Generates shell completions
    #[clap(hide = true)]
    Completions {
        #[arg()]
        // Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Clone, Debug, Default, ValueEnum, Display)]
pub enum Goal {
    /// Make the configuration the boot default and activate now
    #[default]
    Switch,
    /// Build the configuration but do nothing with it
    Build,
    /// Copy system derivation to remote hosts
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

impl TryFrom<Goal> for HiveGoal {
    type Error = miette::Error;

    fn try_from(value: Goal) -> Result<Self, Self::Error> {
        match value {
            Goal::Build => Ok(HiveGoal::Build),
            Goal::Push => Ok(HiveGoal::Push),
            Goal::Boot => Ok(HiveGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::Boot,
            )),
            Goal::Switch => Ok(HiveGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::Switch,
            )),
            Goal::Test => Ok(HiveGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::Test,
            )),
            Goal::DryActivate => Ok(HiveGoal::SwitchToConfiguration(
                SwitchToConfigurationGoal::DryActivate,
            )),
            Goal::Keys => Ok(HiveGoal::Keys),
        }
    }
}

pub trait ToSubCommandModifiers {
    fn to_subcommand_modifiers(&self) -> SubCommandModifiers;
}

impl ToSubCommandModifiers for Cli {
    fn to_subcommand_modifiers(&self) -> SubCommandModifiers {
        SubCommandModifiers {
            show_trace: self.show_trace,
            non_interactive: self.non_interactive,
        }
    }
}

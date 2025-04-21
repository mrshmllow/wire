#![deny(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]
use std::io::BufRead;
use std::os::fd::AsRawFd;

use crate::cli::Cli;
use crate::cli::ToSubCommandModifiers;
use anyhow::bail;
use anyhow::Ok;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use indicatif::style::ProgressStyle;
use lib::hive::Hive;
use nix::fcntl::fcntl;
use nix::fcntl::FcntlArg::F_GETFL;
use nix::fcntl::FcntlArg::F_SETFL;
use nix::fcntl::OFlag;
use nix::libc::O_NONBLOCK;
use tracing::warn;
use tracing_indicatif::IndicatifLayer;
use tracing_log::AsTrace;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

#[macro_use]
extern crate enum_display_derive;

mod apply;
mod cli;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let args = Cli::parse();

    let modifiers = args.to_subcommand_modifiers();
    setup_logging(args.no_progress, &args.verbose);

    if args.markdown_help {
        clap_markdown::print_help_markdown::<Cli>();
        return Ok(());
    }

    match args.command {
        cli::Commands::Apply {
            goal,
            on,
            parallel,
            no_keys,
            always_build_local,
        } => {
            let stdin = std::io::stdin();

            // set fd to nonblocking so it won't hang the program for piping
            let fdflags = fcntl(stdin.as_raw_fd(), F_GETFL)?;
            let newflags = OFlag::from_bits_retain(fdflags | O_NONBLOCK);
            fcntl(stdin.as_raw_fd(), F_SETFL(newflags))?;

            let mut handle = stdin.lock();

            // I am proud of this monstrosity
            let nodes = {
                let mut buf: Vec<u8> = Vec::new();
                let mut c: u8 = 0;
                loop {
                    match handle.read_until(b'\n', &mut buf) {
                        Result::Ok(0) => break,
                        Result::Ok(_) => {
                            break;
                        }
                        Err(e) => match e.raw_os_error() {
                            Some(11) => {
                                // Resource temporarily unavailable is a normal
                                // error response when reading from stdin
                                // nonblocking
                                //
                                // as for if there should be a counter for this,
                                // this is undecided.
                                if c >= 2 {
                                    break;
                                }
                                c += 1;
                            }
                            Some(code) => {
                                bail!("unhandled stdin error: {code} {e}");
                            }
                            None => bail!(e),
                        },
                    }
                }
                String::from_utf8(buf)
            };
            println!("\nnodes: {nodes:?}\n");
            let mut hive = Hive::new_from_path(args.path.as_path(), modifiers).await?;

            apply::apply(
                &mut hive,
                goal.try_into()?,
                on,
                parallel,
                no_keys,
                always_build_local,
                modifiers,
            )
            .await?;
        }
        cli::Commands::Inspect { online: _, json } => println!("{}", {
            let hive = Hive::new_from_path(args.path.as_path(), modifiers).await?;
            if json {
                serde_json::to_string_pretty(&hive)?
            } else {
                warn!("use --json to output something scripting suitable");
                format!("{hive:#?}")
            }
        }),
        cli::Commands::Log { .. } => {
            todo!()
        }
        cli::Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.clone();
            generate(shell, &mut cmd, name.get_name(), &mut std::io::stdout());
        }
    }

    Ok(())
}

pub fn setup_logging(no_progress: bool, verbosity: &Verbosity<WarnLevel>) {
    let layer = tracing_subscriber::fmt::layer::<Registry>().without_time();
    let filter = verbosity.log_level_filter().as_trace();
    let registry = tracing_subscriber::registry();

    if no_progress {
        let layer = layer.with_filter(filter);

        registry.with(layer).init();
    } else {
        let indicatif_layer = IndicatifLayer::new().with_progress_style(
            ProgressStyle::with_template(
                "{span_child_prefix}[{spinner}] {span_name}{{{span_fields}}} {wide_msg}",
            )
            .expect("Failed to create progress style"),
        );

        let layer = layer
            .with_writer(indicatif_layer.get_stderr_writer())
            .with_filter(filter);

        registry.with(layer).with(indicatif_layer).init();
    }
}

#![deny(clippy::pedantic)]
#![allow(clippy::missing_panics_doc)]
use crate::cli::Cli;
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use indicatif::style::ProgressStyle;
use lib::hive::Hive;
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
    setup_logging(args.no_progress, &args.verbose);

    if args.markdown_help {
        clap_markdown::print_help_markdown::<Cli>();
        return Ok(());
    }

    let hive = Hive::new_from_path(args.path.as_path()).await?;

    match args.command {
        cli::Commands::Apply {
            goal,
            on,
            parallel,
            no_keys,
        } => apply::apply(hive, goal.try_into()?, on, parallel, no_keys).await?,
        cli::Commands::Inspect { online: _, json } => println!(
            "{}",
            if json {
                serde_json::to_string_pretty(&hive)?
            } else {
                format!("{hive:#?}")
            }
        ),
        cli::Commands::Log { .. } => {
            todo!()
        }
    };

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

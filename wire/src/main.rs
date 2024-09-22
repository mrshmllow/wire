use crate::cli::WireCli;
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use indicatif::style::ProgressStyle;
use lib::hive::{Hive, HiveBuilder};
use tracing_indicatif::IndicatifLayer;
use tracing_log::AsTrace;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

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

    let args = WireCli::parse();
    setup_logging(args.verbose);

    if args.markdown_help {
        clap_markdown::print_help_markdown::<WireCli>();
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
            match json {
                true => serde_json::to_string_pretty(&hive)?,
                false => format!("{hive:#?}"),
            }
        ),
        _ => {
            todo!()
        }
    };

    Ok(())
}

pub fn setup_logging(verbosity: Verbosity<WarnLevel>) {
    let indicatif_layer = IndicatifLayer::new().with_progress_style(
        ProgressStyle::with_template(
            "{span_child_prefix}[{spinner}] {span_name}{{{span_fields}}} {wide_msg}",
        )
        .expect("Failed to create progress style"),
    );

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_writer(indicatif_layer.get_stderr_writer())
                .with_filter(verbosity.log_level_filter().as_trace()),
        )
        .with(indicatif_layer)
        .init();
}

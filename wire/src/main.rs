use crate::cli::WireCli;
use clap::Parser;
use clap_verbosity_flag::{ErrorLevel, Verbosity};
use lib::Hive;
use lib::HiveBuilder;
use tracing_indicatif::IndicatifLayer;
use tracing_log::AsTrace;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod apply;
mod cli;
mod inspect;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = WireCli::parse();
    setup_logging(args.verbose);

    // let hive = Hive::new_from_path(args.path.as_path()).await?;

    match args.command {
        cli::Commands::Apply { goal, on } => apply::apply(goal, on, &args.path).await,
        // cli::Commands::Inspect { online, json } => inspect::inspect(hive, online, json).await?,
        _ => {
            todo!()
        }
    };

    Ok(())
}

pub fn setup_logging(verbosity: Verbosity<ErrorLevel>) {
    let indicatif_layer = IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().without_time().with_writer(
                indicatif_layer.get_stderr_writer().with_max_level(
                    verbosity
                        .log_level_filter()
                        .as_trace()
                        .into_level()
                        .unwrap(),
                ),
            ),
        )
        .with(indicatif_layer)
        .init();
}

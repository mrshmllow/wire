use crate::cli::Goal;
use indicatif::ProgressStyle;
use lib::hive::node::Evaluatable;
use lib::hive::Hive;
use tokio::task::JoinSet;
use tracing::{info, info_span, instrument};
use tracing_indicatif::span_ext::IndicatifSpanExt;

#[instrument(skip_all, fields(goal = %goal, on = ?on))]
pub async fn apply(hive: Hive, goal: Goal, on: Vec<String>) -> Result<(), anyhow::Error> {
    let header_span = info_span!("header");
    header_span.pb_set_style(&ProgressStyle::default_bar());
    header_span.pb_set_length(4);

    let header_span_enter = header_span.enter();

    let mut set = JoinSet::new();

    for node in hive.nodes {
        // cloning a header span doesnt make much sense
        set.spawn(node.build(hive.path.clone(), header_span.clone()));
    }

    while let Some(res) = set.join_next().await {
        let out = res??;

        header_span.pb_inc(1);

        info!("Built {}", out.0);
    }

    std::mem::drop(header_span_enter);
    std::mem::drop(header_span);

    Ok(())
}

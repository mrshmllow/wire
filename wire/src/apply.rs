use futures::{future::join_all, StreamExt};
use indicatif::ProgressStyle;
use lib::hive::node::Evaluatable;
use lib::hive::Hive;
use lib::HiveLibError;
use std::collections::HashSet;
use tracing::{instrument, warn, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::cli::{ApplyTarget, Goal};

#[instrument(skip_all, fields(goal = %goal, on = ?on))]
pub async fn apply(hive: Hive, goal: Goal, on: Vec<ApplyTarget>) -> Result<(), HiveLibError> {
    let header_span = Span::current();
    header_span.pb_set_style(&ProgressStyle::default_bar());
    header_span.pb_set_length(4);

    let header_span_enter = header_span.enter();

    let (tags, names) = on.iter().fold(
        (HashSet::new(), HashSet::new()),
        |(mut tags, mut names), target| {
            match target {
                ApplyTarget::Tag(tag) => tags.insert(tag.clone()),
                ApplyTarget::Node(name) => names.insert(name.clone()),
            };
            (tags, names)
        },
    );

    let mut set = hive
        .nodes
        .iter()
        .filter(|(name, node)| {
            on.is_empty() || names.contains(name) || node.tags.iter().any(|tag| tags.contains(tag))
        })
        .map(|node| {
            let path = hive.path.clone();
            let span = header_span.clone();

            node.build(path, span)
        })
        .peekable();

    if set.peek().is_none() {
        warn!("There are no nodes selected for deployment");
    }

    let result: Result<(), _> = join_all(set).await.into_iter().collect();

    std::mem::drop(header_span_enter);
    std::mem::drop(header_span);

    result
}

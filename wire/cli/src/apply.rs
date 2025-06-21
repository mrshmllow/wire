use anyhow::anyhow;
use futures::{FutureExt, StreamExt};
use indicatif::ProgressStyle;
use itertools::{Either, Itertools};
use lib::SubCommandModifiers;
use lib::hive::Hive;
use lib::hive::node::{Context, Goal, GoalExecutor, StepState};
use std::collections::HashSet;
use std::fmt::Write;
use tracing::{Span, error, info, instrument};
use tracing_indicatif::span_ext::IndicatifSpanExt;

use crate::cli::ApplyTarget;

#[instrument(skip_all, fields(goal = %goal, on = %on.iter().join(", ")))]
pub async fn apply(
    hive: &mut Hive,
    goal: Goal,
    on: Vec<ApplyTarget>,
    parallel: usize,
    no_keys: bool,
    always_build_local: Vec<String>,
    modifiers: SubCommandModifiers,
) -> Result<(), anyhow::Error> {
    let header_span = Span::current();
    header_span.pb_set_style(&ProgressStyle::default_bar());
    header_span.pb_set_length(1);

    // Respect user's --always-build-local arg
    hive.force_always_local(always_build_local)?;

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
        .iter_mut()
        .filter(|(name, node)| {
            on.is_empty() || names.contains(name) || node.tags.iter().any(|tag| tags.contains(tag))
        })
        .map(|node| {
            let path = hive.path.clone();
            let span = header_span.clone();

            info!("Resolved {on:?} to include {}", node.0);

            let context = Context {
                node: node.1,
                name: node.0,
                goal,
                state: StepState::default(),
                no_keys,
                hivepath: path,
                modifiers,
            };

            GoalExecutor::new(context)
                .execute(span)
                .map(move |result| (node.0, result))
        })
        .peekable();

    if set.peek().is_none() {
        error!("There are no nodes selected for deployment");
    }

    let futures = futures::stream::iter(set).buffer_unordered(parallel);
    let result = futures.collect::<Vec<_>>().await;
    let (successful, errors): (Vec<_>, Vec<_>) =
        result
            .into_iter()
            .partition_map(|(name, result)| match result {
                Ok(..) => Either::Left(name),
                Err(err) => Either::Right((name, err)),
            });

    if !successful.is_empty() {
        info!(
            "Successfully applied goal to {} node(s): {:?}",
            successful.len(),
            successful
        );
    }

    std::mem::drop(header_span_enter);
    std::mem::drop(header_span);

    if !errors.is_empty() {
        return Err(anyhow!(
            "{} node(s) failed to apply. {}",
            errors.len(),
            errors
                .iter()
                .fold(String::new(), |mut output, (name, error)| {
                    let _ = write!(output, "\n\n{name}: {error}");
                    output
                })
        ));
    }

    Ok(())
}

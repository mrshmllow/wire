use std::path::PathBuf;

use crate::cli::Goal;
use lib::eval_node;
use tracing::instrument;

#[instrument]
pub async fn apply(goal: Goal, on: Vec<String>, path: &PathBuf) {
    eval_node(path).await.unwrap();
}

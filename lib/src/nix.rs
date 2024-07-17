use std::{path::PathBuf, process::Stdio};
use thiserror::Error;
use tokio::process::Command;

pub enum EvalGoal<'a> {
    Inspect,
    GetTopLevel(&'a String),
}

pub fn get_eval_command(path: PathBuf, goal: EvalGoal) -> Command {
    let mut command = Command::new("nix");
    command.args(["eval", "--json", "--impure", "--verbose", "--expr"]);

    command.arg(format!(
        "let evaluate = import ./lib/src/evaluate.nix; hive = evaluate {{hivePath = {path};}}; in {goal}",
        path = path.to_str().unwrap(),
        goal = match goal {
            EvalGoal::Inspect => "hive.inspect".to_string(),
            EvalGoal::GetTopLevel(node) => format!("hive.getTopLevel \"{node}\"", node = node),
        }
    ));

    command
}

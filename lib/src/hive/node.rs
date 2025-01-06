#![allow(clippy::missing_errors_doc)]
use async_trait::async_trait;
use gethostname::gethostname;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tracing::{info_span, warn, Instrument};

use crate::nix::StreamTracing;
use crate::SubCommandModifiers;

use super::key::{Key, PushKeyAgentOutput, PushKeyAgentStep, UploadKeyAt, UploadKeyStep};
use super::steps::activate::SwitchToConfigurationStep;
use super::HiveLibError;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, derive_more::Display)]
pub struct Name(pub Arc<str>);

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Target {
    #[serde(rename = "host")]
    pub host: Arc<str>,

    #[serde(rename = "user")]
    pub user: Arc<str>,

    #[serde(rename = "port")]
    pub port: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Node {
    #[serde(rename = "target")]
    pub target: Target,

    #[serde(rename = "buildOnTarget")]
    pub build_remotely: bool,

    #[serde(rename = "allowLocalDeployment")]
    pub allow_local_deployment: bool,

    #[serde(default)]
    pub tags: im::HashSet<String>,

    #[serde(rename(deserialize = "_keys", serialize = "keys"))]
    pub keys: im::Vector<Key>,
}

pub fn should_apply_locally(allow_local_deployment: bool, name: &str) -> bool {
    *name == *gethostname() && allow_local_deployment
}

#[derive(derive_more::Display)]
pub enum Push<'a> {
    Derivation(&'a Derivation),
    Path(&'a String),
}

#[derive(Deserialize, Debug)]
pub struct Derivation(pub String);

impl Display for Derivation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f).and_then(|()| write!(f, "^*"))
    }
}

#[derive(derive_more::Display, Debug, Clone, Copy)]
pub enum SwitchToConfigurationGoal {
    Switch,
    Boot,
    Test,
    DryActivate,
}

#[derive(derive_more::Display, Clone, Copy)]
pub enum Goal {
    SwitchToConfiguration(SwitchToConfigurationGoal),
    Build,
    Push,
    Keys,
}

#[async_trait]
pub trait ExecuteStep: Send + Sync {
    async fn execute(&self, ctx: &mut Context<'_>) -> Result<(), HiveLibError>;

    fn should_execute(&self, context: &Context) -> bool;

    fn name(&self) -> &'static str;
}

pub enum StepOutput {
    Evaluation(super::steps::evaluate::Output),
    KeyAgentDirectory(PushKeyAgentOutput),
    BuildOutput(super::steps::build::Output),
}

#[derive(PartialEq, Eq, Hash)]
enum StepOutputKind {
    Evaluation,
    KeyAgentDirectory,
    BuildOutput,
}

#[derive(Default)]
pub struct StepState {
    values: HashMap<StepOutputKind, StepOutput>,
}

impl StepState {
    pub fn insert(&mut self, value: StepOutput) {
        self.values.insert(
            match value {
                StepOutput::Evaluation(_) => StepOutputKind::Evaluation,
                StepOutput::KeyAgentDirectory(_) => StepOutputKind::KeyAgentDirectory,
                StepOutput::BuildOutput(_) => StepOutputKind::BuildOutput,
            },
            value,
        );
    }

    fn get(&self, kind: &StepOutputKind) -> Option<&StepOutput> {
        self.values.get(kind)
    }

    pub fn get_evaluation(&self) -> Option<&super::steps::evaluate::Output> {
        match self.get(&StepOutputKind::Evaluation) {
            Some(StepOutput::Evaluation(evaluation)) => Some(evaluation),
            _ => None,
        }
    }

    pub fn get_build(&self) -> Option<&super::steps::build::Output> {
        match self.get(&StepOutputKind::BuildOutput) {
            Some(StepOutput::BuildOutput(value)) => Some(value),
            _ => None,
        }
    }

    pub fn get_key_agent_directory(&self) -> Option<&PushKeyAgentOutput> {
        match self.get(&StepOutputKind::KeyAgentDirectory) {
            Some(StepOutput::KeyAgentDirectory(value)) => Some(value),
            _ => None,
        }
    }
}

pub struct Context<'a> {
    pub name: &'a Name,
    pub node: &'a Node,
    pub hivepath: PathBuf,
    pub modifiers: SubCommandModifiers,
    pub no_keys: bool,
    pub state: StepState,
    pub goal: Goal,
}

pub struct GoalExecutor<'a> {
    steps: Vec<Box<dyn ExecuteStep>>,
    context: Context<'a>,
}

impl<'a> GoalExecutor<'a> {
    pub fn new(context: Context<'a>) -> Self {
        Self {
            steps: vec![
                Box::new(PushKeyAgentStep),
                Box::new(UploadKeyStep {
                    moment: UploadKeyAt::AnyOpportunity,
                }),
                Box::new(UploadKeyStep {
                    moment: UploadKeyAt::PreActivation,
                }),
                Box::new(super::steps::evaluate::Step),
                Box::new(super::steps::push::EvaluatedOutputStep),
                Box::new(super::steps::build::Step),
                Box::new(super::steps::push::BuildOutputStep),
                Box::new(SwitchToConfigurationStep),
                Box::new(UploadKeyStep {
                    moment: UploadKeyAt::PostActivation,
                }),
            ],
            context,
        }
    }

    pub async fn execute(mut self) -> Result<(), HiveLibError> {
        for step in self.steps {
            if step.should_execute(&self.context) {
                warn!("Executing step {}", step.name());
                step.execute(&mut self.context).await?;
            } else {
                warn!("Skipping step {}", step.name());
            }
        }

        Ok(())
    }
}

pub async fn push(node: &Node, name: &Name, push: Push<'_>) -> Result<(), HiveLibError> {
    let mut command = Command::new("nix");

    command
        .args(["--extra-experimental-features", "nix-command"])
        .arg("copy")
        .arg("--substitute-on-destination")
        .arg("--to")
        .arg(format!("ssh://{}@{}", node.target.user, node.target.host))
        .env("NIX_SSHOPTS", format!("-p {}", node.target.port));

    match push {
        Push::Derivation(drv) => command.args([drv.to_string(), "--derivation".to_string()]),
        Push::Path(path) => command.arg(path),
    };

    let (status, _stdout, stderr_vec) =
        command.execute(true).instrument(info_span!("copy")).await?;

    if !status.success() {
        return Err(HiveLibError::NixCopyError(name.clone(), stderr_vec));
    }

    Ok(())
}

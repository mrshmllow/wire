// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

#![feature(assert_matches)]
#![feature(iter_intersperse)]

use std::{io::IsTerminal, sync::LazyLock};

use tokio::sync::Semaphore;

use crate::{errors::HiveLibError, hive::node::Name};

pub mod commands;
pub mod hive;

#[cfg(test)]
mod test_macros;

#[cfg(test)]
mod test_support;

pub mod errors;

#[derive(Clone, Debug, Copy, Default)]
pub enum StrictHostKeyChecking {
    /// do not accept new host. dangerous!
    No,

    /// accept-new, default
    #[default]
    AcceptNew,
}

#[derive(Debug, Clone, Copy)]
pub struct SubCommandModifiers {
    pub show_trace: bool,
    pub non_interactive: bool,
    pub ssh_accept_host: StrictHostKeyChecking,
}

impl Default for SubCommandModifiers {
    fn default() -> Self {
        SubCommandModifiers {
            show_trace: false,
            non_interactive: !std::io::stdin().is_terminal(),
            ssh_accept_host: StrictHostKeyChecking::default(),
        }
    }
}

pub enum EvalGoal<'a> {
    Inspect,
    GetTopLevel(&'a Name),
}

pub static STDIN_CLOBBER_LOCK: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(1));

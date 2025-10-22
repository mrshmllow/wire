// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

#![deny(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::missing_panics_doc
)]
#![feature(assert_matches)]
#![feature(iter_intersperse)]

use std::{
    io::IsTerminal,
    sync::{Arc, LazyLock, Mutex},
};

use crate::{errors::HiveLibError, hive::node::Name};

pub mod commands;
pub mod hive;

#[cfg(test)]
mod test_macros;

#[cfg(test)]
mod test_support;

pub mod errors;

#[derive(Debug, Clone, Copy)]
pub struct SubCommandModifiers {
    pub show_trace: bool,
    pub non_interactive: bool,
    pub ssh_accept_host: bool,
}

impl Default for SubCommandModifiers {
    fn default() -> Self {
        SubCommandModifiers {
            show_trace: false,
            non_interactive: !std::io::stdin().is_terminal(),
            ssh_accept_host: false,
        }
    }
}

pub enum EvalGoal<'a> {
    Inspect,
    GetTopLevel(&'a Name),
}

pub static STDIN_CLOBBER_LOCK: LazyLock<Arc<Mutex<()>>> =
    LazyLock::new(|| Arc::new(Mutex::new(())));

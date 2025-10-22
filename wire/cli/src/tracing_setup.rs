// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::{
    collections::VecDeque,
    io::{self, Stderr, Write, stderr},
    sync::TryLockError,
};

use clap_verbosity_flag::{Verbosity, WarnLevel};
use lib::STDIN_CLOBBER_LOCK;
use tracing_log::AsTrace;
use tracing_subscriber::{Layer, Registry, layer::SubscriberExt, util::SubscriberInitExt};

struct NonClobberingWriter {
    queue: VecDeque<Vec<u8>>,
    stderr: Stderr,
}

impl NonClobberingWriter {
    fn new() -> Self {
        NonClobberingWriter {
            queue: VecDeque::with_capacity(100),
            stderr: stderr(),
        }
    }

    fn dump_previous(&mut self) -> Result<(), io::Error> {
        for buf in self.queue.iter().rev() {
            self.stderr.write(buf).map(|_| ())?;
        }

        self.stderr.flush()?;

        Ok(())
    }
}

impl Write for NonClobberingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match STDIN_CLOBBER_LOCK.clone().try_lock() {
            Ok(_) => {
                self.dump_previous().map(|()| 0)?;

                self.stderr.write(buf)
            }
            Err(e) => match e {
                TryLockError::Poisoned(_) => {
                    panic!("Internal stdout clobber lock is posioned. Please create an issue.");
                }
                TryLockError::WouldBlock => {
                    self.queue.push_front(buf.to_vec());

                    Ok(buf.len())
                }
            },
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stderr.flush()
    }
}

pub fn setup_logging(verbosity: Verbosity<WarnLevel>) {
    let filter = verbosity.log_level_filter().as_trace();
    let registry = tracing_subscriber::registry();

    let layer = tracing_subscriber::fmt::layer::<Registry>()
        .without_time()
        .with_target(false)
        .with_writer(NonClobberingWriter::new)
        .with_filter(filter);

    registry.with(layer).init();
}

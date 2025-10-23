// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright 2024-2025 wire Contributors

use std::{
    collections::VecDeque,
    io::{self, Stderr, Write, stderr},
    sync::TryLockError,
};

use clap_verbosity_flag::{Verbosity, WarnLevel};
use lib::STDIN_CLOBBER_LOCK;
use owo_colors::{OwoColorize, Stream};
use tracing::{Subscriber};
use tracing_log::AsTrace;
use tracing_subscriber::{
    Layer, field::{RecordFields, VisitFmt}, fmt::{
        FormatEvent, FormatFields, FormattedFields, format::{self, DefaultFields, DefaultVisitor, Format, Full}
    }, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt
};

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

struct WireEventFormat(Format<Full, ()>);
struct WireFieldFormat;
struct WireFieldVisitor<'a>(DefaultVisitor<'a>);

impl<'a> WireFieldVisitor<'a> {
    fn new(writer: format::Writer<'a>, is_empty: bool) -> Self {
        Self(DefaultVisitor::new(writer, is_empty))
    }
}

impl<'writer> FormatFields<'writer> for WireFieldFormat {
    fn format_fields<R: RecordFields>(&self, writer: format::Writer<'writer>, fields: R) -> std::fmt::Result {
        let mut v = WireFieldVisitor::new(writer, true);
        fields.record(&mut v);
        // v.finish()

        Ok(())
    }
}

impl tracing::field::Visit for WireFieldVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "node" => {
                let _ = write!(self.0.writer(), "{:?}", value.if_supports_color(Stream::Stderr, |text| text.bold()));
            },
            _ => return,
        }
    }
}

impl<S, N> FormatEvent<S, N> for WireEventFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();

        // if !matches!(metadata.level(), &tracing::Level::INFO) {
        //     return self.0.format_event(ctx, writer, event);
        // }

        let Some(scope) = ctx.event_scope() else {
            return self.0.format_event(ctx, writer, event);
        };

        let Some(parent) = scope.last() else {
            return self.0.format_event(ctx, writer, event);
        };

        if parent.name() != "execute" {
            return self.0.format_event(ctx, writer, event);
        }

        let Some(node_name) = parent.fields().field("node") else {
            return self.0.format_event(ctx, writer, event);
        };

        let format = WireFieldFormat;

        let ext = parent.extensions();
        let fields = &ext
            .get::<FormattedFields<WireFieldFormat>>()
            .expect("will never be `None`");

        write!(writer, "{fields}")?;

        writeln!(writer)?;

        Ok(())
    }
}

pub fn setup_logging(verbosity: Verbosity<WarnLevel>) {
    let filter = verbosity.log_level_filter().as_trace();
    let registry = tracing_subscriber::registry();

    let event_formatter = WireEventFormat(format::format().without_time().with_target(false));

    let layer = tracing_subscriber::fmt::layer()
        .fmt_fields(WireFieldFormat)
        .event_format(event_formatter)
        .with_writer(NonClobberingWriter::new)
        .with_filter(filter);

    registry.with(layer).init();
}

// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Global logger definition and functions

use crate::{counters::STRUCT_LOG_COUNT, error, Event, Metadata};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tracing_subscriber::prelude::*;

/// The global `Logger`
static LOGGER: OnceCell<Arc<dyn Logger>> = OnceCell::new();

/// A trait encapsulating the operations required of a logger.
pub trait Logger: Sync + Send + 'static {
    /// Determines if an event with the specified metadata would be logged
    fn enabled(&self, metadata: &Metadata) -> bool;

    /// Record an event
    fn record(&self, event: &Event);

    /// Flush any buffered events
    fn flush(&self);
}

/// Record a logging event to the global `Logger`
pub(crate) fn dispatch(event: &Event) {
    if let Some(logger) = LOGGER.get() {
        STRUCT_LOG_COUNT.inc();
        logger.record(event)
    }
}

/// Check if the global `Logger` is enabled
pub(crate) fn enabled(metadata: &Metadata) -> bool {
    LOGGER
        .get()
        .map(|logger| logger.enabled(metadata))
        .unwrap_or(false)
}

/// Sets the global `Logger` exactly once
pub fn set_global_logger(logger: Arc<dyn Logger>, tokio_console_port: Option<u16>) {
    if LOGGER.set(logger).is_err() {
        eprintln!("Global logger has already been set");
        error!("Global logger has already been set");
        return;
    }

    // If tokio-console is enabled, all tracing::log events are captured by the
    // tokio-tracing infrastructure. Otherwise, velor-logger intercepts all
    // tracing::log events. In both scenarios *all* velor-logger::log events are
    // captured by the velor-logger (as usual).
    #[cfg(feature = "tokio-console")]
    {
        if let Some(tokio_console_port) = tokio_console_port {
            let console_layer = console_subscriber::ConsoleLayer::builder()
                .server_addr(([0, 0, 0, 0], tokio_console_port))
                .spawn();

            tracing_subscriber::registry().with(console_layer).init();
            return;
        }
    }
    if tokio_console_port.is_none() {
        let _ = tracing::subscriber::set_global_default(
            crate::tracing_adapter::TracingToVelorDataLayer
                .with_subscriber(tracing_subscriber::Registry::default()),
        );
    } else {
        error!("tokio_console_port was set but has no effect! Build the crate with the 'tokio-console' feature enabled!");
    }
}

/// Flush the global `Logger`. Note this is expensive, only use off the critical path.
pub fn flush() {
    if let Some(logger) = LOGGER.get() {
        logger.flush();
    }
}

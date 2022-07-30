// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Global logger definition and functions

use crate::{counters::STRUCT_LOG_COUNT, error, Event, Metadata};

use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing_subscriber::prelude::*;

use rand::distributions::{Alphanumeric, DistString};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::RollingFileAppender;
use tracing_flame::{FlameLayer, FlushGuard};

/// The global `Logger`
static LOGGER: OnceCell<Arc<dyn Logger>> = OnceCell::new();
static GUARDS: OnceCell<Arc<(WorkerGuard, FlushGuard<RollingFileAppender>)>> = OnceCell::new();

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
pub fn set_global_logger(logger: Arc<dyn Logger>, console_port: Option<u16>) {
    if LOGGER.set(logger).is_err() {
        eprintln!("Global logger has already been set");
        error!("Global logger has already been set");
        return;
    }

    /*
     * if console_port is set all tracing::log are captured by the tokio-tracing infrastructure.
     * else aptos-logger intercepts all tracing::log events
     * In both scenarios *ALL* aptos-logger::log events are captured by aptos-logger as usual.
     */
    #[cfg(feature = "aptos-console")]
    {
        let string = Alphanumeric.sample_string(&mut rand::thread_rng(), 8);
        println!("My Rand: {}", string);

        if let Some(p) = console_port {
            let tracing_path = "/tmp/tracing/";

            let console_layer = console_subscriber::ConsoleLayer::builder()
                .server_addr(([0, 0, 0, 0], p))
                .spawn();

            if p == 6669 {
                if std::fs::create_dir_all(tracing_path).is_err() {
                    println!("{tracing_path} cant be created (probably already exists)");
                }
                thread::spawn(move || {
                    let mut tracing_paths = std::fs::read_dir(tracing_path).unwrap();

                    loop {
                        for file in tracing_paths {
                            std::fs::remove_file(file.unwrap().path()).unwrap_or_default();
                        }
                        tracing_paths = std::fs::read_dir(tracing_path).unwrap();
                        thread::sleep(Duration::from_secs(300));
                    }
                });
            }

            let file_appender =
                tracing_appender::rolling::minutely(tracing_path, format!("{}{}", string, ".log"));
            let file_appender2 =
                tracing_appender::rolling::minutely(tracing_path, format!("{}.folded", string));
            let (non_blocking, appender_guard) = tracing_appender::non_blocking(file_appender);
            let mut flame_layer = FlameLayer::new(file_appender2); //.unwrap();
            let flame_guard = flame_layer.flush_on_drop();
            GUARDS
                .set(Arc::new((appender_guard, flame_guard)))
                .expect("This shouldn't Fail");

            flame_layer = flame_layer
                .with_threads_collapsed(true)
                .with_file_and_line(false);

            tracing_subscriber::registry()
                .with(console_layer)
                .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
                .with(flame_layer)
                .init();

            return;
        }
    }
    if None == console_port {
        let _ = tracing::subscriber::set_global_default(
            crate::tracing_adapter::TracingToAptosDataLayer
                .with_subscriber(tracing_subscriber::Registry::default()),
        );
    } else {
        error!("console_port was set but has no effect, build with --cfg aptos-console");
    }
}

/// Flush the global `Logger`
pub fn flush() {
    if let Some(logger) = LOGGER.get() {
        logger.flush();
    }
}

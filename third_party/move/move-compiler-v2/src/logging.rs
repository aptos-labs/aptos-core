// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Configures the logging backend for the compiler and related code which uses
//! the `log` crate (macros `info!`, `error!`, etc.).
//!
//! # How to Run the Logger
//!
//! By default logging is turned off. The environment variable `MVC_LOG` is
//! used to control logging (MVC stands for "Move Compiler"). Usages:
//!
//! ```ignore
//!     MVC_LOG = ""                # logs everything to console (stderr)
//!     MVC_LOG = "info"            # logs only info or higher to console
//!     MVC_LOG = "<module_path_prefix>=info" # logs only info+ in matching modules
//!     MVC_LOG = "@<file>"         # logs everything to given file
//!     MVC_LOG = "info@<file>"     # as above
//! ```
//!
//! A module path prefix must consist of crate name and module name, as in
//! `move_stackless_bytecode::function_target`, which matches any full module name
//! with this prefix. The general format is `<spec>[@<file>]`, where `spec` is defined
//! as described for the `LogSpecification` type.
//!
//! # How to write Logs
//!
//! One can import and use the following macros in decreasing severity: `error!`, `warn!`, `info!`,
//!`debug!`, and `trace!`. Also any other code in external crates already using `log` will
//! use those macros. Invocations of those macros will be redirected to the logger configured
//! via the `MVC_LOG` environment variable.
//!
//! Those macros expand to a conditional, and it is a quick check and branch to skip
//! logging if the level is not enabled (which is the default). In addition, one can
//! use the boolean macro `log_enabled!(log::Level::Debug)` to check whether a given
//! level is enabled.
//!
//! In general it is good to keep INFO and higher severity log messages in the code. They
//! may help to debug customer issues in production. However, for keeping the code base clean,
//! uses of the `debug!` and lower logging level should be kept minimal and possibly removed
//! eliminated. Its a judgement call where and for how long to leave them in code.

use flexi_logger::{DeferredNow, FileSpec, LogSpecification, Logger};
use log::Record;
use std::env;

const MVC_LOG_VAR: &str = "MVC_LOG";

/// Configures logging for applications.
pub fn setup_logging() {
    // Currently no different to testing
    setup_logging_for_testing()
}

/// Configures logging for testing. Can be called multiple times.
pub fn setup_logging_for_testing() {
    if let Some(logger) = configure_logger() {
        // Will produce error if a logger is already installed, which we ignore. Its either this same logger
        // already installed, or some outer code overriding the logger.
        let _ = logger.start();
    }
}

fn configure_logger() -> Option<Logger> {
    let var = env::var(MVC_LOG_VAR).ok()?;
    let mut parts = var.rsplitn(2, '@').collect::<Vec<_>>();
    parts.reverse();
    let spec = if parts[0].trim().is_empty() {
        // Show everything
        LogSpecification::trace()
    } else {
        LogSpecification::parse(parts[0]).expect("log spec")
    };
    let mut logger = Logger::with(spec).format(format_record);
    if parts.len() > 1 {
        let fname = if !parts[1].contains('/') {
            // Flex logger somehow does not like relative file names, help them.
            format!("./{}", parts[1])
        } else {
            parts[1].to_string()
        };
        logger = logger.log_to_file(FileSpec::try_from(fname).expect("file name"))
    }
    Some(logger)
}

fn format_record(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{} {}] {}",
        record.level(),
        record.module_path().unwrap_or_default(),
        &record.args()
    )
}

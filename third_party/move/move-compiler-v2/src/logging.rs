// Copyright © Velor Foundation
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
//! General practice is to use, besides `warn!`, `info!` for progress messages,
//! and `debug!` for helping to debug problems with ready code. `debug!` can also be used
//! during development of a module, but should then be made conditional as in:
//!
//! ```ignore
//! const DEBUG: bool = false;
//! ...
//! if DEBUG { debug!(..) }
//! ```

use colored::Colorize;
use flexi_logger::{DeferredNow, FileSpec, LogSpecification, Logger};
use log::{Level, LevelFilter, Record};
use std::env;

const MVC_LOG_VAR: &str = "MVC_LOG";

/// Configures logging for applications.
pub fn setup_logging(level: Option<LevelFilter>) {
    // Currently no different to testing
    setup_logging_for_testing(level)
}

/// Configures logging for testing. Can be called multiple times.
pub fn setup_logging_for_testing(level: Option<LevelFilter>) {
    if let Some(logger) = configure_logger(level) {
        // Will produce error if a logger is already installed, which we ignore. Its either this same logger
        // already installed, or some outer code overriding the logger.
        let _ = logger.start();
    }
}

#[allow(clippy::unnecessary_unwrap)]
fn configure_logger(level: Option<LevelFilter>) -> Option<Logger> {
    let var = env::var(MVC_LOG_VAR);
    let (spec, fname) = if var.is_err() && level.is_some() {
        (
            match level.unwrap() {
                LevelFilter::Error => LogSpecification::error(),
                LevelFilter::Warn => LogSpecification::warn(),
                LevelFilter::Info => LogSpecification::info(),
                LevelFilter::Debug => LogSpecification::debug(),
                LevelFilter::Trace => LogSpecification::trace(),
                LevelFilter::Off => LogSpecification::off(),
            },
            None,
        )
    } else {
        let var = var.ok()?;
        let mut parts = var.rsplitn(2, '@').collect::<Vec<_>>();
        parts.reverse();
        let spec = if parts[0].trim().is_empty() {
            // Show everything
            LogSpecification::trace()
        } else {
            LogSpecification::parse(parts[0]).expect("log spec")
        };
        if parts.len() > 1 {
            if !parts[1].contains('/') {
                // Flex logger somehow does not like relative file names, help them.
                (spec, Some(format!("./{}", parts[1])))
            } else {
                (spec, Some(parts[1].to_string()))
            }
        } else {
            (spec, None)
        }
    };
    Some(
        if let Some(fname) = fname {
            Logger::with(spec)
                .format(format_record_file)
                .log_to_file(FileSpec::try_from(fname).expect("file name"))
        } else {
            Logger::with(spec).format(format_record_colored)
        },
    )
}

fn format_record_file(
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

fn format_record_colored(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level_color = match record.level() {
        Level::Error => "ERROR".red().bold(),
        Level::Warn => "WARN".yellow().bold(),
        Level::Info => "INFO".green().bold(),
        Level::Debug => "DEBUG".cyan().bold(),
        Level::Trace => "TRACE".normal(),
    };
    write!(w, "[{}] {}", level_color, &record.args())
}

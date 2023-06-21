// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod experiments;
mod options;

use anyhow::anyhow;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, WriteColor};
pub use experiments::*;
use move_model::{model::GlobalEnv, PackageInfo};
pub use options::*;

/// Run Move compiler and print errors to stderr.
pub fn run_move_compiler_to_stderr(options: Options) -> anyhow::Result<()> {
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    run_move_compiler(&mut error_writer, options)
}

/// Run move compiler and print errors to given writer.
pub fn run_move_compiler<W: WriteColor>(
    error_writer: &mut W,
    options: Options,
) -> anyhow::Result<()> {
    // Run the model builder, which performs context checking.
    let addrs = move_model::parse_addresses_from_options(options.named_address_mapping.clone())?;
    let env = move_model::run_model_builder_in_compiler_mode(
        PackageInfo {
            sources: options.sources.clone(),
            address_map: addrs.clone(),
        },
        vec![PackageInfo {
            sources: options.dependencies.clone(),
            address_map: addrs,
        }],
    )?;
    // If the model contains any errors, report them now and exit.
    check_errors(
        &env,
        &options,
        error_writer,
        "exiting with Move build errors",
    )?;
    if options.experiment_on(Experiment::CHECK_ONLY) {
        // Stop here
        return Ok(());
    }
    panic!("code generation NYI")
}

pub fn check_errors<W: WriteColor>(
    env: &GlobalEnv,
    options: &Options,
    error_writer: &mut W,
    msg: &'static str,
) -> anyhow::Result<()> {
    env.report_diag(error_writer, options.report_severity());
    if env.has_errors() {
        Err(anyhow!(msg))
    } else {
        Ok(())
    }
}

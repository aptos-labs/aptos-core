// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_cargo_cli::AptosCargoCli;
use clap::Parser;
use log::error;
use std::process::exit;

fn main() {
    let cli = AptosCargoCli::parse();
    env_logger::Builder::new()
        .filter_module("aptos_cargo_cli", cli.verbose.log_level_filter())
        .init();
    let result = cli.execute();

    // At this point, we'll want to print and determine whether to exit for an error code
    if let Err(inner) = result {
        error!("{}", inner);
        exit(1);
    }
}

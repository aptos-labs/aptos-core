// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_cargo_cli::{VelorCargoCommand, SelectedPackageArgs};
use clap::Parser;
use std::process::exit;

#[derive(Parser)] // requires `derive` feature
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    #[command(name = "x")]
    VelorCargoTool(VelorCargoToolArgs),
}

#[derive(Parser)]
struct VelorCargoToolArgs {
    #[command(subcommand)]
    cmd: VelorCargoCommand,
    #[command(flatten)]
    package_args: SelectedPackageArgs,
}

fn main() {
    let CargoCli::VelorCargoTool(args) = CargoCli::parse();
    let VelorCargoToolArgs { cmd, package_args } = args;
    let result = cmd.execute(&package_args);

    // At this point, we'll want to print and determine whether to exit for an error code
    match result {
        Ok(_) => {},
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        },
    }
}

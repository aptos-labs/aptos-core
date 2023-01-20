// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cargo::{build_args::BuildArgs, selected_package::SelectedPackageArgs, CargoCommand},
    context::XContext,
    Result,
};
use clap::Parser;
use std::ffi::OsString;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(flatten)]
    pub(crate) package_args: SelectedPackageArgs,
    #[clap(flatten)]
    pub(crate) build_args: BuildArgs,
    #[clap(
        name = "ARGS",
        parse(from_os_str),
        last = true,
        takes_value(true),
        multiple_values(true),
        multiple_occurrences(true)
    )]
    args: Vec<OsString>,
}

pub fn run(mut args: Args, xctx: XContext) -> Result<()> {
    let mut pass_through_args = vec![];
    pass_through_args.extend(args.args);

    // Always run fix on all targets.
    args.build_args.all_targets = true;

    let mut direct_args = vec![];
    args.build_args.add_args(&mut direct_args);

    let cmd = CargoCommand::Fix {
        cargo_config: xctx.config().cargo_config(),
        direct_args: &direct_args,
        args: &pass_through_args,
    };
    let packages = args.package_args.to_selected_packages(&xctx)?;
    cmd.run_on_packages(&packages)
}

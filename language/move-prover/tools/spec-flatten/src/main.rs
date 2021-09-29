// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use structopt::StructOpt;

use spec_flatten::{run, FlattenOptions};

fn main() -> Result<()> {
    let options = FlattenOptions::from_args();
    run(&options)
}

// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "move-analyzer", about = "A language server for Move")]
struct Options {}

fn main() {
    Options::from_args();
    todo!()
}

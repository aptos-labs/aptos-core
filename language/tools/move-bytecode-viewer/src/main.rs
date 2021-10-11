// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use move_bytecode_viewer::BytecodeViewerConfig;
use structopt::StructOpt;

fn main() {
    BytecodeViewerConfig::from_args().start_viewer()
}
